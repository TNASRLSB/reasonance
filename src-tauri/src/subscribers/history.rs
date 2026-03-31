use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::agent_event::AgentEvent;
use crate::error::ReasonanceError;
use crate::event_bus::{Event, EventHandler};

/// In-memory history recorder that implements the EventBus `EventHandler` trait.
///
/// Stores events grouped by session ID, extracted from the generic `Event.payload`.
/// Replaces the former `transport::event_bus::HistoryRecorder`.
pub struct HistoryRecorder {
    /// In-memory event history grouped by session ID. Plain HashMap: bounded
    /// by active session count, entries grow per-session but are read-only
    /// lookups — no lifecycle tracking needed.
    history: Arc<Mutex<HashMap<String, Vec<AgentEvent>>>>,
}

impl Default for HistoryRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoryRecorder {
    pub fn new() -> Self {
        info!("HistoryRecorder: created");
        Self {
            history: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Retrieve recorded events for a session. Returns an empty vec for unknown sessions.
    pub fn get_events(&self, session_id: &str) -> Vec<AgentEvent> {
        self.history
            .lock()
            .unwrap_or_else(|e| {
                warn!("HistoryRecorder: lock poisoned in get_events, recovering");
                e.into_inner()
            })
            .get(session_id)
            .cloned()
            .unwrap_or_default()
    }

}

impl EventHandler for HistoryRecorder {
    fn handle(&self, event: &Event) -> Result<(), ReasonanceError> {
        let session_id = match event.payload.get("session_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => {
                trace!(
                    "HistoryRecorder: ignoring event {} — no session_id in payload",
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
                    "HistoryRecorder: ignoring event {} — could not parse AgentEvent from payload",
                    event.id
                );
                return Ok(());
            }
        };

        trace!(
            "HistoryRecorder: recording event type={:?} for session={}",
            agent_event.event_type,
            session_id
        );

        self.history
            .lock()
            .unwrap_or_else(|e| {
                warn!("HistoryRecorder: lock poisoned in handle, recovering");
                e.into_inner()
            })
            .entry(session_id.to_string())
            .or_default()
            .push(agent_event);

        Ok(())
    }

    fn id(&self) -> &str {
        "history-recorder"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_event::AgentEvent;
    use crate::event_bus::Event;

    /// Build a bus `Event` from an `AgentEvent` using the standard conversion.
    fn make_event(session_id: &str, agent_event: &AgentEvent) -> Event {
        Event::from_agent_event("agent:stream", session_id, agent_event)
    }

    #[test]
    fn handle_stores_events_retrievable_via_get_events() {
        let recorder = HistoryRecorder::new();

        let ae1 = AgentEvent::text("hello", "claude");
        let ae2 = AgentEvent::text("world", "claude");

        recorder.handle(&make_event("s1", &ae1)).unwrap();
        recorder.handle(&make_event("s1", &ae2)).unwrap();

        let events = recorder.get_events("s1");
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn invalid_payload_is_silently_ignored() {
        let recorder = HistoryRecorder::new();

        // Event with no session_id or agent event in payload
        let event = Event::new("agent:stream", serde_json::json!({"random": true}), "test");
        recorder.handle(&event).unwrap();

        // Event with session_id but no parseable agent event
        let event = Event::new(
            "agent:stream",
            serde_json::json!({"session_id": "s1", "event": "not-an-agent-event"}),
            "test",
        );
        recorder.handle(&event).unwrap();

        // No events should have been recorded
        assert_eq!(recorder.get_events("s1").len(), 0);
    }

    #[test]
    fn multiple_sessions_tracked_independently() {
        let recorder = HistoryRecorder::new();

        let ae_a = AgentEvent::text("alpha", "claude");
        let ae_b1 = AgentEvent::text("beta-1", "openai");
        let ae_b2 = AgentEvent::text("beta-2", "openai");

        recorder.handle(&make_event("session-a", &ae_a)).unwrap();
        recorder.handle(&make_event("session-b", &ae_b1)).unwrap();
        recorder.handle(&make_event("session-b", &ae_b2)).unwrap();

        assert_eq!(recorder.get_events("session-a").len(), 1);
        assert_eq!(recorder.get_events("session-b").len(), 2);
        assert_eq!(recorder.get_events("session-c").len(), 0);
    }
}
