use crate::agent_event::ErrorSeverity;
use std::time::Duration;

#[derive(Debug, Clone)]
#[allow(dead_code)] // Roadmap: retry logic wired into transport send loop
pub struct RetryPolicy {
    pub max_retries: u32,
    pub backoff: BackoffStrategy,
    pub retryable_codes: Vec<String>,
    pub retryable_severities: Vec<ErrorSeverity>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Used by RetryPolicy
pub enum BackoffStrategy {
    Fixed { delay_ms: u64 },
    Exponential { base_ms: u64, max_ms: u64 },
}

impl RetryPolicy {
    pub fn from_toml_config(config: &crate::normalizer::TomlConfig) -> Self {
        let retry = config.retry.as_ref();
        let max_retries = retry.and_then(|r| r.max_retries).unwrap_or(0);
        let retryable_codes = retry
            .and_then(|r| r.retryable_codes.clone())
            .unwrap_or_default();

        let backoff = retry
            .and_then(|r| r.backoff.as_ref())
            .and_then(|b| {
                let table = b.as_table()?;
                let strategy = table.get("strategy")?.as_str()?;
                match strategy {
                    "exponential" => {
                        let base = table.get("base_ms")?.as_integer()? as u64;
                        let max = table.get("max_ms")?.as_integer()? as u64;
                        Some(BackoffStrategy::Exponential { base_ms: base, max_ms: max })
                    }
                    "fixed" => {
                        let delay = table.get("delay_ms")?.as_integer()? as u64;
                        Some(BackoffStrategy::Fixed { delay_ms: delay })
                    }
                    _ => None,
                }
            })
            .unwrap_or(BackoffStrategy::Exponential { base_ms: 1000, max_ms: 30000 });

        Self {
            max_retries,
            backoff,
            retryable_codes,
            retryable_severities: vec![ErrorSeverity::Recoverable],
        }
    }

    #[allow(dead_code)] // Roadmap: retry logic
    pub fn should_retry(&self, error_code: Option<&str>, severity: Option<&ErrorSeverity>, attempt: u32) -> bool {
        if attempt >= self.max_retries {
            return false;
        }
        if let Some(code) = error_code {
            if self.retryable_codes.iter().any(|c| c == code) {
                return true;
            }
        }
        if let Some(sev) = severity {
            if self.retryable_severities.contains(sev) {
                return true;
            }
        }
        false
    }

    #[allow(dead_code)] // Roadmap: retry logic
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        match &self.backoff {
            BackoffStrategy::Fixed { delay_ms } => Duration::from_millis(*delay_ms),
            BackoffStrategy::Exponential { base_ms, max_ms } => {
                let delay = (*base_ms as u64).saturating_mul(2u64.saturating_pow(attempt));
                Duration::from_millis(delay.min(*max_ms))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_event::ErrorSeverity;

    fn test_policy() -> RetryPolicy {
        RetryPolicy {
            max_retries: 3,
            backoff: BackoffStrategy::Exponential { base_ms: 1000, max_ms: 30000 },
            retryable_codes: vec!["overloaded".to_string(), "rate_limit".to_string()],
            retryable_severities: vec![ErrorSeverity::Recoverable],
        }
    }

    #[test]
    fn test_should_retry_matching_code() {
        let policy = test_policy();
        assert!(policy.should_retry(Some("overloaded"), None, 0));
        assert!(policy.should_retry(Some("rate_limit"), None, 1));
    }

    #[test]
    fn test_should_not_retry_unknown_code() {
        let policy = test_policy();
        assert!(!policy.should_retry(Some("auth_error"), None, 0));
    }

    #[test]
    fn test_should_retry_matching_severity() {
        let policy = test_policy();
        assert!(policy.should_retry(None, Some(&ErrorSeverity::Recoverable), 0));
    }

    #[test]
    fn test_should_not_retry_fatal() {
        let policy = test_policy();
        assert!(!policy.should_retry(None, Some(&ErrorSeverity::Fatal), 0));
    }

    #[test]
    fn test_should_not_retry_exhausted() {
        let policy = test_policy();
        assert!(!policy.should_retry(Some("overloaded"), None, 3));
    }

    #[test]
    fn test_exponential_backoff() {
        let policy = test_policy();
        assert_eq!(policy.delay_for_attempt(0), Duration::from_millis(1000));
        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(2000));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(4000));
        assert_eq!(policy.delay_for_attempt(5), Duration::from_millis(30000)); // capped
    }

    #[test]
    fn test_fixed_backoff() {
        let policy = RetryPolicy {
            max_retries: 3,
            backoff: BackoffStrategy::Fixed { delay_ms: 500 },
            retryable_codes: vec![],
            retryable_severities: vec![],
        };
        assert_eq!(policy.delay_for_attempt(0), Duration::from_millis(500));
        assert_eq!(policy.delay_for_attempt(5), Duration::from_millis(500));
    }

    #[test]
    fn test_from_toml_config() {
        let toml_str = r#"
[cli]
name = "test"
binary = "test"

[retry]
max_retries = 2
backoff = { strategy = "exponential", base_ms = 500, max_ms = 10000 }
retryable_codes = ["overloaded"]
"#;
        let config = crate::normalizer::TomlConfig::parse(toml_str).unwrap();
        let policy = RetryPolicy::from_toml_config(&config);
        assert_eq!(policy.max_retries, 2);
        assert!(policy.retryable_codes.contains(&"overloaded".to_string()));
        assert_eq!(policy.delay_for_attempt(0), Duration::from_millis(500));
    }
}
