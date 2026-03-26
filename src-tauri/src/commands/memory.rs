use crate::agent_memory_v2::{AgentMemoryV2, MemoryEntryV2, MemoryScope, SortBy};
use crate::error::ReasonanceError;
use tauri::State;

#[tauri::command]
pub fn memory_add_entry(
    store: State<'_, AgentMemoryV2>,
    entry: MemoryEntryV2,
) -> Result<String, ReasonanceError> {
    store.add_entry(entry)
}

#[tauri::command]
pub fn memory_search(
    store: State<'_, AgentMemoryV2>,
    query: String,
    scope: MemoryScope,
    limit: u32,
) -> Result<Vec<MemoryEntryV2>, ReasonanceError> {
    store.search(&query, scope, limit)
}

#[tauri::command]
pub fn memory_list(
    store: State<'_, AgentMemoryV2>,
    scope: MemoryScope,
    sort: SortBy,
    limit: u32,
    offset: u32,
) -> Result<Vec<MemoryEntryV2>, ReasonanceError> {
    store.list(scope, sort, limit, offset)
}

#[tauri::command]
pub fn memory_get(
    store: State<'_, AgentMemoryV2>,
    id: String,
) -> Result<Option<MemoryEntryV2>, ReasonanceError> {
    store.get_entry(&id)
}
