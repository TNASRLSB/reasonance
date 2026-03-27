use criterion::{criterion_group, criterion_main, Criterion};

fn bench_json_serialization(c: &mut Criterion) {
    // Benchmark JSON serialization of a typical AgentEvent-sized struct
    let data = serde_json::json!({
        "event_type": "text",
        "content": { "value": "Hello world ".repeat(100) },
        "metadata": {
            "provider": "claude",
            "model": "opus",
            "session_id": "abc-123",
            "timestamp": "2026-03-26T00:00:00Z"
        }
    });

    c.bench_function("json_serialize_event", |b| {
        b.iter(|| serde_json::to_string(&data).unwrap())
    });

    let json_str = serde_json::to_string(&data).unwrap();
    c.bench_function("json_deserialize_event", |b| {
        b.iter(|| serde_json::from_str::<serde_json::Value>(&json_str).unwrap())
    });
}

fn bench_toml_parsing(c: &mut Criterion) {
    // Benchmark TOML config parsing (representative of settings load)
    let toml_str = r#"
[models]
default = "claude-opus-4-6"

[permissions]
level = "ask"

[editor]
font_size = 14
tab_size = 2
font_family = "Atkinson Hyperlegible Mono"

[terminal]
shell = "/bin/bash"
font_size = 13
"#;

    c.bench_function("toml_parse_config", |b| {
        b.iter(|| toml::from_str::<toml::Value>(toml_str).unwrap())
    });
}

fn bench_uuid_generation(c: &mut Criterion) {
    c.bench_function("uuid_v4_generate", |b| {
        b.iter(|| uuid::Uuid::new_v4().to_string())
    });
}

fn bench_sha256_hash(c: &mut Criterion) {
    use sha2::{Digest, Sha256};
    let data = "a".repeat(10_000); // ~10KB, typical source file

    c.bench_function("sha256_10kb", |b| {
        b.iter(|| {
            let mut hasher = Sha256::new();
            hasher.update(data.as_bytes());
            format!("{:x}", hasher.finalize())
        })
    });
}

fn bench_storage_put_get(c: &mut Criterion) {
    use std::sync::Arc;

    use reasonance_lib::storage::{InMemoryBackend, StorageBackend};

    let rt = tokio::runtime::Runtime::new().unwrap();
    let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryBackend::new());

    let value = serde_json::to_vec(&serde_json::json!({
        "event_type": "text",
        "content": "Hello world ".repeat(50),
        "session_id": "bench-session-001"
    }))
    .unwrap();

    c.bench_function("storage_put_inmemory", |b| {
        let backend = backend.clone();
        let value = value.clone();
        b.iter(|| {
            rt.block_on(backend.put("bench-ns", "key-1", &value))
                .unwrap();
        });
    });

    // Pre-populate for get benchmark
    rt.block_on(backend.put("bench-ns", "get-key", &value))
        .unwrap();
    c.bench_function("storage_get_inmemory", |b| {
        let backend = backend.clone();
        b.iter(|| {
            rt.block_on(backend.get("bench-ns", "get-key")).unwrap();
        });
    });

    // Populate keys for list benchmark
    for i in 0..100 {
        rt.block_on(backend.put("bench-ns", &format!("item-{i:03}"), &value))
            .unwrap();
    }
    c.bench_function("storage_list_keys_100", |b| {
        let backend = backend.clone();
        b.iter(|| {
            rt.block_on(backend.list_keys("bench-ns", None)).unwrap();
        });
    });
}

criterion_group!(
    benches,
    bench_json_serialization,
    bench_toml_parsing,
    bench_uuid_generation,
    bench_sha256_hash,
    bench_storage_put_get
);
criterion_main!(benches);
