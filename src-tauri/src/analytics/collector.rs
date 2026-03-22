use super::{SessionMetrics, ErrorRecord, ProviderAnalytics, ModelAnalytics, DailyStats, TimeRange};
use super::store::AnalyticsStore;
use crate::agent_event::{AgentEvent, AgentEventType, ErrorSeverity};
use crate::transport::event_bus::{AgentEventSubscriber, EventFilter};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct AnalyticsCollector {
    active: Mutex<HashMap<String, SessionMetrics>>,
    store: Arc<AnalyticsStore>,
}

impl AnalyticsCollector {
    pub fn new(store: Arc<AnalyticsStore>) -> Self {
        Self {
            active: Mutex::new(HashMap::new()),
            store,
        }
    }

    pub fn get_active_sessions(&self) -> Vec<SessionMetrics> {
        self.active.lock().unwrap().values().cloned().collect()
    }

    pub fn get_session_metrics(&self, session_id: &str) -> Option<SessionMetrics> {
        // Check active first
        if let Some(m) = self.active.lock().unwrap().get(session_id) {
            return Some(m.clone());
        }
        // Then check completed
        self.store.all_completed().into_iter()
            .find(|m| m.session_id == session_id)
    }
}

impl AgentEventSubscriber for AnalyticsCollector {
    fn on_event(&self, session_id: &str, event: &AgentEvent) {
        let mut active = self.active.lock().unwrap();

        // Handle Done separately to avoid borrow conflict:
        // entry() borrows `active` mutably, and remove() would need another mutable borrow.
        if event.event_type == AgentEventType::Done {
            if let Some(mut m) = active.remove(session_id) {
                m.ended_at = Some(event.timestamp);
                drop(active); // release lock before I/O
                if let Err(e) = self.store.append(&m) {
                    eprintln!("AnalyticsCollector: failed to flush session {}: {}", session_id, e);
                }
            }
            return;
        }

        // Bootstrap new session on first event
        let metrics = active.entry(session_id.to_string()).or_insert_with(|| {
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
                        metrics.peak_context_usage.map_or(cu, |current| current.max(cu))
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
                metrics.errors.push(ErrorRecord {
                    timestamp: event.timestamp,
                    code: event.metadata.error_code.clone().unwrap_or_else(|| "unknown".to_string()),
                    severity: event.metadata.error_severity.clone().unwrap_or(ErrorSeverity::Fatal),
                    recovered: false,
                });
            }
            _ => {}
        }
    }

    fn filter(&self) -> Option<EventFilter> {
        Some(EventFilter {
            event_types: Some(vec![
                AgentEventType::Usage,
                AgentEventType::Metrics,
                AgentEventType::ToolUse,
                AgentEventType::Error,
                AgentEventType::Done,
            ]),
            providers: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_store() -> (Arc<AnalyticsStore>, tempfile::TempDir) {
        let dir = tempfile::TempDir::new().unwrap();
        let store = Arc::new(AnalyticsStore::new(dir.path()).unwrap());
        (store, dir)
    }

    #[test]
    fn test_accumulate_usage_tokens() {
        let (store, _dir) = make_store();
        let collector = AnalyticsCollector::new(store);

        let event = AgentEvent::usage(100, 50, "claude");
        collector.on_event("s1", &event);

        let active = collector.active.lock().unwrap();
        let m = active.get("s1").unwrap();
        assert_eq!(m.input_tokens, 100);
        assert_eq!(m.output_tokens, 50);
    }

    #[test]
    fn test_accumulate_multiple_usage() {
        let (store, _dir) = make_store();
        let collector = AnalyticsCollector::new(store);

        collector.on_event("s1", &AgentEvent::usage(100, 50, "claude"));
        collector.on_event("s1", &AgentEvent::usage(200, 100, "claude"));

        let active = collector.active.lock().unwrap();
        let m = active.get("s1").unwrap();
        assert_eq!(m.input_tokens, 300);
        assert_eq!(m.output_tokens, 150);
    }

    #[test]
    fn test_accumulate_tool_use() {
        let (store, _dir) = make_store();
        let collector = AnalyticsCollector::new(store);

        let event = AgentEvent::tool_use("read_file", r#"{"path":"test"}"#, "claude");
        collector.on_event("s1", &event);
        collector.on_event("s1", &event);

        let active = collector.active.lock().unwrap();
        let m = active.get("s1").unwrap();
        assert_eq!(m.tools_used.get("read_file"), Some(&2));
    }

    #[test]
    fn test_accumulate_error_with_recovery() {
        let (store, _dir) = make_store();
        let collector = AnalyticsCollector::new(store);

        let err = AgentEvent::error("rate limit", "overloaded", ErrorSeverity::Recoverable, "claude");
        collector.on_event("s1", &err);

        // Another event arrives — previous error is recovered
        collector.on_event("s1", &AgentEvent::usage(100, 50, "claude"));

        let active = collector.active.lock().unwrap();
        let m = active.get("s1").unwrap();
        assert_eq!(m.errors.len(), 1);
        assert!(m.errors[0].recovered);
    }

    #[test]
    fn test_done_flushes_to_store() {
        let (store, _dir) = make_store();
        let collector = AnalyticsCollector::new(store.clone());

        collector.on_event("s1", &AgentEvent::usage(100, 50, "claude"));
        collector.on_event("s1", &AgentEvent::done("s1", "claude"));

        // Active should be empty now
        assert!(collector.active.lock().unwrap().is_empty());

        // Store should have 1 completed session
        assert_eq!(store.all_completed().len(), 1);
        let m = &store.all_completed()[0];
        assert_eq!(m.session_id, "s1");
        assert_eq!(m.input_tokens, 100);
        assert!(m.ended_at.is_some());
    }

    #[test]
    fn test_active_sessions() {
        let (store, _dir) = make_store();
        let collector = AnalyticsCollector::new(store);

        collector.on_event("s1", &AgentEvent::usage(100, 50, "claude"));
        collector.on_event("s2", &AgentEvent::usage(200, 100, "gemini"));

        let active = collector.get_active_sessions();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn test_session_metrics_query() {
        let (store, _dir) = make_store();
        let collector = AnalyticsCollector::new(store);

        collector.on_event("s1", &AgentEvent::usage(100, 50, "claude"));
        collector.on_event("s1", &AgentEvent::done("s1", "claude"));

        let m = collector.get_session_metrics("s1");
        assert!(m.is_some());
        assert_eq!(m.unwrap().input_tokens, 100);

        assert!(collector.get_session_metrics("nonexistent").is_none());
    }
}
