use log::{debug, info, warn};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfHealConfig {
    pub max_iterations: u32,
    pub feedback_to_llm: bool,
}

impl Default for SelfHealConfig {
    fn default() -> Self {
        Self {
            max_iterations: 3,
            feedback_to_llm: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
#[allow(dead_code)] // Used in tests; reserved for future structured return from heal command
pub enum SelfHealResult {
    Fixed {
        new_toml: String,
        iterations: u32,
        tokens_used: u64,
    },
    Failed {
        best_attempt: Option<String>,
        remaining_failures: Vec<crate::normalizer_health::TestCaseResult>,
    },
}

impl SelfHealResult {
    #[allow(dead_code)] // Used in tests
    pub fn is_fixed(&self) -> bool {
        matches!(self, SelfHealResult::Fixed { .. })
    }
}

/// Build the prompt sent to the LLM to fix a broken normalizer TOML.
pub fn build_heal_prompt(
    current_toml: &str,
    failures: &[crate::normalizer_health::TestCaseResult],
    previous_attempt: Option<&str>,
) -> String {
    info!(
        "Building self-heal prompt: {} failures, previous_attempt={}",
        failures.len(),
        previous_attempt.is_some()
    );
    let mut prompt = String::new();

    prompt.push_str("You are a REASONANCE normalizer engineer. A normalizer TOML file defines how to parse JSON output from an LLM CLI into structured AgentEvent objects.\n\n");
    prompt.push_str("The current normalizer TOML is:\n\n```toml\n");
    prompt.push_str(current_toml);
    prompt.push_str("\n```\n\n");

    prompt.push_str("The following test cases are FAILING:\n\n");
    for failure in failures {
        prompt.push_str(&format!(
            "- **{}**: {}\n",
            failure.name,
            failure.failure_reason.as_deref().unwrap_or("unknown")
        ));
    }

    if let Some(prev) = previous_attempt {
        prompt.push_str("\n\nPrevious attempt (also failed):\n\n```toml\n");
        prompt.push_str(prev);
        prompt.push_str("\n```\n\n");
        prompt.push_str("Analyze why the previous attempt failed and try a different approach.\n");
    }

    prompt.push_str("\nFix the TOML so all tests pass. Return ONLY the complete fixed TOML wrapped in a ```toml code block. Do not change the [cli] section (name/binary must stay the same).\n");

    prompt
}

/// Extract TOML content from an LLM response that wraps it in a code block.
pub fn extract_toml_from_response(response: &str) -> Option<String> {
    debug!(
        "Extracting TOML from LLM response ({} chars)",
        response.len()
    );
    // Look for ```toml ... ``` block
    let toml_start = response.find("```toml")?;
    let content_start = response[toml_start..].find('\n')? + toml_start + 1;
    let content_end = response[content_start..].find("```")? + content_start;
    let toml = response[content_start..content_end].trim().to_string();
    if toml.is_empty() {
        warn!("Extracted TOML block was empty");
        None
    } else {
        debug!("Successfully extracted TOML ({} chars)", toml.len());
        Some(toml)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_heal_config_defaults() {
        let config = SelfHealConfig::default();
        assert_eq!(config.max_iterations, 3);
        assert!(config.feedback_to_llm);
    }

    #[test]
    fn test_build_heal_prompt_first_iteration() {
        let current_toml = r#"[cli]
name = "test"
binary = "test"
"#;
        let failures = vec![crate::normalizer_health::TestCaseResult {
            name: "basic_text".into(),
            passed: false,
            failure_reason: Some("Required event 'text' not found".into()),
        }];

        let prompt = build_heal_prompt(current_toml, &failures, None);
        assert!(prompt.contains("name = \"test\""));
        assert!(prompt.contains("basic_text"));
        assert!(prompt.contains("Required event 'text' not found"));
        assert!(!prompt.contains("Previous attempt")); // no previous attempt
    }

    #[test]
    fn test_build_heal_prompt_with_previous_attempt() {
        let current_toml = "[cli]\nname = \"test\"\n";
        let failures = vec![crate::normalizer_health::TestCaseResult {
            name: "basic_text".into(),
            passed: false,
            failure_reason: Some("still failing".into()),
        }];
        let previous = "previous toml attempt";

        let prompt = build_heal_prompt(current_toml, &failures, Some(previous));
        assert!(prompt.contains("Previous attempt"));
        assert!(prompt.contains("previous toml attempt"));
    }

    #[test]
    fn test_extract_toml_from_response() {
        let response = r#"Here is the fixed TOML:

```toml
[cli]
name = "test"
binary = "test"
```

This should fix the issue."#;

        let extracted = extract_toml_from_response(response);
        assert!(extracted.is_some());
        let toml = extracted.unwrap();
        assert!(toml.contains("[cli]"));
        assert!(toml.contains("name = \"test\""));
    }

    #[test]
    fn test_extract_toml_no_block() {
        let response = "No TOML here, just text.";
        let extracted = extract_toml_from_response(response);
        assert!(extracted.is_none());
    }

    #[test]
    fn test_self_heal_result_fixed() {
        let result = SelfHealResult::Fixed {
            new_toml: "fixed".into(),
            iterations: 2,
            tokens_used: 1500,
        };
        assert!(result.is_fixed());
    }

    #[test]
    fn test_self_heal_result_failed() {
        let result = SelfHealResult::Failed {
            best_attempt: Some("attempt".into()),
            remaining_failures: vec![],
        };
        assert!(!result.is_fixed());
    }
}
