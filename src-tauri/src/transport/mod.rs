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
    /// Create a transport with an empty registry (no providers).
    /// Used as fallback when normalizer configs are missing.
    pub fn empty() -> Self {
        warn!("StructuredAgentTransport: starting with empty registry (no normalizers found)");
        let event_bus = Arc::new(AgentEventBus::new());
        let history = Arc::new(HistoryRecorder::new());

        struct HistoryWrapper(Arc<HistoryRecorder>);
        impl AgentEventSubscriber for HistoryWrapper {
            fn on_event(&self, session_id: &str, event: &AgentEvent) {
                self.0.on_event(session_id, event);
            }
        }
        event_bus.subscribe(Box::new(HistoryWrapper(history.clone())));

        Self {
            registry: Arc::new(Mutex::new(NormalizerRegistry::default())),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            event_bus,
            history,
            retry_policies: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn new(normalizers_dir: &Path) -> Result<Self, crate::error::ReasonanceError> {
        info!("StructuredAgentTransport: initializing from {}", normalizers_dir.display());
        let registry = NormalizerRegistry::load_from_dir(normalizers_dir)
            .map_err(crate::error::ReasonanceError::config)?;

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

    pub fn send(&self, request: AgentRequest, trust_store: &crate::workspace_trust::TrustStore) -> Result<String, crate::error::ReasonanceError> {
        let provider = request.provider.to_lowercase();
        info!("Transport: send request provider={} model={:?} session_id={:?} yolo={} cwd={:?}", provider, request.model, request.session_id, request.yolo, request.cwd);

        // Backend safety net: query trust from the store, not from frontend-supplied value
        let trust = request.cwd.as_deref().map(|cwd| {
            trust_store.check_trust(cwd)
        });
        let trust_level = trust.as_ref().and_then(|r| r.level);
        info!("Transport: trust_level={:?} yolo={}", trust_level, request.yolo);

        if matches!(trust_level, Some(crate::workspace_trust::TrustLevel::Blocked)) {
            warn!("Transport: send blocked — workspace is not trusted cwd={:?}", request.cwd);
            return Err(crate::error::ReasonanceError::Security {
                message: "Workspace is not trusted".to_string(),
                code: crate::error::SecurityErrorCode::BlockedWorkspace,
            });
        }
        // When yolo=true, skip the trust gate — user has explicitly opted into full permissions
        if !request.yolo && trust_level.is_none() && request.cwd.is_some() {
            warn!("Transport: send blocked — workspace trust not established cwd={:?}", request.cwd);
            return Err(crate::error::ReasonanceError::Security {
                message: "Workspace trust has not been established".to_string(),
                code: crate::error::SecurityErrorCode::UnauthorizedAccess,
            });
        }

        let registry = self.registry.lock().unwrap_or_else(|e| {
            warn!("Transport: registry lock poisoned, recovering");
            e.into_inner()
        });

        if !registry.has_provider(&provider) {
            warn!("Transport: unknown provider={}", provider);
            return Err(crate::error::ReasonanceError::not_found("provider", &provider));
        }

        let config = registry.get_config(&provider)
            .ok_or_else(|| crate::error::ReasonanceError::not_found("provider config", &provider))?;

        let binary = config.cli.binary.clone();

        // Determine session ID
        let session_id = request.session_id.clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // Look up existing session for CLI session ID (for resume) and validate state
        // Use a single lock scope to avoid TOCTOU race
        let cli_session_id = {
            let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(existing) = sessions.get_mut(&session_id) {
                if existing.status == SessionStatus::Active {
                    // The frontend already received the done event (which re-enabled input),
                    // but the async task that updates session status hasn't completed yet.
                    // Force-stop the stale session so the user can send a follow-up message.
                    if let Some(handle) = existing.abort_handle.take() {
                        handle.abort();
                    }
                    existing.set_status(SessionStatus::Terminated);
                    warn!("Transport: force-stopped stale active session={} to allow new message", session_id);
                }
                debug!("Transport: reusing session={} cli_session_id={:?}", session_id, existing.cli_session_id);
                existing.cli_session_id.clone()
            } else {
                None
            }
        };

        let args = Self::build_cli_args(config, &request, cli_session_id.as_deref());
        let permission_args = Self::build_permission_args_with_trust(config, request.cwd.as_deref(), request.yolo, trust_level);
        info!("Transport: permission_args={:?}", permission_args);
        let allowed_tools_args = if matches!(trust_level, Some(crate::workspace_trust::TrustLevel::ReadOnly)) {
            Self::build_read_only_tools_args(config)
        } else {
            Self::build_allowed_tools_args(config, &request.allowed_tools)
        };
        let rules = config.to_rules();
        let session_id_path = config.session_id_path().map(|s| s.to_string());
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

        // Create or reactivate session
        {
            let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(existing) = sessions.get_mut(&session_id) {
                existing.set_status(SessionStatus::Active);
                existing.request = request.clone();
                debug!("Transport: reactivated agent session={}", session_id);
            } else {
                let mut session = AgentSession::new(request.clone(), CliMode::Structured);
                // Ensure session ID matches (AgentSession::new may generate a new one if request.session_id is None)
                session.id = session_id.clone();
                debug!("Transport: created agent session={}", session_id);
                sessions.insert(session_id.clone(), session);
            }
        }

        let mut cmd = Command::new(&binary);
        cmd.args(&args);
        // Append permission args from normalizer config (e.g. --dangerously-skip-permissions)
        cmd.args(&permission_args);
        // Append allowed tools args (e.g. --allowedTools Read,Edit)
        cmd.args(&allowed_tools_args);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Set working directory to project root if provided
        if let Some(ref cwd) = request.cwd {
            if !cwd.is_empty() {
                cmd.current_dir(cwd);
                debug!("Transport: set cwd={}", cwd);
            }
        }

        info!("Transport: spawning CLI binary={} args={:?} permission_args={:?} allowed_tools_args={:?}", binary, args, permission_args, allowed_tools_args);
        let mut child = cmd.spawn().map_err(|e| {
            error!("Transport: failed to spawn {}: {}", binary, e);
            crate::error::ReasonanceError::Transport {
                provider: provider.clone(),
                message: format!("Failed to spawn {}: {}", binary, e),
                retryable: false,
            }
        })?;
        let stdout = child.stdout.take().ok_or_else(|| crate::error::ReasonanceError::Transport {
            provider: provider.clone(),
            message: "Failed to capture stdout".to_string(),
            retryable: false,
        })?;

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

        let cli_session_id_ref = Arc::new(Mutex::new(None::<String>));
        let cli_sid_for_reader = cli_session_id_ref.clone();
        let rx = spawn_stream_reader(stdout, pipeline, event_bus, sid.clone(), session_id_path, cli_sid_for_reader, None);

        let join_handle = tokio::spawn(async move {
            let _ = child.wait().await;

            // Store captured CLI session ID in the session
            {
                let captured = cli_session_id_ref.lock().unwrap_or_else(|e| e.into_inner()).clone();
                if let Some(ref cli_sid) = captured {
                    let mut sessions = sessions_ref.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(sess) = sessions.get_mut(&sid) {
                        sess.set_cli_session_id(cli_sid.clone());
                        log::info!("Transport: session={} stored CLI session ID={}", sid, cli_sid);
                    }
                }
            }

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

    pub fn stop(&self, session_id: &str) -> Result<(), crate::error::ReasonanceError> {
        info!("Transport: stopping session={}", session_id);
        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| {
                warn!("Transport: stop requested for unknown session={}", session_id);
                crate::error::ReasonanceError::not_found("session", session_id)
            })?;
        if let Some(handle) = session.abort_handle.take() {
            handle.abort();
        }
        session.set_status(SessionStatus::Terminated);
        info!("Transport: session={} stopped", session_id);
        Ok(())
    }

    pub fn get_status(&self, session_id: &str) -> Result<SessionStatus, crate::error::ReasonanceError> {
        let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let session = sessions.get(session_id)
            .ok_or_else(|| {
                warn!("Transport: get_status for unknown session={}", session_id);
                crate::error::ReasonanceError::not_found("session", session_id)
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

    /// Build permission args from the normalizer config, substituting `{project_root}` with the actual path.
    /// Only returns permission args when `yolo` is true. In non-yolo mode the CLI runs without
    /// permission flags and auto-denies tool use (reporting `permission_denials` in the result event),
    /// which the UI renders as approval prompts.
    fn build_permission_args(config: &crate::normalizer::TomlConfig, cwd: Option<&str>, yolo: bool) -> Vec<String> {
        if !yolo {
            return Vec::new();
        }
        let project_root = cwd.unwrap_or(".");
        config.cli.permission_args.iter().map(|arg| {
            arg.replace("{project_root}", project_root)
        }).collect()
    }

    /// Build `--allowedTools tool1,tool2` args if the provider supports it and tools are provided.
    fn build_allowed_tools_args(config: &crate::normalizer::TomlConfig, allowed_tools: &Option<Vec<String>>) -> Vec<String> {
        match (allowed_tools, &config.cli.allowed_tools_arg) {
            (Some(tools), Some(flag)) if !tools.is_empty() => {
                vec![flag.clone(), tools.join(",")]
            }
            _ => Vec::new(),
        }
    }

    /// Build permission args considering both yolo flag and workspace trust level.
    fn build_permission_args_with_trust(
        config: &crate::normalizer::TomlConfig,
        cwd: Option<&str>,
        yolo: bool,
        trust_level: Option<crate::workspace_trust::TrustLevel>,
    ) -> Vec<String> {
        use crate::workspace_trust::TrustLevel;
        let project_root = cwd.unwrap_or(".");

        // When yolo=true, always pass permission args regardless of trust level
        if yolo {
            return config.cli.permission_args.iter().map(|arg| {
                arg.replace("{project_root}", project_root)
            }).collect();
        }

        match trust_level {
            Some(TrustLevel::Trusted) => {
                // Trusted workspace without yolo — still pass permission args for stdin=null CLIs
                config.cli.permission_args.iter().map(|arg| {
                    arg.replace("{project_root}", project_root)
                }).collect()
            }
            Some(TrustLevel::ReadOnly) => {
                // Pass permission_args to avoid interactive prompts (stdin=null)
                // Tool restriction handled separately via read_only_tools_args
                config.cli.permission_args.iter().map(|arg| {
                    arg.replace("{project_root}", project_root)
                }).collect()
            }
            _ => Vec::new(),
        }
    }

    /// Build `--allowedTools` args for read-only mode using the normalizer's read_only_tools list.
    fn build_read_only_tools_args(config: &crate::normalizer::TomlConfig) -> Vec<String> {
        match &config.cli.allowed_tools_arg {
            Some(flag) if !config.cli.read_only_tools.is_empty() => {
                vec![flag.clone(), config.cli.read_only_tools.join(",")]
            }
            _ => Vec::new(),
        }
    }

    /// Build CLI args. `cli_session_id` is the session ID returned by the CLI
    /// (e.g., from a previous invocation), NOT the internal Reasonance session ID.
    /// Only when a real CLI session ID is provided do we use `resume_args`.
    fn build_cli_args(config: &crate::normalizer::TomlConfig, request: &AgentRequest, cli_session_id: Option<&str>) -> Vec<String> {
        let args_template = if cli_session_id.is_some() {
            &config.cli.resume_args
        } else {
            &config.cli.programmatic_args
        };

        args_template.iter().map(|arg| {
            arg.replace("{prompt}", &request.prompt)
                .replace("{session_id}", cli_session_id.unwrap_or(""))
                .replace("{model}", request.model.as_deref().unwrap_or(""))
        }).collect()
    }
}

#[cfg(test)]
mod integration_tests;

#[cfg(test)]
mod session_integration_tests;

#[cfg(test)]
mod chat_flow_tests;

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
        use crate::workspace_trust::TrustStore;
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let trust_store = TrustStore::new(tmp.path().join("trust.json"));

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
            cwd: None,
            yolo: false,
        };
        let result = transport.send(request, &trust_store);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
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
            cwd: None,
            yolo: false,
        };
        let args = StructuredAgentTransport::build_cli_args(config, &request, None);
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
            cwd: None,
            yolo: false,
        };
        // Resume only happens when a CLI session ID is provided
        let args = StructuredAgentTransport::build_cli_args(config, &request, Some("sess-123"));
        assert!(args.contains(&"--resume".to_string()));
        assert!(args.contains(&"sess-123".to_string()));
    }

    #[test]
    fn test_permission_args_conditional_on_yolo() {
        let transport = StructuredAgentTransport::new(Path::new("normalizers")).unwrap();
        let registry = transport.registry.lock().unwrap_or_else(|e| e.into_inner());
        let config = registry.get_config("claude").unwrap();

        // Permission args are only included when yolo=true
        let args_off = StructuredAgentTransport::build_permission_args(config, Some("/project"), false);
        assert!(args_off.is_empty());

        let args_on = StructuredAgentTransport::build_permission_args(config, Some("/project"), true);
        assert!(!args_on.is_empty());
        assert!(args_on.iter().any(|a| a.contains("dangerously-skip-permissions")));
    }

    #[test]
    fn test_allowed_tools_args() {
        let transport = StructuredAgentTransport::new(Path::new("normalizers")).unwrap();
        let registry = transport.registry.lock().unwrap_or_else(|e| e.into_inner());
        let config = registry.get_config("claude").unwrap();

        // With tools and supported provider
        let tools = Some(vec!["Read".to_string(), "Edit".to_string()]);
        let args = StructuredAgentTransport::build_allowed_tools_args(config, &tools);
        assert_eq!(args, vec!["--allowedTools".to_string(), "Read,Edit".to_string()]);

        // Without tools
        let args_none = StructuredAgentTransport::build_allowed_tools_args(config, &None);
        assert!(args_none.is_empty());

        // Empty tools list
        let args_empty = StructuredAgentTransport::build_allowed_tools_args(config, &Some(vec![]));
        assert!(args_empty.is_empty());
    }

    #[test]
    fn test_get_events_empty() {
        let transport = StructuredAgentTransport::new(Path::new("normalizers")).unwrap();
        let events = transport.get_events("nonexistent");
        assert!(events.is_empty());
    }

    #[test]
    fn test_build_cli_args_resume_uses_resume_args() {
        let transport = StructuredAgentTransport::new(Path::new("normalizers")).unwrap();
        let registry = transport.registry.lock().unwrap_or_else(|e| e.into_inner());
        let config = registry.get_config("claude").unwrap();
        let request = AgentRequest {
            prompt: "follow-up question".to_string(),
            provider: "claude".to_string(),
            model: None,
            context: vec![],
            session_id: Some("existing-session".to_string()),
            system_prompt: None,
            max_tokens: None,
            allowed_tools: None,
            cwd: None,
            yolo: false,
        };
        let args = StructuredAgentTransport::build_cli_args(config, &request, Some("msg_abc123"));
        assert!(args.contains(&"--resume".to_string()));
        assert!(args.contains(&"msg_abc123".to_string()));
        assert!(args.contains(&"follow-up question".to_string()));
    }

    #[test]
    fn test_get_status_not_found() {
        let transport = StructuredAgentTransport::new(Path::new("normalizers")).unwrap();
        let result = transport.get_status("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_build_permission_args_with_trust() {
        let transport = StructuredAgentTransport::new(Path::new("normalizers")).unwrap();
        let registry = transport.registry.lock().unwrap_or_else(|e| e.into_inner());
        let config = registry.get_config("claude").unwrap();

        // Trusted + yolo → permission args
        let args = StructuredAgentTransport::build_permission_args_with_trust(
            config, Some("/project"), true, Some(crate::workspace_trust::TrustLevel::Trusted)
        );
        assert!(!args.is_empty());

        // Trusted + not yolo → permission args (trusted workspace gets args for stdin=null CLIs)
        let args = StructuredAgentTransport::build_permission_args_with_trust(
            config, Some("/project"), false, Some(crate::workspace_trust::TrustLevel::Trusted)
        );
        assert!(!args.is_empty());

        // ReadOnly → permission args (to avoid interactive prompt) regardless of yolo
        let args = StructuredAgentTransport::build_permission_args_with_trust(
            config, Some("/project"), false, Some(crate::workspace_trust::TrustLevel::ReadOnly)
        );
        assert!(!args.is_empty());

        // yolo + Blocked → permission args (yolo overrides; send() blocks Blocked upstream)
        let args = StructuredAgentTransport::build_permission_args_with_trust(
            config, Some("/project"), true, Some(crate::workspace_trust::TrustLevel::Blocked)
        );
        assert!(!args.is_empty());

        // yolo + None trust → permission args (yolo bypasses trust gate)
        let args = StructuredAgentTransport::build_permission_args_with_trust(
            config, Some("/project"), true, None
        );
        assert!(!args.is_empty());

        // not yolo + None trust → no permission args
        let args = StructuredAgentTransport::build_permission_args_with_trust(
            config, Some("/project"), false, None
        );
        assert!(args.is_empty());

        // not yolo + Blocked → no permission args
        let args = StructuredAgentTransport::build_permission_args_with_trust(
            config, Some("/project"), false, Some(crate::workspace_trust::TrustLevel::Blocked)
        );
        assert!(args.is_empty());
    }

    #[test]
    fn test_read_only_uses_allowed_tools_whitelist() {
        let transport = StructuredAgentTransport::new(Path::new("normalizers")).unwrap();
        let registry = transport.registry.lock().unwrap_or_else(|e| e.into_inner());
        let config = registry.get_config("claude").unwrap();

        let args = StructuredAgentTransport::build_read_only_tools_args(config);
        assert!(args.contains(&"--allowedTools".to_string()));
        // Should contain comma-separated read-only tools
        let tools_str = &args[1];
        assert!(tools_str.contains("Read"));
        assert!(tools_str.contains("Grep"));
    }
}
