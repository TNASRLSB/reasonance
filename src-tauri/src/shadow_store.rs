use log::{debug, trace};
use std::collections::HashMap;
use std::sync::Mutex;

pub struct ShadowStore {
    copies: Mutex<HashMap<String, String>>,
}

impl ShadowStore {
    pub fn new() -> Self {
        Self {
            copies: Mutex::new(HashMap::new()),
        }
    }

    pub fn store(&self, path: &str, content: &str) {
        debug!("Shadow store: storing copy for path='{}'", path);
        self.copies
            .lock()
            .unwrap()
            .insert(path.to_string(), content.to_string());
    }

    pub fn get(&self, path: &str) -> Option<String> {
        trace!("Shadow store: retrieving path='{}'", path);
        self.copies.lock().unwrap_or_else(|e| e.into_inner()).get(path).cloned()
    }

    pub fn remove(&self, path: &str) {
        debug!("Shadow store: removing path='{}'", path);
        self.copies.lock().unwrap_or_else(|e| e.into_inner()).remove(path);
    }
}
