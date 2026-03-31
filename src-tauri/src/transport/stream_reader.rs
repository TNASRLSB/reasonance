use crate::normalizer::pipeline::NormalizerPipeline;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::ChildStdout;
use tokio::sync::oneshot;
use tokio::time::{timeout, Duration};

#[allow(dead_code)] // Field events_count read in tests
pub struct StreamResult {
    pub events_count: u32,
    pub error: Option<String>,
}

pub fn spawn_stream_reader(
    stdout: ChildStdout,
    pipeline: Arc<Mutex<NormalizerPipeline>>,
    event_bus: Arc<crate::event_bus::EventBus>,
    session_id: String,
    session_id_path: Option<String>,
    cli_session_id: Arc<Mutex<Option<String>>>,
    line_timeout: Option<Duration>,
) -> oneshot::Receiver<StreamResult> {
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        let mut events_count: u32 = 0;
        let mut error: Option<String> = None;
        let effective_timeout = line_timeout.unwrap_or(Duration::from_secs(300)); // default 5 minutes

        loop {
            match timeout(effective_timeout, lines.next_line()).await {
                Ok(Ok(Some(raw_line))) => {
                    let line = raw_line.trim().to_string();
                    if line.is_empty() {
                        continue;
                    }

                    // Parse JSON once for both debug logging and session ID extraction
                    let parsed_json = serde_json::from_str::<serde_json::Value>(&line).ok();
                    let json_type = parsed_json.as_ref().and_then(|v| {
                        v.get("type")
                            .and_then(|t| t.as_str().map(|s| s.to_string()))
                    });
                    log::debug!(
                        "StreamReader[{}]: raw line type={:?} len={}",
                        session_id,
                        json_type,
                        line.len()
                    );

                    // Extract CLI session ID if configured and not yet captured
                    if let Some(ref sid_path) = session_id_path {
                        let already_captured = cli_session_id
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .is_some();
                        if !already_captured {
                            if let Some(ref parsed) = parsed_json {
                                if let Some(extracted) =
                                    crate::normalizer::rules_engine::resolve_path(parsed, sid_path)
                                {
                                    if let Some(id_str) = extracted.as_str() {
                                        log::info!(
                                            "StreamReader[{}]: captured CLI session ID: {}",
                                            session_id,
                                            id_str
                                        );
                                        *cli_session_id.lock().unwrap_or_else(|e| e.into_inner()) =
                                            Some(id_str.to_string());
                                    }
                                }
                            }
                        }
                    }

                    let events = {
                        let mut pl = pipeline.lock().unwrap_or_else(|e| e.into_inner());
                        pl.process(&line)
                    };

                    if events.is_empty() {
                        log::trace!(
                            "StreamReader[{}]: no events from line type={:?}",
                            session_id,
                            json_type
                        );
                    }
                    for event in &events {
                        events_count += 1;
                        log::debug!(
                            "StreamReader[{}]: emitting {:?}",
                            session_id,
                            event.event_type
                        );
                        event_bus.publish(crate::event_bus::Event::from_agent_event(
                            "transport:event",
                            &session_id,
                            event,
                        ));
                    }
                }
                Ok(Ok(None)) => {
                    // Emit a synthetic done event when the CLI process closes stdout
                    let done_event = crate::agent_event::AgentEvent::done(&session_id, "system");
                    events_count += 1;
                    event_bus.publish(crate::event_bus::Event::from_agent_event(
                        "transport:complete",
                        &session_id,
                        &done_event,
                    ));
                    break;
                }
                Ok(Err(e)) => {
                    let err_msg = format!("Stream read error: {}", e);
                    log::error!("StreamReader[{}]: {}", session_id, err_msg);

                    // Publish error event so frontend knows something went wrong
                    let error_event = crate::agent_event::AgentEvent::error(
                        &err_msg,
                        "STREAM_IO_ERROR",
                        crate::agent_event::ErrorSeverity::Fatal,
                        "system",
                    );
                    event_bus.publish(crate::event_bus::Event::from_agent_event(
                        "transport:error",
                        &session_id,
                        &error_event,
                    ));
                    events_count += 1;

                    // Publish done event so frontend clears streaming state
                    let done_event = crate::agent_event::AgentEvent::done(&session_id, "system");
                    event_bus.publish(crate::event_bus::Event::from_agent_event(
                        "transport:complete",
                        &session_id,
                        &done_event,
                    ));
                    events_count += 1;

                    error = Some(err_msg);
                    break;
                }
                Err(_elapsed) => {
                    // Timeout — CLI is hung
                    let err_msg = format!(
                        "Stream timeout: no output for {} seconds",
                        effective_timeout.as_secs()
                    );
                    log::error!("StreamReader[{}]: {}", session_id, err_msg);

                    let error_event = crate::agent_event::AgentEvent::error(
                        &err_msg,
                        "STREAM_TIMEOUT",
                        crate::agent_event::ErrorSeverity::Fatal,
                        "system",
                    );
                    event_bus.publish(crate::event_bus::Event::from_agent_event(
                        "transport:error",
                        &session_id,
                        &error_event,
                    ));
                    events_count += 1;

                    let done_event = crate::agent_event::AgentEvent::done(&session_id, "system");
                    event_bus.publish(crate::event_bus::Event::from_agent_event(
                        "transport:complete",
                        &session_id,
                        &done_event,
                    ));
                    events_count += 1;

                    error = Some(err_msg);
                    break;
                }
            }
        }

        let _ = tx.send(StreamResult {
            events_count,
            error,
        });
    });

    rx
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalizer::NormalizerRegistry;
    use crate::subscribers::history::HistoryRecorder;
    use std::process::Stdio;
    use tokio::process::Command;

    /// Helper: create a v2 EventBus with a HistoryRecorder wired to transport channels.
    fn make_bus_with_recorder() -> (Arc<crate::event_bus::EventBus>, Arc<HistoryRecorder>) {
        let bus = Arc::new(crate::event_bus::EventBus::new(
            tokio::runtime::Handle::current(),
        ));
        bus.register_channel("transport:event", true);
        bus.register_channel("transport:complete", true);
        bus.register_channel("transport:error", true);

        let recorder = Arc::new(HistoryRecorder::new());
        bus.subscribe("transport:event", recorder.clone());
        bus.subscribe("transport:complete", recorder.clone());
        bus.subscribe("transport:error", recorder.clone());

        (bus, recorder)
    }

    #[tokio::test]
    async fn test_stream_reader_with_echo() {
        // Use Claude CLI stream-json format
        let json1 = r#"{"type":"assistant","message":{"id":"msg_1","model":"claude-sonnet-4-6","content":[{"type":"text","text":"Hello"}]}}"#;
        let json2 = r#"{"type":"result","subtype":"success","duration_ms":100,"usage":{"input_tokens":5,"output_tokens":2}}"#;

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(format!("echo '{}'; echo '{}'", json1, json2))
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn echo");

        let stdout = child.stdout.take().unwrap();

        let registry =
            NormalizerRegistry::load_from_dir(std::path::Path::new("normalizers")).unwrap();

        let config = registry.get_config("claude").unwrap();
        let rules = config.to_rules();
        let state_machine =
            Box::new(crate::normalizer::state_machines::generic::GenericStateMachine::new());
        let pipeline = Arc::new(Mutex::new(
            crate::normalizer::pipeline::NormalizerPipeline::new(
                rules,
                state_machine,
                "claude".to_string(),
            ),
        ));

        let (bus, recorder) = make_bus_with_recorder();

        let rx = spawn_stream_reader(
            stdout,
            pipeline,
            bus,
            "test-session".to_string(),
            None,
            Arc::new(Mutex::new(None)),
            None,
        );

        let result = rx.await.unwrap();

        assert!(result.error.is_none());
        // text + usage + synthetic done from stream close
        assert!(result.events_count >= 3);

        let events = recorder.get_events("test-session");
        assert!(events.len() >= 3);
    }

    #[tokio::test]
    async fn test_stream_reader_captures_cli_session_id() {
        let json1 = r#"{"type":"assistant","message":{"id":"msg_test_123","model":"claude-sonnet-4-6","content":[{"type":"text","text":"Hello"}]}}"#;

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(format!("echo '{}'", json1))
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn echo");

        let stdout = child.stdout.take().unwrap();

        let registry =
            NormalizerRegistry::load_from_dir(std::path::Path::new("normalizers")).unwrap();
        let config = registry.get_config("claude").unwrap();
        let rules = config.to_rules();
        let state_machine =
            Box::new(crate::normalizer::state_machines::generic::GenericStateMachine::new());
        let pipeline = Arc::new(Mutex::new(
            crate::normalizer::pipeline::NormalizerPipeline::new(
                rules,
                state_machine,
                "claude".to_string(),
            ),
        ));

        let (bus, _recorder) = make_bus_with_recorder();
        let cli_sid = Arc::new(Mutex::new(None::<String>));

        let rx = spawn_stream_reader(
            stdout,
            pipeline,
            bus,
            "test-session".to_string(),
            Some("message.id".to_string()),
            cli_sid.clone(),
            None,
        );

        let result = rx.await.unwrap();
        assert!(result.error.is_none());

        let captured = cli_sid.lock().unwrap().clone();
        assert_eq!(captured, Some("msg_test_123".to_string()));
    }

    #[tokio::test]
    async fn test_stream_reader_timeout() {
        // Spawn a process that writes nothing to stdout (hangs)
        let mut child = Command::new("sh")
            .arg("-c")
            .arg("sleep 9999")
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn");

        let stdout = child.stdout.take().unwrap();

        let registry =
            NormalizerRegistry::load_from_dir(std::path::Path::new("normalizers")).unwrap();
        let config = registry.get_config("claude").unwrap();
        let rules = config.to_rules();
        let state_machine =
            Box::new(crate::normalizer::state_machines::generic::GenericStateMachine::new());
        let pipeline = Arc::new(Mutex::new(
            crate::normalizer::pipeline::NormalizerPipeline::new(
                rules,
                state_machine,
                "claude".to_string(),
            ),
        ));

        let (bus, recorder) = make_bus_with_recorder();

        // Use a short timeout (2 seconds) to actually test the timeout path
        let rx = spawn_stream_reader(
            stdout,
            pipeline,
            bus,
            "test-timeout".to_string(),
            None,
            Arc::new(Mutex::new(None)),
            Some(Duration::from_secs(2)),
        );

        let result = rx.await.unwrap();
        assert!(result.error.is_some(), "should have timeout error");
        assert!(
            result.error.unwrap().contains("timeout"),
            "error should mention timeout"
        );

        let events = recorder.get_events("test-timeout");
        assert!(
            events
                .iter()
                .any(|e| e.event_type == crate::agent_event::AgentEventType::Error),
            "should have error event"
        );
        assert!(
            events
                .iter()
                .any(|e| e.event_type == crate::agent_event::AgentEventType::Done),
            "should have done event after timeout"
        );

        // Clean up the hanging process
        let _ = child.kill().await;
    }
}
