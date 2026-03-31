use crate::error::ReasonanceError;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileOperation {
    Create { path: String },
    Delete { path: String, trash_path: String },
    Rename { old_path: String, new_path: String },
    Move { old_path: String, new_path: String },
}

pub struct FileOpsManager {
    undo_stack: Mutex<VecDeque<FileOperation>>,
    redo_stack: Mutex<Vec<FileOperation>>,
    trash_dir: PathBuf,
    max_undo: usize,
}

impl FileOpsManager {
    pub fn new(project_root: &Path) -> Self {
        let trash_dir = project_root.join(".reasonance").join(".trash");
        Self {
            undo_stack: Mutex::new(VecDeque::new()),
            redo_stack: Mutex::new(Vec::new()),
            trash_dir,
            max_undo: 50,
        }
    }

    /// Delete a file by moving it to trash.
    pub fn delete_file(&self, path: &str) -> Result<(), ReasonanceError> {
        let source = Path::new(path);
        if !source.exists() {
            return Err(ReasonanceError::not_found("file", path));
        }

        std::fs::create_dir_all(&self.trash_dir)
            .map_err(|e| ReasonanceError::io("create trash dir", e))?;

        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
        let filename = source.file_name().unwrap_or_default().to_string_lossy();
        let trash_name = format!("{}-{}", timestamp, filename);
        let trash_path = self.trash_dir.join(&trash_name);

        std::fs::rename(source, &trash_path)
            .map_err(|e| ReasonanceError::io("move to trash", e))?;

        self.push_undo(FileOperation::Delete {
            path: path.to_string(),
            trash_path: trash_path.to_string_lossy().to_string(),
        });

        Ok(())
    }

    /// Record a file creation (for undo).
    pub fn record_create(&self, path: &str) {
        self.push_undo(FileOperation::Create {
            path: path.to_string(),
        });
    }

    /// Record a rename (for undo).
    pub fn record_rename(&self, old_path: &str, new_path: &str) {
        self.push_undo(FileOperation::Rename {
            old_path: old_path.to_string(),
            new_path: new_path.to_string(),
        });
    }


    /// Perform a move and record it for undo.
    pub fn move_file(&self, old_path: &str, new_path: &str) -> Result<(), ReasonanceError> {
        let src = Path::new(old_path);
        if !src.exists() {
            return Err(ReasonanceError::not_found("file", old_path));
        }
        if let Some(parent) = Path::new(new_path).parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| ReasonanceError::io("create destination dir", e))?;
            }
        }
        std::fs::rename(src, new_path).map_err(|e| ReasonanceError::io("move file", e))?;
        self.push_undo(FileOperation::Move {
            old_path: old_path.to_string(),
            new_path: new_path.to_string(),
        });
        Ok(())
    }

    /// Undo the last operation. Returns a description of what was undone, or None
    /// if the stack is empty.
    pub fn undo(&self) -> Result<Option<String>, ReasonanceError> {
        let op = self.undo_stack.lock().unwrap().pop_back();
        match op {
            Some(FileOperation::Delete { path, trash_path }) => {
                // Restore the parent directory if it was removed
                if let Some(parent) = Path::new(&path).parent() {
                    if !parent.exists() {
                        std::fs::create_dir_all(parent).map_err(|e| {
                            ReasonanceError::io("recreate parent dir for restore", e)
                        })?;
                    }
                }
                std::fs::rename(&trash_path, &path)
                    .map_err(|e| ReasonanceError::io("restore from trash", e))?;
                self.redo_stack.lock().unwrap().push(FileOperation::Delete {
                    path: path.clone(),
                    trash_path,
                });
                Ok(Some(format!("Restored: {}", path)))
            }
            Some(FileOperation::Create { path }) => {
                if Path::new(&path).exists() {
                    std::fs::remove_file(&path)
                        .map_err(|e| ReasonanceError::io("undo create", e))?;
                }
                self.redo_stack
                    .lock()
                    .unwrap()
                    .push(FileOperation::Create { path: path.clone() });
                Ok(Some(format!("Removed: {}", path)))
            }
            Some(FileOperation::Rename { old_path, new_path }) => {
                std::fs::rename(&new_path, &old_path)
                    .map_err(|e| ReasonanceError::io("undo rename", e))?;
                self.redo_stack
                    .lock()
                    .unwrap()
                    .push(FileOperation::Rename { old_path, new_path });
                Ok(Some("Rename undone".to_string()))
            }
            Some(FileOperation::Move { old_path, new_path }) => {
                // Restore destination parent if needed
                if let Some(parent) = Path::new(&old_path).parent() {
                    if !parent.exists() {
                        std::fs::create_dir_all(parent).map_err(|e| {
                            ReasonanceError::io("recreate source dir for move undo", e)
                        })?;
                    }
                }
                std::fs::rename(&new_path, &old_path)
                    .map_err(|e| ReasonanceError::io("undo move", e))?;
                self.redo_stack.lock().unwrap().push(FileOperation::Move {
                    old_path: old_path.clone(),
                    new_path,
                });
                Ok(Some(format!("Move undone: {}", old_path)))
            }
            None => Ok(None),
        }
    }

    /// Return the current undo stack depth.
    #[cfg(test)]
    pub fn undo_depth(&self) -> usize {
        self.undo_stack.lock().unwrap().len()
    }

    fn push_undo(&self, op: FileOperation) {
        let mut stack = self.undo_stack.lock().unwrap();
        stack.push_back(op);
        while stack.len() > self.max_undo {
            stack.pop_front();
        }
        // New operation clears redo
        self.redo_stack.lock().unwrap().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, FileOpsManager) {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let mgr = FileOpsManager::new(dir.path());
        (dir, mgr)
    }

    #[test]
    fn delete_moves_to_trash() {
        let (dir, mgr) = setup();
        let file = dir.path().join("victim.txt");
        std::fs::write(&file, "precious data").unwrap();

        mgr.delete_file(file.to_str().unwrap()).unwrap();

        // Original file should no longer exist
        assert!(!file.exists());
        // Trash dir should contain exactly one file
        let trash_entries: Vec<_> = std::fs::read_dir(&mgr.trash_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(trash_entries.len(), 1);
        // Trash file should contain the original data
        let trash_content = std::fs::read_to_string(trash_entries[0].path()).unwrap();
        assert_eq!(trash_content, "precious data");
    }

    #[test]
    fn undo_restores_from_trash() {
        let (dir, mgr) = setup();
        let file = dir.path().join("restore_me.txt");
        std::fs::write(&file, "important").unwrap();

        mgr.delete_file(file.to_str().unwrap()).unwrap();
        assert!(!file.exists());

        let result = mgr.undo().unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("Restored"));
        assert!(file.exists());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "important");
    }

    #[test]
    fn create_record_and_undo_removes_file() {
        let (dir, mgr) = setup();
        let file = dir.path().join("new_file.txt");
        std::fs::write(&file, "created").unwrap();

        mgr.record_create(file.to_str().unwrap());

        let result = mgr.undo().unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("Removed"));
        assert!(!file.exists());
    }

    #[test]
    fn rename_record_and_undo_reverses() {
        let (dir, mgr) = setup();
        let old = dir.path().join("old_name.txt");
        let new = dir.path().join("new_name.txt");
        std::fs::write(&old, "data").unwrap();
        std::fs::rename(&old, &new).unwrap();

        mgr.record_rename(old.to_str().unwrap(), new.to_str().unwrap());

        let result = mgr.undo().unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("Rename undone"));
        assert!(old.exists());
        assert!(!new.exists());
        assert_eq!(std::fs::read_to_string(&old).unwrap(), "data");
    }

    #[test]
    fn max_undo_stack_enforced() {
        let (dir, mgr) = setup();

        for i in 0..60 {
            let file = dir.path().join(format!("file_{}.txt", i));
            std::fs::write(&file, "x").unwrap();
            mgr.record_create(file.to_str().unwrap());
        }

        assert_eq!(mgr.undo_depth(), 50);
    }

    #[test]
    fn redo_cleared_on_new_operation() {
        let (dir, mgr) = setup();
        let file1 = dir.path().join("f1.txt");
        let file2 = dir.path().join("f2.txt");
        std::fs::write(&file1, "a").unwrap();
        std::fs::write(&file2, "b").unwrap();

        mgr.record_create(file1.to_str().unwrap());
        mgr.undo().unwrap(); // redo stack now has 1 entry

        // New operation should clear redo
        mgr.record_create(file2.to_str().unwrap());
        assert!(mgr.redo_stack.lock().unwrap().is_empty());
    }

    #[test]
    fn undo_empty_stack_returns_none() {
        let (_dir, mgr) = setup();
        let result = mgr.undo().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn delete_nonexistent_file_returns_error() {
        let (_dir, mgr) = setup();
        let result = mgr.delete_file("/tmp/this_does_not_exist_reasonance_test.txt");
        assert!(result.is_err());
    }
}
