use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::Arc;

use reasonance_lib::error::ReasonanceError;
use reasonance_lib::event_bus::{AsyncEventHandler, Event, EventBus, EventHandler};

struct NoopSyncHandler;
impl EventHandler for NoopSyncHandler {
    fn handle(&self, _: &Event) -> Result<(), ReasonanceError> {
        Ok(())
    }
    fn id(&self) -> &str {
        "noop-sync"
    }
}

struct NoopAsyncHandler;
#[async_trait::async_trait]
impl AsyncEventHandler for NoopAsyncHandler {
    async fn handle(&self, _: Event) -> Result<(), ReasonanceError> {
        Ok(())
    }
    fn id(&self) -> &str {
        "noop-async"
    }
}

fn bench_publish_10_subscribers(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let bus = EventBus::new(rt.handle().clone());
    bus.register_channel("bench", false);

    // 7 sync + 3 async subscribers
    for _ in 0..7 {
        let h: Arc<dyn EventHandler> = Arc::new(NoopSyncHandler);
        bus.subscribe("bench", h);
    }
    for _ in 0..3 {
        let h: Arc<dyn AsyncEventHandler> = Arc::new(NoopAsyncHandler);
        bus.subscribe_async("bench", h);
    }

    c.bench_function("publish_10_mixed_subs", |b| {
        b.iter(|| {
            bus.publish(Event::new("bench", serde_json::json!(null), "bench"));
        });
    });
}

fn bench_publish_sync_only(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let bus = EventBus::new(rt.handle().clone());
    bus.register_channel("bench-sync", false);

    for _ in 0..10 {
        let h: Arc<dyn EventHandler> = Arc::new(NoopSyncHandler);
        bus.subscribe("bench-sync", h);
    }

    c.bench_function("publish_10_sync_subs", |b| {
        b.iter(|| {
            bus.publish(Event::new("bench-sync", serde_json::json!(null), "bench"));
        });
    });
}

criterion_group!(
    benches,
    bench_publish_10_subscribers,
    bench_publish_sync_only
);
criterion_main!(benches);
