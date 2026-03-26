use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::ReasonanceError;

/// Circuit breaker state.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Per-circuit configuration.
#[derive(Debug, Clone)]
pub struct CircuitConfig {
    pub failure_threshold: u32,
    pub cooldown: Duration,
    pub half_open_max_probes: u32,
}

impl Default for CircuitConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 3,
            cooldown: Duration::from_secs(60),
            half_open_max_probes: 1,
        }
    }
}

/// Internal per-circuit tracking data.
struct CircuitData {
    state: CircuitState,
    failure_count: u32,
    last_failure: Option<Instant>,
    last_state_change: Instant,
    config: CircuitConfig,
}

impl CircuitData {
    fn new(config: CircuitConfig) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            last_failure: None,
            last_state_change: Instant::now(),
            config,
        }
    }
}

/// Three-state circuit breaker with per-fingerprint error deduplication.
///
/// Tracks independent circuits by context ID (e.g. "transport:claude", "workflow:node-1").
/// Thread-safe via internal `Mutex` — suitable for Tauri managed state.
pub struct CircuitBreaker {
    circuits: Mutex<HashMap<String, CircuitData>>,
    /// Error fingerprint dedup: hash -> (count, first_seen)
    fingerprints: Mutex<HashMap<String, (u32, Instant)>>,
    fingerprint_window: Duration,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            circuits: Mutex::new(HashMap::new()),
            fingerprints: Mutex::new(HashMap::new()),
            fingerprint_window: Duration::from_secs(10),
        }
    }

    /// Register a circuit for the given context ID with custom config.
    pub fn register(&self, context_id: &str, config: CircuitConfig) {
        let mut circuits = self.circuits.lock().unwrap_or_else(|e| e.into_inner());
        circuits.insert(context_id.to_string(), CircuitData::new(config));
    }

    /// Check whether the circuit allows a request through.
    ///
    /// - **Closed / HalfOpen**: returns `Ok(())`
    /// - **Open**: if cooldown has elapsed, transitions to HalfOpen and returns `Ok(())`;
    ///   otherwise returns `Err`.
    pub fn check(&self, context_id: &str) -> Result<(), ReasonanceError> {
        let mut circuits = self.circuits.lock().unwrap_or_else(|e| e.into_inner());
        let data = circuits
            .entry(context_id.to_string())
            .or_insert_with(|| CircuitData::new(CircuitConfig::default()));

        match data.state {
            CircuitState::Closed | CircuitState::HalfOpen => Ok(()),
            CircuitState::Open => {
                // Check if cooldown has elapsed
                if data.last_state_change.elapsed() >= data.config.cooldown {
                    data.state = CircuitState::HalfOpen;
                    data.last_state_change = Instant::now();
                    Ok(())
                } else {
                    Err(ReasonanceError::Transport {
                        provider: context_id.to_string(),
                        message: format!(
                            "Circuit open for '{}' — cooldown {}s remaining",
                            context_id,
                            (data.config.cooldown.as_secs() as i64
                                - data.last_state_change.elapsed().as_secs() as i64)
                                .max(0)
                        ),
                        retryable: true,
                    })
                }
            }
        }
    }

    /// Record a successful operation — resets failure count and transitions HalfOpen -> Closed.
    pub fn record_success(&self, context_id: &str) {
        let mut circuits = self.circuits.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(data) = circuits.get_mut(context_id) {
            data.failure_count = 0;
            if data.state == CircuitState::HalfOpen {
                data.state = CircuitState::Closed;
                data.last_state_change = Instant::now();
            }
        }
    }

    /// Record a failure — increments count, may transition Closed -> Open or HalfOpen -> Open.
    pub fn record_failure(&self, context_id: &str, _error: &ReasonanceError) {
        let mut circuits = self.circuits.lock().unwrap_or_else(|e| e.into_inner());
        let data = circuits
            .entry(context_id.to_string())
            .or_insert_with(|| CircuitData::new(CircuitConfig::default()));

        data.failure_count += 1;
        data.last_failure = Some(Instant::now());

        match data.state {
            CircuitState::Closed => {
                if data.failure_count >= data.config.failure_threshold {
                    data.state = CircuitState::Open;
                    data.last_state_change = Instant::now();
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open goes straight back to open
                data.state = CircuitState::Open;
                data.last_state_change = Instant::now();
                // Keep failure_count so threshold is already met
            }
            CircuitState::Open => {
                // Already open, nothing to do
            }
        }
    }

    /// Get the current state of a circuit, if registered.
    pub fn get_state(&self, context_id: &str) -> Option<CircuitState> {
        let circuits = self.circuits.lock().unwrap_or_else(|e| e.into_inner());
        circuits.get(context_id).map(|d| d.state.clone())
    }

    /// Check if an error is a duplicate within the dedup window.
    ///
    /// Fingerprint = SHA-256 of (variant name + first 100 chars of message).
    /// Returns `true` if this fingerprint was already seen within the window.
    pub fn is_duplicate(&self, error: &ReasonanceError) -> bool {
        let fingerprint = self.compute_fingerprint(error);
        let now = Instant::now();

        let mut fps = self.fingerprints.lock().unwrap_or_else(|e| e.into_inner());

        // Prune expired entries opportunistically
        fps.retain(|_, (_, first_seen)| now.duration_since(*first_seen) < self.fingerprint_window);

        if let Some((count, first_seen)) = fps.get_mut(&fingerprint) {
            if now.duration_since(*first_seen) < self.fingerprint_window {
                *count += 1;
                return true;
            }
        }

        // First occurrence — record it
        fps.insert(fingerprint, (1, now));
        false
    }

    /// Compute a SHA-256 fingerprint for error deduplication.
    fn compute_fingerprint(&self, error: &ReasonanceError) -> String {
        let variant = match error {
            ReasonanceError::Io { .. } => "Io",
            ReasonanceError::Serialization { .. } => "Serialization",
            ReasonanceError::Transport { .. } => "Transport",
            ReasonanceError::PermissionDenied { .. } => "PermissionDenied",
            ReasonanceError::NotFound { .. } => "NotFound",
            ReasonanceError::Validation { .. } => "Validation",
            ReasonanceError::Workflow { .. } => "Workflow",
            ReasonanceError::Config { .. } => "Config",
            ReasonanceError::Security { .. } => "Security",
            ReasonanceError::Timeout { .. } => "Timeout",
            ReasonanceError::Internal { .. } => "Internal",
        };

        let msg = error.to_string();
        let truncated: String = msg.chars().take(100).collect();
        let input = format!("{}:{}", variant, truncated);

        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_retryable_error(msg: &str) -> ReasonanceError {
        ReasonanceError::Transport {
            provider: "test".to_string(),
            message: msg.to_string(),
            retryable: true,
        }
    }

    #[test]
    fn test_new_circuit_starts_closed() {
        let cb = CircuitBreaker::new();
        cb.register("ctx", CircuitConfig::default());
        assert_eq!(cb.get_state("ctx"), Some(CircuitState::Closed));
    }

    #[test]
    fn test_failures_below_threshold_stay_closed() {
        let cb = CircuitBreaker::new();
        cb.register("ctx", CircuitConfig::default()); // threshold = 3
        let err = make_retryable_error("fail");

        cb.record_failure("ctx", &err);
        cb.record_failure("ctx", &err);
        assert_eq!(cb.get_state("ctx"), Some(CircuitState::Closed));
    }

    #[test]
    fn test_failures_at_threshold_transition_to_open() {
        let cb = CircuitBreaker::new();
        cb.register("ctx", CircuitConfig::default());
        let err = make_retryable_error("fail");

        for _ in 0..3 {
            cb.record_failure("ctx", &err);
        }
        assert_eq!(cb.get_state("ctx"), Some(CircuitState::Open));
    }

    #[test]
    fn test_open_check_returns_error() {
        let cb = CircuitBreaker::new();
        cb.register("ctx", CircuitConfig::default());
        let err = make_retryable_error("fail");

        for _ in 0..3 {
            cb.record_failure("ctx", &err);
        }

        let result = cb.check("ctx");
        assert!(result.is_err());
        let e = result.unwrap_err();
        assert!(e.is_retryable());
    }

    #[test]
    fn test_open_to_half_open_after_cooldown() {
        let cb = CircuitBreaker::new();
        cb.register(
            "ctx",
            CircuitConfig {
                failure_threshold: 1,
                cooldown: Duration::from_millis(0), // instant cooldown for test
                half_open_max_probes: 1,
            },
        );
        let err = make_retryable_error("fail");
        cb.record_failure("ctx", &err);
        assert_eq!(cb.get_state("ctx"), Some(CircuitState::Open));

        // Cooldown is 0ms, so check should transition to HalfOpen
        let result = cb.check("ctx");
        assert!(result.is_ok());
        assert_eq!(cb.get_state("ctx"), Some(CircuitState::HalfOpen));
    }

    #[test]
    fn test_half_open_success_transitions_to_closed() {
        let cb = CircuitBreaker::new();
        cb.register(
            "ctx",
            CircuitConfig {
                failure_threshold: 1,
                cooldown: Duration::from_millis(0),
                half_open_max_probes: 1,
            },
        );
        let err = make_retryable_error("fail");
        cb.record_failure("ctx", &err);
        // transition to half-open
        let _ = cb.check("ctx");
        assert_eq!(cb.get_state("ctx"), Some(CircuitState::HalfOpen));

        cb.record_success("ctx");
        assert_eq!(cb.get_state("ctx"), Some(CircuitState::Closed));
    }

    #[test]
    fn test_half_open_failure_transitions_to_open() {
        let cb = CircuitBreaker::new();
        cb.register(
            "ctx",
            CircuitConfig {
                failure_threshold: 1,
                cooldown: Duration::from_millis(0),
                half_open_max_probes: 1,
            },
        );
        let err = make_retryable_error("fail");
        cb.record_failure("ctx", &err);
        let _ = cb.check("ctx");
        assert_eq!(cb.get_state("ctx"), Some(CircuitState::HalfOpen));

        cb.record_failure("ctx", &err);
        assert_eq!(cb.get_state("ctx"), Some(CircuitState::Open));
    }

    #[test]
    fn test_fingerprint_dedup_within_window() {
        let cb = CircuitBreaker::new();
        let err = make_retryable_error("rate limited");

        assert!(!cb.is_duplicate(&err)); // first time — not a dup
        assert!(cb.is_duplicate(&err)); // second time — dup
        assert!(cb.is_duplicate(&err)); // third time — still dup
    }

    #[test]
    fn test_fingerprint_expires_after_window() {
        let cb = CircuitBreaker {
            circuits: Mutex::new(HashMap::new()),
            fingerprints: Mutex::new(HashMap::new()),
            fingerprint_window: Duration::from_millis(0), // instant expiry
        };
        let err = make_retryable_error("rate limited");

        assert!(!cb.is_duplicate(&err)); // first time
                                         // With 0ms window, the entry is already expired
        assert!(!cb.is_duplicate(&err)); // should NOT be a dup
    }

    #[test]
    fn test_unregistered_circuit_defaults() {
        let cb = CircuitBreaker::new();
        // No register() call — check should auto-create with defaults
        let result = cb.check("unknown");
        assert!(result.is_ok());
        assert_eq!(cb.get_state("unknown"), Some(CircuitState::Closed));
    }

    #[test]
    fn test_different_errors_have_different_fingerprints() {
        let cb = CircuitBreaker::new();
        let err1 = make_retryable_error("rate limited");
        let err2 = make_retryable_error("connection refused");

        assert!(!cb.is_duplicate(&err1));
        assert!(!cb.is_duplicate(&err2)); // different error — not a dup
        assert!(cb.is_duplicate(&err1)); // same as first — dup
    }

    #[test]
    fn test_success_resets_failure_count() {
        let cb = CircuitBreaker::new();
        cb.register("ctx", CircuitConfig::default()); // threshold = 3
        let err = make_retryable_error("fail");

        cb.record_failure("ctx", &err);
        cb.record_failure("ctx", &err);
        // 2 failures, then success resets
        cb.record_success("ctx");

        // Need 3 more failures to trip
        cb.record_failure("ctx", &err);
        cb.record_failure("ctx", &err);
        assert_eq!(cb.get_state("ctx"), Some(CircuitState::Closed));

        cb.record_failure("ctx", &err);
        assert_eq!(cb.get_state("ctx"), Some(CircuitState::Open));
    }

    #[test]
    fn test_closed_check_returns_ok() {
        let cb = CircuitBreaker::new();
        cb.register("ctx", CircuitConfig::default());
        assert!(cb.check("ctx").is_ok());
    }
}
