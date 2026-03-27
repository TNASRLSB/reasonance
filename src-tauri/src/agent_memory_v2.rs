use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use uuid::Uuid;

use crate::error::ReasonanceError;

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS memories (
    id TEXT PRIMARY KEY,
    node_id TEXT NOT NULL,
    project_id TEXT,
    session_id TEXT,
    run_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    input_summary TEXT NOT NULL,
    output_summary TEXT NOT NULL,
    outcome TEXT NOT NULL,
    importance REAL NOT NULL DEFAULT 0.5,
    tags TEXT DEFAULT '',
    context_json TEXT DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_memories_node ON memories(node_id);
CREATE INDEX IF NOT EXISTS idx_memories_project ON memories(project_id);
CREATE INDEX IF NOT EXISTS idx_memories_importance ON memories(importance DESC);
CREATE INDEX IF NOT EXISTS idx_memories_timestamp ON memories(timestamp DESC);

CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
    input_summary, output_summary, tags,
    content=memories,
    content_rowid=rowid
);

CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
    INSERT INTO memories_fts(rowid, input_summary, output_summary, tags)
    VALUES (new.rowid, new.input_summary, new.output_summary, new.tags);
END;

CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, input_summary, output_summary, tags)
    VALUES ('delete', old.rowid, old.input_summary, old.output_summary, old.tags);
END;

CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, input_summary, output_summary, tags)
    VALUES ('delete', old.rowid, old.input_summary, old.output_summary, old.tags);
    INSERT INTO memories_fts(rowid, input_summary, output_summary, tags)
    VALUES (new.rowid, new.input_summary, new.output_summary, new.tags);
END;
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntryV2 {
    pub id: String,
    pub node_id: String,
    pub project_id: Option<String>,
    pub session_id: Option<String>,
    pub run_id: String,
    pub timestamp: String,
    pub input_summary: String,
    pub output_summary: String,
    pub outcome: String,
    pub importance: f64,
    pub tags: String,
    pub context: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryScope {
    Node(String),
    Project(String),
    Global,
    NodeInProject(String, String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortBy {
    Recency,
    Importance,
    Relevance,
}

pub struct AgentMemoryV2 {
    conn: Mutex<Connection>,
}

impl AgentMemoryV2 {
    pub fn new(db_path: &std::path::Path) -> Result<Self, ReasonanceError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ReasonanceError::internal(format!("Failed to create memory DB directory: {}", e))
            })?;
        }
        let conn = Connection::open(db_path)
            .map_err(|e| ReasonanceError::internal(format!("Failed to open memory DB: {}", e)))?;
        conn.execute_batch(SCHEMA_SQL).map_err(|e| {
            ReasonanceError::internal(format!("Failed to init memory schema: {}", e))
        })?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| ReasonanceError::internal(format!("Failed to set WAL mode: {}", e)))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// For testing — in-memory database
    pub fn in_memory() -> Result<Self, ReasonanceError> {
        let conn = Connection::open_in_memory().map_err(|e| {
            ReasonanceError::internal(format!("Failed to open in-memory DB: {}", e))
        })?;
        conn.execute_batch(SCHEMA_SQL)
            .map_err(|e| ReasonanceError::internal(format!("Failed to init schema: {}", e)))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn add_entry(&self, mut entry: MemoryEntryV2) -> Result<String, ReasonanceError> {
        if entry.id.is_empty() {
            entry.id = Uuid::new_v4().to_string();
        }
        let conn = self
            .conn
            .lock()
            .map_err(|e| ReasonanceError::internal(format!("Memory DB lock poisoned: {}", e)))?;
        let context_json =
            serde_json::to_string(&entry.context).unwrap_or_else(|_| "{}".to_string());
        conn.execute(
            "INSERT INTO memories (id, node_id, project_id, session_id, run_id, timestamp, input_summary, output_summary, outcome, importance, tags, context_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                entry.id,
                entry.node_id,
                entry.project_id,
                entry.session_id,
                entry.run_id,
                entry.timestamp,
                entry.input_summary,
                entry.output_summary,
                entry.outcome,
                entry.importance,
                entry.tags,
                context_json,
            ],
        )
        .map_err(|e| ReasonanceError::internal(format!("Failed to insert memory: {}", e)))?;
        Ok(entry.id)
    }

    pub fn get_entry(&self, id: &str) -> Result<Option<MemoryEntryV2>, ReasonanceError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| ReasonanceError::internal(format!("Memory DB lock poisoned: {}", e)))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, node_id, project_id, session_id, run_id, timestamp, input_summary, output_summary, outcome, importance, tags, context_json FROM memories WHERE id = ?1",
            )
            .map_err(|e| ReasonanceError::internal(format!("Failed to prepare query: {}", e)))?;
        let mut rows = stmt
            .query_map(params![id], |row| Ok(row_to_entry(row)))
            .map_err(|e| ReasonanceError::internal(format!("Failed to query memory: {}", e)))?;
        match rows.next() {
            Some(Ok(entry)) => Ok(Some(entry)),
            Some(Err(e)) => Err(ReasonanceError::internal(format!("Row error: {}", e))),
            None => Ok(None),
        }
    }

    pub fn search(
        &self,
        query: &str,
        scope: MemoryScope,
        limit: u32,
    ) -> Result<Vec<MemoryEntryV2>, ReasonanceError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| ReasonanceError::internal(format!("Memory DB lock poisoned: {}", e)))?;
        let (where_clause, scope_params) = scope_to_where(&scope, "m", 2);
        let sql = format!(
            "SELECT m.id, m.node_id, m.project_id, m.session_id, m.run_id, m.timestamp, m.input_summary, m.output_summary, m.outcome, m.importance, m.tags, m.context_json
             FROM memories m
             JOIN memories_fts f ON m.rowid = f.rowid
             WHERE f.memories_fts MATCH ?1 {}
             ORDER BY rank
             LIMIT ?{}",
            if where_clause.is_empty() {
                String::new()
            } else {
                format!("AND {}", where_clause)
            },
            scope_params.len() + 2
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| ReasonanceError::internal(format!("FTS query prepare failed: {}", e)))?;

        let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        all_params.push(Box::new(query.to_string()));
        for p in &scope_params {
            all_params.push(Box::new(p.clone()));
        }
        all_params.push(Box::new(limit));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            all_params.iter().map(|p| p.as_ref()).collect();

        let rows = stmt
            .query_map(param_refs.as_slice(), |row| Ok(row_to_entry(row)))
            .map_err(|e| ReasonanceError::internal(format!("FTS query failed: {}", e)))?;

        let mut results = Vec::new();
        for row in rows {
            match row {
                Ok(entry) => results.push(entry),
                Err(e) => {
                    return Err(ReasonanceError::internal(format!("Row error: {}", e)));
                }
            }
        }
        Ok(results)
    }

    pub fn list(
        &self,
        scope: MemoryScope,
        sort: SortBy,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<MemoryEntryV2>, ReasonanceError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| ReasonanceError::internal(format!("Memory DB lock poisoned: {}", e)))?;
        let (where_clause, scope_params) = scope_to_where(&scope, "", 1);
        let order_by = match sort {
            SortBy::Recency => "timestamp DESC",
            SortBy::Importance => "importance DESC",
            SortBy::Relevance => "timestamp DESC", // fallback for non-FTS
        };
        let sql = format!(
            "SELECT id, node_id, project_id, session_id, run_id, timestamp, input_summary, output_summary, outcome, importance, tags, context_json
             FROM memories
             {} ORDER BY {} LIMIT ?{} OFFSET ?{}",
            if where_clause.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", where_clause)
            },
            order_by,
            scope_params.len() + 1,
            scope_params.len() + 2,
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| ReasonanceError::internal(format!("List query failed: {}", e)))?;

        let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        for p in &scope_params {
            all_params.push(Box::new(p.clone()));
        }
        all_params.push(Box::new(limit));
        all_params.push(Box::new(offset));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            all_params.iter().map(|p| p.as_ref()).collect();

        let rows = stmt
            .query_map(param_refs.as_slice(), |row| Ok(row_to_entry(row)))
            .map_err(|e| ReasonanceError::internal(format!("List query failed: {}", e)))?;

        let mut results = Vec::new();
        for row in rows {
            match row {
                Ok(entry) => results.push(entry),
                Err(e) => return Err(ReasonanceError::internal(format!("Row error: {}", e))),
            }
        }
        Ok(results)
    }

    pub fn update_importance(&self, id: &str, delta: f64) -> Result<(), ReasonanceError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| ReasonanceError::internal(format!("Memory DB lock poisoned: {}", e)))?;
        conn.execute(
            "UPDATE memories SET importance = MIN(1.0, MAX(0.0, importance + ?1)) WHERE id = ?2",
            params![delta, id],
        )
        .map_err(|e| ReasonanceError::internal(format!("Failed to update importance: {}", e)))?;
        Ok(())
    }

    pub fn evict(&self, scope: MemoryScope, max_entries: u32) -> Result<u32, ReasonanceError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| ReasonanceError::internal(format!("Memory DB lock poisoned: {}", e)))?;
        let (where_clause, scope_params) = scope_to_where(&scope, "", 1);

        // Count current entries
        let count_sql = format!(
            "SELECT COUNT(*) FROM memories {}",
            if where_clause.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", where_clause)
            }
        );
        let mut count_stmt = conn
            .prepare(&count_sql)
            .map_err(|e| ReasonanceError::internal(format!("Count query failed: {}", e)))?;

        let mut count_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        for p in &scope_params {
            count_params.push(Box::new(p.clone()));
        }
        let count_refs: Vec<&dyn rusqlite::types::ToSql> =
            count_params.iter().map(|p| p.as_ref()).collect();

        let count: u32 = count_stmt
            .query_row(count_refs.as_slice(), |row| row.get(0))
            .map_err(|e| ReasonanceError::internal(format!("Count failed: {}", e)))?;

        if count <= max_entries {
            return Ok(0);
        }

        let to_remove = count - max_entries;

        // Delete lowest-importance entries (break ties by oldest timestamp)
        let delete_sql = format!(
            "DELETE FROM memories WHERE id IN (
                SELECT id FROM memories {} ORDER BY importance ASC, timestamp ASC LIMIT ?{}
            )",
            if where_clause.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", where_clause)
            },
            scope_params.len() + 1,
        );
        let mut delete_stmt = conn
            .prepare(&delete_sql)
            .map_err(|e| ReasonanceError::internal(format!("Delete query failed: {}", e)))?;

        let mut del_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        for p in &scope_params {
            del_params.push(Box::new(p.clone()));
        }
        del_params.push(Box::new(to_remove));
        let del_refs: Vec<&dyn rusqlite::types::ToSql> =
            del_params.iter().map(|p| p.as_ref()).collect();

        let deleted = delete_stmt
            .execute(del_refs.as_slice())
            .map_err(|e| ReasonanceError::internal(format!("Eviction failed: {}", e)))?;

        Ok(deleted as u32)
    }

    pub fn count(&self, scope: MemoryScope) -> Result<u32, ReasonanceError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| ReasonanceError::internal(format!("Memory DB lock poisoned: {}", e)))?;
        let (where_clause, scope_params) = scope_to_where(&scope, "", 1);
        let sql = format!(
            "SELECT COUNT(*) FROM memories {}",
            if where_clause.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", where_clause)
            }
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| ReasonanceError::internal(format!("Count query failed: {}", e)))?;

        let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        for p in &scope_params {
            all_params.push(Box::new(p.clone()));
        }
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            all_params.iter().map(|p| p.as_ref()).collect();

        let count: u32 = stmt
            .query_row(param_refs.as_slice(), |row| row.get(0))
            .map_err(|e| ReasonanceError::internal(format!("Count failed: {}", e)))?;
        Ok(count)
    }

    pub fn migrate_from_json(&self, json_dir: &std::path::Path) -> Result<u32, ReasonanceError> {
        use crate::agent_memory::AgentMemoryStore;

        let mut imported = 0u32;
        if !json_dir.exists() {
            return Ok(0);
        }

        let entries = std::fs::read_dir(json_dir)
            .map_err(|e| ReasonanceError::internal(format!("Cannot read json dir: {}", e)))?;

        for entry in entries {
            let entry =
                entry.map_err(|e| ReasonanceError::internal(format!("Dir entry error: {}", e)))?;
            let path = entry.path();
            if path.extension().map_or(true, |ext| ext != "json") {
                continue;
            }
            let path_str = path.to_str().unwrap_or("");
            let store = match AgentMemoryStore::load(path_str) {
                Ok(s) => s,
                Err(_) => continue,
            };

            for mem in &store.entries {
                let v2 = MemoryEntryV2 {
                    id: Uuid::new_v4().to_string(),
                    node_id: store.node_id.clone(),
                    project_id: None,
                    session_id: None,
                    run_id: mem.run_id.clone(),
                    timestamp: mem.timestamp.clone(),
                    input_summary: mem.input_summary.clone(),
                    output_summary: mem.output_summary.clone(),
                    outcome: mem.outcome.clone(),
                    importance: 0.5,
                    tags: String::new(),
                    context: mem.context.clone(),
                };
                self.add_entry(v2)?;
                imported += 1;
            }
        }

        Ok(imported)
    }
}

/// Build a WHERE clause fragment and parameter values for a MemoryScope.
///
/// `table_prefix` is prepended to column names (e.g. "m" → "m.node_id").
/// `param_offset` is the starting `?N` number (1-based) — callers that already
/// use `?1` for another purpose pass `param_offset = 2`.
fn scope_to_where(
    scope: &MemoryScope,
    table_prefix: &str,
    param_offset: usize,
) -> (String, Vec<String>) {
    let p = if table_prefix.is_empty() {
        String::new()
    } else {
        format!("{}.", table_prefix)
    };
    match scope {
        MemoryScope::Node(node_id) => (
            format!("{}node_id = ?{}", p, param_offset),
            vec![node_id.clone()],
        ),
        MemoryScope::Project(project_id) => (
            format!("{}project_id = ?{}", p, param_offset),
            vec![project_id.clone()],
        ),
        MemoryScope::Global => (String::new(), vec![]),
        MemoryScope::NodeInProject(node_id, project_id) => (
            format!(
                "{}node_id = ?{} AND {}project_id = ?{}",
                p,
                param_offset,
                p,
                param_offset + 1
            ),
            vec![node_id.clone(), project_id.clone()],
        ),
    }
}

fn row_to_entry(row: &rusqlite::Row) -> MemoryEntryV2 {
    let context_json: String = row.get(11).unwrap_or_default();
    let context: serde_json::Value = serde_json::from_str(&context_json)
        .unwrap_or(serde_json::Value::Object(Default::default()));
    MemoryEntryV2 {
        id: row.get(0).unwrap_or_default(),
        node_id: row.get(1).unwrap_or_default(),
        project_id: row.get(2).ok(),
        session_id: row.get(3).ok(),
        run_id: row.get(4).unwrap_or_default(),
        timestamp: row.get(5).unwrap_or_default(),
        input_summary: row.get(6).unwrap_or_default(),
        output_summary: row.get(7).unwrap_or_default(),
        outcome: row.get(8).unwrap_or_default(),
        importance: row.get(9).unwrap_or(0.5),
        tags: row.get(10).unwrap_or_default(),
        context,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(node_id: &str) -> MemoryEntryV2 {
        MemoryEntryV2 {
            id: String::new(),
            node_id: node_id.to_string(),
            project_id: None,
            session_id: None,
            run_id: format!("run-{}", Uuid::new_v4()),
            timestamp: "2026-03-26T10:00:00Z".to_string(),
            input_summary: "analyze the code structure".to_string(),
            output_summary: "found 5 modules with clear separation".to_string(),
            outcome: "success".to_string(),
            importance: 0.5,
            tags: "analysis code".to_string(),
            context: serde_json::json!({"files_scanned": 42}),
        }
    }

    fn sample_entry_with_project(node_id: &str, project_id: &str) -> MemoryEntryV2 {
        let mut e = sample_entry(node_id);
        e.project_id = Some(project_id.to_string());
        e
    }

    #[test]
    fn test_add_and_get() {
        let store = AgentMemoryV2::in_memory().unwrap();
        let entry = sample_entry("node-1");
        let id = store.add_entry(entry).unwrap();
        assert!(!id.is_empty());

        let fetched = store.get_entry(&id).unwrap();
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.node_id, "node-1");
        assert_eq!(fetched.outcome, "success");
    }

    #[test]
    fn test_get_nonexistent() {
        let store = AgentMemoryV2::in_memory().unwrap();
        let result = store.get_entry("does-not-exist").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_search_fts5() {
        let store = AgentMemoryV2::in_memory().unwrap();
        let mut e1 = sample_entry("node-1");
        e1.input_summary = "deploy the kubernetes cluster".to_string();
        e1.output_summary = "cluster deployed successfully".to_string();
        store.add_entry(e1).unwrap();

        let mut e2 = sample_entry("node-2");
        e2.input_summary = "analyze database performance".to_string();
        e2.output_summary = "found slow queries in users table".to_string();
        store.add_entry(e2).unwrap();

        let results = store.search("kubernetes", MemoryScope::Global, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].node_id, "node-1");

        let results = store.search("database", MemoryScope::Global, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].node_id, "node-2");
    }

    #[test]
    fn test_scope_node() {
        let store = AgentMemoryV2::in_memory().unwrap();
        store.add_entry(sample_entry("node-a")).unwrap();
        store.add_entry(sample_entry("node-a")).unwrap();
        store.add_entry(sample_entry("node-b")).unwrap();

        let count = store
            .count(MemoryScope::Node("node-a".to_string()))
            .unwrap();
        assert_eq!(count, 2);

        let count = store
            .count(MemoryScope::Node("node-b".to_string()))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_scope_project() {
        let store = AgentMemoryV2::in_memory().unwrap();
        store
            .add_entry(sample_entry_with_project("n1", "proj-1"))
            .unwrap();
        store
            .add_entry(sample_entry_with_project("n2", "proj-1"))
            .unwrap();
        store
            .add_entry(sample_entry_with_project("n3", "proj-2"))
            .unwrap();

        let count = store
            .count(MemoryScope::Project("proj-1".to_string()))
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_scope_global() {
        let store = AgentMemoryV2::in_memory().unwrap();
        store.add_entry(sample_entry("n1")).unwrap();
        store.add_entry(sample_entry("n2")).unwrap();
        store.add_entry(sample_entry("n3")).unwrap();

        let count = store.count(MemoryScope::Global).unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_importance_update() {
        let store = AgentMemoryV2::in_memory().unwrap();
        let id = store.add_entry(sample_entry("node-1")).unwrap();

        let before = store.get_entry(&id).unwrap().unwrap();
        assert!((before.importance - 0.5).abs() < f64::EPSILON);

        store.update_importance(&id, 0.1).unwrap();
        let after = store.get_entry(&id).unwrap().unwrap();
        assert!((after.importance - 0.6).abs() < 0.001);
    }

    #[test]
    fn test_importance_capped_at_1() {
        let store = AgentMemoryV2::in_memory().unwrap();
        let mut entry = sample_entry("node-1");
        entry.importance = 0.95;
        let id = store.add_entry(entry).unwrap();

        store.update_importance(&id, 0.2).unwrap();
        let after = store.get_entry(&id).unwrap().unwrap();
        assert!((after.importance - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_importance_floored_at_0() {
        let store = AgentMemoryV2::in_memory().unwrap();
        let mut entry = sample_entry("node-1");
        entry.importance = 0.1;
        let id = store.add_entry(entry).unwrap();

        store.update_importance(&id, -0.5).unwrap();
        let after = store.get_entry(&id).unwrap().unwrap();
        assert!((after.importance).abs() < f64::EPSILON);
    }

    #[test]
    fn test_eviction_removes_lowest() {
        let store = AgentMemoryV2::in_memory().unwrap();

        // Add 5 entries with varying importance
        for i in 0..5 {
            let mut e = sample_entry("node-1");
            e.importance = 0.1 * (i as f64 + 1.0); // 0.1, 0.2, 0.3, 0.4, 0.5
            e.timestamp = format!("2026-03-26T10:0{}:00Z", i);
            store.add_entry(e).unwrap();
        }

        assert_eq!(store.count(MemoryScope::Global).unwrap(), 5);

        let removed = store.evict(MemoryScope::Global, 3).unwrap();
        assert_eq!(removed, 2);
        assert_eq!(store.count(MemoryScope::Global).unwrap(), 3);

        // Remaining should be the 3 highest importance (0.3, 0.4, 0.5)
        let remaining = store
            .list(MemoryScope::Global, SortBy::Importance, 10, 0)
            .unwrap();
        assert!(remaining[0].importance >= 0.3 - 0.001);
        assert!(remaining[2].importance >= 0.3 - 0.001);
    }

    #[test]
    fn test_count() {
        let store = AgentMemoryV2::in_memory().unwrap();
        assert_eq!(store.count(MemoryScope::Global).unwrap(), 0);

        store.add_entry(sample_entry("n1")).unwrap();
        assert_eq!(store.count(MemoryScope::Global).unwrap(), 1);

        store.add_entry(sample_entry("n2")).unwrap();
        assert_eq!(store.count(MemoryScope::Global).unwrap(), 2);
    }

    #[test]
    fn test_list_sorted_by_recency() {
        let store = AgentMemoryV2::in_memory().unwrap();

        let mut e1 = sample_entry("n1");
        e1.timestamp = "2026-03-26T08:00:00Z".to_string();
        store.add_entry(e1).unwrap();

        let mut e2 = sample_entry("n1");
        e2.timestamp = "2026-03-26T12:00:00Z".to_string();
        store.add_entry(e2).unwrap();

        let mut e3 = sample_entry("n1");
        e3.timestamp = "2026-03-26T10:00:00Z".to_string();
        store.add_entry(e3).unwrap();

        let results = store
            .list(MemoryScope::Global, SortBy::Recency, 10, 0)
            .unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].timestamp, "2026-03-26T12:00:00Z");
        assert_eq!(results[1].timestamp, "2026-03-26T10:00:00Z");
        assert_eq!(results[2].timestamp, "2026-03-26T08:00:00Z");
    }

    #[test]
    fn test_list_sorted_by_importance() {
        let store = AgentMemoryV2::in_memory().unwrap();

        let mut e1 = sample_entry("n1");
        e1.importance = 0.3;
        store.add_entry(e1).unwrap();

        let mut e2 = sample_entry("n1");
        e2.importance = 0.9;
        store.add_entry(e2).unwrap();

        let mut e3 = sample_entry("n1");
        e3.importance = 0.6;
        store.add_entry(e3).unwrap();

        let results = store
            .list(MemoryScope::Global, SortBy::Importance, 10, 0)
            .unwrap();
        assert_eq!(results.len(), 3);
        assert!((results[0].importance - 0.9).abs() < 0.001);
        assert!((results[1].importance - 0.6).abs() < 0.001);
        assert!((results[2].importance - 0.3).abs() < 0.001);
    }

    #[test]
    fn test_search_with_scope() {
        let store = AgentMemoryV2::in_memory().unwrap();

        let mut e1 = sample_entry_with_project("n1", "proj-a");
        e1.input_summary = "deploy microservices".to_string();
        store.add_entry(e1).unwrap();

        let mut e2 = sample_entry_with_project("n2", "proj-b");
        e2.input_summary = "deploy containers".to_string();
        store.add_entry(e2).unwrap();

        // Global search finds both
        let results = store.search("deploy", MemoryScope::Global, 10).unwrap();
        assert_eq!(results.len(), 2);

        // Scoped to proj-a finds only one
        let results = store
            .search("deploy", MemoryScope::Project("proj-a".to_string()), 10)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].node_id, "n1");
    }

    #[test]
    fn test_context_roundtrip() {
        let store = AgentMemoryV2::in_memory().unwrap();
        let mut entry = sample_entry("n1");
        entry.context = serde_json::json!({"key": "value", "count": 42, "nested": {"a": true}});
        let id = store.add_entry(entry).unwrap();

        let fetched = store.get_entry(&id).unwrap().unwrap();
        assert_eq!(fetched.context["key"], "value");
        assert_eq!(fetched.context["count"], 42);
        assert_eq!(fetched.context["nested"]["a"], true);
    }

    #[test]
    fn test_eviction_no_op_when_under_limit() {
        let store = AgentMemoryV2::in_memory().unwrap();
        store.add_entry(sample_entry("n1")).unwrap();
        store.add_entry(sample_entry("n2")).unwrap();

        let removed = store.evict(MemoryScope::Global, 10).unwrap();
        assert_eq!(removed, 0);
        assert_eq!(store.count(MemoryScope::Global).unwrap(), 2);
    }
}
