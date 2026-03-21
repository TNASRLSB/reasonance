use crate::fs_watcher::FsWatcherState;
use serde::Serialize;
use std::fs;
use std::path::Path;
use tauri::{AppHandle, State};

#[derive(Serialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: u64,
}

#[tauri::command]
pub fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, &content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_dir(path: String, respect_gitignore: bool) -> Result<Vec<FileEntry>, String> {
    let entries = fs::read_dir(&path).map_err(|e| e.to_string())?;

    let gitignore = if respect_gitignore {
        ignore::gitignore::Gitignore::new(Path::new(&path).join(".gitignore")).0
    } else {
        ignore::gitignore::Gitignore::empty()
    };

    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let metadata = entry.metadata().map_err(|e| e.to_string())?;

        if respect_gitignore {
            let matched = gitignore.matched_path_or_any_parents(&entry.path(), metadata.is_dir());
            if matched.is_ignore() {
                continue;
            }
        }

        let modified = metadata
            .modified()
            .map_err(|e| e.to_string())?
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        result.push(FileEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path: entry.path().to_string_lossy().to_string(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            modified,
        });
    }
    result.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(result)
}

#[tauri::command]
pub fn start_watching(
    path: String,
    app: AppHandle,
    state: State<'_, FsWatcherState>,
) -> Result<(), String> {
    crate::fs_watcher::start_watching(&path, app, &state)
}
