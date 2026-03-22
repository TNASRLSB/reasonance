pub mod request;
pub mod retry;
pub mod event_bus;
pub mod session;
pub mod stream_reader;

use crate::agent_event::{AgentEvent, ErrorSeverity};
use crate::normalizer::NormalizerRegistry;
use event_bus::{AgentEventBus, AgentEventSubscriber, HistoryRecorder};
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
    retry_policies: Arc<Mutex<HashMap<String, RetryPolicy>>>,
}

impl StructuredAgentTransport {
    pub fn new(normalizers_dir: &Path) -> Result<Self, String> {
        let registry = NormalizerRegistry::load_from_dir(normalizers_dir)?;

        let mut retry_policies = HashMap::new();
        for provider in registry.providers() {
            if let Some(config) = registry.get_config(&provider) {
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

        let registry = self.registry.lock().unwrap();

        if !registry.has_provider(&provider) {
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
            _ => Box::new(crate::normalizer::state_machines::generic::GenericStateMachine::new()),
        };
        let pipeline = Arc::new(Mutex::new(
            crate::normalizer::pipeline::NormalizerPipeline::new(rules, state_machine, provider.clone())
        ));

        let session = AgentSession::new(request.clone(), CliMode::Structured);
        let session_id = session.id.clone();
        self.sessions.lock().unwrap().insert(session_id.clone(), session);

        let mut cmd = Command::new(&binary);
        cmd.args(&args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::null());

        let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn {}: {}", binary, e))?;
        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;

        let event_bus = self.event_bus.clone();
        let sid = session_id.clone();
        let sessions_ref = self.sessions.clone();

        let rx = spawn_stream_reader(stdout, pipeline, event_bus, sid.clone());

        let join_handle = tokio::spawn(async move {
            let _ = child.wait().await;
            if let Ok(result) = rx.await {
                let mut sessions = sessions_ref.lock().unwrap();
                if let Some(sess) = sessions.get_mut(&sid) {
                    if result.error.is_some() {
                        sess.set_status(SessionStatus::Error { severity: ErrorSeverity::Fatal });
                    } else {
                        sess.set_status(SessionStatus::Terminated);
                    }
                }
            }
        });

        self.sessions.lock().unwrap()
            .get_mut(&session_id)
            .unwrap()
            .set_abort_handle(join_handle.abort_handle());

        Ok(session_id)
    }

    pub fn stop(&self, session_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;
        if let Some(handle) = session.abort_handle.take() {
            handle.abort();
        }
        session.set_status(SessionStatus::Terminated);
        Ok(())
    }

    pub fn get_status(&self, session_id: &str) -> Result<SessionStatus, String> {
        let sessions = self.sessions.lock().unwrap();
        let session = sessions.get(session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;
        Ok(session.status.clone())
    }

    pub fn get_events(&self, session_id: &str) -> Vec<AgentEvent> {
        self.history.get_events(session_id)
    }

    pub fn event_bus(&self) -> Arc<AgentEventBus> {
        self.event_bus.clone()
    }

    pub fn active_sessions(&self) -> Vec<String> {
        self.sessions.lock().unwrap().keys().cloned().collect()
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
        let registry = transport.registry.lock().unwrap();
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
        let registry = transport.registry.lock().unwrap();
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
