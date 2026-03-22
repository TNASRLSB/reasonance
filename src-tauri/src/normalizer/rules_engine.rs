use serde_json::Value;
use std::collections::HashMap;

pub fn resolve_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in path.split('.') {
        match current.get(segment) {
            Some(v) => current = v,
            None => return None,
        }
    }
    Some(current)
}

/// Evaluates a simple expression against a JSON value.
/// Supports: ==, !=, &&, ||, exists()
/// Missing fields evaluate to null; comparisons with null return false.
pub fn eval_expr(value: &Value, expr: &str) -> bool {
    let expr = expr.trim();

    // Handle || first (lower precedence — split outermost so && binds tighter)
    if let Some((left, right)) = split_operator(expr, "||") {
        return eval_expr(value, left) || eval_expr(value, right);
    }

    // Handle && (higher precedence)
    if let Some((left, right)) = split_operator(expr, "&&") {
        return eval_expr(value, left) && eval_expr(value, right);
    }

    // Handle exists()
    if expr.starts_with("exists(") && expr.ends_with(')') {
        let path = &expr[7..expr.len() - 1].trim();
        return resolve_path(value, path).is_some();
    }

    // Handle !=
    if let Some((left, right)) = split_operator(expr, "!=") {
        return !compare_eq(value, left.trim(), right.trim());
    }

    // Handle ==
    if let Some((left, right)) = split_operator(expr, "==") {
        return compare_eq(value, left.trim(), right.trim());
    }

    false
}

fn split_operator<'a>(expr: &'a str, op: &str) -> Option<(&'a str, &'a str)> {
    let mut in_quotes = false;
    let bytes = expr.as_bytes();
    let op_bytes = op.as_bytes();

    for i in 0..bytes.len() {
        if bytes[i] == b'"' {
            in_quotes = !in_quotes;
        }
        if !in_quotes && i + op_bytes.len() <= bytes.len() && &bytes[i..i + op_bytes.len()] == op_bytes {
            return Some((&expr[..i], &expr[i + op_bytes.len()..]));
        }
    }
    None
}

fn compare_eq(value: &Value, left: &str, right: &str) -> bool {
    let resolved = match resolve_path(value, left) {
        Some(v) => v,
        None => return false,
    };

    if right.starts_with('"') && right.ends_with('"') {
        let literal = &right[1..right.len() - 1];
        return resolved.as_str() == Some(literal);
    }

    if let Ok(n) = right.parse::<i64>() {
        return resolved.as_i64() == Some(n);
    }
    if let Ok(n) = right.parse::<f64>() {
        return resolved.as_f64() == Some(n);
    }

    if right == "true" {
        return resolved.as_bool() == Some(true);
    }
    if right == "false" {
        return resolved.as_bool() == Some(false);
    }

    if right == "null" {
        return resolved.is_null();
    }

    false
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub name: String,
    pub when: String,
    pub emit: String,
    pub mappings: HashMap<String, String>,
}

pub fn find_matching_rule<'a>(rules: &'a [Rule], value: &Value) -> Option<&'a Rule> {
    rules.iter().find(|r| eval_expr(value, &r.when))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_json() -> Value {
        json!({
            "type": "content_block_delta",
            "delta": {
                "type": "text_delta",
                "text": "Hello world"
            },
            "index": 0
        })
    }

    // --- dot-path resolution ---

    #[test]
    fn test_resolve_path_simple() {
        let v = sample_json();
        assert_eq!(resolve_path(&v, "type"), Some(&json!("content_block_delta")));
    }

    #[test]
    fn test_resolve_path_nested() {
        let v = sample_json();
        assert_eq!(resolve_path(&v, "delta.type"), Some(&json!("text_delta")));
    }

    #[test]
    fn test_resolve_path_deep() {
        let v = sample_json();
        assert_eq!(resolve_path(&v, "delta.text"), Some(&json!("Hello world")));
    }

    #[test]
    fn test_resolve_path_missing() {
        let v = sample_json();
        assert_eq!(resolve_path(&v, "nonexistent"), None);
    }

    #[test]
    fn test_resolve_path_missing_nested() {
        let v = sample_json();
        assert_eq!(resolve_path(&v, "delta.nonexistent"), None);
    }

    // --- expression evaluation ---

    #[test]
    fn test_eval_simple_equality() {
        let v = sample_json();
        assert!(eval_expr(&v, r#"type == "content_block_delta""#));
    }

    #[test]
    fn test_eval_simple_inequality() {
        let v = sample_json();
        assert!(eval_expr(&v, r#"type != "error""#));
    }

    #[test]
    fn test_eval_nested_equality() {
        let v = sample_json();
        assert!(eval_expr(&v, r#"delta.type == "text_delta""#));
    }

    #[test]
    fn test_eval_and_operator() {
        let v = sample_json();
        assert!(eval_expr(&v, r#"type == "content_block_delta" && delta.type == "text_delta""#));
    }

    #[test]
    fn test_eval_and_one_false() {
        let v = sample_json();
        assert!(!eval_expr(&v, r#"type == "content_block_delta" && delta.type == "thinking_delta""#));
    }

    #[test]
    fn test_eval_or_operator() {
        let v = sample_json();
        assert!(eval_expr(&v, r#"type == "error" || delta.type == "text_delta""#));
    }

    #[test]
    fn test_eval_exists_true() {
        let v = sample_json();
        assert!(eval_expr(&v, r#"exists(delta.text)"#));
    }

    #[test]
    fn test_eval_exists_false() {
        let v = sample_json();
        assert!(!eval_expr(&v, r#"exists(delta.thinking)"#));
    }

    #[test]
    fn test_eval_missing_field_returns_false() {
        let v = sample_json();
        assert!(!eval_expr(&v, r#"nonexistent == "value""#));
    }

    #[test]
    fn test_eval_numeric_equality() {
        let v = sample_json();
        assert!(eval_expr(&v, "index == 0"));
    }

    // --- Rule matching ---

    #[test]
    fn test_rule_first_match_wins() {
        let rules = vec![
            Rule { name: "specific".into(), when: r#"type == "content_block_delta" && delta.type == "text_delta""#.into(), emit: "text".into(), mappings: Default::default() },
            Rule { name: "generic".into(), when: r#"type == "content_block_delta""#.into(), emit: "thinking".into(), mappings: Default::default() },
        ];
        let v = sample_json();
        let matched = find_matching_rule(&rules, &v);
        assert_eq!(matched.unwrap().name, "specific");
    }

    #[test]
    fn test_rule_no_match_returns_none() {
        let rules = vec![
            Rule { name: "error".into(), when: r#"type == "error""#.into(), emit: "error".into(), mappings: Default::default() },
        ];
        let v = sample_json();
        let matched = find_matching_rule(&rules, &v);
        assert!(matched.is_none());
    }
}
