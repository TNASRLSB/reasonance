use log::{debug, error, info, trace, warn};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

/// Configuration for PTY reconnection with exponential backoff.
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// Maximum number of reconnection attempts before giving up.
    pub max_attempts: u32,
    /// Initial delay in milliseconds before the first retry.
    pub initial_delay_ms: u64,
    /// Maximum delay in milliseconds (cap for exponential growth).
    pub max_delay_ms: u64,
    /// Multiplicative factor applied per attempt.
    pub backoff_factor: f64,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            max_attempts: 10,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_factor: 2.0,
        }
    }
}

impl ReconnectConfig {
    /// Calculate the wait duration before attempt `attempt` (0-indexed).
    ///
    /// Attempt 0 → `initial_delay_ms`, doubling each time, capped at `max_delay_ms`.
    pub fn delay_for_attempt(&self, attempt: u32) -> std::time::Duration {
        let delay = self.initial_delay_ms as f64 * self.backoff_factor.powi(attempt as i32);
        let capped = delay.min(self.max_delay_ms as f64) as u64;
        std::time::Duration::from_millis(capped)
    }
}

pub struct PtyInstance {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
}

pub struct PtyManager {
    instances: Arc<Mutex<HashMap<String, PtyInstance>>>,
    project_map: Arc<Mutex<HashMap<String, String>>>, // pty_id -> project_id
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            instances: Arc::new(Mutex::new(HashMap::new())),
            project_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Associate a PTY instance with a project.
    #[allow(dead_code)] // Used by project management PTY lifecycle
    pub fn set_project(&self, pty_id: &str, project_id: &str) {
        debug!("PTY set_project: pty_id={}, project_id={}", pty_id, project_id);
        let mut map = self.project_map.lock().unwrap_or_else(|e| e.into_inner());
        map.insert(pty_id.to_string(), project_id.to_string());
    }

    /// Kill all PTY instances belonging to a project. Returns the IDs of killed PTYs.
    pub fn kill_project_ptys(&self, project_id: &str) -> Vec<String> {
        info!("PTY kill_project_ptys: project_id={}", project_id);
        let map = self.project_map.lock().unwrap_or_else(|e| e.into_inner());
        let pty_ids: Vec<String> = map
            .iter()
            .filter(|(_, pid)| pid.as_str() == project_id)
            .map(|(id, _)| id.clone())
            .collect();
        drop(map);

        let mut killed = Vec::new();
        for id in pty_ids {
            if self.kill(&id).is_ok() {
                killed.push(id);
            }
        }
        info!("PTY kill_project_ptys: project_id={}, killed={}", project_id, killed.len());
        killed
    }

    pub fn spawn(
        &self,
        command: &str,
        args: &[String],
        cwd: &str,
        app: AppHandle,
    ) -> Result<String, crate::error::ReasonanceError> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| crate::error::ReasonanceError::internal(e.to_string()))?;

        let mut cmd = CommandBuilder::new(command);
        for arg in args {
            cmd.arg(arg);
        }
        cmd.cwd(cwd);

        let _child = pair.slave.spawn_command(cmd).map_err(|e| {
            error!("Failed to spawn PTY command '{}' in cwd '{}': {}", command, cwd, e);
            crate::error::ReasonanceError::Transport {
                provider: command.to_string(),
                message: e.to_string(),
                retryable: false,
            }
        })?;
        drop(pair.slave);

        let id = Uuid::new_v4().to_string();
        info!("PTY spawned: id={}, command='{}', cwd='{}'", id, command, cwd);
        let writer = pair.master.take_writer().map_err(|e| crate::error::ReasonanceError::internal(e.to_string()))?;
        let mut reader = pair.master.try_clone_reader().map_err(|e| crate::error::ReasonanceError::internal(e.to_string()))?;

        let read_id = id.clone();
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        debug!("PTY reader EOF: id={}", read_id);
                        let _ = app.emit(&format!("pty-exit-{}", read_id), 0);
                        break;
                    }
                    Ok(n) => {
                        trace!("PTY data: id={}, bytes={}", read_id, n);
                        let data = String::from_utf8_lossy(&buf[..n]).to_string();
                        let _ = app.emit(&format!("pty-data-{}", read_id), data);
                    }
                    Err(e) => {
                        warn!("PTY read error: id={}, error={}", read_id, e);
                        let _ = app.emit(&format!("pty-exit-{}", read_id), 1);
                        break;
                    }
                }
            }
        });

        let instance = PtyInstance {
            master: pair.master,
            writer,
        };
        self.instances.lock().unwrap_or_else(|e| e.into_inner()).insert(id.clone(), instance);
        Ok(id)
    }

    pub fn write(&self, id: &str, data: &str) -> Result<(), crate::error::ReasonanceError> {
        const MAX_PAYLOAD: usize = 65_536; // 64 KB
        if data.len() > MAX_PAYLOAD {
            warn!("PTY write payload too large: id={}, size={} bytes (max {})", id, data.len(), MAX_PAYLOAD);
            return Err(crate::error::ReasonanceError::validation(
                "data",
                format!("PTY write payload too large: {} bytes (max {})", data.len(), MAX_PAYLOAD),
            ));
        }
        trace!("PTY write: id={}, bytes={}", id, data.len());
        let mut instances = self.instances.lock().unwrap_or_else(|e| e.into_inner());
        let instance = instances.get_mut(id).ok_or_else(|| {
            error!("PTY write failed: id={} not found", id);
            crate::error::ReasonanceError::not_found("pty", id)
        })?;
        instance
            .writer
            .write_all(data.as_bytes())
            .map_err(|e| {
                error!("PTY write I/O error: id={}, error={}", id, e);
                crate::error::ReasonanceError::io(format!("PTY write id={}", id), e)
            })
    }

    pub fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<(), crate::error::ReasonanceError> {
        debug!("PTY resize: id={}, cols={}, rows={}", id, cols, rows);
        let instances = self.instances.lock().unwrap_or_else(|e| e.into_inner());
        let instance = instances.get(id).ok_or_else(|| crate::error::ReasonanceError::not_found("pty", id))?;
        instance
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| crate::error::ReasonanceError::internal(e.to_string()))
    }

    /// Kill a PTY by ID — alias kept for call sites that use `kill_process` naming.
    pub fn kill_process(&self, id: &str) -> Result<(), crate::error::ReasonanceError> {
        self.kill(id)
    }

    pub fn kill(&self, id: &str) -> Result<(), crate::error::ReasonanceError> {
        info!("PTY kill: id={}", id);
        let mut instances = self.instances.lock().unwrap_or_else(|e| e.into_inner());
        instances
            .remove(id)
            .ok_or_else(|| {
                error!("PTY kill failed: id={} not found", id);
                crate::error::ReasonanceError::not_found("pty", id)
            })?;
        // Clean up project association
        let mut project_map = self.project_map.lock().unwrap_or_else(|e| e.into_inner());
        project_map.remove(id);
        Ok(())
    }

    /// Returns the IDs of all currently tracked PTY instances.
    pub fn list_active_ptys(&self) -> Vec<String> {
        let instances = self.instances.lock().unwrap_or_else(|e| e.into_inner());
        instances.keys().cloned().collect()
    }

    /// Removes entries for PTYs whose master PTY has been closed (EOF reached).
    ///
    /// The reader thread emits `pty-exit-{id}` and terminates when the process
    /// exits, but the `PtyInstance` entry is only removed when the frontend
    /// explicitly calls `kill_process`. This method proactively sweeps entries
    /// whose master has become unresponsive (resize returns an error), freeing
    /// the associated handles.
    ///
    /// Note: this is a best-effort heuristic. A resize error is the only
    /// portable indicator of a dead master without storing the child handle.
    /// Active PTYs are never killed — only dead ones are removed.
    pub fn sweep_dead_ptys(&self) -> Vec<String> {
        let dead_ids: Vec<String> = {
            let instances = self.instances.lock().unwrap_or_else(|e| e.into_inner());
            instances
                .iter()
                .filter_map(|(id, inst)| {
                    // Try a no-op resize to check if the master is still alive.
                    // We re-use the current size (24×80 default) — an error means the
                    // process has already exited and the master fd is closed.
                    let probe = inst.master.resize(portable_pty::PtySize {
                        rows: 24,
                        cols: 80,
                        pixel_width: 0,
                        pixel_height: 0,
                    });
                    if probe.is_err() { Some(id.clone()) } else { None }
                })
                .collect()
        };

        let mut swept = Vec::new();
        for id in &dead_ids {
            let mut instances = self.instances.lock().unwrap_or_else(|e| e.into_inner());
            if instances.remove(id).is_some() {
                swept.push(id.clone());
            }
            drop(instances);
            let mut project_map = self.project_map.lock().unwrap_or_else(|e| e.into_inner());
            project_map.remove(id);
        }

        if !swept.is_empty() {
            info!("PTY sweep_dead_ptys: removed {} dead entries: {:?}", swept.len(), swept);
        } else {
            debug!("PTY sweep_dead_ptys: no dead PTYs found");
        }
        swept
    }

    /// Kills all active PTY instances. Intended for app shutdown only.
    ///
    /// Errors from individual kills are logged and ignored — the goal is
    /// best-effort cleanup, not hard guarantees.
    pub fn kill_all(&self) -> usize {
        info!("PTY kill_all: killing all active PTY instances");
        let ids: Vec<String> = {
            let instances = self.instances.lock().unwrap_or_else(|e| e.into_inner());
            instances.keys().cloned().collect()
        };
        let count = ids.len();
        for id in &ids {
            if let Err(e) = self.kill(id) {
                warn!("PTY kill_all: failed to kill id={}: {}", id, e);
            }
        }
        info!("PTY kill_all: killed {} instances", count);
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_timing() {
        let config = ReconnectConfig::default();
        assert_eq!(config.delay_for_attempt(0).as_millis(), 1000);  // 1s
        assert_eq!(config.delay_for_attempt(1).as_millis(), 2000);  // 2s
        assert_eq!(config.delay_for_attempt(2).as_millis(), 4000);  // 4s
        assert_eq!(config.delay_for_attempt(3).as_millis(), 8000);  // 8s
        assert_eq!(config.delay_for_attempt(4).as_millis(), 16000); // 16s
        assert_eq!(config.delay_for_attempt(5).as_millis(), 30000); // capped at 30s
        assert_eq!(config.delay_for_attempt(10).as_millis(), 30000); // still capped
    }

    #[test]
    fn test_max_attempts() {
        let config = ReconnectConfig::default();
        assert_eq!(config.max_attempts, 10);
    }

    #[test]
    fn test_custom_config() {
        let config = ReconnectConfig {
            max_attempts: 5,
            initial_delay_ms: 500,
            max_delay_ms: 10000,
            backoff_factor: 3.0,
        };
        assert_eq!(config.delay_for_attempt(0).as_millis(), 500);   // 500ms
        assert_eq!(config.delay_for_attempt(1).as_millis(), 1500);  // 1.5s
        assert_eq!(config.delay_for_attempt(2).as_millis(), 4500);  // 4.5s
        assert_eq!(config.delay_for_attempt(3).as_millis(), 10000); // capped at 10s
    }

    #[test]
    fn test_list_active_ptys_empty() {
        let manager = PtyManager::new();
        assert!(manager.list_active_ptys().is_empty());
    }

    #[test]
    fn test_kill_all_empty() {
        let manager = PtyManager::new();
        // kill_all on an empty manager should return 0 and not panic
        assert_eq!(manager.kill_all(), 0);
    }

    #[test]
    fn test_sweep_dead_ptys_empty() {
        let manager = PtyManager::new();
        // sweep on empty manager should return empty vec and not panic
        let swept = manager.sweep_dead_ptys();
        assert!(swept.is_empty());
    }
}
