use super::SessionMetrics;
use log::{debug, error, info, warn};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct AnalyticsStore {
    path: PathBuf,
    completed: Mutex<Vec<SessionMetrics>>,
}

impl AnalyticsStore {
    pub fn new(dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|e| format!("Failed to create analytics dir: {}", e))?;
        info!("AnalyticsStore initialized at {}", dir.display());

        let path = dir.to_path_buf();
        let mut store = Self {
            path,
            completed: Mutex::new(Vec::new()),
        };
        store.load()?;
        Ok(store)
    }

    pub fn append(&self, metrics: &SessionMetrics) -> Result<(), String> {
        let json = serde_json::to_string(metrics)
            .map_err(|e| {
                error!("Failed to serialize metrics for session {}: {}", metrics.session_id, e);
                format!("Failed to serialize metrics: {}", e)
            })?;

        let file_path = self.path.join("metrics.jsonl");
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .map_err(|e| {
                error!("Failed to open metrics file {}: {}", file_path.display(), e);
                format!("Failed to open metrics file: {}", e)
            })?;

        writeln!(file, "{}", json)
            .map_err(|e| {
                error!("Failed to write metrics to {}: {}", file_path.display(), e);
                format!("Failed to write metrics: {}", e)
            })?;

        debug!("Appended metrics for session {} to {}", metrics.session_id, file_path.display());
        self.completed.lock().unwrap_or_else(|e| e.into_inner()).push(metrics.clone());
        Ok(())
    }

    #[allow(dead_code)] // Used in tests; with_completed preferred in production
    pub fn all_completed(&self) -> Vec<SessionMetrics> {
        self.completed.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Run a closure with a read-only reference to the completed metrics,
    /// avoiding a full Vec clone for queries that filter/aggregate.
    pub fn with_completed<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&[SessionMetrics]) -> R,
    {
        let guard = self.completed.lock().unwrap_or_else(|e| e.into_inner());
        f(&guard)
    }

    fn load(&mut self) -> Result<(), String> {
        let file_path = self.path.join("metrics.jsonl");
        if !file_path.exists() {
            debug!("No existing metrics file at {}, starting fresh", file_path.display());
            return Ok(());
        }

        debug!("Loading metrics from {}", file_path.display());
        let file = fs::File::open(&file_path)
            .map_err(|e| {
                error!("Failed to open metrics file {}: {}", file_path.display(), e);
                format!("Failed to open metrics file: {}", e)
            })?;
        let reader = BufReader::new(file);
        let mut completed = Vec::new();
        let mut skipped = 0u32;

        for line in reader.lines() {
            let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<SessionMetrics>(&line) {
                Ok(metrics) => completed.push(metrics),
                Err(_) => { skipped += 1; continue; } // skip corrupted lines
            }
        }

        if skipped > 0 {
            warn!("Skipped {} corrupted lines while loading metrics from {}", skipped, file_path.display());
        }
        info!("Loaded {} session metrics from {}", completed.len(), file_path.display());
        *self.completed.lock().unwrap_or_else(|e| e.into_inner()) = completed;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_metrics(id: &str, provider: &str) -> SessionMetrics {
        let mut m = SessionMetrics::new(id, provider, "test-model", 1000);
        m.input_tokens = 100;
        m.output_tokens = 50;
        m.ended_at = Some(2000);
        m
    }

    #[test]
    fn test_store_new_creates_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        let store_dir = dir.path().join("analytics");
        let store = AnalyticsStore::new(&store_dir).unwrap();
        assert!(store_dir.exists());
        assert!(store.all_completed().is_empty());
    }

    #[test]
    fn test_store_append_and_read() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = AnalyticsStore::new(dir.path()).unwrap();

        let m1 = sample_metrics("s1", "claude");
        let m2 = sample_metrics("s2", "gemini");
        store.append(&m1).unwrap();
        store.append(&m2).unwrap();

        let all = store.all_completed();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].session_id, "s1");
        assert_eq!(all[1].session_id, "s2");
    }

    #[test]
    fn test_store_persists_across_instances() {
        let dir = tempfile::TempDir::new().unwrap();

        {
            let store = AnalyticsStore::new(dir.path()).unwrap();
            store.append(&sample_metrics("s1", "claude")).unwrap();
            store.append(&sample_metrics("s2", "gemini")).unwrap();
        }

        let store2 = AnalyticsStore::new(dir.path()).unwrap();
        assert_eq!(store2.all_completed().len(), 2);
    }

    #[test]
    fn test_store_handles_empty_file() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(dir.path().join("metrics.jsonl"), "").unwrap();
        let store = AnalyticsStore::new(dir.path()).unwrap();
        assert!(store.all_completed().is_empty());
    }

    #[test]
    fn test_store_skips_corrupted_lines() {
        let dir = tempfile::TempDir::new().unwrap();
        let m = sample_metrics("s1", "claude");
        let good_line = serde_json::to_string(&m).unwrap();
        let content = format!("{}\nnot valid json\n{}\n", good_line, good_line);
        fs::write(dir.path().join("metrics.jsonl"), content).unwrap();

        let store = AnalyticsStore::new(dir.path()).unwrap();
        assert_eq!(store.all_completed().len(), 2);
    }
}
