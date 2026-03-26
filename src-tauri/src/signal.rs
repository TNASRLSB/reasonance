use tokio::sync::watch;

/// A thread-safe signal that notifies subscribers when the value changes.
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
