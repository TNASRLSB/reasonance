use super::store::AnalyticsStore;
use super::{
    DailyStats, ErrorRecord, ModelAnalytics, ProviderAnalytics, SessionMetrics, TimeRange,
};
use crate::agent_event::{AgentEvent, AgentEventType, ErrorSeverity};
use log::{debug, error, trace, warn};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct AnalyticsCollector {
    active: Mutex<HashMap<String, SessionMetrics>>,
    store: Arc<AnalyticsStore>,
}

impl AnalyticsCollector {
    pub fn new(store: Arc<AnalyticsStore>) -> Self {
        debug!("AnalyticsCollector initialized");
        Self {
            active: Mutex::new(HashMap::new()),
            store,
        }
    }

    pub fn get_active_sessions(&self) -> Vec<SessionMetrics> {
        self.active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .cloned()
            .collect()
    }

    pub fn get_session_metrics(&self, session_id: &str) -> Option<SessionMetrics> {
        // Check active first
        if let Some(m) = self
            .active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(session_id)
        {
            return Some(m.clone());
        }
        // Then check completed (avoid cloning the entire Vec)
        self.store.with_completed(|completed| {
            completed
                .iter()
                .find(|m| m.session_id == session_id)
                .cloned()
        })
    }

    pub fn get_provider_analytics(
        &self,
        provider: &str,
        range: Option<TimeRange>,
    ) -> ProviderAnalytics {
        debug!("Computing provider analytics for provider={}", provider);
        self.store.with_completed(|completed| {
            let sessions: Vec<&SessionMetrics> = completed
                .iter()
                .filter(|m| m.provider == provider)
                .filter(|m| Self::in_range(m, &range))
                .collect();
            Self::aggregate_provider(provider, &sessions.into_iter().cloned().collect::<Vec<_>>())
        })
    }

    pub fn compare_providers(&self, range: Option<TimeRange>) -> Vec<ProviderAnalytics> {
        debug!("Comparing providers across all completed sessions");
        self.store.with_completed(|completed| {
            let mut by_provider: HashMap<String, Vec<&SessionMetrics>> = HashMap::new();
            for m in completed {
                if Self::in_range(m, &range) {
                    by_provider.entry(m.provider.clone()).or_default().push(m);
                }
            }
            by_provider
                .into_iter()
                .map(|(provider, sessions)| {
                    let owned: Vec<SessionMetrics> = sessions.into_iter().cloned().collect();
                    Self::aggregate_provider(&provider, &owned)
                })
                .collect()
        })
    }

    pub fn get_model_breakdown(
        &self,
        provider: &str,
        range: Option<TimeRange>,
    ) -> Vec<ModelAnalytics> {
        debug!("Computing model breakdown for provider={}", provider);
        self.store.with_completed(|completed| {
            let sessions: Vec<&SessionMetrics> = completed
                .iter()
                .filter(|m| m.provider == provider)
                .filter(|m| Self::in_range(m, &range))
                .collect();

            let mut by_model: HashMap<String, Vec<&&SessionMetrics>> = HashMap::new();
            for m in &sessions {
                by_model.entry(m.model.clone()).or_default().push(m);
            }

            by_model
                .into_iter()
                .map(|(model, sessions)| {
                    let count = sessions.len() as u64;
                    let total_input: u64 = sessions.iter().map(|s| s.input_tokens).sum();
                    let total_output: u64 = sessions.iter().map(|s| s.output_tokens).sum();
                    let durations: Vec<u64> =
                        sessions.iter().filter_map(|s| s.duration_ms).collect();
                    let avg_dur = if durations.is_empty() {
                        0.0
                    } else {
                        durations.iter().sum::<u64>() as f64 / durations.len() as f64
                    };
                    let total_errors: u64 = sessions.iter().map(|s| s.errors.len() as u64).sum();

                    let tps = if avg_dur > 0.0 {
                        (total_output as f64 / count as f64) / (avg_dur / 1000.0)
                    } else {
                        0.0
                    };

                    ModelAnalytics {
                        model,
                        provider: provider.to_string(),
                        session_count: count,
                        avg_input_tokens: total_input as f64 / count as f64,
                        avg_output_tokens: total_output as f64 / count as f64,
                        avg_duration_ms: avg_dur,
                        avg_tokens_per_second: tps as f32,
                        error_rate: if count > 0 {
                            total_errors as f32 / count as f32
                        } else {
                            0.0
                        },
                    }
                })
                .collect()
        })
    }

    pub fn get_daily_stats(&self, provider: Option<&str>, days: u32) -> Vec<DailyStats> {
        debug!(
            "Computing daily stats for provider={:?}, days={}",
            provider, days
        );
        self.store.with_completed(|completed| {
            let mut by_date: HashMap<String, Vec<&SessionMetrics>> = HashMap::new();

            for m in completed {
                if let Some(p) = provider {
                    if m.provider != p {
                        continue;
                    }
                }
                // Convert ms timestamp to date string
                let secs = m.started_at / 1000;
                let date = unix_secs_to_date_string(secs);
                by_date.entry(date).or_default().push(m);
            }

            let mut stats: Vec<DailyStats> = by_date
                .into_iter()
                .map(|(date, sessions)| {
                    let count = sessions.len() as u64;
                    let total_in: u64 = sessions.iter().map(|s| s.input_tokens).sum();
                    let total_out: u64 = sessions.iter().map(|s| s.output_tokens).sum();
                    let errors: u64 = sessions.iter().map(|s| s.errors.len() as u64).sum();
                    let durs: Vec<u64> = sessions.iter().filter_map(|s| s.duration_ms).collect();
                    let avg_dur = if durs.is_empty() {
                        0.0
                    } else {
                        durs.iter().sum::<u64>() as f64 / durs.len() as f64
                    };

                    DailyStats {
                        date,
                        provider: provider.map(|s| s.to_string()),
                        sessions: count,
                        input_tokens: total_in,
                        output_tokens: total_out,
                        errors,
                        avg_duration_ms: avg_dur,
                    }
                })
                .collect();

            stats.sort_by(|a, b| a.date.cmp(&b.date));
            // Keep only last N days
            if stats.len() > days as usize {
                stats = stats.split_off(stats.len() - days as usize);
            }
            stats
        }) // end with_completed
    }

    fn in_range(m: &SessionMetrics, range: &Option<TimeRange>) -> bool {
        match range {
            None => true,
            Some(r) => {
                if let Some(from) = r.from {
                    if m.started_at < from {
                        return false;
                    }
                }
                if let Some(to) = r.to {
                    if m.started_at >= to {
                        return false;
                    }
                }
                true
            }
        }
    }

    fn aggregate_provider(provider: &str, sessions: &[SessionMetrics]) -> ProviderAnalytics {
        let count = sessions.len() as u64;
        if count == 0 {
            return ProviderAnalytics {
                provider: provider.to_string(),
                total_sessions: 0,
                total_input_tokens: 0,
                total_output_tokens: 0,
                total_cache_creation_tokens: 0,
                total_cache_read_tokens: 0,
                cache_hit_rate: 0.0,
                total_errors: 0,
                recovered_errors: 0,
                error_rate: 0.0,
                avg_duration_ms: 0.0,
                avg_tokens_per_second: 0.0,
                most_used_model: String::new(),
                total_tool_invocations: 0,
                total_cost_usd: None,
            };
        }

        let total_in: u64 = sessions.iter().map(|s| s.input_tokens).sum();
        let total_out: u64 = sessions.iter().map(|s| s.output_tokens).sum();
        let total_cache_create: u64 = sessions.iter().map(|s| s.cache_creation_tokens).sum();
        let total_cache_read: u64 = sessions.iter().map(|s| s.cache_read_tokens).sum();
        let total_errors: u64 = sessions.iter().map(|s| s.errors.len() as u64).sum();
        let recovered: u64 = sessions
            .iter()
            .flat_map(|s| s.errors.iter())
            .filter(|e| e.recovered)
            .count() as u64;
        let total_tools: u64 = sessions
            .iter()
            .map(|s| s.tools_used.values().sum::<u32>() as u64)
            .sum();

        let durs: Vec<u64> = sessions.iter().filter_map(|s| s.duration_ms).collect();
        let avg_dur = if durs.is_empty() {
            0.0
        } else {
            durs.iter().sum::<u64>() as f64 / durs.len() as f64
        };

        let cache_denom = total_in + total_cache_read;
        let cache_hit = if cache_denom > 0 {
            total_cache_read as f32 / cache_denom as f32
        } else {
            0.0
        };

        let tps = if avg_dur > 0.0 {
            ((total_out as f64 / count as f64) / (avg_dur / 1000.0)) as f32
        } else {
            0.0
        };

        // Most used model
        let mut model_counts: HashMap<&str, u64> = HashMap::new();
        for s in sessions {
            *model_counts.entry(&s.model).or_insert(0) += 1;
        }
        let most_used = model_counts
            .into_iter()
            .max_by_key(|(_, c)| *c)
            .map(|(m, _)| m.to_string())
            .unwrap_or_default();

        let cost: Option<f64> = {
            let costs: Vec<f64> = sessions.iter().filter_map(|s| s.total_cost_usd).collect();
            if costs.is_empty() {
                None
            } else {
                Some(costs.iter().sum())
            }
        };

        ProviderAnalytics {
            provider: provider.to_string(),
            total_sessions: count,
            total_input_tokens: total_in,
            total_output_tokens: total_out,
            total_cache_creation_tokens: total_cache_create,
            total_cache_read_tokens: total_cache_read,
            cache_hit_rate: cache_hit,
            total_errors,
            recovered_errors: recovered,
            error_rate: total_errors as f32 / count as f32,
            avg_duration_ms: avg_dur,
            avg_tokens_per_second: tps,
            most_used_model: most_used,
            total_tool_invocations: total_tools,
            total_cost_usd: cost,
        }
    }
}

/// Convert Unix seconds to "YYYY-MM-DD" date string (no chrono dependency).
fn unix_secs_to_date_string(secs: u64) -> String {
    let days = secs / 86400;
    let (year, month, day) = days_to_ymd(days as i64);
    format!("{:04}-{:02}-{:02}", year, month, day)
}

fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

impl AnalyticsCollector {
    /// Process an agent event for analytics tracking.
    /// Extracts metrics, token counts, tool usage, errors, and session lifecycle.
    pub fn on_event(&self, session_id: &str, event: &AgentEvent) {
        let mut active = self.active.lock().unwrap_or_else(|e| e.into_inner());

        // Handle Done separately to avoid borrow conflict:
        // entry() borrows `active` mutably, and remove() would need another mutable borrow.
        if event.event_type == AgentEventType::Done {
            if let Some(mut m) = active.remove(session_id) {
                debug!(
                    "Session ended: session_id={}, input_tokens={}, output_tokens={}",
                    session_id, m.input_tokens, m.output_tokens
                );
                m.ended_at = Some(event.timestamp);
                drop(active); // release lock before I/O
                let result = {
                    let rt = tokio::runtime::Handle::current();
                    // Use block_in_place so this works both from a sync thread
                    // (production EventHandler) and from within a tokio runtime (tests).
                    tokio::task::block_in_place(|| rt.block_on(self.store.append(&m)))
                };
                if let Err(e) = result {
                    error!("Failed to flush session {} to store: {}", session_id, e);
                }
            }
            return;
        }

        // Bootstrap new session on first event
        let metrics = active.entry(session_id.to_string()).or_insert_with(|| {
            debug!(
                "Session tracking started: session_id={}, provider={}",
                session_id, event.metadata.provider
            );
            SessionMetrics::new(
                session_id,
                &event.metadata.provider,
                event.metadata.model.as_deref().unwrap_or(""),
                event.timestamp,
            )
        });

        // Any non-Error event means the session continued past the last error → mark recovered
        if event.event_type != AgentEventType::Error {
            if let Some(last) = metrics.errors.last_mut() {
                if !last.recovered {
                    last.recovered = true;
                }
            }
        }

        trace!(
            "Processing event: session_id={}, type={:?}",
            session_id,
            event.event_type
        );

        match event.event_type {
            AgentEventType::Usage => {
                if let Some(t) = event.metadata.input_tokens {
                    metrics.input_tokens += t;
                }
                if let Some(t) = event.metadata.output_tokens {
                    metrics.output_tokens += t;
                }
                if let Some(t) = event.metadata.cache_creation_tokens {
                    metrics.cache_creation_tokens += t;
                }
                if let Some(t) = event.metadata.cache_read_tokens {
                    metrics.cache_read_tokens += t;
                }
                // Last-write-wins fields
                if let Some(v) = event.metadata.duration_ms {
                    metrics.duration_ms = Some(v);
                }
                if let Some(v) = event.metadata.duration_api_ms {
                    metrics.duration_api_ms = Some(v);
                }
                if let Some(v) = event.metadata.num_turns {
                    metrics.num_turns = v;
                }
                if event.metadata.stop_reason.is_some() {
                    metrics.stop_reason = event.metadata.stop_reason.clone();
                }
                if event.metadata.total_cost_usd.is_some() {
                    metrics.total_cost_usd = event.metadata.total_cost_usd;
                }
            }
            AgentEventType::Metrics => {
                // context_tokens is not stored directly; peak_context_usage captures the derived metric
                if let Some(cu) = event.metadata.context_usage {
                    metrics.peak_context_usage = Some(
                        metrics
                            .peak_context_usage
                            .map_or(cu, |current| current.max(cu)),
                    );
                }
                if let Some(v) = event.metadata.max_context_tokens {
                    metrics.max_context_tokens = Some(v);
                }
            }
            AgentEventType::ToolUse => {
                if let Some(ref name) = event.metadata.tool_name {
                    *metrics.tools_used.entry(name.clone()).or_insert(0) += 1;
                }
            }
            AgentEventType::Error => {
                warn!(
                    "Error recorded for session_id={}, code={:?}, severity={:?}",
                    session_id, event.metadata.error_code, event.metadata.error_severity
                );
                metrics.errors.push(ErrorRecord {
                    timestamp: event.timestamp,
                    code: event
                        .metadata
                        .error_code
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string()),
                    severity: event
                        .metadata
                        .error_severity
                        .clone()
                        .unwrap_or(ErrorSeverity::Fatal),
                    recovered: false,
                });
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InMemoryBackend;

    async fn make_store() -> Arc<AnalyticsStore> {
        let backend = Arc::new(InMemoryBackend::new());
        Arc::new(AnalyticsStore::new(backend).await.unwrap())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_accumulate_usage_tokens() {
        let store = make_store().await;
        let collector = AnalyticsCollector::new(store);

        let event = AgentEvent::usage(100, 50, "claude");
        collector.on_event("s1", &event);

        let active = collector.active.lock().unwrap_or_else(|e| e.into_inner());
        let m = active.get("s1").unwrap();
        assert_eq!(m.input_tokens, 100);
        assert_eq!(m.output_tokens, 50);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_accumulate_multiple_usage() {
        let store = make_store().await;
        let collector = AnalyticsCollector::new(store);

        collector.on_event("s1", &AgentEvent::usage(100, 50, "claude"));
        collector.on_event("s1", &AgentEvent::usage(200, 100, "claude"));

        let active = collector.active.lock().unwrap_or_else(|e| e.into_inner());
        let m = active.get("s1").unwrap();
        assert_eq!(m.input_tokens, 300);
        assert_eq!(m.output_tokens, 150);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_accumulate_tool_use() {
        let store = make_store().await;
        let collector = AnalyticsCollector::new(store);

        let event = AgentEvent::tool_use("read_file", r#"{"path":"test"}"#, "claude");
        collector.on_event("s1", &event);
        collector.on_event("s1", &event);

        let active = collector.active.lock().unwrap_or_else(|e| e.into_inner());
        let m = active.get("s1").unwrap();
        assert_eq!(m.tools_used.get("read_file"), Some(&2));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_accumulate_error_with_recovery() {
        let store = make_store().await;
        let collector = AnalyticsCollector::new(store);

        let err = AgentEvent::error(
            "rate limit",
            "overloaded",
            ErrorSeverity::Recoverable,
            "claude",
        );
        collector.on_event("s1", &err);

        // Another event arrives -- previous error is recovered
        collector.on_event("s1", &AgentEvent::usage(100, 50, "claude"));

        let active = collector.active.lock().unwrap_or_else(|e| e.into_inner());
        let m = active.get("s1").unwrap();
        assert_eq!(m.errors.len(), 1);
        assert!(m.errors[0].recovered);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_done_flushes_to_store() {
        let store = make_store().await;
        let collector = AnalyticsCollector::new(store.clone());

        collector.on_event("s1", &AgentEvent::usage(100, 50, "claude"));
        collector.on_event("s1", &AgentEvent::done("s1", "claude"));

        // Active should be empty now
        assert!(collector
            .active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_empty());

        // Store should have 1 completed session
        assert_eq!(store.all_completed().len(), 1);
        let m = &store.all_completed()[0];
        assert_eq!(m.session_id, "s1");
        assert_eq!(m.input_tokens, 100);
        assert!(m.ended_at.is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_active_sessions() {
        let store = make_store().await;
        let collector = AnalyticsCollector::new(store);

        collector.on_event("s1", &AgentEvent::usage(100, 50, "claude"));
        collector.on_event("s2", &AgentEvent::usage(200, 100, "gemini"));

        let active = collector.get_active_sessions();
        assert_eq!(active.len(), 2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_session_metrics_query() {
        let store = make_store().await;
        let collector = AnalyticsCollector::new(store);

        collector.on_event("s1", &AgentEvent::usage(100, 50, "claude"));
        collector.on_event("s1", &AgentEvent::done("s1", "claude"));

        let m = collector.get_session_metrics("s1");
        assert!(m.is_some());
        assert_eq!(m.unwrap().input_tokens, 100);

        assert!(collector.get_session_metrics("nonexistent").is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_provider_analytics() {
        let store = make_store().await;
        let collector = AnalyticsCollector::new(store);

        // Two claude sessions
        collector.on_event("s1", &AgentEvent::usage(100, 50, "claude"));
        collector.on_event("s1", &AgentEvent::done("s1", "claude"));
        collector.on_event("s2", &AgentEvent::usage(200, 100, "claude"));
        collector.on_event("s2", &AgentEvent::done("s2", "claude"));

        let analytics = collector.get_provider_analytics("claude", None);
        assert_eq!(analytics.total_sessions, 2);
        assert_eq!(analytics.total_input_tokens, 300);
        assert_eq!(analytics.total_output_tokens, 150);
        assert_eq!(analytics.error_rate, 0.0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_compare_providers() {
        let store = make_store().await;
        let collector = AnalyticsCollector::new(store);

        collector.on_event("s1", &AgentEvent::usage(100, 50, "claude"));
        collector.on_event("s1", &AgentEvent::done("s1", "claude"));
        collector.on_event("s2", &AgentEvent::usage(200, 100, "gemini"));
        collector.on_event("s2", &AgentEvent::done("s2", "gemini"));

        let comparison = collector.compare_providers(None);
        assert_eq!(comparison.len(), 2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_model_breakdown() {
        let store = make_store().await;
        let collector = AnalyticsCollector::new(store);

        // We need events with model set
        let mut event1 = AgentEvent::usage(100, 50, "claude");
        event1.metadata.model = Some("claude-sonnet-4-6".to_string());
        collector.on_event("s1", &event1);
        collector.on_event("s1", &AgentEvent::done("s1", "claude"));

        let mut event2 = AgentEvent::usage(200, 100, "claude");
        event2.metadata.model = Some("claude-opus-4-6".to_string());
        collector.on_event("s2", &event2);
        collector.on_event("s2", &AgentEvent::done("s2", "claude"));

        let breakdown = collector.get_model_breakdown("claude", None);
        assert_eq!(breakdown.len(), 2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_daily_stats() {
        let store = make_store().await;
        let collector = AnalyticsCollector::new(store);

        collector.on_event("s1", &AgentEvent::usage(100, 50, "claude"));
        collector.on_event("s1", &AgentEvent::done("s1", "claude"));

        let stats = collector.get_daily_stats(None, 7);
        assert!(!stats.is_empty());
        assert_eq!(stats[0].sessions, 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_provider_analytics_empty() {
        let store = make_store().await;
        let collector = AnalyticsCollector::new(store);

        let analytics = collector.get_provider_analytics("nonexistent", None);
        assert_eq!(analytics.total_sessions, 0);
    }
}
