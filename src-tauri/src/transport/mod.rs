pub mod request;
pub mod retry;
pub mod session;
pub mod session_handle;
pub mod session_manager;
pub mod session_store;
pub mod stream_reader;

use crate::agent_event::{AgentEvent, ErrorSeverity};
use crate::circuit_breaker::CircuitBreaker;
use crate::normalizer::NormalizerRegistry;
use crate::permission_engine::{
    EvaluationResult, PermissionContext, PermissionDecision, PermissionEngine, PermissionMemory,
};
use crate::policy_file::PolicyFile;
use crate::tracked_map::TrackedMap;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use request::{AgentRequest, CliMode, SessionStatus};
use retry::RetryPolicy;
use session::AgentSession;
use stream_reader::spawn_stream_reader;

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::process::Command;

pub struct StructuredAgentTransport {
    registry: Arc<Mutex<NormalizerRegistry>>,
    sessions: Arc<Mutex<TrackedMap<String, AgentSession>>>,
    /// Retry policies loaded from provider configs, keyed by provider name.
    /// Used by `send()` to retry failed CLI spawns with exponential backoff.
    /// Plain HashMap: small, bounded (one entry per provider), loaded from disk on init.
    retry_policies: Arc<Mutex<HashMap<String, RetryPolicy>>>,
    /// Circuit breaker for transport-level fault isolation per provider.
    circuit_breaker: Arc<CircuitBreaker>,
    /// The sole event bus. Uses `Mutex` for interior mutability so
    /// `set_event_bus` can be called through `&self` (Tauri `State`
    /// only provides shared refs).
    event_bus: Mutex<Option<Arc<crate::event_bus::EventBus>>>,
}

impl StructuredAgentTransport {
    /// Create a transport with an empty registry (no providers).
    /// Used as fallback when normalizer configs are missing.
    pub fn empty() -> Self {
        warn!("StructuredAgentTransport: starting with empty registry (no normalizers found)");
        Self {
            registry: Arc::new(Mutex::new(NormalizerRegistry::default())),
            sessions: Arc::new(Mutex::new(TrackedMap::new())),
            retry_policies: Arc::new(Mutex::new(HashMap::new())),
            circuit_breaker: Arc::new(CircuitBreaker::new()),
            event_bus: Mutex::new(None),
        }
    }

    pub fn new(normalizers_dir: &Path) -> Result<Self, crate::error::ReasonanceError> {
        info!(
            "StructuredAgentTransport: initializing from {}",
            normalizers_dir.display()
        );
        let registry = NormalizerRegistry::load_from_dir(normalizers_dir)
            .map_err(crate::error::ReasonanceError::config)?;

        let mut retry_policies = HashMap::new();
        for provider in registry.providers() {
            if let Some(config) = registry.get_config(&provider) {
                debug!(
                    "StructuredAgentTransport: loaded retry policy for provider={}",
                    provider
                );
                retry_policies.insert(provider, RetryPolicy::from_toml_config(config));
            }
        }

        let circuit_breaker = Arc::new(CircuitBreaker::new());

        info!(
            "StructuredAgentTransport: initialized with {} providers",
            registry.providers().len()
        );
        Ok(Self {
            registry: Arc::new(Mutex::new(registry)),
            sessions: Arc::new(Mutex::new(TrackedMap::new())),
            retry_policies: Arc::new(Mutex::new(retry_policies)),
            circuit_breaker,
            event_bus: Mutex::new(None),
        })
    }

    pub fn send(
        &self,
        mut request: AgentRequest,
        trust_store: &crate::workspace_trust::TrustStore,
        memory: &PermissionMemory,
        policy: &PolicyFile,
        slot_registry: &Mutex<crate::model_slots::ModelSlotRegistry>,
        settings: &Mutex<crate::settings::LayeredSettings>,
    ) -> Result<String, crate::error::ReasonanceError> {
        let provider = request.provider.to_lowercase();

        // ── Model slot resolution ────────────────────────────────────────
        // Priority: explicit request.model > LayeredSettings override > slot registry > unchanged
        if request.model.is_none() {
            // 1. Try LayeredSettings "model_slots.chat" first (per-project override)
            let settings_model = {
                let s = settings.lock().unwrap_or_else(|e| e.into_inner());
                s.get::<String>("model_slots.chat")
            };
            if let Some(m) = settings_model {
                debug!("Transport: resolved model from settings: {}", m);
                request.model = Some(m);
            } else {
                // 2. Fall back to slot registry Chat slot
                let reg = slot_registry.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(m) = reg.resolve_model(&provider, &crate::model_slots::ModelSlot::Chat)
                {
                    debug!("Transport: resolved model from slot registry: {}", m);
                    request.model = Some(m);
                }
            }
        }

        info!(
            "Transport: send request provider={} model={:?} session_id={:?} yolo={} cwd={:?}",
            provider, request.model, request.session_id, request.yolo, request.cwd
        );

        // Backend safety net: query trust from the store, not from frontend-supplied value
        let trust = request
            .cwd
            .as_deref()
            .map(|cwd| trust_store.check_trust(cwd));
        let trust_level = trust.as_ref().and_then(|r| r.level);
        info!(
            "Transport: trust_level={:?} yolo={}",
            trust_level, request.yolo
        );

        // Map workspace trust to the string key used by PermissionEngine
        let trust_level_str = match trust_level {
            Some(crate::workspace_trust::TrustLevel::Trusted) => "trusted",
            Some(crate::workspace_trust::TrustLevel::ReadOnly) => "read_only",
            Some(crate::workspace_trust::TrustLevel::Blocked) => "blocked",
            None => {
                if request.yolo {
                    "trusted"
                } else {
                    "untrusted"
                }
            }
        };

        let permission_level = if request.yolo { "yolo" } else { "ask" };

        let ctx = PermissionContext {
            tool_name: "*".to_string(), // pre-flight overall mode check
            tool_args: None,
            provider: provider.clone(),
            permission_level: permission_level.to_string(),
            trust_level: trust_level_str.to_string(),
            project_root: request.cwd.clone(),
        };

        let session_id_for_eval = request.session_id.as_deref().unwrap_or("");
        let eval_result =
            PermissionEngine::evaluate_with_session(&ctx, memory, policy, session_id_for_eval);

        // Publish audit event via the EventBus
        Self::publish_permission_audit(&self.event_bus, &eval_result, session_id_for_eval);

        match eval_result.decision {
            PermissionDecision::Deny { reason } => {
                warn!(
                    "Transport: send blocked by PermissionEngine layer={} reason={}",
                    eval_result.deciding_layer, reason
                );
                return Err(crate::error::ReasonanceError::PermissionDenied {
                    action: reason,
                    tool: None,
                });
            }
            _ => { /* Allow or Confirm — proceed */ }
        }

        let registry = self.registry.lock().unwrap_or_else(|e| {
            warn!("Transport: registry lock poisoned, recovering");
            e.into_inner()
        });

        if !registry.has_provider(&provider) {
            warn!("Transport: unknown provider={}", provider);
            return Err(crate::error::ReasonanceError::not_found(
                "provider", &provider,
            ));
        }

        // Circuit breaker gate: reject early if provider circuit is open
        let circuit_id = format!("transport:{}", provider);
        self.circuit_breaker.check(&circuit_id)?;

        let config = registry.get_config(&provider).ok_or_else(|| {
            crate::error::ReasonanceError::not_found("provider config", &provider)
        })?;

        let binary = config.cli.binary.clone();

        // Determine session ID
        let session_id = request
            .session_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // Build CLI args and pipeline config while holding only the registry lock
        // (before touching sessions) so we don't hold two locks simultaneously.
        // Pre-capture resume_args template so we can build resume args later without
        // needing the config reference (which is tied to the registry lock).
        let resume_args_template = config.cli.resume_args.clone();
        let args_for_new_session = Self::build_cli_args(config, &request, None);
        // Use the engine's decision to determine permission args:
        // - Allow (yolo/trusted) → pass --dangerously-skip-permissions
        // - Confirm (ask mode) → pass permission args for stdin=null CLIs if trusted
        // - ReadOnly trust → restrict to read-only tools
        let permission_args = Self::build_permission_args_from_eval(
            config,
            request.cwd.as_deref(),
            &eval_result,
            trust_level,
        );
        info!("Transport: permission_args={:?}", permission_args);
        let allowed_tools_args = if matches!(
            trust_level,
            Some(crate::workspace_trust::TrustLevel::ReadOnly)
        ) {
            Self::build_read_only_tools_args(config)
        } else {
            Self::build_allowed_tools_args(config, &request.allowed_tools)
        };
        let rules = config.to_rules();
        let session_id_path = config.session_id_path().map(|s| s.to_string());
        drop(registry);

        let state_machine: Box<dyn crate::normalizer::state_machines::StateMachine> = match provider
            .as_str()
        {
            "claude" => {
                Box::new(crate::normalizer::state_machines::claude::ClaudeStateMachine::new())
            }
            "gemini" => {
                Box::new(crate::normalizer::state_machines::gemini::GeminiStateMachine::new())
            }
            "kimi" => Box::new(crate::normalizer::state_machines::kimi::KimiStateMachine::new()),
            "qwen" => Box::new(crate::normalizer::state_machines::qwen::QwenStateMachine::new()),
            "codex" => Box::new(crate::normalizer::state_machines::codex::CodexStateMachine::new()),
            _ => Box::new(crate::normalizer::state_machines::generic::GenericStateMachine::new()),
        };
        let pipeline = Arc::new(Mutex::new(
            crate::normalizer::pipeline::NormalizerPipeline::new(
                rules,
                state_machine,
                provider.clone(),
            ),
        ));

        // Atomic check-and-activate: single lock scope eliminates TOCTOU window.
        // Between checking session state and setting it to Active, no other thread
        // can observe the session as non-Active and also proceed to spawn.
        let args = {
            let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(arc) = sessions.get(&session_id) {
                let mut existing = arc.lock().unwrap_or_else(|e| e.into_inner());
                if existing.status == SessionStatus::Active {
                    // The frontend already received the done event (which re-enabled input),
                    // but the async task that updates session status hasn't completed yet.
                    // Force-stop the stale session so the user can send a follow-up message.
                    if let Some(handle) = existing.abort_handle.take() {
                        handle.abort();
                    }
                    warn!(
                        "Transport: force-stopped stale active session={} to allow new message",
                        session_id
                    );
                }
                debug!(
                    "Transport: reusing session={} cli_session_id={:?}",
                    session_id, existing.cli_session_id
                );
                let cli_session_id = existing.cli_session_id.clone();
                // Set to Active immediately — while we still hold the lock
                existing.set_status(SessionStatus::Active);
                existing.request = request.clone();
                debug!("Transport: reactivated agent session={}", session_id);
                // Build args using pre-captured resume template if CLI session ID exists,
                // otherwise use the pre-built new-session args (programmatic_args).
                if let Some(ref cli_sid) = cli_session_id {
                    resume_args_template
                        .iter()
                        .map(|arg| {
                            arg.replace("{prompt}", &request.prompt)
                                .replace("{session_id}", cli_sid)
                                .replace("{model}", request.model.as_deref().unwrap_or(""))
                        })
                        .collect()
                } else {
                    args_for_new_session
                }
            } else {
                let mut session = AgentSession::new(request.clone(), CliMode::Structured);
                // Ensure session ID matches (AgentSession::new may generate a new one if request.session_id is None)
                session.id = session_id.clone();
                // Session starts as Active (set by AgentSession::new)
                debug!("Transport: created agent session={}", session_id);
                sessions.insert(session_id.clone(), session, session_id.clone());
                // New session — use pre-built args (no CLI session ID)
                args_for_new_session
            }
        };

        // Get retry policy for this provider (if configured)
        let retry_policy = {
            let policies = self
                .retry_policies
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            policies.get(&provider).cloned()
        };

        // Spawn CLI with retry loop for transient spawn failures
        let mut attempt: u32 = 0;
        let mut child = loop {
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

            info!("Transport: spawning CLI binary={} args={:?} permission_args={:?} allowed_tools_args={:?} attempt={}", binary, args, permission_args, allowed_tools_args, attempt);
            match cmd.spawn() {
                Ok(child) => break child,
                Err(e) => {
                    let spawn_err = crate::error::ReasonanceError::Transport {
                        provider: provider.clone(),
                        message: format!("Failed to spawn {}: {}", binary, e),
                        retryable: e.kind() != std::io::ErrorKind::NotFound, // binary missing is not retryable
                    };

                    // Check if we should retry this spawn failure.
                    // Map error::ErrorSeverity to agent_event::ErrorSeverity for the retry policy.
                    let agent_severity = match spawn_err.severity() {
                        crate::error::ErrorSeverity::Recoverable => {
                            Some(ErrorSeverity::Recoverable)
                        }
                        crate::error::ErrorSeverity::Degraded => Some(ErrorSeverity::Degraded),
                        crate::error::ErrorSeverity::Fatal => Some(ErrorSeverity::Fatal),
                    };
                    let should_retry = spawn_err.is_retryable()
                        && retry_policy.as_ref().map_or(false, |rp| {
                            rp.should_retry(None, agent_severity.as_ref(), attempt)
                        });

                    if should_retry {
                        let delay = retry_policy.as_ref().unwrap().delay_for_attempt(attempt);
                        warn!(
                            "Transport: spawn retry attempt {} after {}ms: {}",
                            attempt + 1,
                            delay.as_millis(),
                            spawn_err
                        );
                        attempt += 1;
                        // Sleep synchronously — we're not in an async context here and
                        // the session lock was already released above.
                        std::thread::sleep(delay);
                        continue;
                    }

                    error!(
                        "Transport: failed to spawn {} (no retry): {}",
                        binary, spawn_err
                    );
                    // Record failure in circuit breaker
                    if let Some((old, new)) =
                        self.circuit_breaker.record_failure(&circuit_id, &spawn_err)
                    {
                        Self::publish_circuit_state(&self.event_bus, &circuit_id, &old, &new);
                    }
                    // Restore session status since we set it to Active but spawn failed
                    let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(arc) = sessions.get(&session_id) {
                        arc.lock().unwrap_or_else(|e| e.into_inner()).set_status(
                            SessionStatus::Error {
                                severity: ErrorSeverity::Fatal,
                            },
                        );
                    }
                    return Err(spawn_err);
                }
            }
        };
        let stdout =
            child
                .stdout
                .take()
                .ok_or_else(|| crate::error::ReasonanceError::Transport {
                    provider: provider.clone(),
                    message: "Failed to capture stdout".to_string(),
                    retryable: false,
                })?;

        // Capture stderr and emit as warning events
        if let Some(stderr) = child.stderr.take() {
            let stderr_bus = self
                .event_bus
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            let stderr_sid = session_id.clone();
            tokio::spawn(async move {
                use tokio::io::{AsyncBufReadExt, BufReader};
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if !line.trim().is_empty() {
                        let error_event = AgentEvent::error(
                            &format!("[stderr] {}", line),
                            "STDERR",
                            ErrorSeverity::Recoverable,
                            "system",
                        );
                        if let Some(ref bus) = stderr_bus {
                            bus.publish(crate::event_bus::Event::from_agent_event(
                                "transport:error",
                                &stderr_sid,
                                &error_event,
                            ));
                        }
                    }
                }
            });
        }

        // Record successful spawn in circuit breaker
        if let Some((old, new)) = self.circuit_breaker.record_success(&circuit_id) {
            Self::publish_circuit_state(&self.event_bus, &circuit_id, &old, &new);
        }

        let sid = session_id.clone();
        // Get the session Arc directly — the spawned task only needs this session,
        // not the whole map. This avoids locking the map from the async task.
        let session_arc = self
            .sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&session_id)
            .expect("session must exist after send setup");
        let cb_ref = self.circuit_breaker.clone();
        let cb_circuit_id = circuit_id.clone();

        let cli_session_id_ref = Arc::new(Mutex::new(None::<String>));
        let cli_sid_for_reader = cli_session_id_ref.clone();
        let event_bus = self
            .event_bus
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .expect("EventBus must be set before send()");
        // Clone for the async task before moving into spawn_stream_reader
        let task_event_bus = event_bus.clone();
        let rx = spawn_stream_reader(
            stdout,
            pipeline,
            event_bus,
            sid.clone(),
            session_id_path,
            cli_sid_for_reader,
            None,
        );

        let session_for_task = session_arc.clone();
        let join_handle = tokio::spawn(async move {
            let _ = child.wait().await;

            // Store captured CLI session ID in the session
            {
                let captured = cli_session_id_ref
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                if let Some(ref cli_sid) = captured {
                    let mut sess = session_for_task.lock().unwrap_or_else(|e| e.into_inner());
                    sess.set_cli_session_id(cli_sid.clone());
                    log::info!(
                        "Transport: session={} stored CLI session ID={}",
                        sid,
                        cli_sid
                    );
                }
            }

            if let Ok(result) = rx.await {
                let mut sess = session_for_task.lock().unwrap_or_else(|e| e.into_inner());
                if result.error.is_some() {
                    // Record failure in circuit breaker for CLI-level errors
                    let err = crate::error::ReasonanceError::Transport {
                        provider: cb_circuit_id.clone(),
                        message: result.error.clone().unwrap_or_default(),
                        retryable: true,
                    };
                    if let Some((old, new)) = cb_ref.record_failure(&cb_circuit_id, &err) {
                        task_event_bus.publish(crate::event_bus::Event::new(
                            "transport:circuit-state",
                            serde_json::json!({
                                "circuit_id": cb_circuit_id,
                                "from": format!("{:?}", old),
                                "to": format!("{:?}", new),
                            }),
                            "circuit_breaker",
                        ));
                    }
                    sess.set_status(SessionStatus::Error {
                        severity: ErrorSeverity::Fatal,
                    });
                } else {
                    if let Some((old, new)) = cb_ref.record_success(&cb_circuit_id) {
                        task_event_bus.publish(crate::event_bus::Event::new(
                            "transport:circuit-state",
                            serde_json::json!({
                                "circuit_id": cb_circuit_id,
                                "from": format!("{:?}", old),
                                "to": format!("{:?}", new),
                            }),
                            "circuit_breaker",
                        ));
                    }
                    sess.set_status(SessionStatus::Terminated);
                }
            }
        });

        session_arc
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .set_abort_handle(join_handle.abort_handle());

        info!(
            "Transport: session={} started for provider={}",
            session_id, provider
        );
        Ok(session_id)
    }

    pub fn stop(&self, session_id: &str) -> Result<(), crate::error::ReasonanceError> {
        info!("Transport: stopping session={}", session_id);
        let session_arc = {
            let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            sessions.get(session_id).ok_or_else(|| {
                warn!(
                    "Transport: stop requested for unknown session={}",
                    session_id
                );
                crate::error::ReasonanceError::not_found("session", session_id)
            })?
        };
        let mut session = session_arc.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(handle) = session.abort_handle.take() {
            handle.abort();
        }
        session.set_status(SessionStatus::Terminated);
        info!("Transport: session={} stopped", session_id);
        Ok(())
    }

    pub fn get_status(
        &self,
        session_id: &str,
    ) -> Result<SessionStatus, crate::error::ReasonanceError> {
        let session_arc = {
            let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            sessions.get(session_id).ok_or_else(|| {
                warn!("Transport: get_status for unknown session={}", session_id);
                crate::error::ReasonanceError::not_found("session", session_id)
            })?
        };
        let session = session_arc.lock().unwrap_or_else(|e| e.into_inner());
        let status = session.status.clone();
        debug!("Transport: session={} status={:?}", session_id, status);
        Ok(status)
    }

    /// Set the EventBus. Called from `setup()` after the bus is constructed.
    pub fn set_event_bus(&self, bus: Arc<crate::event_bus::EventBus>) {
        *self.event_bus.lock().unwrap_or_else(|e| e.into_inner()) = Some(bus);
    }

    pub fn active_sessions(&self) -> Vec<String> {
        self.sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect()
    }

    pub fn registry(&self) -> Arc<Mutex<NormalizerRegistry>> {
        self.registry.clone()
    }

    /// Access the underlying TrackedMap for sweep_exclusive (used by periodic GC).
    pub fn sessions_map(&self) -> &Arc<Mutex<TrackedMap<String, AgentSession>>> {
        &self.sessions
    }

    /// Build permission args from the normalizer config, substituting `{project_root}` with the actual path.
    /// Only returns permission args when `yolo` is true. In non-yolo mode the CLI runs without
    /// permission flags and auto-denies tool use (reporting `permission_denials` in the result event),
    /// which the UI renders as approval prompts.
    #[allow(dead_code)] // Superseded by build_permission_args_with_trust; kept for tests
    fn build_permission_args(
        config: &crate::normalizer::TomlConfig,
        cwd: Option<&str>,
        yolo: bool,
    ) -> Vec<String> {
        if !yolo {
            return Vec::new();
        }
        let project_root = cwd.unwrap_or(".");
        config
            .cli
            .permission_args
            .iter()
            .map(|arg| arg.replace("{project_root}", project_root))
            .collect()
    }

    /// Build `--allowedTools tool1,tool2` args if the provider supports it and tools are provided.
    fn build_allowed_tools_args(
        config: &crate::normalizer::TomlConfig,
        allowed_tools: &Option<Vec<String>>,
    ) -> Vec<String> {
        match (allowed_tools, &config.cli.allowed_tools_arg) {
            (Some(tools), Some(flag)) if !tools.is_empty() => {
                vec![flag.clone(), tools.join(",")]
            }
            _ => Vec::new(),
        }
    }

    /// Build permission args considering both yolo flag and workspace trust level.
    #[allow(dead_code)] // Superseded by build_permission_args_from_eval; kept for tests
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
            return config
                .cli
                .permission_args
                .iter()
                .map(|arg| arg.replace("{project_root}", project_root))
                .collect();
        }

        match trust_level {
            Some(TrustLevel::Trusted) => {
                // Trusted workspace without yolo — still pass permission args for stdin=null CLIs
                config
                    .cli
                    .permission_args
                    .iter()
                    .map(|arg| arg.replace("{project_root}", project_root))
                    .collect()
            }
            Some(TrustLevel::ReadOnly) => {
                // Pass permission_args to avoid interactive prompts (stdin=null)
                // Tool restriction handled separately via read_only_tools_args
                config
                    .cli
                    .permission_args
                    .iter()
                    .map(|arg| arg.replace("{project_root}", project_root))
                    .collect()
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

    /// Build permission args based on the engine's evaluation result.
    ///
    /// - `Allow` (yolo or trusted) -> pass `--dangerously-skip-permissions` from normalizer config
    /// - `Confirm` with trusted/read_only trust -> pass permission args (stdin=null CLIs need them)
    /// - `Confirm` with untrusted -> no permission args (CLI handles interactively)
    fn build_permission_args_from_eval(
        config: &crate::normalizer::TomlConfig,
        cwd: Option<&str>,
        eval_result: &EvaluationResult,
        trust_level: Option<crate::workspace_trust::TrustLevel>,
    ) -> Vec<String> {
        let project_root = cwd.unwrap_or(".");

        match eval_result.decision {
            PermissionDecision::Allow => {
                // Engine says Allow — pass permission args to skip interactive prompts
                config
                    .cli
                    .permission_args
                    .iter()
                    .map(|arg| arg.replace("{project_root}", project_root))
                    .collect()
            }
            PermissionDecision::Confirm => {
                // Confirm mode: pass permission args only if workspace is trusted or
                // read-only (stdin=null CLIs need them to avoid hanging)
                match trust_level {
                    Some(crate::workspace_trust::TrustLevel::Trusted)
                    | Some(crate::workspace_trust::TrustLevel::ReadOnly) => config
                        .cli
                        .permission_args
                        .iter()
                        .map(|arg| arg.replace("{project_root}", project_root))
                        .collect(),
                    _ => Vec::new(),
                }
            }
            PermissionDecision::Deny { .. } => {
                // Deny is handled before we reach arg building — this branch
                // should never execute, but return empty for safety.
                Vec::new()
            }
        }
    }

    /// Publish a circuit state transition event to the EventBus.
    fn publish_circuit_state(
        event_bus: &Mutex<Option<Arc<crate::event_bus::EventBus>>>,
        circuit_id: &str,
        from: &crate::circuit_breaker::CircuitState,
        to: &crate::circuit_breaker::CircuitState,
    ) {
        let bus = event_bus.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref bus) = *bus {
            info!(
                "Circuit breaker state transition: {} {:?} -> {:?}",
                circuit_id, from, to
            );
            bus.publish(crate::event_bus::Event::new(
                "transport:circuit-state",
                serde_json::json!({
                    "circuit_id": circuit_id,
                    "from": format!("{:?}", from),
                    "to": format!("{:?}", to),
                }),
                "circuit_breaker",
            ));
        }
    }

    /// Publish an audit event for a permission engine decision.
    fn publish_permission_audit(
        event_bus: &Mutex<Option<Arc<crate::event_bus::EventBus>>>,
        eval_result: &EvaluationResult,
        session_id: &str,
    ) {
        let bus = event_bus.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref bus) = *bus {
            bus.publish(crate::event_bus::Event::new(
                "permission:decision",
                serde_json::json!({
                    "tool": eval_result.tool_name,
                    "decision": format!("{:?}", eval_result.decision),
                    "layer": eval_result.deciding_layer,
                    "session_id": session_id,
                    "trust_level": eval_result.trust_level,
                    "permission_level": eval_result.permission_level,
                }),
                "permission_engine",
            ));
        }
    }

    /// Build CLI args. `cli_session_id` is the session ID returned by the CLI
    /// (e.g., from a previous invocation), NOT the internal Reasonance session ID.
    /// Only when a real CLI session ID is provided do we use `resume_args`.
    fn build_cli_args(
        config: &crate::normalizer::TomlConfig,
        request: &AgentRequest,
        cli_session_id: Option<&str>,
    ) -> Vec<String> {
        let args_template = if cli_session_id.is_some() {
            &config.cli.resume_args
        } else {
            &config.cli.programmatic_args
        };

        args_template
            .iter()
            .map(|arg| {
                arg.replace("{prompt}", &request.prompt)
                    .replace("{session_id}", cli_session_id.unwrap_or(""))
                    .replace("{model}", request.model.as_deref().unwrap_or(""))
            })
            .collect()
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
        let memory = PermissionMemory::new();
        let policy = PolicyFile::new();
        let slot_registry = Mutex::new(crate::model_slots::ModelSlotRegistry::new());
        let settings = Mutex::new(crate::settings::LayeredSettings::new());

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
            yolo: true, // yolo so engine doesn't block on untrusted
        };
        let result = transport.send(
            request,
            &trust_store,
            &memory,
            &policy,
            &slot_registry,
            &settings,
        );
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
        let args_off =
            StructuredAgentTransport::build_permission_args(config, Some("/project"), false);
        assert!(args_off.is_empty());

        let args_on =
            StructuredAgentTransport::build_permission_args(config, Some("/project"), true);
        assert!(!args_on.is_empty());
        assert!(args_on
            .iter()
            .any(|a| a.contains("dangerously-skip-permissions")));
    }

    #[test]
    fn test_allowed_tools_args() {
        let transport = StructuredAgentTransport::new(Path::new("normalizers")).unwrap();
        let registry = transport.registry.lock().unwrap_or_else(|e| e.into_inner());
        let config = registry.get_config("claude").unwrap();

        // With tools and supported provider
        let tools = Some(vec!["Read".to_string(), "Edit".to_string()]);
        let args = StructuredAgentTransport::build_allowed_tools_args(config, &tools);
        assert_eq!(
            args,
            vec!["--allowedTools".to_string(), "Read,Edit".to_string()]
        );

        // Without tools
        let args_none = StructuredAgentTransport::build_allowed_tools_args(config, &None);
        assert!(args_none.is_empty());

        // Empty tools list
        let args_empty = StructuredAgentTransport::build_allowed_tools_args(config, &Some(vec![]));
        assert!(args_empty.is_empty());
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
            config,
            Some("/project"),
            true,
            Some(crate::workspace_trust::TrustLevel::Trusted),
        );
        assert!(!args.is_empty());

        // Trusted + not yolo → permission args (trusted workspace gets args for stdin=null CLIs)
        let args = StructuredAgentTransport::build_permission_args_with_trust(
            config,
            Some("/project"),
            false,
            Some(crate::workspace_trust::TrustLevel::Trusted),
        );
        assert!(!args.is_empty());

        // ReadOnly → permission args (to avoid interactive prompt) regardless of yolo
        let args = StructuredAgentTransport::build_permission_args_with_trust(
            config,
            Some("/project"),
            false,
            Some(crate::workspace_trust::TrustLevel::ReadOnly),
        );
        assert!(!args.is_empty());

        // yolo + Blocked → permission args (yolo overrides; send() blocks Blocked upstream)
        let args = StructuredAgentTransport::build_permission_args_with_trust(
            config,
            Some("/project"),
            true,
            Some(crate::workspace_trust::TrustLevel::Blocked),
        );
        assert!(!args.is_empty());

        // yolo + None trust → permission args (yolo bypasses trust gate)
        let args = StructuredAgentTransport::build_permission_args_with_trust(
            config,
            Some("/project"),
            true,
            None,
        );
        assert!(!args.is_empty());

        // not yolo + None trust → no permission args
        let args = StructuredAgentTransport::build_permission_args_with_trust(
            config,
            Some("/project"),
            false,
            None,
        );
        assert!(args.is_empty());

        // not yolo + Blocked → no permission args
        let args = StructuredAgentTransport::build_permission_args_with_trust(
            config,
            Some("/project"),
            false,
            Some(crate::workspace_trust::TrustLevel::Blocked),
        );
        assert!(args.is_empty());
    }

    #[test]
    fn test_build_permission_args_from_eval_allow() {
        let transport = StructuredAgentTransport::new(Path::new("normalizers")).unwrap();
        let registry = transport.registry.lock().unwrap_or_else(|e| e.into_inner());
        let config = registry.get_config("claude").unwrap();

        let eval = EvaluationResult {
            decision: PermissionDecision::Allow,
            deciding_layer: 4,
            tool_name: "*".to_string(),
            permission_level: "yolo".to_string(),
            trust_level: "trusted".to_string(),
        };

        let args = StructuredAgentTransport::build_permission_args_from_eval(
            config,
            Some("/project"),
            &eval,
            Some(crate::workspace_trust::TrustLevel::Trusted),
        );
        assert!(!args.is_empty());
        assert!(args
            .iter()
            .any(|a| a.contains("dangerously-skip-permissions")));
    }

    #[test]
    fn test_build_permission_args_from_eval_confirm_trusted() {
        let transport = StructuredAgentTransport::new(Path::new("normalizers")).unwrap();
        let registry = transport.registry.lock().unwrap_or_else(|e| e.into_inner());
        let config = registry.get_config("claude").unwrap();

        let eval = EvaluationResult {
            decision: PermissionDecision::Confirm,
            deciding_layer: 6,
            tool_name: "*".to_string(),
            permission_level: "ask".to_string(),
            trust_level: "trusted".to_string(),
        };

        // Confirm + trusted -> permission args (stdin=null CLIs)
        let args = StructuredAgentTransport::build_permission_args_from_eval(
            config,
            Some("/project"),
            &eval,
            Some(crate::workspace_trust::TrustLevel::Trusted),
        );
        assert!(!args.is_empty());
    }

    #[test]
    fn test_build_permission_args_from_eval_confirm_untrusted() {
        let transport = StructuredAgentTransport::new(Path::new("normalizers")).unwrap();
        let registry = transport.registry.lock().unwrap_or_else(|e| e.into_inner());
        let config = registry.get_config("claude").unwrap();

        let eval = EvaluationResult {
            decision: PermissionDecision::Confirm,
            deciding_layer: 6,
            tool_name: "*".to_string(),
            permission_level: "ask".to_string(),
            trust_level: "untrusted".to_string(),
        };

        // Confirm + no trust -> no permission args
        let args = StructuredAgentTransport::build_permission_args_from_eval(
            config,
            Some("/project"),
            &eval,
            None,
        );
        assert!(args.is_empty());
    }

    #[test]
    fn test_build_permission_args_from_eval_deny() {
        let transport = StructuredAgentTransport::new(Path::new("normalizers")).unwrap();
        let registry = transport.registry.lock().unwrap_or_else(|e| e.into_inner());
        let config = registry.get_config("claude").unwrap();

        let eval = EvaluationResult {
            decision: PermissionDecision::Deny {
                reason: "blocked".to_string(),
            },
            deciding_layer: 2,
            tool_name: "*".to_string(),
            permission_level: "ask".to_string(),
            trust_level: "blocked".to_string(),
        };

        let args = StructuredAgentTransport::build_permission_args_from_eval(
            config,
            Some("/project"),
            &eval,
            Some(crate::workspace_trust::TrustLevel::Blocked),
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
