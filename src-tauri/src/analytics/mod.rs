pub mod collector;
pub mod store;

use crate::agent_event::ErrorSeverity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub session_id: String,
    pub provider: String,
    pub model: String,
    pub started_at: u64,
    pub ended_at: Option<u64>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub duration_ms: Option<u64>,
    pub duration_api_ms: Option<u64>,
    pub num_turns: u32,
    pub tools_used: HashMap<String, u32>,
    pub stop_reason: Option<String>,
    pub peak_context_usage: Option<f64>,
    pub max_context_tokens: Option<u64>,
    pub total_cost_usd: Option<f64>,
    pub errors: Vec<ErrorRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecord {
    pub timestamp: u64,
    pub code: String,
    pub severity: ErrorSeverity,
    pub recovered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderAnalytics {
    pub provider: String,
    pub total_sessions: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_creation_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub cache_hit_rate: f32,
    pub total_errors: u64,
    pub recovered_errors: u64,
    pub error_rate: f32,
    pub avg_duration_ms: f64,
    pub avg_tokens_per_second: f32,
    pub most_used_model: String,
    pub total_tool_invocations: u64,
    pub total_cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelAnalytics {
    pub model: String,
    pub provider: String,
    pub session_count: u64,
    pub avg_input_tokens: f64,
    pub avg_output_tokens: f64,
    pub avg_duration_ms: f64,
    pub avg_tokens_per_second: f32,
    pub error_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStats {
    pub date: String,
    pub provider: Option<String>,
    pub sessions: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub errors: u64,
    pub avg_duration_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub from: Option<u64>,
    pub to: Option<u64>,
}

impl SessionMetrics {
    pub fn new(session_id: &str, provider: &str, model: &str, started_at: u64) -> Self {
        Self {
            session_id: session_id.to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            started_at,
            ended_at: None,
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            duration_ms: None,
            duration_api_ms: None,
            num_turns: 0,
            tools_used: HashMap::new(),
            stop_reason: None,
            peak_context_usage: None,
            max_context_tokens: None,
            total_cost_usd: None,
            errors: Vec::new(),
        }
    }
}
