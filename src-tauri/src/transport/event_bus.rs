use crate::agent_event::{AgentEvent, AgentEventType};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct EventFilter {
    pub event_types: Option<Vec<AgentEventType>>,
    pub providers: Option<Vec<String>>,
}

impl EventFilter {
    pub fn matches(&self, event: &AgentEvent) -> bool {
        if let Some(ref types) = self.event_types {
            if !types.contains(&event.event_type) {
                return false;
            }
        }
        if let Some(ref providers) = self.providers {
            if !providers.contains(&event.metadata.provider) {
                return false;
            }
        }
        true
    }
}

pub trait AgentEventSubscriber: Send + Sync {
    fn on_event(&self, session_id: &str, event: &AgentEvent);
    fn filter(&self) -> Option<EventFilter> {
        None
    }
}

pub struct AgentEventBus {
    subscribers: Mutex<Vec<Box<dyn AgentEventSubscriber>>>,
}

impl AgentEventBus {
    pub fn new() -> Self {
        Self {
            subscribers: Mutex::new(Vec::new()),
        }
    }

    pub fn subscribe(&self, subscriber: Box<dyn AgentEventSubscriber>) {
        self.subscribers.lock().unwrap().push(subscriber);
    }

    pub fn publish(&self, session_id: &str, event: &AgentEvent) {
        let subscribers = self.subscribers.lock().unwrap();
        for sub in subscribers.iter() {
            let should_send = match sub.filter() {
                Some(filter) => filter.matches(event),
                None => true,
            };
            if should_send {
                sub.on_event(session_id, event);
            }
        }
    }

    pub fn publish_batch(&self, session_id: &str, events: &[AgentEvent]) {
        for event in events {
            self.publish(session_id, event);
        }
    }
}

pub struct HistoryRecorder {
    history: Arc<Mutex<std::collections::HashMap<String, Vec<AgentEvent>>>>,
}

impl HistoryRecorder {
    pub fn new() -> Self {
        Self {
            history: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub fn get_events(&self, session_id: &str) -> Vec<AgentEvent> {
        self.history
            .lock()
            .unwrap()
            .get(session_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn history_ref(&self) -> Arc<Mutex<std::collections::HashMap<String, Vec<AgentEvent>>>> {
        self.history.clone()
    }
}

impl AgentEventSubscriber for HistoryRecorder {
    fn on_event(&self, session_id: &str, event: &AgentEvent) {
        self.history
            .lock()
            .unwrap()
            .entry(session_id.to_string())
            .or_default()
            .push(event.clone());
    }
}

use tauri::Emitter;

pub struct FrontendEmitter {
    app_handle: tauri::AppHandle,
}

impl FrontendEmitter {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }
}

impl AgentEventSubscriber for FrontendEmitter {
    fn on_event(&self, session_id: &str, event: &AgentEvent) {
        #[derive(serde::Serialize, Clone)]
        struct AgentEventPayload {
            session_id: String,
            event: AgentEvent,
        }

        let payload = AgentEventPayload {
            session_id: session_id.to_string(),
            event: event.clone(),
        };

        let _ = self.app_handle.emit("agent-event", payload);
    }
}

use crate::transport::session_store::SessionStore;
use crate::transport::session_handle::SessionHandle;

/// Event bus subscriber that appends events to disk (JSONL) as they arrive.
/// Also updates session metadata (event_count, last_active_at).
pub struct SessionHistoryRecorder {
    store: Arc<SessionStore>,
    handles: Arc<Mutex<std::collections::HashMap<String, SessionHandle>>>,
}

impl SessionHistoryRecorder {
    pub fn new(store: Arc<SessionStore>) -> Self {
        Self {
            store,
            handles: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Register a session to be tracked by this recorder.
    pub fn track_session(&self, handle: SessionHandle) {
        self.handles.lock().unwrap().insert(handle.id.clone(), handle);
    }

    /// Get a reference to the handles map for the SessionManager.
    pub fn handles_ref(&self) -> Arc<Mutex<std::collections::HashMap<String, SessionHandle>>> {
        self.handles.clone()
    }
}

impl AgentEventSubscriber for SessionHistoryRecorder {
    fn on_event(&self, session_id: &str, event: &AgentEvent) {
        // Append event to JSONL (fire and forget — log errors but don't panic)
        if let Err(e) = self.store.append_event(session_id, event) {
            eprintln!("SessionHistoryRecorder: failed to append event: {}", e);
        }

        // Update metadata
        let mut handles = self.handles.lock().unwrap();
        if let Some(handle) = handles.get_mut(session_id) {
            handle.event_count += 1;
            handle.touch();
            // Persist metadata periodically (every 10 events) to reduce I/O
            if handle.event_count % 10 == 0 {
                if let Err(e) = self.store.write_metadata(handle) {
                    eprintln!("SessionHistoryRecorder: failed to write metadata: {}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_event::{AgentEvent, AgentEventType};
    use std::sync::atomic::{AtomicU32, Ordering};

    struct CountingSubscriber {
        count: AtomicU32,
    }

    impl CountingSubscriber {
        fn new() -> Self {
            Self { count: AtomicU32::new(0) }
        }
        fn count(&self) -> u32 {
            self.count.load(Ordering::SeqCst)
        }
    }

    impl AgentEventSubscriber for CountingSubscriber {
        fn on_event(&self, _session_id: &str, _event: &AgentEvent) {
            self.count.fetch_add(1, Ordering::SeqCst);
        }
    }

    struct FilteredSubscriber {
        count: AtomicU32,
    }

    impl FilteredSubscriber {
        fn new() -> Self {
            Self { count: AtomicU32::new(0) }
        }
        fn count(&self) -> u32 {
            self.count.load(Ordering::SeqCst)
        }
    }

    impl AgentEventSubscriber for FilteredSubscriber {
        fn on_event(&self, _session_id: &str, _event: &AgentEvent) {
            self.count.fetch_add(1, Ordering::SeqCst);
        }
        fn filter(&self) -> Option<EventFilter> {
            Some(EventFilter {
                event_types: Some(vec![AgentEventType::Error]),
                providers: None,
            })
        }
    }

    #[test]
    fn test_publish_to_all_subscribers() {
        let bus = AgentEventBus::new();
        let sub1 = Arc::new(CountingSubscriber::new());
        let sub2 = Arc::new(CountingSubscriber::new());

        let sub1_ref = sub1.clone();
        let sub2_ref = sub2.clone();

        bus.subscribe(Box::new(CountingSubscriberWrapper(sub1)));
        bus.subscribe(Box::new(CountingSubscriberWrapper(sub2)));

        let event = AgentEvent::text("hello", "claude");
        bus.publish("session-1", &event);

        assert_eq!(sub1_ref.count(), 1);
        assert_eq!(sub2_ref.count(), 1);
    }

    struct CountingSubscriberWrapper(Arc<CountingSubscriber>);
    impl AgentEventSubscriber for CountingSubscriberWrapper {
        fn on_event(&self, session_id: &str, event: &AgentEvent) {
            self.0.on_event(session_id, event);
        }
    }

    struct FilteredSubscriberWrapper(Arc<FilteredSubscriber>);
    impl AgentEventSubscriber for FilteredSubscriberWrapper {
        fn on_event(&self, session_id: &str, event: &AgentEvent) {
            self.0.on_event(session_id, event);
        }
        fn filter(&self) -> Option<EventFilter> {
            self.0.filter()
        }
    }

    #[test]
    fn test_filter_by_event_type() {
        let bus = AgentEventBus::new();
        let sub = Arc::new(FilteredSubscriber::new());
        let sub_ref = sub.clone();

        bus.subscribe(Box::new(FilteredSubscriberWrapper(sub)));

        let text = AgentEvent::text("hello", "claude");
        let error = AgentEvent::error("bad", "err", crate::agent_event::ErrorSeverity::Fatal, "claude");

        bus.publish("s1", &text);
        bus.publish("s1", &error);

        assert_eq!(sub_ref.count(), 1);
    }

    #[test]
    fn test_publish_batch() {
        let bus = AgentEventBus::new();
        let sub = Arc::new(CountingSubscriber::new());
        let sub_ref = sub.clone();

        bus.subscribe(Box::new(CountingSubscriberWrapper(sub)));

        let events = vec![
            AgentEvent::text("a", "claude"),
            AgentEvent::text("b", "claude"),
            AgentEvent::text("c", "claude"),
        ];
        bus.publish_batch("s1", &events);

        assert_eq!(sub_ref.count(), 3);
    }

    #[test]
    fn test_history_recorder() {
        let recorder = HistoryRecorder::new();

        let event1 = AgentEvent::text("hello", "claude");
        let event2 = AgentEvent::text("world", "claude");

        recorder.on_event("session-1", &event1);
        recorder.on_event("session-1", &event2);
        recorder.on_event("session-2", &event1);

        let s1_events = recorder.get_events("session-1");
        assert_eq!(s1_events.len(), 2);

        let s2_events = recorder.get_events("session-2");
        assert_eq!(s2_events.len(), 1);

        let s3_events = recorder.get_events("session-3");
        assert_eq!(s3_events.len(), 0);
    }

    #[test]
    fn test_event_filter_matches() {
        let filter = EventFilter {
            event_types: Some(vec![AgentEventType::Text, AgentEventType::Error]),
            providers: Some(vec!["claude".to_string()]),
        };

        let text_claude = AgentEvent::text("hi", "claude");
        let text_openai = AgentEvent::text("hi", "openai");
        let error_claude = AgentEvent::error("bad", "e", crate::agent_event::ErrorSeverity::Fatal, "claude");
        let usage_claude = AgentEvent::usage(1, 2, "claude");

        assert!(filter.matches(&text_claude));
        assert!(!filter.matches(&text_openai));
        assert!(filter.matches(&error_claude));
        assert!(!filter.matches(&usage_claude));
    }

    #[test]
    fn test_session_history_recorder() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = Arc::new(crate::transport::session_store::SessionStore::new(dir.path()).unwrap());
        let handle = crate::transport::session_handle::SessionHandle::new("claude", "sonnet");
        let session_id = handle.id.clone();

        store.create_session(&handle).unwrap();

        let recorder = SessionHistoryRecorder::new(store.clone());
        recorder.track_session(handle);

        let event = AgentEvent::text("hello", "claude");
        recorder.on_event(&session_id, &event);

        // Verify event was written to disk
        let events = store.read_events(&session_id).unwrap();
        assert_eq!(events.len(), 1);

        // Verify handle was updated
        let handles = recorder.handles_ref();
        let handles = handles.lock().unwrap();
        assert_eq!(handles.get(&session_id).unwrap().event_count, 1u32);
    }
}
