use std::sync::Arc;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::agent_event::{AgentEvent, AgentEventType};
use crate::analytics::collector::AnalyticsCollector;
use crate::error::ReasonanceError;
use crate::event_bus::{Event, EventHandler};

/// Synchronous event handler that bridges the new EventBus to the existing
/// `AnalyticsCollector`. Extracts `session_id` and `AgentEvent` from the
/// generic `Event.payload` and delegates to `AnalyticsCollector::on_event`.
///
/// Only processes events whose `AgentEventType` matches the collector's
/// filter (Usage, Metrics, ToolUse, Error, Done).
pub struct AnalyticsHandler {
    collector: Arc<AnalyticsCollector>,
}

impl AnalyticsHandler {
    pub fn new(collector: Arc<AnalyticsCollector>) -> Self {
        info!("AnalyticsHandler(v2): created");
        Self { collector }
    }
}

/// The event types that the analytics collector cares about.
const ANALYTICS_EVENT_TYPES: &[AgentEventType] = &[
    AgentEventType::Usage,
    AgentEventType::Metrics,
    AgentEventType::ToolUse,
    AgentEventType::Error,
    AgentEventType::Done,
];

impl EventHandler for AnalyticsHandler {
    fn handle(&self, event: &Event) -> Result<(), ReasonanceError> {
        let session_id = match event.payload.get("session_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => {
                trace!(
                    "AnalyticsHandler(v2): ignoring event {} -- no session_id in payload",
                    event.id
                );
                return Ok(());
            }
        };

        let agent_event: AgentEvent = match serde_json::from_value(
            event
                .payload
                .get("event")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        ) {
            Ok(evt) => evt,
            Err(_) => {
                trace!(
                    "AnalyticsHandler(v2): ignoring event {} -- could not parse AgentEvent",
                    event.id
                );
                return Ok(());
            }
        };

        // Apply the same filter the old AgentEventSubscriber impl used.
        if !ANALYTICS_EVENT_TYPES.contains(&agent_event.event_type) {
            return Ok(());
        }

        trace!(
            "AnalyticsHandler(v2): forwarding event type={:?} for session={}",
            agent_event.event_type,
            session_id
        );

        // Delegate to the existing collector logic.
        self.collector.on_event(session_id, &agent_event);

        Ok(())
    }

    fn id(&self) -> &str {
        "analytics-handler-v2"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_event::AgentEvent;
    use crate::analytics::store::AnalyticsStore;
    use crate::event_bus::Event;
    use crate::storage::InMemoryBackend;

    async fn make_store() -> Arc<AnalyticsStore> {
        let backend = Arc::new(InMemoryBackend::new());
        Arc::new(AnalyticsStore::new(backend).await.unwrap())
    }

    fn make_event(session_id: &str, agent_event: &AgentEvent) -> Event {
        Event::from_agent_event("transport:event", session_id, agent_event)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn handles_usage_event() {
        let store = make_store().await;
        let collector = Arc::new(AnalyticsCollector::new(store));
        let handler = AnalyticsHandler::new(collector.clone());

        let ae = AgentEvent::usage(100, 50, "claude");
        handler.handle(&make_event("s1", &ae)).unwrap();

        let active = collector.get_active_sessions();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].input_tokens, 100);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn ignores_text_event() {
        let store = make_store().await;
        let collector = Arc::new(AnalyticsCollector::new(store));
        let handler = AnalyticsHandler::new(collector.clone());

        let ae = AgentEvent::text("hello", "claude");
        handler.handle(&make_event("s1", &ae)).unwrap();

        // Text events are filtered out -- no active sessions
        let active = collector.get_active_sessions();
        assert!(active.is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn ignores_invalid_payload() {
        let store = make_store().await;
        let collector = Arc::new(AnalyticsCollector::new(store));
        let handler = AnalyticsHandler::new(collector.clone());

        let event = Event::new(
            "transport:event",
            serde_json::json!({"random": true}),
            "test",
        );
        handler.handle(&event).unwrap();

        assert!(collector.get_active_sessions().is_empty());
    }
}
