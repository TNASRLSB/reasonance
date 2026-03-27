use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::time::Instant;

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
    pub fn new(
        channel: impl Into<String>,
        payload: serde_json::Value,
        source: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            channel: channel.into(),
            payload,
            timestamp: chrono::Utc::now().to_rfc3339(),
            source: source.into(),
        }
    }
}

/// Trait for synchronous event subscribers.
pub trait EventHandler: Send + Sync {
    fn handle(&self, event: &Event) -> Result<(), ReasonanceError>;
    fn id(&self) -> &str;
}

/// Trait for asynchronous event subscribers.
/// Dispatched via `tokio::spawn` — the publisher never blocks on these.
#[async_trait::async_trait]
pub trait AsyncEventHandler: Send + Sync {
    async fn handle(&self, event: Event) -> Result<(), ReasonanceError>;
    fn id(&self) -> &str;
}

/// A subscriber is either synchronous or asynchronous.
enum Subscriber {
    Sync(Weak<dyn EventHandler>),
    Async(Weak<dyn AsyncEventHandler>),
}

/// Internal channel with weak-ref subscribers and backpressure tracking.
struct Channel {
    #[allow(dead_code)]
    name: String,
    subscribers: Vec<Subscriber>,
    max_buffer_size: usize,
    frontend_visible: bool,
    /// Rolling count of events published to this channel (for backpressure).
    publish_count: usize,
    /// Number of events dropped due to backpressure.
    drop_count: AtomicUsize,
}

/// General-purpose event bus with channel-based pub/sub, deferred queue for
/// recursion prevention, weak-ref subscribers, async dispatch, and backpressure.
pub struct EventBus {
    channels: RwLock<HashMap<String, Channel>>,
    deferred_queue: Mutex<VecDeque<Event>>,
    processing: AtomicBool,
    runtime: tokio::runtime::Handle,
}

// SAFETY: All fields use thread-safe primitives.
unsafe impl Send for EventBus {}
unsafe impl Sync for EventBus {}

impl EventBus {
    pub fn new(runtime: tokio::runtime::Handle) -> Self {
        Self {
            channels: RwLock::new(HashMap::new()),
            deferred_queue: Mutex::new(VecDeque::new()),
            processing: AtomicBool::new(false),
            runtime,
        }
    }

    /// Register a named channel with the default buffer size (1000).
    /// If it already exists, this is a no-op.
    pub fn register_channel(&self, name: &str, frontend_visible: bool) {
        let mut channels = self.channels.write().unwrap_or_else(|e| e.into_inner());
        channels.entry(name.to_string()).or_insert_with(|| Channel {
            name: name.to_string(),
            subscribers: Vec::new(),
            max_buffer_size: 1000,
            frontend_visible,
            publish_count: 0,
            drop_count: AtomicUsize::new(0),
        });
    }

    /// Register a named channel with a custom buffer size.
    /// If it already exists, this is a no-op.
    pub fn register_channel_with_buffer(
        &self,
        name: &str,
        frontend_visible: bool,
        max_buffer_size: usize,
    ) {
        let mut channels = self.channels.write().unwrap_or_else(|e| e.into_inner());
        channels.entry(name.to_string()).or_insert_with(|| Channel {
            name: name.to_string(),
            subscribers: Vec::new(),
            max_buffer_size,
            frontend_visible,
            publish_count: 0,
            drop_count: AtomicUsize::new(0),
        });
    }

    /// Subscribe a synchronous handler to a channel. The bus holds a weak reference.
    /// If the channel doesn't exist, it is auto-registered as non-frontend-visible.
    pub fn subscribe(&self, channel_name: &str, handler: Arc<dyn EventHandler>) {
        let mut channels = self.channels.write().unwrap_or_else(|e| e.into_inner());
        let channel = channels
            .entry(channel_name.to_string())
            .or_insert_with(|| Channel {
                name: channel_name.to_string(),
                subscribers: Vec::new(),
                max_buffer_size: 1000,
                frontend_visible: false,
                publish_count: 0,
                drop_count: AtomicUsize::new(0),
            });
        channel
            .subscribers
            .push(Subscriber::Sync(Arc::downgrade(&handler)));
    }

    /// Subscribe an asynchronous handler to a channel. The bus holds a weak reference.
    /// If the channel doesn't exist, it is auto-registered as non-frontend-visible.
    pub fn subscribe_async(&self, channel_name: &str, handler: Arc<dyn AsyncEventHandler>) {
        let mut channels = self.channels.write().unwrap_or_else(|e| e.into_inner());
        let channel = channels
            .entry(channel_name.to_string())
            .or_insert_with(|| Channel {
                name: channel_name.to_string(),
                subscribers: Vec::new(),
                max_buffer_size: 1000,
                frontend_visible: false,
                publish_count: 0,
                drop_count: AtomicUsize::new(0),
            });
        channel
            .subscribers
            .push(Subscriber::Async(Arc::downgrade(&handler)));
    }

    /// Publish an event to its channel's subscribers.
    ///
    /// If called reentrantly (from within a handler), the event is deferred.
    /// After the top-level dispatch completes, deferred events are drained
    /// iteratively (max 1000 iterations to prevent infinite loops).
    ///
    /// Backpressure: if the channel's publish_count exceeds max_buffer_size,
    /// the event is dropped and a warning is logged.
    pub fn publish(&self, event: Event) {
        // If we're already processing, defer this event.
        if self.processing.load(Ordering::SeqCst) {
            let mut queue = self
                .deferred_queue
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            queue.push_back(event);
            return;
        }

        self.processing.store(true, Ordering::SeqCst);

        self.dispatch(&event);

        // Drain deferred queue iteratively.
        let mut iterations = 0;
        loop {
            let next = {
                let mut queue = self
                    .deferred_queue
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                queue.pop_front()
            };
            match next {
                Some(deferred) => {
                    self.dispatch(&deferred);
                    iterations += 1;
                    if iterations >= 1000 {
                        log::warn!(
                            "EventBus: deferred queue exceeded 1000 iterations, stopping drain"
                        );
                        break;
                    }
                }
                None => break,
            }
        }

        self.processing.store(false, Ordering::SeqCst);
    }

    /// Dispatch a single event to all live subscribers on its channel.
    /// Applies backpressure and slow-handler detection.
    fn dispatch(&self, event: &Event) {
        let mut channels = self.channels.write().unwrap_or_else(|e| e.into_inner());
        if let Some(channel) = channels.get_mut(&event.channel) {
            // Backpressure check
            channel.publish_count += 1;
            if channel.publish_count > channel.max_buffer_size {
                channel.drop_count.fetch_add(1, Ordering::SeqCst);
                log::warn!(
                    "EventBus: backpressure on channel '{}' — publish_count {} exceeds max_buffer_size {}, dropping event",
                    event.channel,
                    channel.publish_count,
                    channel.max_buffer_size,
                );
                return;
            }

            for subscriber in &channel.subscribers {
                match subscriber {
                    Subscriber::Sync(weak) => {
                        if let Some(handler) = weak.upgrade() {
                            let start = Instant::now();
                            if let Err(e) = handler.handle(event) {
                                log::error!(
                                    "EventBus: handler '{}' failed on channel '{}': {}",
                                    handler.id(),
                                    event.channel,
                                    e
                                );
                            }
                            let elapsed = start.elapsed();
                            if elapsed.as_millis() > 100 {
                                log::warn!(
                                    "EventBus: slow sync handler '{}' on channel '{}' took {}ms",
                                    handler.id(),
                                    event.channel,
                                    elapsed.as_millis(),
                                );
                            }
                        }
                    }
                    Subscriber::Async(weak) => {
                        if let Some(handler) = weak.upgrade() {
                            let event_clone = event.clone();
                            let channel_name = event.channel.clone();
                            self.runtime.spawn(async move {
                                if let Err(e) = handler.handle(event_clone).await {
                                    log::error!(
                                        "EventBus: async handler '{}' failed on channel '{}': {}",
                                        handler.id(),
                                        channel_name,
                                        e
                                    );
                                }
                            });
                        }
                    }
                }
            }
        }
    }

    /// Remove dead weak references from all channels.
    pub fn sweep(&self) {
        let mut channels = self.channels.write().unwrap_or_else(|e| e.into_inner());
        for channel in channels.values_mut() {
            channel.subscribers.retain(|s| match s {
                Subscriber::Sync(w) => w.strong_count() > 0,
                Subscriber::Async(w) => w.strong_count() > 0,
            });
        }
    }

    /// Check if a channel is marked as frontend-visible.
    pub fn is_frontend_visible(&self, channel_name: &str) -> bool {
        let channels = self.channels.read().unwrap_or_else(|e| e.into_inner());
        channels
            .get(channel_name)
            .map_or(false, |c| c.frontend_visible)
    }

    /// Return the number of events dropped due to backpressure on a channel.
    /// Returns 0 for unknown channels.
    pub fn drop_count(&self, channel_name: &str) -> usize {
        let channels = self.channels.read().unwrap_or_else(|e| e.into_inner());
        channels
            .get(channel_name)
            .map_or(0, |c| c.drop_count.load(Ordering::SeqCst))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    /// Helper: get a tokio runtime handle for tests.
    fn test_runtime_handle() -> tokio::runtime::Handle {
        // When running under #[tokio::test], we can grab the current handle.
        // For plain #[test] functions, build a small runtime.
        tokio::runtime::Handle::try_current().unwrap_or_else(|_| {
            // Leak a runtime so the handle stays valid for the test's duration.
            let rt = Box::leak(Box::new(
                tokio::runtime::Runtime::new().expect("failed to create test runtime"),
            ));
            rt.handle().clone()
        })
    }

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
        let bus = EventBus::new(test_runtime_handle());
        bus.register_channel("test", false);

        let handler = Arc::new(CountingHandler::new("h1"));
        bus.subscribe("test", handler.clone());

        let event = Event::new("test", serde_json::json!({"msg": "hello"}), "test-source");
        bus.publish(event);

        assert_eq!(handler.count(), 1);
    }

    #[test]
    fn publish_to_multiple_subscribers() {
        let bus = EventBus::new(test_runtime_handle());
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
        let bus = EventBus::new(test_runtime_handle());
        bus.register_channel("empty", false);
        // Should not panic
        bus.publish(Event::new("empty", serde_json::json!(null), "src"));
    }

    #[test]
    fn weak_ref_cleanup_after_drop() {
        let bus = EventBus::new(test_runtime_handle());
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
        let bus = EventBus::new(test_runtime_handle());
        // Channel "nonexistent" was never registered — should not panic
        bus.publish(Event::new("nonexistent", serde_json::json!(null), "src"));
    }

    #[test]
    fn frontend_visibility_flag() {
        let bus = EventBus::new(test_runtime_handle());
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

        let bus = EventBus::new(test_runtime_handle());
        bus.register_channel("mixed", false);

        let failing: Arc<dyn EventHandler> = Arc::new(FailingHandler);
        let counting = Arc::new(CountingHandler::new("after-fail"));

        bus.subscribe("mixed", failing);
        bus.subscribe("mixed", counting.clone());

        bus.publish(Event::new("mixed", serde_json::json!(null), "src"));

        // The counting handler after the failing one should still receive the event
        assert_eq!(counting.count(), 1);
    }

    // ── New tests for async, backpressure, and mixed subscribers ────────

    #[tokio::test]
    async fn async_handler_receives_events() {
        /// Async handler that increments a shared counter.
        struct AsyncCounter {
            handler_id: String,
            count: Arc<AtomicUsize>,
        }

        #[async_trait::async_trait]
        impl AsyncEventHandler for AsyncCounter {
            async fn handle(&self, _event: Event) -> Result<(), ReasonanceError> {
                self.count.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
            fn id(&self) -> &str {
                &self.handler_id
            }
        }

        let bus = EventBus::new(tokio::runtime::Handle::current());
        bus.register_channel("async-ch", false);

        let count = Arc::new(AtomicUsize::new(0));
        let handler = Arc::new(AsyncCounter {
            handler_id: "async-h1".to_string(),
            count: count.clone(),
        });
        bus.subscribe_async("async-ch", handler.clone());

        bus.publish(Event::new("async-ch", serde_json::json!({"x": 1}), "src"));

        // Yield to let the spawned task run.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn backpressure_drop_counting() {
        let bus = EventBus::new(test_runtime_handle());
        // Register a channel with a very small buffer
        bus.register_channel_with_buffer("bp", false, 5);

        let handler = Arc::new(CountingHandler::new("bp-handler"));
        bus.subscribe("bp", handler.clone());

        // Publish 10 events — first 5 delivered, next 5 dropped
        for i in 0..10 {
            bus.publish(Event::new("bp", serde_json::json!(i), "src"));
        }

        assert_eq!(handler.count(), 5);
        assert_eq!(bus.drop_count("bp"), 5);
    }

    #[tokio::test]
    async fn mixed_sync_and_async_subscribers() {
        struct AsyncCounter {
            handler_id: String,
            count: Arc<AtomicUsize>,
        }

        #[async_trait::async_trait]
        impl AsyncEventHandler for AsyncCounter {
            async fn handle(&self, _event: Event) -> Result<(), ReasonanceError> {
                self.count.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
            fn id(&self) -> &str {
                &self.handler_id
            }
        }

        let bus = EventBus::new(tokio::runtime::Handle::current());
        bus.register_channel("mixed-ch", false);

        // Sync subscriber
        let sync_handler = Arc::new(CountingHandler::new("sync-h"));
        bus.subscribe("mixed-ch", sync_handler.clone());

        // Async subscriber
        let async_count = Arc::new(AtomicUsize::new(0));
        let async_handler = Arc::new(AsyncCounter {
            handler_id: "async-h".to_string(),
            count: async_count.clone(),
        });
        bus.subscribe_async("mixed-ch", async_handler.clone());

        bus.publish(Event::new("mixed-ch", serde_json::json!("hello"), "src"));

        // Yield to let the async task complete.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert_eq!(sync_handler.count(), 1);
        assert_eq!(async_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn drop_count_returns_zero_for_unknown_channel() {
        let bus = EventBus::new(test_runtime_handle());
        assert_eq!(bus.drop_count("nonexistent"), 0);
    }

    #[test]
    fn register_channel_with_buffer_custom_size() {
        let bus = EventBus::new(test_runtime_handle());
        bus.register_channel_with_buffer("custom", true, 50);

        assert!(bus.is_frontend_visible("custom"));

        let channels = bus.channels.read().unwrap();
        let ch = channels.get("custom").unwrap();
        assert_eq!(ch.max_buffer_size, 50);
    }
}
