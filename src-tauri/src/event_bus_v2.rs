use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock, Weak};

use serde::Serialize;

use crate::error::ReasonanceError;

/// A general-purpose event for channel-based pub/sub.
#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub id: String,
    pub channel: String,
    pub payload: serde_json::Value,
    pub timestamp: String,
    pub source: String,
}

impl Event {
    /// Create a new event with auto-generated ID and timestamp.
    pub fn new(channel: impl Into<String>, payload: serde_json::Value, source: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            channel: channel.into(),
            payload,
            timestamp: chrono::Utc::now().to_rfc3339(),
            source: source.into(),
        }
    }
}

/// Trait for event subscribers.
pub trait EventHandler: Send + Sync {
    fn handle(&self, event: &Event) -> Result<(), ReasonanceError>;
    fn id(&self) -> &str;
}

/// Internal channel with weak-ref subscribers.
struct Channel {
    #[allow(dead_code)]
    name: String,
    subscribers: Vec<Weak<dyn EventHandler>>,
    #[allow(dead_code)]
    max_buffer_size: usize,
    frontend_visible: bool,
}

/// General-purpose event bus with channel-based pub/sub, deferred queue for
/// recursion prevention, and weak-ref subscribers.
pub struct EventBus {
    channels: RwLock<HashMap<String, Channel>>,
    deferred_queue: Mutex<VecDeque<Event>>,
    processing: AtomicBool,
}

// SAFETY: All fields use thread-safe primitives.
unsafe impl Send for EventBus {}
unsafe impl Sync for EventBus {}

impl EventBus {
    pub fn new() -> Self {
        Self {
            channels: RwLock::new(HashMap::new()),
            deferred_queue: Mutex::new(VecDeque::new()),
            processing: AtomicBool::new(false),
        }
    }

    /// Register a named channel. If it already exists, this is a no-op.
    pub fn register_channel(&self, name: &str, frontend_visible: bool) {
        let mut channels = self.channels.write().unwrap_or_else(|e| e.into_inner());
        channels.entry(name.to_string()).or_insert_with(|| Channel {
            name: name.to_string(),
            subscribers: Vec::new(),
            max_buffer_size: 1000,
            frontend_visible,
        });
    }

    /// Subscribe a handler to a channel. The bus holds a weak reference.
    /// If the channel doesn't exist, it is auto-registered as non-frontend-visible.
    pub fn subscribe(&self, channel_name: &str, handler: Arc<dyn EventHandler>) {
        let mut channels = self.channels.write().unwrap_or_else(|e| e.into_inner());
        let channel = channels.entry(channel_name.to_string()).or_insert_with(|| Channel {
            name: channel_name.to_string(),
            subscribers: Vec::new(),
            max_buffer_size: 1000,
            frontend_visible: false,
        });
        channel.subscribers.push(Arc::downgrade(&handler));
    }

    /// Publish an event to its channel's subscribers.
    ///
    /// If called reentrantly (from within a handler), the event is deferred.
    /// After the top-level dispatch completes, deferred events are drained
    /// iteratively (max 1000 iterations to prevent infinite loops).
    pub fn publish(&self, event: Event) {
        // If we're already processing, defer this event.
        if self.processing.load(Ordering::SeqCst) {
            let mut queue = self.deferred_queue.lock().unwrap_or_else(|e| e.into_inner());
            queue.push_back(event);
            return;
        }

        self.processing.store(true, Ordering::SeqCst);

        self.dispatch(&event);

        // Drain deferred queue iteratively.
        let mut iterations = 0;
        loop {
            let next = {
                let mut queue = self.deferred_queue.lock().unwrap_or_else(|e| e.into_inner());
                queue.pop_front()
            };
            match next {
                Some(deferred) => {
                    self.dispatch(&deferred);
                    iterations += 1;
                    if iterations >= 1000 {
                        log::warn!("EventBus: deferred queue exceeded 1000 iterations, stopping drain");
                        break;
                    }
                }
                None => break,
            }
        }

        self.processing.store(false, Ordering::SeqCst);
    }

    /// Dispatch a single event to all live subscribers on its channel.
    fn dispatch(&self, event: &Event) {
        let channels = self.channels.read().unwrap_or_else(|e| e.into_inner());
        if let Some(channel) = channels.get(&event.channel) {
            for weak in &channel.subscribers {
                if let Some(handler) = weak.upgrade() {
                    if let Err(e) = handler.handle(event) {
                        log::error!(
                            "EventBus: handler '{}' failed on channel '{}': {}",
                            handler.id(),
                            event.channel,
                            e
                        );
                    }
                }
            }
        }
    }

    /// Remove dead weak references from all channels.
    pub fn sweep(&self) {
        let mut channels = self.channels.write().unwrap_or_else(|e| e.into_inner());
        for channel in channels.values_mut() {
            channel.subscribers.retain(|w| w.strong_count() > 0);
        }
    }

    /// Check if a channel is marked as frontend-visible.
    pub fn is_frontend_visible(&self, channel_name: &str) -> bool {
        let channels = self.channels.read().unwrap_or_else(|e| e.into_inner());
        channels.get(channel_name).map_or(false, |c| c.frontend_visible)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    /// Simple test handler that counts how many events it received.
    struct CountingHandler {
        handler_id: String,
        count: AtomicUsize,
    }

    impl CountingHandler {
        fn new(id: &str) -> Self {
            Self {
                handler_id: id.to_string(),
                count: AtomicUsize::new(0),
            }
        }

        fn count(&self) -> usize {
            self.count.load(Ordering::SeqCst)
        }
    }

    impl EventHandler for CountingHandler {
        fn handle(&self, _event: &Event) -> Result<(), ReasonanceError> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn id(&self) -> &str {
            &self.handler_id
        }
    }

    #[test]
    fn publish_to_single_subscriber() {
        let bus = EventBus::new();
        bus.register_channel("test", false);

        let handler = Arc::new(CountingHandler::new("h1"));
        bus.subscribe("test", handler.clone());

        let event = Event::new("test", serde_json::json!({"msg": "hello"}), "test-source");
        bus.publish(event);

        assert_eq!(handler.count(), 1);
    }

    #[test]
    fn publish_to_multiple_subscribers() {
        let bus = EventBus::new();
        bus.register_channel("multi", false);

        let h1 = Arc::new(CountingHandler::new("h1"));
        let h2 = Arc::new(CountingHandler::new("h2"));
        let h3 = Arc::new(CountingHandler::new("h3"));

        bus.subscribe("multi", h1.clone());
        bus.subscribe("multi", h2.clone());
        bus.subscribe("multi", h3.clone());

        bus.publish(Event::new("multi", serde_json::json!(null), "src"));

        assert_eq!(h1.count(), 1);
        assert_eq!(h2.count(), 1);
        assert_eq!(h3.count(), 1);
    }

    #[test]
    fn publish_no_subscribers_no_panic() {
        let bus = EventBus::new();
        bus.register_channel("empty", false);
        // Should not panic
        bus.publish(Event::new("empty", serde_json::json!(null), "src"));
    }

    #[test]
    fn weak_ref_cleanup_after_drop() {
        let bus = EventBus::new();
        bus.register_channel("sweep-test", false);

        let handler = Arc::new(CountingHandler::new("ephemeral"));
        bus.subscribe("sweep-test", handler.clone());

        // Publish once — handler is alive
        bus.publish(Event::new("sweep-test", serde_json::json!(null), "src"));
        assert_eq!(handler.count(), 1);

        // Drop the strong reference
        drop(handler);

        // Sweep should remove the dead weak ref
        bus.sweep();

        let channels = bus.channels.read().unwrap();
        let channel = channels.get("sweep-test").unwrap();
        assert_eq!(channel.subscribers.len(), 0);
    }

    #[test]
    fn publish_to_unregistered_channel_no_panic() {
        let bus = EventBus::new();
        // Channel "nonexistent" was never registered — should not panic
        bus.publish(Event::new("nonexistent", serde_json::json!(null), "src"));
    }

    #[test]
    fn frontend_visibility_flag() {
        let bus = EventBus::new();
        bus.register_channel("visible", true);
        bus.register_channel("hidden", false);

        assert!(bus.is_frontend_visible("visible"));
        assert!(!bus.is_frontend_visible("hidden"));
        // Unknown channel returns false
        assert!(!bus.is_frontend_visible("unknown"));
    }

    #[test]
    fn event_new_generates_id_and_timestamp() {
        let event = Event::new("ch", serde_json::json!(42), "origin");
        assert!(!event.id.is_empty());
        assert_eq!(event.channel, "ch");
        assert_eq!(event.source, "origin");
        // Timestamp should be valid RFC3339
        assert!(chrono::DateTime::parse_from_rfc3339(&event.timestamp).is_ok());
    }

    #[test]
    fn handler_error_does_not_stop_other_subscribers() {
        struct FailingHandler;
        impl EventHandler for FailingHandler {
            fn handle(&self, _event: &Event) -> Result<(), ReasonanceError> {
                Err(ReasonanceError::internal("boom"))
            }
            fn id(&self) -> &str {
                "failing"
            }
        }

        let bus = EventBus::new();
        bus.register_channel("mixed", false);

        let failing: Arc<dyn EventHandler> = Arc::new(FailingHandler);
        let counting = Arc::new(CountingHandler::new("after-fail"));

        bus.subscribe("mixed", failing);
        bus.subscribe("mixed", counting.clone());

        bus.publish(Event::new("mixed", serde_json::json!(null), "src"));

        // The counting handler after the failing one should still receive the event
        assert_eq!(counting.count(), 1);
    }
}
