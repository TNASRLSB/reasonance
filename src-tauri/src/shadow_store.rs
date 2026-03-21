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
        self.copies
            .lock()
            .unwrap()
            .insert(path.to_string(), content.to_string());
    }

    pub fn get(&self, path: &str) -> Option<String> {
        self.copies.lock().unwrap().get(path).cloned()
    }

    pub fn remove(&self, path: &str) {
        self.copies.lock().unwrap().remove(path);
    }
}
