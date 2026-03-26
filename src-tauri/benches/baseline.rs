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

criterion_group!(
    benches,
    bench_json_serialization,
    bench_toml_parsing,
    bench_uuid_generation,
    bench_sha256_hash
);
criterion_main!(benches);
