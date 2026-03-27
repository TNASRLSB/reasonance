use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupBaseline {
    pub timestamp: String,
    pub total_setup_ms: u64,
    pub parallel_init_ms: u64,
}

/// Record a startup baseline to benchmarks/baselines.json (project root).
/// Keeps the last 100 entries. Errors are silently ignored (best-effort).
pub fn record_startup(baseline: &StartupBaseline) {
    let path = Path::new("benchmarks/baselines.json");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut entries: Vec<StartupBaseline> = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    entries.push(baseline.clone());
    if entries.len() > 100 {
        entries.drain(..entries.len() - 100);
    }
    let _ = std::fs::write(
        path,
        serde_json::to_string_pretty(&entries).unwrap_or_default(),
    );
}
