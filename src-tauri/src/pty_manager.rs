use log::{debug, error, info, trace, warn};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

pub struct PtyInstance {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
}

pub struct PtyManager {
    instances: Arc<Mutex<HashMap<String, PtyInstance>>>,
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            instances: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn spawn(
        &self,
        command: &str,
        args: &[String],
        cwd: &str,
        app: AppHandle,
    ) -> Result<String, String> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;

        let mut cmd = CommandBuilder::new(command);
        for arg in args {
            cmd.arg(arg);
        }
        cmd.cwd(cwd);

        let _child = pair.slave.spawn_command(cmd).map_err(|e| {
            error!("Failed to spawn PTY command '{}' in cwd '{}': {}", command, cwd, e);
            e.to_string()
        })?;
        drop(pair.slave);

        let id = Uuid::new_v4().to_string();
        info!("PTY spawned: id={}, command='{}', cwd='{}'", id, command, cwd);
        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;

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

    pub fn write(&self, id: &str, data: &str) -> Result<(), String> {
        const MAX_PAYLOAD: usize = 65_536; // 64 KB
        if data.len() > MAX_PAYLOAD {
            warn!("PTY write payload too large: id={}, size={} bytes (max {})", id, data.len(), MAX_PAYLOAD);
            return Err(format!(
                "PTY write payload too large: {} bytes (max {})",
                data.len(),
                MAX_PAYLOAD
            ));
        }
        trace!("PTY write: id={}, bytes={}", id, data.len());
        let mut instances = self.instances.lock().unwrap_or_else(|e| e.into_inner());
        let instance = instances.get_mut(id).ok_or_else(|| {
            error!("PTY write failed: id={} not found", id);
            "PTY not found".to_string()
        })?;
        instance
            .writer
            .write_all(data.as_bytes())
            .map_err(|e| {
                error!("PTY write I/O error: id={}, error={}", id, e);
                e.to_string()
            })
    }

    pub fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<(), String> {
        debug!("PTY resize: id={}, cols={}, rows={}", id, cols, rows);
        let instances = self.instances.lock().unwrap_or_else(|e| e.into_inner());
        let instance = instances.get(id).ok_or("PTY not found")?;
        instance
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())
    }

    pub fn kill(&self, id: &str) -> Result<(), String> {
        info!("PTY kill: id={}", id);
        let mut instances = self.instances.lock().unwrap_or_else(|e| e.into_inner());
        instances
            .remove(id)
            .ok_or_else(|| {
                error!("PTY kill failed: id={} not found", id);
                "PTY not found".to_string()
            })?;
        Ok(())
    }
}
