use super::SessionMetrics;
use crate::error::ReasonanceError;
use crate::storage::StorageBackend;
use log::{debug, error, info, warn};
use std::sync::{Arc, Mutex};

const NAMESPACE: &str = "analytics";
const STREAM_KEY: &str = "metrics";

pub struct AnalyticsStore {
    backend: Arc<dyn StorageBackend>,
    completed: Mutex<Vec<SessionMetrics>>,
}

impl AnalyticsStore {
    pub async fn new(backend: Arc<dyn StorageBackend>) -> Result<Self, ReasonanceError> {
        info!("AnalyticsStore initializing with StorageBackend");

        let lines = backend.read_stream(NAMESPACE, STREAM_KEY).await?;
        let mut completed = Vec::new();
        let mut skipped = 0u32;

        for line in &lines {
            if line.is_empty() {
                continue;
            }
            match serde_json::from_slice::<SessionMetrics>(line) {
                Ok(metrics) => completed.push(metrics),
                Err(_) => {
                    skipped += 1;
                }
            }
        }

        if skipped > 0 {
            warn!(
                "Skipped {} corrupted lines while loading analytics metrics",
                skipped
            );
        }
        info!("Loaded {} session metrics from storage", completed.len());

        Ok(Self {
            backend,
            completed: Mutex::new(completed),
        })
    }

    pub async fn append(&self, metrics: &SessionMetrics) -> Result<(), ReasonanceError> {
        let json = serde_json::to_vec(metrics).map_err(|e| {
            error!(
                "Failed to serialize metrics for session {}: {}",
                metrics.session_id, e
            );
            ReasonanceError::serialization("analytics metrics", e.to_string())
        })?;

        self.backend.append(NAMESPACE, STREAM_KEY, &json).await?;

        debug!(
            "Appended metrics for session {} to storage",
            metrics.session_id,
        );
        self.completed
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(metrics.clone());
        Ok(())
    }

    #[cfg(test)]
    pub fn all_completed(&self) -> Vec<SessionMetrics> {
        self.completed
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InMemoryBackend;

    fn sample_metrics(id: &str, provider: &str) -> SessionMetrics {
        let mut m = SessionMetrics::new(id, provider, "test-model", 1000);
        m.input_tokens = 100;
        m.output_tokens = 50;
        m.ended_at = Some(2000);
        m
    }

    #[tokio::test]
    async fn test_store_new_empty() {
        let backend = Arc::new(InMemoryBackend::new());
        let store = AnalyticsStore::new(backend).await.unwrap();
        assert!(store.all_completed().is_empty());
    }

    #[tokio::test]
    async fn test_store_append_and_read() {
        let backend = Arc::new(InMemoryBackend::new());
        let store = AnalyticsStore::new(backend).await.unwrap();

        let m1 = sample_metrics("s1", "claude");
        let m2 = sample_metrics("s2", "gemini");
        store.append(&m1).await.unwrap();
        store.append(&m2).await.unwrap();

        let all = store.all_completed();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].session_id, "s1");
        assert_eq!(all[1].session_id, "s2");
    }

    #[tokio::test]
    async fn test_store_persists_across_instances() {
        let backend = Arc::new(InMemoryBackend::new());

        {
            let store = AnalyticsStore::new(backend.clone()).await.unwrap();
            store.append(&sample_metrics("s1", "claude")).await.unwrap();
            store.append(&sample_metrics("s2", "gemini")).await.unwrap();
        }

        let store2 = AnalyticsStore::new(backend).await.unwrap();
        assert_eq!(store2.all_completed().len(), 2);
    }

    #[tokio::test]
    async fn test_store_handles_empty_stream() {
        let backend = Arc::new(InMemoryBackend::new());
        // No data appended -- read_stream returns empty Vec
        let store = AnalyticsStore::new(backend).await.unwrap();
        assert!(store.all_completed().is_empty());
    }

    #[tokio::test]
    async fn test_store_skips_corrupted_lines() {
        let backend = Arc::new(InMemoryBackend::new());

        // Write a good line, a bad line, and another good line directly
        let m = sample_metrics("s1", "claude");
        let good_bytes = serde_json::to_vec(&m).unwrap();
        backend
            .append(NAMESPACE, STREAM_KEY, &good_bytes)
            .await
            .unwrap();
        backend
            .append(NAMESPACE, STREAM_KEY, b"not valid json")
            .await
            .unwrap();
        backend
            .append(NAMESPACE, STREAM_KEY, &good_bytes)
            .await
            .unwrap();

        let store = AnalyticsStore::new(backend).await.unwrap();
        assert_eq!(store.all_completed().len(), 2);
    }
}
