use crate::normalizer::pipeline::NormalizerPipeline;
use crate::transport::event_bus::AgentEventBus;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::ChildStdout;
use tokio::sync::oneshot;

pub struct StreamResult {
    pub events_count: u32,
    pub error: Option<String>,
}

pub fn spawn_stream_reader(
    stdout: ChildStdout,
    pipeline: Arc<Mutex<NormalizerPipeline>>,
    event_bus: Arc<AgentEventBus>,
    session_id: String,
) -> oneshot::Receiver<StreamResult> {
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        let mut events_count: u32 = 0;
        let mut error: Option<String> = None;

        loop {
            match lines.next_line().await {
                Ok(Some(raw_line)) => {
                    let line = raw_line.trim().to_string();
                    if line.is_empty() {
                        continue;
                    }

                    // Log the raw JSON type for debugging
                    let json_type = serde_json::from_str::<serde_json::Value>(&line)
                        .ok()
                        .and_then(|v| v.get("type").and_then(|t| t.as_str().map(|s| s.to_string())));
                    log::debug!("StreamReader[{}]: raw line type={:?} len={}", session_id, json_type, line.len());

                    let events = {
                        let mut pl = pipeline.lock().unwrap_or_else(|e| e.into_inner());
                        pl.process(&line)
                    };

                    if events.is_empty() {
                        log::trace!("StreamReader[{}]: no events from line type={:?}", session_id, json_type);
                    }
                    for event in &events {
                        events_count += 1;
                        log::debug!("StreamReader[{}]: emitting {:?}", session_id, event.event_type);
                        event_bus.publish(&session_id, event);
                    }
                }
                Ok(None) => {
                    // Emit a synthetic done event when the CLI process closes stdout
                    let done_event = crate::agent_event::AgentEvent::done(&session_id, "system");
                    events_count += 1;
                    event_bus.publish(&session_id, &done_event);
                    break;
                }
                Err(e) => {
                    error = Some(format!("{}", e));
                    break;
                }
            }
        }

        let _ = tx.send(StreamResult { events_count, error });
    });

    rx
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_event::AgentEvent;
    use crate::normalizer::NormalizerRegistry;
    use crate::transport::event_bus::HistoryRecorder;
    use std::process::Stdio;
    use tokio::process::Command;

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

        let registry = NormalizerRegistry::load_from_dir(
            std::path::Path::new("normalizers")
        ).unwrap();

        let config = registry.get_config("claude").unwrap();
        let rules = config.to_rules();
        let state_machine = Box::new(
            crate::normalizer::state_machines::generic::GenericStateMachine::new()
        );
        let pipeline = Arc::new(Mutex::new(
            crate::normalizer::pipeline::NormalizerPipeline::new(rules, state_machine, "claude".to_string())
        ));

        let bus = Arc::new(AgentEventBus::new());
        let recorder = Arc::new(HistoryRecorder::new());
        let recorder_ref = recorder.clone();

        struct RecorderWrapper(Arc<HistoryRecorder>);
        impl crate::transport::event_bus::AgentEventSubscriber for RecorderWrapper {
            fn on_event(&self, session_id: &str, event: &AgentEvent) {
                self.0.on_event(session_id, event);
            }
        }
        bus.subscribe(Box::new(RecorderWrapper(recorder.clone())));

        let rx = spawn_stream_reader(stdout, pipeline, bus, "test-session".to_string());

        let result = rx.await.unwrap();

        assert!(result.error.is_none());
        // text + usage + synthetic done from stream close
        assert!(result.events_count >= 3);

        let events = recorder_ref.get_events("test-session");
        assert!(events.len() >= 3);
    }
}
