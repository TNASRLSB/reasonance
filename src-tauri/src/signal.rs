use std::sync::Arc;

use serde::Serialize;
use tokio::sync::watch;

use crate::event_bus::{Event, EventBus};

/// A thread-safe signal that notifies subscribers when the value changes.
///
/// Built on `tokio::watch`, which naturally coalesces rapid updates: a subscriber
/// that hasn't polled yet will only see the latest value, not every intermediate
/// one. This makes `Signal` safe to use with bursty producers (e.g., FS watcher)
/// without additional debouncing.
pub struct Signal<T: Clone + Send + Sync + 'static> {
    sender: watch::Sender<T>,
    receiver: watch::Receiver<T>,
}

impl<T: Clone + Send + Sync + 'static> Signal<T> {
    pub fn new(initial: T) -> Self {
        let (sender, receiver) = watch::channel(initial);
        Self { sender, receiver }
    }

    pub fn send(&self, value: T) {
        let _ = self.sender.send(value);
    }

    pub fn subscribe(&self) -> watch::Receiver<T> {
        self.receiver.clone()
    }

    pub fn current(&self) -> T {
        self.receiver.borrow().clone()
    }

    pub fn modify(&self, f: impl FnOnce(&mut T)) {
        self.sender.send_modify(f);
    }
}

impl<T: Clone + Send + Sync + Serialize + 'static> Signal<T> {
    /// Bridge this signal to an EventBus channel: whenever the signal value
    /// changes, publish the new value as an event on the given channel.
    ///
    /// The bridge runs as a spawned tokio task and stops when the signal is
    /// dropped (the watch sender closes).
    pub fn bridge_to_event_bus(&self, bus: Arc<EventBus>, channel: &str) {
        let mut rx = self.subscribe();
        let channel = channel.to_string();
        tauri::async_runtime::spawn(async move {
            while rx.changed().await.is_ok() {
                let val = rx.borrow_and_update().clone();
                let payload = serde_json::to_value(&val).unwrap_or(serde_json::Value::Null);
                bus.publish(Event::new(&channel, payload, "signal"));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_send_and_current() {
        let signal = Signal::new(0u32);
        assert_eq!(signal.current(), 0);
        signal.send(42);
        assert_eq!(signal.current(), 42);
    }

    #[tokio::test]
    async fn test_subscriber_receives_update() {
        let signal = Signal::new(0u32);
        let mut rx = signal.subscribe();
        signal.send(1);
        rx.changed().await.unwrap();
        assert_eq!(*rx.borrow(), 1);
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let signal = Signal::new("initial".to_string());
        let mut rx1 = signal.subscribe();
        let mut rx2 = signal.subscribe();
        signal.send("updated".to_string());
        rx1.changed().await.unwrap();
        rx2.changed().await.unwrap();
        assert_eq!(*rx1.borrow(), "updated");
        assert_eq!(*rx2.borrow(), "updated");
    }

    #[tokio::test]
    async fn test_modify() {
        let signal = Signal::new(vec![1, 2, 3]);
        signal.modify(|v| v.push(4));
        assert_eq!(signal.current(), vec![1, 2, 3, 4]);
    }

    /// tokio::watch naturally coalesces rapid changes — the subscriber only
    /// sees the latest value, not every intermediate one.
    #[tokio::test]
    async fn coalesced_batches_rapid_changes() {
        let signal = Signal::new(0u32);
        let mut rx = signal.subscribe();

        // Send 5 rapid changes
        for i in 1..=5 {
            signal.send(i);
        }

        // Only the last value should be received after coalescing
        tokio::time::sleep(Duration::from_millis(50)).await;
        let val = *rx.borrow_and_update();
        assert_eq!(val, 5);
    }

    #[tokio::test]
    async fn bridge_publishes_to_event_bus() {
        let bus = Arc::new(EventBus::new(tokio::runtime::Handle::current()));
        bus.register_channel("test:signal", false);

        // Track events via a counting handler
        use crate::error::ReasonanceError;
        use crate::event_bus::EventHandler;
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct Counter(AtomicUsize);
        impl EventHandler for Counter {
            fn handle(&self, _event: &Event) -> Result<(), ReasonanceError> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
            fn id(&self) -> &str {
                "counter"
            }
        }

        let counter = Arc::new(Counter(AtomicUsize::new(0)));
        bus.subscribe("test:signal", counter.clone());

        let signal = Signal::new(0u32);
        signal.bridge_to_event_bus(bus.clone(), "test:signal");

        signal.send(42);
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert!(
            counter.0.load(Ordering::SeqCst) >= 1,
            "EventBus should have received at least one event from the signal bridge"
        );
    }
}
