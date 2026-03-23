pub mod request;
pub mod retry;
pub mod event_bus;
pub mod session;
pub mod stream_reader;
pub mod session_handle;
pub mod session_store;
pub mod session_manager;

use crate::agent_event::{AgentEvent, ErrorSeverity};
use crate::normalizer::NormalizerRegistry;
use event_bus::{AgentEventBus, AgentEventSubscriber, HistoryRecorder};
#[allow(unused_imports)]
use log::{info, warn, error, debug, trace};
use request::{AgentRequest, CliMode, SessionStatus};
use retry::RetryPolicy;
use session::AgentSession;
use stream_reader::spawn_stream_reader;

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::process::Command;
use std::process::Stdio;

pub struct StructuredAgentTransport {
    registry: Arc<Mutex<NormalizerRegistry>>,
    sessions: Arc<Mutex<HashMap<String, AgentSession>>>,
    event_bus: Arc<AgentEventBus>,
    history: Arc<HistoryRecorder>,
    /// Retry policies loaded from provider configs. Not yet wired into `send()` —
    /// planned for future use when automatic retry-on-error is implemented.
    #[allow(dead_code)]
    retry_policies: Arc<Mutex<HashMap<String, RetryPolicy>>>,
}

impl StructuredAgentTransport {
    pub fn new(normalizers_dir: &Path) -> Result<Self, String> {
        info!("StructuredAgentTransport: initializing from {}", normalizers_dir.display());
        let registry = NormalizerRegistry::load_from_dir(normalizers_dir)?;

        let mut retry_policies = HashMap::new();
        for provider in registry.providers() {
            if let Some(config) = registry.get_config(&provider) {
                debug!("StructuredAgentTransport: loaded retry policy for provider={}", provider);
                retry_policies.insert(provider, RetryPolicy::from_toml_config(config));
            }
        }

        let event_bus = Arc::new(AgentEventBus::new());
        let history = Arc::new(HistoryRecorder::new());

        struct HistoryWrapper(Arc<HistoryRecorder>);
        impl AgentEventSubscriber for HistoryWrapper {
            fn on_event(&self, session_id: &str, event: &AgentEvent) {
                self.0.on_event(session_id, event);
            }
        }
        event_bus.subscribe(Box::new(HistoryWrapper(history.clone())));

        info!("StructuredAgentTransport: initialized with {} providers", registry.providers().len());
        Ok(Self {
            registry: Arc::new(Mutex::new(registry)),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            event_bus,
            history,
            retry_policies: Arc::new(Mutex::new(retry_policies)),
        })
    }

    pub fn send(&self, request: AgentRequest) -> Result<String, String> {
        let provider = request.provider.clone();
        info!("Transport: send request provider={} model={:?} session_id={:?}", provider, request.model, request.session_id);

        let registry = self.registry.lock().unwrap_or_else(|e| {
            warn!("Transport: registry lock poisoned, recovering");
            e.into_inner()
        });

        if !registry.has_provider(&provider) {
            warn!("Transport: unknown provider={}", provider);
            return Err(format!("Unknown provider: {}", provider));
        }

        let config = registry.get_config(&provider)
            .ok_or_else(|| format!("No config for provider: {}", provider))?;

        let binary = config.cli.binary.clone();
        let args = Self::build_cli_args(config, &request);
        let rules = config.to_rules();
        drop(registry);

        let state_machine: Box<dyn crate::normalizer::state_machines::StateMachine> = match provider.as_str() {
            "claude" => Box::new(crate::normalizer::state_machines::claude::ClaudeStateMachine::new()),
            "gemini" => Box::new(crate::normalizer::state_machines::gemini::GeminiStateMachine::new()),
            "kimi" => Box::new(crate::normalizer::state_machines::kimi::KimiStateMachine::new()),
            "qwen" => Box::new(crate::normalizer::state_machines::qwen::QwenStateMachine::new()),
            "codex" => Box::new(crate::normalizer::state_machines::codex::CodexStateMachine::new()),
            _ => Box::new(crate::normalizer::state_machines::generic::GenericStateMachine::new()),
        };
        let pipeline = Arc::new(Mutex::new(
            crate::normalizer::pipeline::NormalizerPipeline::new(rules, state_machine, provider.clone())
        ));

        let session = AgentSession::new(request.clone(), CliMode::Structured);
        let session_id = session.id.clone();
        debug!("Transport: created agent session={}", session_id);
        self.sessions.lock().unwrap_or_else(|e| e.into_inner()).insert(session_id.clone(), session);

        let mut cmd = Command::new(&binary);
        cmd.args(&args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        debug!("Transport: spawning CLI binary={} with {} args", binary, args.len());
        let mut child = cmd.spawn().map_err(|e| {
            error!("Transport: failed to spawn {}: {}", binary, e);
            format!("Failed to spawn {}: {}", binary, e)
        })?;
        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;

        // Capture stderr and emit as warning events
        if let Some(stderr) = child.stderr.take() {
            let stderr_bus = self.event_bus.clone();
            let stderr_sid = session_id.clone();
            tokio::spawn(async move {
                use tokio::io::{AsyncBufReadExt, BufReader};
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if !line.trim().is_empty() {
                        let event = AgentEvent::error(
                            &format!("[stderr] {}", line),
                            "STDERR",
                            ErrorSeverity::Recoverable,
                            "system",
                        );
                        stderr_bus.publish(&stderr_sid, &event);
                    }
                }
            });
        }

        let event_bus = self.event_bus.clone();
        let sid = session_id.clone();
        let sessions_ref = self.sessions.clone();

        let rx = spawn_stream_reader(stdout, pipeline, event_bus, sid.clone());

        let join_handle = tokio::spawn(async move {
            let _ = child.wait().await;
            if let Ok(result) = rx.await {
                let mut sessions = sessions_ref.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(sess) = sessions.get_mut(&sid) {
                    if result.error.is_some() {
                        sess.set_status(SessionStatus::Error { severity: ErrorSeverity::Fatal });
                    } else {
                        sess.set_status(SessionStatus::Terminated);
                    }
                }
            }
        });

        self.sessions.lock().unwrap_or_else(|e| e.into_inner())
            .get_mut(&session_id)
            .unwrap()
            .set_abort_handle(join_handle.abort_handle());

        info!("Transport: session={} started for provider={}", session_id, provider);
        Ok(session_id)
    }

    pub fn stop(&self, session_id: &str) -> Result<(), String> {
        info!("Transport: stopping session={}", session_id);
        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| {
                warn!("Transport: stop requested for unknown session={}", session_id);
                format!("Session {} not found", session_id)
            })?;
        if let Some(handle) = session.abort_handle.take() {
            handle.abort();
        }
        session.set_status(SessionStatus::Terminated);
        info!("Transport: session={} stopped", session_id);
        Ok(())
    }

    pub fn get_status(&self, session_id: &str) -> Result<SessionStatus, String> {
        let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let session = sessions.get(session_id)
            .ok_or_else(|| {
                warn!("Transport: get_status for unknown session={}", session_id);
                format!("Session {} not found", session_id)
            })?;
        let status = session.status.clone();
        debug!("Transport: session={} status={:?}", session_id, status);
        Ok(status)
    }

    pub fn get_events(&self, session_id: &str) -> Vec<AgentEvent> {
        self.history.get_events(session_id)
    }

    pub fn event_bus(&self) -> Arc<AgentEventBus> {
        self.event_bus.clone()
    }

    pub fn active_sessions(&self) -> Vec<String> {
        self.sessions.lock().unwrap_or_else(|e| e.into_inner()).keys().cloned().collect()
    }

    pub fn registry(&self) -> Arc<Mutex<NormalizerRegistry>> {
        self.registry.clone()
    }

    fn build_cli_args(config: &crate::normalizer::TomlConfig, request: &AgentRequest) -> Vec<String> {
        let args_template = if request.session_id.is_some() {
            &config.cli.resume_args
        } else {
            &config.cli.programmatic_args
        };

        args_template.iter().map(|arg| {
            arg.replace("{prompt}", &request.prompt)
                .replace("{session_id}", request.session_id.as_deref().unwrap_or(""))
                .replace("{model}", request.model.as_deref().unwrap_or(""))
        }).collect()
    }
}

#[cfg(test)]
mod integration_tests;

#[cfg(test)]
mod session_integration_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_creation() {
        let transport = StructuredAgentTransport::new(Path::new("normalizers"));
        assert!(transport.is_ok());
        let t = transport.unwrap();
        assert!(t.active_sessions().is_empty());
    }

    #[test]
    fn test_transport_unknown_provider() {
        let transport = StructuredAgentTransport::new(Path::new("normalizers")).unwrap();
        let request = AgentRequest {
            prompt: "hello".to_string(),
            provider: "unknown_provider".to_string(),
            model: None,
            context: vec![],
            session_id: None,
            system_prompt: None,
            max_tokens: None,
            allowed_tools: None,
        };
        let result = transport.send(request);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown provider"));
    }

    #[test]
    fn test_build_cli_args() {
        let transport = StructuredAgentTransport::new(Path::new("normalizers")).unwrap();
        let registry = transport.registry.lock().unwrap_or_else(|e| e.into_inner());
        let config = registry.get_config("claude").unwrap();
        let request = AgentRequest {
            prompt: "test prompt".to_string(),
            provider: "claude".to_string(),
            model: Some("claude-sonnet-4-6".to_string()),
            context: vec![],
            session_id: None,
            system_prompt: None,
            max_tokens: None,
            allowed_tools: None,
        };
        let args = StructuredAgentTransport::build_cli_args(config, &request);
        assert!(args.contains(&"test prompt".to_string()));
        assert!(args.contains(&"--output-format".to_string()));
        assert!(args.contains(&"stream-json".to_string()));
    }

    #[test]
    fn test_build_cli_args_resume() {
        let transport = StructuredAgentTransport::new(Path::new("normalizers")).unwrap();
        let registry = transport.registry.lock().unwrap_or_else(|e| e.into_inner());
        let config = registry.get_config("claude").unwrap();
        let request = AgentRequest {
            prompt: "continue".to_string(),
            provider: "claude".to_string(),
            model: None,
            context: vec![],
            session_id: Some("sess-123".to_string()),
            system_prompt: None,
            max_tokens: None,
            allowed_tools: None,
        };
        let args = StructuredAgentTransport::build_cli_args(config, &request);
        assert!(args.contains(&"--resume".to_string()));
        assert!(args.contains(&"sess-123".to_string()));
    }

    #[test]
    fn test_get_events_empty() {
        let transport = StructuredAgentTransport::new(Path::new("normalizers")).unwrap();
        let events = transport.get_events("nonexistent");
        assert!(events.is_empty());
    }

    #[test]
    fn test_get_status_not_found() {
        let transport = StructuredAgentTransport::new(Path::new("normalizers")).unwrap();
        let result = transport.get_status("nonexistent");
        assert!(result.is_err());
    }
}
