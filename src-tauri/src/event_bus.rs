use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::Emitter;

use crate::agent_event::AgentEvent;
use crate::error::ReasonanceError;

const DEFAULT_CHANNEL_BUFFER: usize = 1000;
const MAX_DEFERRED_ITERATIONS: usize = 1000;
const SLOW_HANDLER_THRESHOLD: Duration = Duration::from_millis(100);

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

    /// Convert a transport-specific `AgentEvent` into a generic bus `Event`.
    ///
    /// The payload wraps `session_id` and the serialized `AgentEvent` so that
    /// downstream subscribers (including the frontend) can reconstruct context.
    pub fn from_agent_event(channel: &str, session_id: &str, agent_event: &AgentEvent) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            channel: channel.to_string(),
            payload: serde_json::json!({
                "session_id": session_id,
                "event": agent_event,
            }),
            timestamp: chrono::Utc::now().to_rfc3339(),
            source: format!("transport:{}", agent_event.metadata.provider),
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
    subscribers: Vec<Subscriber>,
    max_buffer_size: usize,
    frontend_visible: bool,
    /// Number of async events currently in-flight (for backpressure).
    pending_count: Arc<AtomicUsize>,
    /// Number of events dropped due to backpressure.
    drop_count: AtomicUsize,
}

impl Channel {
    fn new(frontend_visible: bool, max_buffer_size: usize) -> Self {
        Self {
            subscribers: Vec::new(),
            max_buffer_size,
            pending_count: Arc::new(AtomicUsize::new(0)),
            drop_count: AtomicUsize::new(0),
            frontend_visible,
        }
    }
}

/// General-purpose event bus with channel-based pub/sub, deferred queue for
/// recursion prevention, weak-ref subscribers, async dispatch, and backpressure.
pub struct EventBus {
    channels: RwLock<HashMap<String, Channel>>,
    deferred_queue: Mutex<VecDeque<Event>>,
    processing: AtomicBool,
    runtime: tokio::runtime::Handle,
}

impl EventBus {
    pub fn new(runtime: tokio::runtime::Handle) -> Self {
        let bus = Self {
            channels: RwLock::new(HashMap::new()),
            deferred_queue: Mutex::new(VecDeque::new()),
            processing: AtomicBool::new(false),
            runtime,
        };
        // Auto-register the lifecycle:warning channel with no backpressure limit
        // to prevent recursion when emitting event:dropped warnings.
        bus.register_channel_with_buffer("lifecycle:warning", false, usize::MAX);
        bus
    }

    /// Register a named channel with the default buffer size (1000).
    /// If it already exists, this is a no-op.
    pub fn register_channel(&self, name: &str, frontend_visible: bool) {
        let mut channels = self.channels.write().unwrap_or_else(|e| e.into_inner());
        channels
            .entry(name.to_string())
            .or_insert_with(|| Channel::new(frontend_visible, DEFAULT_CHANNEL_BUFFER));
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
        channels
            .entry(name.to_string())
            .or_insert_with(|| Channel::new(frontend_visible, max_buffer_size));
    }

    /// Subscribe a synchronous handler to a channel. The bus holds a weak reference.
    /// If the channel doesn't exist, it is auto-registered as non-frontend-visible.
    pub fn subscribe(&self, channel_name: &str, handler: Arc<dyn EventHandler>) {
        let mut channels = self.channels.write().unwrap_or_else(|e| e.into_inner());
        let channel = channels
            .entry(channel_name.to_string())
            .or_insert_with(|| Channel::new(false, DEFAULT_CHANNEL_BUFFER));
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
            .or_insert_with(|| Channel::new(false, DEFAULT_CHANNEL_BUFFER));
        channel
            .subscribers
            .push(Subscriber::Async(Arc::downgrade(&handler)));
    }

    /// Publish an event to its channel's subscribers.
    ///
    /// If called reentrantly (from within a handler), the event is deferred.
    /// After the top-level dispatch completes, deferred events are drained
    /// iteratively (max `MAX_DEFERRED_ITERATIONS` to prevent infinite loops).
    ///
    /// Backpressure: if the channel's pending async count exceeds max_buffer_size,
    /// the event is dropped, a warning is logged, and a synthetic `lifecycle:warning`
    /// event is emitted.
    pub fn publish(&self, event: Event) {
        // Atomically claim the processing flag. If another thread is already
        // dispatching, defer this event instead of risking concurrent dispatch.
        if self
            .processing
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            // Another call is already dispatching; defer this event.
            let mut queue = self
                .deferred_queue
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            queue.push_back(event);
            return;
        }

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
                    if iterations >= MAX_DEFERRED_ITERATIONS {
                        log::warn!(
                            "EventBus: deferred queue exceeded {} iterations, stopping drain",
                            MAX_DEFERRED_ITERATIONS,
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
    ///
    /// Uses a read lock — `pending_count` and `drop_count` are `AtomicUsize` and
    /// can be mutated through a shared reference.
    fn dispatch(&self, event: &Event) {
        let channels = self.channels.read().unwrap_or_else(|e| e.into_inner());
        if let Some(channel) = channels.get(&event.channel) {
            // Backpressure check: capture once to avoid TOCTOU between check and log.
            let pending = channel.pending_count.load(Ordering::SeqCst);
            if pending >= channel.max_buffer_size {
                channel.drop_count.fetch_add(1, Ordering::SeqCst);
                log::warn!(
                    "EventBus: backpressure on channel '{}' — pending {} >= max {}, dropping event",
                    event.channel,
                    pending,
                    channel.max_buffer_size,
                );
                // Emit a synthetic event:dropped warning on the lifecycle:warning channel.
                let warning = Event::new(
                    "lifecycle:warning",
                    serde_json::json!({
                        "kind": "event:dropped",
                        "channel": event.channel,
                        "event_id": event.id,
                        "reason": "backpressure",
                    }),
                    "event-bus",
                );
                let mut queue = self
                    .deferred_queue
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                queue.push_back(warning);
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
                            if elapsed > SLOW_HANDLER_THRESHOLD {
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
                            let pending = Arc::clone(&channel.pending_count);
                            // Increment pending count *before* spawning so that
                            // subsequent dispatches see the pressure immediately.
                            pending.fetch_add(1, Ordering::SeqCst);
                            self.runtime.spawn(async move {
                                let result = handler.handle(event_clone).await;
                                // Decrement regardless of success/failure so the
                                // channel recovers naturally.
                                pending.fetch_sub(1, Ordering::SeqCst);
                                if let Err(e) = result {
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

    /// Subscribe a handler to all channels currently marked as frontend-visible.
    ///
    /// This is a convenience method for bridge-style handlers that need to
    /// observe every frontend-facing channel without knowing their names upfront.
    pub fn subscribe_to_visible(&self, handler: Arc<dyn EventHandler>) {
        let channels = self.channels.read().unwrap_or_else(|e| e.into_inner());
        let visible: Vec<String> = channels
            .iter()
            .filter(|(_, ch)| ch.frontend_visible)
            .map(|(name, _)| name.clone())
            .collect();
        drop(channels); // release read lock before taking write locks in subscribe()
        for name in visible {
            self.subscribe(&name, handler.clone());
        }
    }
}

/// Bridge that forwards events to the Tauri frontend via `app_handle.emit()`.
///
/// Registered as a sync subscriber on frontend-visible channels. On each event
/// it checks `is_frontend_visible` (in case the channel set changed since
/// subscription) and emits to the webview if appropriate.
pub struct TauriFrontendBridge {
    app_handle: tauri::AppHandle,
    bus: Arc<EventBus>,
}

impl TauriFrontendBridge {
    pub fn new(app_handle: tauri::AppHandle, bus: Arc<EventBus>) -> Self {
        Self { app_handle, bus }
    }
}

impl EventHandler for TauriFrontendBridge {
    fn handle(&self, event: &Event) -> Result<(), ReasonanceError> {
        if self.bus.is_frontend_visible(&event.channel) {
            let _ = self.app_handle.emit(&event.channel, event);
        }
        Ok(())
    }

    fn id(&self) -> &str {
        "tauri-frontend-bridge"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::sync::OnceLock;

    /// Helper: get a tokio runtime handle for tests.
    fn test_runtime_handle() -> tokio::runtime::Handle {
        // When running under #[tokio::test], we can grab the current handle.
        // For plain #[test] functions, use a shared runtime via OnceLock.
        tokio::runtime::Handle::try_current().unwrap_or_else(|_| {
            static TEST_RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
            let rt = TEST_RUNTIME.get_or_init(|| {
                tokio::runtime::Runtime::new().expect("failed to create test runtime")
            });
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

    #[tokio::test]
    async fn backpressure_drop_counting() {
        /// Async handler that blocks until a signal is received, simulating
        /// slow processing to create genuine backpressure.
        struct BlockingAsyncHandler {
            handler_id: String,
            count: Arc<AtomicUsize>,
            release: Arc<tokio::sync::Notify>,
        }

        #[async_trait::async_trait]
        impl AsyncEventHandler for BlockingAsyncHandler {
            async fn handle(&self, _event: Event) -> Result<(), ReasonanceError> {
                self.count.fetch_add(1, Ordering::SeqCst);
                // Block until the test releases us.
                self.release.notified().await;
                Ok(())
            }
            fn id(&self) -> &str {
                &self.handler_id
            }
        }

        let bus = EventBus::new(tokio::runtime::Handle::current());
        // Channel with max 5 pending async events.
        bus.register_channel_with_buffer("bp", false, 5);

        let count = Arc::new(AtomicUsize::new(0));
        let release = Arc::new(tokio::sync::Notify::new());
        let handler = Arc::new(BlockingAsyncHandler {
            handler_id: "bp-handler".to_string(),
            count: count.clone(),
            release: release.clone(),
        });
        bus.subscribe_async("bp", handler.clone());

        // Publish 10 events — first 5 get spawned (filling pending slots),
        // next 5 are dropped because pending_count >= max_buffer_size.
        for i in 0..10 {
            bus.publish(Event::new("bp", serde_json::json!(i), "src"));
        }

        // Let spawned tasks start.
        tokio::task::yield_now().await;

        assert_eq!(
            count.load(Ordering::SeqCst),
            5,
            "only 5 should be dispatched"
        );
        assert_eq!(bus.drop_count("bp"), 5, "5 should be dropped");

        // Release all blocked handlers so pending_count drops back to 0.
        release.notify_waiters();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Verify recovery: channel should accept new events again.
        for i in 10..13 {
            bus.publish(Event::new("bp", serde_json::json!(i), "src"));
        }

        // Release the newly spawned handlers.
        release.notify_waiters();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert_eq!(
            count.load(Ordering::SeqCst),
            8,
            "3 more should be dispatched after recovery"
        );
        // drop_count should remain at 5 (no new drops).
        assert_eq!(bus.drop_count("bp"), 5);
    }

    #[tokio::test]
    async fn dropped_event_emits_lifecycle_warning() {
        /// Async handler that never completes (blocks forever), used to
        /// fill the pending buffer and trigger backpressure.
        struct NeverFinishHandler {
            handler_id: String,
        }

        #[async_trait::async_trait]
        impl AsyncEventHandler for NeverFinishHandler {
            async fn handle(&self, _event: Event) -> Result<(), ReasonanceError> {
                // Block indefinitely so pending_count stays high.
                std::future::pending::<()>().await;
                Ok(())
            }
            fn id(&self) -> &str {
                &self.handler_id
            }
        }

        let bus = EventBus::new(tokio::runtime::Handle::current());
        bus.register_channel_with_buffer("bp-warn", false, 2);

        // Subscribe a lifecycle:warning handler to verify the synthetic event.
        let warning_handler = Arc::new(CountingHandler::new("warn-listener"));
        bus.subscribe("lifecycle:warning", warning_handler.clone());

        let handler = Arc::new(NeverFinishHandler {
            handler_id: "blocker".to_string(),
        });
        bus.subscribe_async("bp-warn", handler.clone());

        // Fill the buffer (2 events) then trigger a drop.
        bus.publish(Event::new("bp-warn", serde_json::json!(1), "src"));
        bus.publish(Event::new("bp-warn", serde_json::json!(2), "src"));
        // This one should be dropped, triggering a lifecycle:warning event.
        bus.publish(Event::new("bp-warn", serde_json::json!(3), "src"));

        // The lifecycle:warning is dispatched via the deferred queue within
        // the same publish() call, so it should already be delivered.
        assert_eq!(
            warning_handler.count(),
            1,
            "lifecycle:warning should have been emitted"
        );
        assert_eq!(bus.drop_count("bp-warn"), 1);
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

    #[test]
    fn lifecycle_warning_channel_auto_registered() {
        let bus = EventBus::new(test_runtime_handle());
        // lifecycle:warning should be registered automatically by EventBus::new()
        let channels = bus.channels.read().unwrap();
        let ch = channels.get("lifecycle:warning").unwrap();
        assert_eq!(ch.max_buffer_size, usize::MAX);
        assert!(!ch.frontend_visible);
    }

    // ── Tests for Event::from_agent_event ───────────────────────────────

    #[test]
    fn from_agent_event_sets_channel_and_source() {
        let agent_evt = AgentEvent::text("hello", "claude");
        let event = Event::from_agent_event("agent:stream", "sess-1", &agent_evt);

        assert_eq!(event.channel, "agent:stream");
        assert_eq!(event.source, "transport:claude");
        assert!(!event.id.is_empty());
        assert!(chrono::DateTime::parse_from_rfc3339(&event.timestamp).is_ok());
    }

    #[test]
    fn from_agent_event_payload_contains_session_and_event() {
        let agent_evt = AgentEvent::text("world", "openai");
        let event = Event::from_agent_event("agent:stream", "sess-42", &agent_evt);

        let payload = &event.payload;
        assert_eq!(payload["session_id"], "sess-42");
        // The nested event should contain the agent event's id
        assert_eq!(payload["event"]["id"], agent_evt.id);
        assert_eq!(payload["event"]["metadata"]["provider"], "openai");
    }

    #[test]
    fn from_agent_event_source_format_varies_by_provider() {
        let claude_evt = AgentEvent::text("a", "claude");
        let openai_evt = AgentEvent::text("b", "openai");

        let e1 = Event::from_agent_event("ch", "s", &claude_evt);
        let e2 = Event::from_agent_event("ch", "s", &openai_evt);

        assert_eq!(e1.source, "transport:claude");
        assert_eq!(e2.source, "transport:openai");
    }

    // ── Tests for subscribe_to_visible ──────────────────────────────────

    #[test]
    fn subscribe_to_visible_only_subscribes_visible_channels() {
        let bus = EventBus::new(test_runtime_handle());
        bus.register_channel("visible-a", true);
        bus.register_channel("visible-b", true);
        bus.register_channel("hidden-c", false);

        let handler = Arc::new(CountingHandler::new("bridge"));
        bus.subscribe_to_visible(handler.clone());

        // Publish to visible channels — handler should receive both
        bus.publish(Event::new("visible-a", serde_json::json!(1), "src"));
        bus.publish(Event::new("visible-b", serde_json::json!(2), "src"));
        assert_eq!(handler.count(), 2);

        // Publish to hidden channel — handler should NOT receive it
        bus.publish(Event::new("hidden-c", serde_json::json!(3), "src"));
        assert_eq!(handler.count(), 2);
    }

    #[test]
    fn subscribe_to_visible_no_visible_channels() {
        let bus = EventBus::new(test_runtime_handle());
        bus.register_channel("hidden-1", false);
        bus.register_channel("hidden-2", false);

        let handler = Arc::new(CountingHandler::new("bridge"));
        bus.subscribe_to_visible(handler.clone());

        bus.publish(Event::new("hidden-1", serde_json::json!(null), "src"));
        bus.publish(Event::new("hidden-2", serde_json::json!(null), "src"));
        assert_eq!(handler.count(), 0);
    }
}
