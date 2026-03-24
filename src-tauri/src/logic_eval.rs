use rhai::{Engine, Scope};
use serde_json::Value;

pub struct LogicEvaluator {
    engine: Engine,
}

impl LogicEvaluator {
    pub fn new() -> Self {
        let mut engine = Engine::new();
        // Sandbox: prevent DoS from untrusted rule expressions
        engine.set_max_operations(10_000);
        engine.set_max_string_size(4096);
        engine.set_max_array_size(1024);
        engine.set_max_map_size(256);
        engine.set_max_call_levels(10);
        Self { engine }
    }

    /// Evaluate a rule expression against the previous node's output.
    /// Returns true/false for routing to onTrue/onFalse edges.
    pub fn evaluate(&self, rule: &str, output: &Value) -> Result<bool, String> {
        let mut scope = Scope::new();
        // Convert serde_json::Value → Rhai Dynamic for nested access
        let dynamic_output = rhai::serde::to_dynamic(output.clone())
            .map_err(|e| format!("Failed to convert output to Rhai dynamic: {}", e))?;
        scope.push("output", dynamic_output);

        self.engine
            .eval_with_scope::<bool>(&mut scope, rule)
            .map_err(|e| format!("Rule evaluation failed: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_bool_rule() {
        let eval = LogicEvaluator::new();
        let output = serde_json::json!({"errors": 0});
        assert!(eval.evaluate("output.errors == 0", &output).unwrap());
    }

    #[test]
    fn test_false_condition() {
        let eval = LogicEvaluator::new();
        let output = serde_json::json!({"errors": 3});
        assert!(!eval.evaluate("output.errors == 0", &output).unwrap());
    }

    #[test]
    fn test_string_comparison() {
        let eval = LogicEvaluator::new();
        let output = serde_json::json!({"status": "success"});
        assert!(eval
            .evaluate(r#"output.status == "success""#, &output)
            .unwrap());
    }

    #[test]
    fn test_invalid_rule_returns_error() {
        let eval = LogicEvaluator::new();
        let output = serde_json::json!({});
        assert!(eval.evaluate("invalid!!!", &output).is_err());
    }

    #[test]
    fn test_numeric_comparison() {
        let eval = LogicEvaluator::new();
        let output = serde_json::json!({"score": 85});
        assert!(eval.evaluate("output.score > 70", &output).unwrap());
    }
}
