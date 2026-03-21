use crate::fs_watcher::FsWatcherState;
use serde::Serialize;
use std::fs;
use std::path::Path;
use tauri::{AppHandle, State};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: u64,
    pub is_gitignored: bool,
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

        let is_ignored = if respect_gitignore {
            let matched = gitignore.matched_path_or_any_parents(&entry.path(), metadata.is_dir());
            matched.is_ignore()
        } else {
            false
        };

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
            is_gitignored: is_ignored,
        });
    }
    result.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(result)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrepResult {
    pub path: String,
    pub line_number: usize,
    pub line: String,
}

#[tauri::command]
pub fn grep_files(
    path: String,
    pattern: String,
    respect_gitignore: bool,
) -> Result<Vec<GrepResult>, String> {
    use ignore::WalkBuilder;
    use std::io::BufRead;

    let mut results = Vec::new();
    let walker = WalkBuilder::new(&path)
        .git_ignore(respect_gitignore)
        .build();

    for entry in walker.flatten() {
        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
            continue;
        }
        let file_path = entry.path().to_owned();
        if let Ok(file) = std::fs::File::open(&file_path) {
            let reader = std::io::BufReader::new(file);
            for (i, line_result) in reader.lines().enumerate() {
                if let Ok(line) = line_result {
                    if line.contains(&pattern) {
                        results.push(GrepResult {
                            path: file_path.to_string_lossy().to_string(),
                            line_number: i + 1,
                            line,
                        });
                        if results.len() >= 500 {
                            return Ok(results);
                        }
                    }
                }
            }
        }
    }
    Ok(results)
}

#[tauri::command]
pub fn start_watching(
    path: String,
    app: AppHandle,
    state: State<'_, FsWatcherState>,
) -> Result<(), String> {
    crate::fs_watcher::start_watching(&path, app, &state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn setup_temp_dir() -> TempDir {
        tempfile::tempdir().expect("failed to create temp dir")
    }

    #[test]
    fn read_file_existing() {
        let dir = setup_temp_dir();
        let file_path = dir.path().join("hello.txt");
        std::fs::write(&file_path, "hello world").unwrap();

        let content = read_file(file_path.to_string_lossy().to_string()).unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn read_file_missing_returns_err() {
        let result = read_file("/nonexistent/path/file.txt".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn write_file_creates_file() {
        let dir = setup_temp_dir();
        let file_path = dir.path().join("output.txt");

        write_file(file_path.to_string_lossy().to_string(), "written content".to_string()).unwrap();

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "written content");
    }

    #[test]
    fn write_file_overwrites_existing() {
        let dir = setup_temp_dir();
        let file_path = dir.path().join("overwrite.txt");
        std::fs::write(&file_path, "original").unwrap();

        write_file(file_path.to_string_lossy().to_string(), "new content".to_string()).unwrap();

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "new content");
    }

    #[test]
    fn list_dir_returns_entries() {
        let dir = setup_temp_dir();
        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        std::fs::write(dir.path().join("b.txt"), "b").unwrap();
        std::fs::create_dir(dir.path().join("subdir")).unwrap();

        let entries = list_dir(dir.path().to_string_lossy().to_string(), false).unwrap();
        assert_eq!(entries.len(), 3);

        // Dirs should come before files
        assert!(entries[0].is_dir);
        assert_eq!(entries[0].name, "subdir");
    }

    #[test]
    fn list_dir_sorts_dirs_first_then_alphabetical() {
        let dir = setup_temp_dir();
        std::fs::write(dir.path().join("z.txt"), "z").unwrap();
        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        std::fs::create_dir(dir.path().join("mydir")).unwrap();

        let entries = list_dir(dir.path().to_string_lossy().to_string(), false).unwrap();
        assert!(entries[0].is_dir);
        assert_eq!(entries[1].name, "a.txt");
        assert_eq!(entries[2].name, "z.txt");
    }

    #[test]
    fn list_dir_nonexistent_returns_err() {
        let result = list_dir("/nonexistent/dir/path".to_string(), false);
        assert!(result.is_err());
    }

    #[test]
    fn grep_files_finds_matches() {
        let dir = setup_temp_dir();
        std::fs::write(dir.path().join("file1.txt"), "hello world\nfoo bar\n").unwrap();
        std::fs::write(dir.path().join("file2.txt"), "no match here\n").unwrap();

        let results = grep_files(
            dir.path().to_string_lossy().to_string(),
            "hello".to_string(),
            false,
        )
        .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line_number, 1);
        assert!(results[0].line.contains("hello world"));
    }

    #[test]
    fn grep_files_no_match_returns_empty() {
        let dir = setup_temp_dir();
        std::fs::write(dir.path().join("file.txt"), "nothing to find here\n").unwrap();

        let results = grep_files(
            dir.path().to_string_lossy().to_string(),
            "ZZZNOMATCH".to_string(),
            false,
        )
        .unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn file_entry_fields_are_populated() {
        let dir = setup_temp_dir();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "content here").unwrap();

        let entries = list_dir(dir.path().to_string_lossy().to_string(), false).unwrap();
        let entry = entries.iter().find(|e| e.name == "test.txt").unwrap();

        assert!(!entry.is_dir);
        assert_eq!(entry.size, 12); // "content here" = 12 bytes
        assert!(entry.modified > 0);
        assert!(!entry.is_gitignored);
    }
}
