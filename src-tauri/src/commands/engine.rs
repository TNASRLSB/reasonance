use crate::agent_runtime::AgentRuntime;
use crate::pty_manager::PtyManager;
use crate::resource_lock::ResourceLockManager;
use crate::workflow_engine::{WorkflowEngine, WorkflowRun};
use crate::workflow_store::WorkflowStore;
use log::{info, debug};
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub fn play_workflow(
    workflow_path: String,
    cwd: String,
    app: AppHandle,
    engine: State<'_, WorkflowEngine>,
    store: State<'_, WorkflowStore>,
    runtime: State<'_, AgentRuntime>,
    pty_manager: State<'_, PtyManager>,
    lock_manager: State<'_, ResourceLockManager>,
) -> Result<String, String> {
    info!("cmd::play_workflow(workflow_path={}, cwd={})", workflow_path, cwd);
    let workflow = store
        .load(&workflow_path)
        .or_else(|_| store.get(&workflow_path).ok_or("Workflow not loaded".to_string()))?;
    let run_id = engine.create_run(&workflow, &workflow_path)?;
    engine.advance_run(&run_id, &workflow, &runtime, &pty_manager, &app, &cwd, &lock_manager)?;
    let _ = app.emit("hive://run-status-changed", serde_json::json!({
        "run_id": run_id, "old_status": "idle", "new_status": "running",
    }));
    debug!("cmd::play_workflow started run_id={}", run_id);
    Ok(run_id)
}

#[tauri::command]
pub fn pause_workflow(
    run_id: String,
    engine: State<'_, WorkflowEngine>,
) -> Result<(), String> {
    info!("cmd::pause_workflow(run_id={})", run_id);
    engine.pause_run(&run_id)
}

#[tauri::command]
pub fn resume_workflow(
    run_id: String,
    workflow_path: String,
    cwd: String,
    app: AppHandle,
    engine: State<'_, WorkflowEngine>,
    store: State<'_, WorkflowStore>,
    runtime: State<'_, AgentRuntime>,
    pty_manager: State<'_, PtyManager>,
    lock_manager: State<'_, ResourceLockManager>,
) -> Result<(), String> {
    info!("cmd::resume_workflow(run_id={})", run_id);
    engine.resume_run(&run_id)?;
    let workflow = store.get(&workflow_path).ok_or("Workflow not loaded")?;
    engine.advance_run(&run_id, &workflow, &runtime, &pty_manager, &app, &cwd, &lock_manager)?;
    Ok(())
}

#[tauri::command]
pub fn stop_workflow(
    run_id: String,
    app: AppHandle,
    engine: State<'_, WorkflowEngine>,
    runtime: State<'_, AgentRuntime>,
    pty_manager: State<'_, PtyManager>,
    lock_manager: State<'_, ResourceLockManager>,
) -> Result<(), String> {
    info!("cmd::stop_workflow(run_id={})", run_id);
    if let Some(run) = engine.get_run(&run_id) {
        for ns in run.node_states.values() {
            // Release all resource locks held by this node
            lock_manager.release_all(&ns.node_id);
            if let Some(ref agent_id) = ns.agent_id {
                if let Some(agent) = runtime.get_agent(agent_id) {
                    if let Some(ref pty_id) = agent.pty_id {
                        let _ = pty_manager.kill(pty_id);
                    }
                }
            }
        }
    }
    engine.stop_run(&run_id)?;
    let _ = app.emit("hive://run-status-changed", serde_json::json!({ "run_id": run_id, "old_status": "running", "new_status": "stopped" }));
    Ok(())
}

#[tauri::command]
pub fn step_workflow(
    run_id: String,
    workflow_path: String,
    cwd: String,
    app: AppHandle,
    engine: State<'_, WorkflowEngine>,
    store: State<'_, WorkflowStore>,
    runtime: State<'_, AgentRuntime>,
    pty_manager: State<'_, PtyManager>,
    lock_manager: State<'_, ResourceLockManager>,
) -> Result<Option<String>, String> {
    info!("cmd::step_workflow(run_id={})", run_id);
    let workflow = store.get(&workflow_path).ok_or("Workflow not loaded")?;
    engine.step_run(&run_id, &workflow, &runtime, &pty_manager, &app, &cwd, &lock_manager)
}

#[tauri::command]
pub fn get_run_status(
    run_id: String,
    engine: State<'_, WorkflowEngine>,
) -> Option<WorkflowRun> {
    debug!("cmd::get_run_status(run_id={})", run_id);
    engine.get_run(&run_id)
}

#[tauri::command]
pub fn approve_node(
    run_id: String,
    node_id: String,
    workflow_path: String,
    cwd: String,
    app: AppHandle,
    engine: State<'_, WorkflowEngine>,
    store: State<'_, WorkflowStore>,
    runtime: State<'_, AgentRuntime>,
    pty_manager: State<'_, PtyManager>,
    lock_manager: State<'_, ResourceLockManager>,
) -> Result<(), String> {
    info!("cmd::approve_node(run_id={}, node_id={})", run_id, node_id);
    let workflow = store.get(&workflow_path).ok_or("Workflow not loaded")?;
    engine.spawn_single_node(&run_id, &node_id, &workflow, &runtime, &pty_manager, &lock_manager, &app, &cwd)
}

#[tauri::command]
pub fn notify_node_completed(
    run_id: String,
    node_id: String,
    success: bool,
    workflow_path: String,
    cwd: String,
    app: AppHandle,
    engine: State<'_, WorkflowEngine>,
    store: State<'_, WorkflowStore>,
    runtime: State<'_, AgentRuntime>,
    pty_manager: State<'_, PtyManager>,
    lock_manager: State<'_, ResourceLockManager>,
) -> Result<(), String> {
    info!("cmd::notify_node_completed(run_id={}, node_id={}, success={})", run_id, node_id, success);
    let workflow = store.get(&workflow_path).ok_or("Workflow not loaded")?;
    engine.on_node_completed(&run_id, &node_id, success, &workflow, &runtime, &pty_manager, &app, &cwd, &lock_manager)
}
