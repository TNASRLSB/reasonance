use crate::agent_memory_v2::{AgentMemoryV2, MemoryEntryV2, MemoryScope, SortBy};
use crate::agent_runtime::{AgentRuntime, AgentState};
use crate::error::ReasonanceError;
use crate::pty_manager::PtyManager;
use crate::resource_lock::ResourceLockManager;
use crate::workflow_store::{
    AgentNodeConfig, NodeType, ResourceNodeConfig, Workflow, WorkflowEdge,
};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Listener, Manager};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunStatus {
    Idle,
    Running,
    Paused,
    Completed,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRunState {
    pub node_id: String,
    pub agent_id: Option<String>,
    pub state: AgentState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: String,
    pub workflow_path: String,
    pub status: RunStatus,
    pub node_states: HashMap<String, NodeRunState>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

pub struct WorkflowEngine {
    /// Active workflow runs. Plain HashMap: bounded by concurrent workflow count
    /// (typically 1-3), entries removed on completion.
    pub runs: Arc<Mutex<HashMap<String, WorkflowRun>>>,
    /// The sole event bus. `None` in tests and when the bus hasn't been wired yet.
    /// Uses `Mutex` for interior mutability so `set_event_bus` can be
    /// called through `&self` (Tauri `State` only provides shared refs).
    event_bus: Mutex<Option<Arc<crate::event_bus::EventBus>>>,
}

impl WorkflowEngine {
    pub fn new() -> Self {
        Self {
            runs: Arc::new(Mutex::new(HashMap::new())),
            event_bus: Mutex::new(None),
        }
    }

    /// Set the EventBus. Called from `setup()` after the bus is constructed.
    pub fn set_event_bus(&self, bus: Arc<crate::event_bus::EventBus>) {
        *self.event_bus.lock().unwrap_or_else(|e| e.into_inner()) = Some(bus);
    }

    /// Publish an event through the EventBus channel.
    fn emit_to_bus(&self, channel: &str, payload: serde_json::Value) {
        let bus = self
            .event_bus
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if let Some(bus) = bus {
            bus.publish(crate::event_bus::Event::new(
                channel,
                payload,
                "workflow-engine",
            ));
        }
    }

    /// Topological sort using Kahn's algorithm. Returns Err if cycle detected.
    pub fn topological_sort(workflow: &Workflow) -> Result<Vec<String>, ReasonanceError> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
        for node in &workflow.nodes {
            in_degree.entry(node.id.clone()).or_insert(0);
            adjacency.entry(node.id.clone()).or_insert_with(Vec::new);
        }
        for edge in &workflow.edges {
            adjacency
                .entry(edge.from.clone())
                .or_insert_with(Vec::new)
                .push(edge.to.clone());
            *in_degree.entry(edge.to.clone()).or_insert(0) += 1;
        }
        let mut sorted_vec: Vec<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();
        sorted_vec.sort();
        let mut queue: VecDeque<String> = sorted_vec.into_iter().collect();
        let mut result = Vec::new();
        while let Some(node_id) = queue.pop_front() {
            result.push(node_id.clone());
            if let Some(neighbors) = adjacency.get(&node_id) {
                for neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(neighbor.clone());
                        }
                    }
                }
            }
        }
        if result.len() != workflow.nodes.len() {
            warn!(
                "Workflow graph cycle detected: sorted {} of {} nodes",
                result.len(),
                workflow.nodes.len()
            );
            return Err(ReasonanceError::workflow(
                "",
                "",
                "Workflow graph contains a cycle",
            ));
        }
        debug!("Topological sort completed: {} nodes", result.len());
        Ok(result)
    }

    pub fn get_predecessors(node_id: &str, edges: &[WorkflowEdge]) -> Vec<String> {
        edges
            .iter()
            .filter(|e| e.to == node_id)
            .map(|e| e.from.clone())
            .collect()
    }

    pub fn get_successors(node_id: &str, edges: &[WorkflowEdge]) -> Vec<String> {
        edges
            .iter()
            .filter(|e| e.from == node_id)
            .map(|e| e.to.clone())
            .collect()
    }

    pub fn all_predecessors_complete(
        node_id: &str,
        edges: &[WorkflowEdge],
        node_states: &HashMap<String, NodeRunState>,
    ) -> bool {
        let preds = Self::get_predecessors(node_id, edges);
        if preds.is_empty() {
            return true;
        }
        preds.iter().all(|pred_id| {
            node_states
                .get(pred_id)
                .map(|s| matches!(s.state, AgentState::Success | AgentState::Skipped))
                .unwrap_or(false)
        })
    }

    pub fn get_ready_nodes(
        edges: &[WorkflowEdge],
        node_states: &HashMap<String, NodeRunState>,
    ) -> Vec<String> {
        node_states
            .iter()
            .filter(|(_, ns)| ns.state == AgentState::Idle)
            .filter(|(node_id, _)| Self::all_predecessors_complete(node_id, edges, node_states))
            .map(|(node_id, _)| node_id.clone())
            .collect()
    }

    // --- Run lifecycle ---

    pub fn create_run(
        &self,
        workflow: &Workflow,
        workflow_path: &str,
    ) -> Result<String, ReasonanceError> {
        Self::topological_sort(workflow)?;
        let run_id = uuid::Uuid::new_v4().to_string();
        info!(
            "Workflow run created: run_id={}, workflow={}, nodes={}",
            run_id,
            workflow_path,
            workflow.nodes.len()
        );
        let mut node_states = HashMap::new();
        for node in &workflow.nodes {
            let state = if node.node_type == NodeType::Resource {
                AgentState::Success // resources are immediately available
            } else {
                AgentState::Idle
            };
            node_states.insert(
                node.id.clone(),
                NodeRunState {
                    node_id: node.id.clone(),
                    agent_id: None,
                    state,
                },
            );
        }
        let run = WorkflowRun {
            id: run_id.clone(),
            workflow_path: workflow_path.to_string(),
            status: RunStatus::Running,
            node_states,
            started_at: Some(chrono::Utc::now().to_rfc3339()),
            finished_at: None,
        };
        self.runs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(run_id.clone(), run);
        Ok(run_id)
    }

    pub fn stop_run(&self, run_id: &str) -> Result<(), ReasonanceError> {
        info!("Workflow run stopped: run_id={}", run_id);
        let mut runs = self.runs.lock().unwrap_or_else(|e| e.into_inner());
        let run = runs
            .get_mut(run_id)
            .ok_or_else(|| ReasonanceError::not_found("run", run_id))?;
        run.status = RunStatus::Stopped;
        run.finished_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(())
    }

    pub fn pause_run(&self, run_id: &str) -> Result<(), ReasonanceError> {
        info!("Workflow run paused: run_id={}", run_id);
        let mut runs = self.runs.lock().unwrap_or_else(|e| e.into_inner());
        let run = runs
            .get_mut(run_id)
            .ok_or_else(|| ReasonanceError::not_found("run", run_id))?;
        if run.status != RunStatus::Running {
            warn!("Cannot pause run {} in {:?} state", run_id, run.status);
            return Err(ReasonanceError::workflow(
                run_id,
                "",
                format!("Cannot pause run in {:?} state", run.status),
            ));
        }
        run.status = RunStatus::Paused;
        Ok(())
    }

    pub fn resume_run(&self, run_id: &str) -> Result<(), ReasonanceError> {
        info!("Workflow run resumed: run_id={}", run_id);
        let mut runs = self.runs.lock().unwrap_or_else(|e| e.into_inner());
        let run = runs
            .get_mut(run_id)
            .ok_or_else(|| ReasonanceError::not_found("run", run_id))?;
        if run.status != RunStatus::Paused {
            warn!("Cannot resume run {} in {:?} state", run_id, run.status);
            return Err(ReasonanceError::workflow(
                run_id,
                "",
                format!("Cannot resume run in {:?} state", run.status),
            ));
        }
        run.status = RunStatus::Running;
        Ok(())
    }

    pub fn get_run(&self, run_id: &str) -> Option<WorkflowRun> {
        self.runs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(run_id)
            .cloned()
    }

    pub fn update_node_state(
        &self,
        run_id: &str,
        node_id: &str,
        new_state: AgentState,
        agent_id: Option<String>,
    ) -> Result<(), ReasonanceError> {
        let mut runs = self.runs.lock().unwrap_or_else(|e| e.into_inner());
        let run = runs
            .get_mut(run_id)
            .ok_or_else(|| ReasonanceError::not_found("run", run_id))?;
        let node_state = run
            .node_states
            .get_mut(node_id)
            .ok_or_else(|| ReasonanceError::not_found("node", node_id))?;
        node_state.state = new_state;
        if agent_id.is_some() {
            node_state.agent_id = agent_id;
        }
        Ok(())
    }

    pub fn check_run_complete(&self, run_id: &str) -> Result<bool, ReasonanceError> {
        let runs = self.runs.lock().unwrap_or_else(|e| e.into_inner());
        let run = runs
            .get(run_id)
            .ok_or_else(|| ReasonanceError::not_found("run", run_id))?;
        Ok(run.node_states.values().all(|ns| {
            matches!(
                ns.state,
                AgentState::Success | AgentState::Error | AgentState::Skipped
            )
        }))
    }

    pub fn finalize_run(&self, run_id: &str) -> Result<RunStatus, ReasonanceError> {
        let mut runs = self.runs.lock().unwrap_or_else(|e| e.into_inner());
        let run = runs
            .get_mut(run_id)
            .ok_or_else(|| ReasonanceError::not_found("run", run_id))?;
        let has_errors = run
            .node_states
            .values()
            .any(|ns| ns.state == AgentState::Error);
        let final_status = if has_errors {
            RunStatus::Failed
        } else {
            RunStatus::Completed
        };
        info!(
            "Workflow run finalized: run_id={}, status={:?}",
            run_id, final_status
        );
        run.status = final_status.clone();
        run.finished_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(final_status)
    }

    // --- Agent PTY spawn helper ---

    /// Spawns a PTY for a single Agent node. Used by both `advance_run` (trusted mode)
    /// and `approve_node` (supervised mode after user approval).
    pub fn spawn_single_node(
        &self,
        run_id: &str,
        node_id: &str,
        workflow: &Workflow,
        runtime: &AgentRuntime,
        pty_manager: &PtyManager,
        lock_manager: &ResourceLockManager,
        app: &AppHandle,
        cwd: &str,
    ) -> Result<(), ReasonanceError> {
        let node = workflow
            .nodes
            .iter()
            .find(|n| n.id == node_id)
            .ok_or_else(|| ReasonanceError::not_found("node", node_id))?;

        // Acquire locks on connected Resource nodes before spawning
        let mut lock_failed = false;
        let mut acquired_resources: Vec<String> = Vec::new();
        for edge in &workflow.edges {
            let resource_node_id = if edge.to == node_id {
                &edge.from
            } else if edge.from == node_id {
                &edge.to
            } else {
                continue;
            };
            if let Some(res_node) = workflow
                .nodes
                .iter()
                .find(|n| n.id == *resource_node_id && n.node_type == NodeType::Resource)
            {
                let write = if let Ok(cfg) =
                    serde_json::from_value::<ResourceNodeConfig>(res_node.config.clone())
                {
                    cfg.access == "write" || cfg.access == "read_write"
                } else {
                    false
                };
                if let Err(e) = lock_manager.acquire(resource_node_id, node_id, write) {
                    debug!(
                        "Lock acquisition failed for node {} on resource {}: {}",
                        node_id, resource_node_id, e
                    );
                    lock_failed = true;
                    break;
                }
                acquired_resources.push(resource_node_id.clone());
            }
        }
        if lock_failed {
            for rid in &acquired_resources {
                lock_manager.release(rid, node_id);
            }
            return Err(ReasonanceError::workflow(
                run_id,
                node_id,
                "Resource lock acquisition failed",
            ));
        }

        let retry = node
            .config
            .get("retry")
            .and_then(|v| v.as_u64())
            .unwrap_or(workflow.settings.default_retry as u64) as u32;
        let fallback = node
            .config
            .get("fallback")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let llm = node
            .config
            .get("llm")
            .and_then(|v| v.as_str())
            .unwrap_or("claude");

        // Load agent memory (v2) if enabled
        let mut memory_context = String::new();
        if let Ok(agent_cfg) = serde_json::from_value::<AgentNodeConfig>(node.config.clone()) {
            if let Some(ref mem_cfg) = agent_cfg.memory {
                if mem_cfg.enabled {
                    if let Some(memory_store) = app.try_state::<AgentMemoryV2>() {
                        let scope = match mem_cfg.persist.as_str() {
                            "workflow" => {
                                let runs = self.runs.lock().unwrap_or_else(|e| e.into_inner());
                                let wf_path = runs
                                    .get(run_id)
                                    .map(|r| r.workflow_path.clone())
                                    .unwrap_or_default();
                                MemoryScope::NodeInProject(node_id.to_string(), wf_path)
                            }
                            "global" => MemoryScope::Node(node_id.to_string()),
                            _ => MemoryScope::Node(node_id.to_string()),
                        };
                        let limit = mem_cfg.max_entries.min(50);
                        match memory_store.list(scope, SortBy::Recency, limit, 0) {
                            Ok(entries) if !entries.is_empty() => {
                                let summary = entries
                                    .iter()
                                    .map(|e| {
                                        format!(
                                            "[{}] {}: {}",
                                            e.timestamp, e.outcome, e.output_summary
                                        )
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                memory_context = format!("\n<memory>\n{}\n</memory>\n", summary);
                                info!(
                                    "Memory v2 loaded for node {}: {} entries injected",
                                    node_id,
                                    entries.len()
                                );
                            }
                            Ok(_) => {
                                debug!("No memory v2 entries for node {}", node_id);
                            }
                            Err(e) => {
                                warn!("Failed to load memory v2 for node {}: {}", node_id, e);
                            }
                        }
                    }
                }
            }
        }

        // Build prompt from node config + routed messages + memory
        let prompt = node
            .config
            .get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let input_msgs = runtime.get_messages_for(node_id);
        let routed_input = if input_msgs.is_empty() {
            String::new()
        } else {
            let parts: Vec<&str> = input_msgs
                .iter()
                .filter_map(|m| m.payload.get("output").and_then(|v| v.as_str()))
                .collect();
            format!("\n<input>\n{}\n</input>\n", parts.join("\n---\n"))
        };

        let full_prompt = format!("{}{}{}\n", memory_context, routed_input, prompt);

        let agent_id =
            runtime.create_agent(node_id, &format!("{}:{}", run_id, node_id), retry, fallback);
        runtime.transition(&agent_id, AgentState::Queued)?;
        runtime.transition(&agent_id, AgentState::Running)?;
        let pty_id = pty_manager.spawn(llm, &[], cwd, app.clone())?;
        runtime.set_pty(&agent_id, &pty_id)?;

        // Inject prompt + memory + routed input into PTY
        if !full_prompt.trim().is_empty() {
            if let Err(e) = pty_manager.write(&pty_id, &full_prompt) {
                warn!("Failed to write prompt to PTY {}: {}", pty_id, e);
            } else {
                debug!(
                    "Injected prompt into PTY {} for node {} ({} bytes)",
                    pty_id,
                    node_id,
                    full_prompt.len()
                );
            }
        }
        self.update_node_state(run_id, node_id, AgentState::Running, Some(agent_id.clone()))?;
        self.emit_to_bus(
            "workflow:node-state",
            serde_json::json!({
                "run_id": run_id,
                "node_id": node_id,
                "old_state": "idle",
                "new_state": "running",
            }),
        );
        self.emit_to_bus(
            "workflow:agent-output",
            serde_json::json!({
                "run_id": run_id,
                "node_id": node_id,
                "pty_id": pty_id,
            }),
        );

        // Subscribe to PTY output for backend buffering (used by message routing + memory)
        let runtime_agents = runtime.agents.clone();
        let buf_agent_id = agent_id.clone();
        app.listen(format!("pty-data-{}", pty_id), move |event| {
            if let Ok(data) = serde_json::from_str::<String>(event.payload()) {
                let mut agents = runtime_agents.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(agent) = agents.get_mut(&buf_agent_id) {
                    for line in data.lines() {
                        agent.output_buffer.push(line.to_string());
                        if agent.output_buffer.len() > 200 {
                            agent.output_buffer.remove(0);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    // --- Scheduler ---

    pub fn advance_run(
        &self,
        run_id: &str,
        workflow: &Workflow,
        runtime: &AgentRuntime,
        pty_manager: &PtyManager,
        app: &AppHandle,
        cwd: &str,
        lock_manager: &ResourceLockManager,
    ) -> Result<Vec<String>, ReasonanceError> {
        {
            let runs = self.runs.lock().unwrap_or_else(|e| e.into_inner());
            let run = runs
                .get(run_id)
                .ok_or_else(|| ReasonanceError::not_found("run", run_id))?;
            if run.status != RunStatus::Running {
                return Ok(vec![]);
            }
        }
        let node_states = {
            let runs = self.runs.lock().unwrap_or_else(|e| e.into_inner());
            runs.get(run_id)
                .ok_or_else(|| ReasonanceError::not_found("run", run_id))?
                .node_states
                .clone()
        };
        let ready = Self::get_ready_nodes(&workflow.edges, &node_states);
        let running_count = node_states
            .values()
            .filter(|ns| ns.state == AgentState::Running || ns.state == AgentState::Queued)
            .count() as u32;
        let max_concurrent = workflow.settings.max_concurrent_agents;
        let mut started = Vec::new();
        let permission_level = &workflow.settings.permission_level;

        debug!(
            "Advancing run {}: {} ready nodes, {} currently running",
            run_id,
            ready.len(),
            running_count
        );

        for node_id in ready {
            if running_count + started.len() as u32 >= max_concurrent {
                debug!(
                    "Concurrency limit reached ({}) for run {}",
                    max_concurrent, run_id
                );
                break;
            }
            let node = workflow
                .nodes
                .iter()
                .find(|n| n.id == node_id)
                .ok_or_else(|| ReasonanceError::not_found("node", &node_id))?;

            match node.node_type {
                NodeType::Agent => {
                    match permission_level.as_str() {
                        "dry-run" => {
                            // Simulate execution — mark as Success without spawning
                            self.update_node_state(run_id, &node_id, AgentState::Success, None)?;
                            self.emit_to_bus(
                                "workflow:node-state",
                                serde_json::json!({
                                    "run_id": run_id,
                                    "node_id": node_id,
                                    "old_state": "idle",
                                    "new_state": "success",
                                }),
                            );
                            log::info!("[dry-run] Node {} simulated as success", node_id);
                            // Don't push to started since it completes immediately
                        }
                        "supervised" => {
                            // Emit permission request — frontend shows approval dialog
                            self.emit_to_bus(
                                "workflow:permission-request",
                                serde_json::json!({
                                    "run_id": run_id,
                                    "node_id": node_id,
                                    "agent_label": node.label,
                                }),
                            );
                            // Don't spawn PTY yet — wait for approval via approve_node command
                            // Mark as Queued so it's visually distinct
                            self.update_node_state(run_id, &node_id, AgentState::Queued, None)?;
                            self.emit_to_bus(
                                "workflow:node-state",
                                serde_json::json!({
                                    "run_id": run_id,
                                    "node_id": node_id,
                                    "old_state": "idle",
                                    "new_state": "queued",
                                }),
                            );
                            log::info!("[supervised] Node {} awaiting approval", node_id);
                        }
                        _ => {
                            // "trusted" — spawn directly (existing behavior)
                            self.spawn_single_node(
                                run_id,
                                &node_id,
                                workflow,
                                runtime,
                                pty_manager,
                                lock_manager,
                                app,
                                cwd,
                            )?;
                            started.push(node_id);
                        }
                    }
                }
                NodeType::Logic => {
                    let config: crate::workflow_store::LogicNodeConfig =
                        serde_json::from_value(node.config.clone()).map_err(|e| {
                            ReasonanceError::workflow(
                                run_id,
                                &node_id,
                                format!("Invalid logic config: {}", e),
                            )
                        })?;

                    // Predecessor output is empty for now — full output capture is a future task
                    let predecessor_output = serde_json::json!({});

                    let evaluator = crate::logic_eval::LogicEvaluator::new();
                    match evaluator.evaluate(&config.rule, &predecessor_output) {
                        Ok(result) => {
                            self.update_node_state(run_id, &node_id, AgentState::Success, None)?;

                            // Route to onTrue or onFalse edge — disable the other branch
                            let inactive_edge_id = if result {
                                &config.on_false
                            } else {
                                &config.on_true
                            };

                            // Mark nodes on the inactive branch as skipped
                            if let Some(ref inactive_id) = inactive_edge_id {
                                let inactive_successors: Vec<String> = workflow
                                    .edges
                                    .iter()
                                    .filter(|e| e.id == *inactive_id)
                                    .map(|e| e.to.clone())
                                    .collect();
                                for succ_id in inactive_successors {
                                    let other_inputs = workflow
                                        .edges
                                        .iter()
                                        .filter(|e| e.to == succ_id && e.id != *inactive_id)
                                        .count();
                                    if other_inputs == 0 {
                                        self.update_node_state(
                                            run_id,
                                            &succ_id,
                                            AgentState::Skipped,
                                            None,
                                        )?;
                                        self.emit_to_bus(
                                            "workflow:node-state",
                                            serde_json::json!({
                                                "run_id": run_id,
                                                "node_id": succ_id,
                                                "old_state": "idle",
                                                "new_state": "skipped",
                                            }),
                                        );
                                    }
                                }
                            }

                            log::info!("Logic node {} evaluated to {}", node_id, result);
                            self.emit_to_bus(
                                "workflow:node-state",
                                serde_json::json!({
                                    "run_id": run_id,
                                    "node_id": node_id,
                                    "old_state": "idle",
                                    "new_state": "success",
                                }),
                            );
                        }
                        Err(e) => {
                            self.update_node_state(run_id, &node_id, AgentState::Error, None)?;
                            log::error!("Logic node {} rule failed: {}", node_id, e);
                            self.emit_to_bus(
                                "workflow:node-state",
                                serde_json::json!({
                                    "run_id": run_id,
                                    "node_id": node_id,
                                    "old_state": "idle",
                                    "new_state": "error",
                                }),
                            );
                        }
                    }
                    // Note: started.push is omitted for Logic nodes — they complete synchronously
                    // and don't count toward the concurrency limit
                }
                NodeType::Resource => {} // already Success from create_run
            }
        }

        if self.check_run_complete(run_id)? {
            let final_status = self.finalize_run(run_id)?;
            self.emit_to_bus(
                "workflow:run-status",
                serde_json::json!({
                    "run_id": run_id,
                    "status": final_status,
                }),
            );
        }
        Ok(started)
    }

    pub fn on_node_completed(
        &self,
        run_id: &str,
        node_id: &str,
        success: bool,
        workflow: &Workflow,
        runtime: &AgentRuntime,
        pty_manager: &PtyManager,
        app: &AppHandle,
        cwd: &str,
        lock_manager: &ResourceLockManager,
    ) -> Result<(), ReasonanceError> {
        let agent_id = {
            let runs = self.runs.lock().unwrap_or_else(|e| e.into_inner());
            let run = runs
                .get(run_id)
                .ok_or_else(|| ReasonanceError::not_found("run", run_id))?;
            run.node_states
                .get(node_id)
                .and_then(|ns| ns.agent_id.clone())
        };

        // Release all locks held by this node
        lock_manager.release_all(node_id);

        if success {
            info!(
                "Workflow node completed successfully: run_id={}, node_id={}",
                run_id, node_id
            );
            if let Some(ref aid) = agent_id {
                let _ = runtime.transition(aid, AgentState::Success);
            }
            self.update_node_state(run_id, node_id, AgentState::Success, None)?;
            self.emit_to_bus(
                "workflow:node-state",
                serde_json::json!({
                    "run_id": run_id,
                    "node_id": node_id,
                    "old_state": "running",
                    "new_state": "success",
                }),
            );

            // Route output to successor nodes via agent messaging
            let output = agent_id
                .as_ref()
                .and_then(|aid| runtime.get_output(aid).ok())
                .unwrap_or_default();
            if !output.is_empty() {
                let successors = Self::get_successors(node_id, &workflow.edges);
                let payload = serde_json::json!({
                    "from_node": node_id,
                    "output": output.join("\n"),
                });
                for succ_id in &successors {
                    runtime.send_message(node_id, succ_id, payload.clone());
                    debug!(
                        "Routed output from {} to {} ({} lines)",
                        node_id,
                        succ_id,
                        output.len()
                    );
                }
            }
        } else {
            warn!(
                "Workflow node failed: run_id={}, node_id={}",
                run_id, node_id
            );
            if let Some(ref aid) = agent_id {
                let _ = runtime.transition(aid, AgentState::Failed);
            }
            self.update_node_state(run_id, node_id, AgentState::Failed, None)?;

            let handled =
                self.handle_failure(run_id, node_id, workflow, runtime, pty_manager, app, cwd)?;
            if !handled {
                if let Some(ref aid) = agent_id {
                    let _ = runtime.transition(aid, AgentState::Error);
                }
                self.update_node_state(run_id, node_id, AgentState::Error, None)?;
                self.emit_to_bus(
                    "workflow:node-state",
                    serde_json::json!({
                        "run_id": run_id,
                        "node_id": node_id,
                        "old_state": "failed",
                        "new_state": "error",
                    }),
                );
            }
        }
        // Save memory entry (v2) if enabled for this node
        if let Some(wf_node) = workflow.nodes.iter().find(|n| n.id == node_id) {
            if let Ok(agent_cfg) = serde_json::from_value::<AgentNodeConfig>(wf_node.config.clone())
            {
                if let Some(ref mem_cfg) = agent_cfg.memory {
                    if mem_cfg.enabled {
                        if let Some(memory_store) = app.try_state::<AgentMemoryV2>() {
                            let (scope, project_id) = match mem_cfg.persist.as_str() {
                                "workflow" => {
                                    let runs = self.runs.lock().unwrap_or_else(|e| e.into_inner());
                                    let wf_path = runs
                                        .get(run_id)
                                        .map(|r| r.workflow_path.clone())
                                        .unwrap_or_default();
                                    (
                                        MemoryScope::NodeInProject(
                                            node_id.to_string(),
                                            wf_path.clone(),
                                        ),
                                        Some(wf_path),
                                    )
                                }
                                "global" => (MemoryScope::Node(node_id.to_string()), None),
                                _ => (MemoryScope::Node(node_id.to_string()), None),
                            };

                            // Build input summary from messages routed to this node
                            let input_msgs = runtime.get_messages_for(node_id);
                            let input_summary = if input_msgs.is_empty() {
                                String::new()
                            } else {
                                input_msgs
                                    .iter()
                                    .filter_map(|m| {
                                        m.payload.get("output").and_then(|v| v.as_str())
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n---\n")
                                    .chars()
                                    .take(2000)
                                    .collect()
                            };

                            // Build output summary from PTY output buffer
                            let output_summary = agent_id
                                .as_ref()
                                .and_then(|aid| runtime.get_output(aid).ok())
                                .map(|lines| {
                                    let last_50: Vec<&str> =
                                        lines.iter().rev().take(50).map(|s| s.as_str()).collect();
                                    let mut reversed = last_50;
                                    reversed.reverse();
                                    reversed.join("\n").chars().take(2000).collect::<String>()
                                })
                                .unwrap_or_default();

                            let entry = MemoryEntryV2 {
                                id: String::new(),
                                node_id: node_id.to_string(),
                                project_id,
                                session_id: None,
                                run_id: run_id.to_string(),
                                timestamp: chrono::Utc::now().to_rfc3339(),
                                input_summary,
                                output_summary,
                                outcome: if success {
                                    "success".to_string()
                                } else {
                                    "failure".to_string()
                                },
                                importance: 0.5,
                                tags: String::new(),
                                context: serde_json::Value::Null,
                            };
                            match memory_store.add_entry(entry) {
                                Ok(entry_id) => {
                                    info!("Memory v2 saved for node {}: id={}", node_id, entry_id);
                                    self.emit_to_bus(
                                        "memory:added",
                                        serde_json::json!({
                                            "entry_id": entry_id,
                                            "node_id": node_id,
                                            "run_id": run_id,
                                        }),
                                    );

                                    // Evict if over max_entries
                                    match memory_store.evict(scope, mem_cfg.max_entries) {
                                        Ok(evicted) if evicted > 0 => {
                                            info!(
                                                "Memory v2 evicted {} entries for node {}",
                                                evicted, node_id
                                            );
                                            self.emit_to_bus(
                                                "memory:evicted",
                                                serde_json::json!({
                                                    "count": evicted,
                                                    "scope": node_id,
                                                    "run_id": run_id,
                                                }),
                                            );
                                        }
                                        Ok(_) => {}
                                        Err(e) => {
                                            warn!(
                                                "Memory v2 eviction failed for node {}: {}",
                                                node_id, e
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to save memory v2 for node {}: {}", node_id, e);
                                }
                            }
                        }
                    }
                }
            }
        }

        self.advance_run(
            run_id,
            workflow,
            runtime,
            pty_manager,
            app,
            cwd,
            lock_manager,
        )?;
        Ok(())
    }

    fn handle_failure(
        &self,
        run_id: &str,
        node_id: &str,
        workflow: &Workflow,
        runtime: &AgentRuntime,
        pty_manager: &PtyManager,
        app: &AppHandle,
        cwd: &str,
    ) -> Result<bool, ReasonanceError> {
        let agent_id = {
            let runs = self.runs.lock().unwrap_or_else(|e| e.into_inner());
            let run = runs
                .get(run_id)
                .ok_or_else(|| ReasonanceError::not_found("run", run_id))?;
            run.node_states
                .get(node_id)
                .and_then(|ns| ns.agent_id.clone())
                .ok_or_else(|| ReasonanceError::workflow(run_id, node_id, "No agent for node"))?
        };
        let agent = runtime
            .get_agent(&agent_id)
            .ok_or_else(|| ReasonanceError::not_found("agent", &agent_id))?;

        // Try retry
        if agent.retry_count < agent.max_retries {
            info!(
                "Retrying node: run_id={}, node_id={}, attempt={}/{}",
                run_id,
                node_id,
                agent.retry_count + 1,
                agent.max_retries
            );
            let _ = runtime.transition(&agent_id, AgentState::Retrying);
            let _ = runtime.transition(&agent_id, AgentState::Running);
            self.update_node_state(run_id, node_id, AgentState::Running, None)?;
            let node = workflow
                .nodes
                .iter()
                .find(|n| n.id == node_id)
                .ok_or_else(|| ReasonanceError::not_found("node", node_id))?;
            let llm = node
                .config
                .get("llm")
                .and_then(|v| v.as_str())
                .unwrap_or("claude");
            let pty_id = pty_manager.spawn(llm, &[], cwd, app.clone())?;
            runtime.set_pty(&agent_id, &pty_id)?;
            self.emit_to_bus(
                "workflow:node-state",
                serde_json::json!({
                    "run_id": run_id,
                    "node_id": node_id,
                    "old_state": "failed",
                    "new_state": "running",
                }),
            );
            self.emit_to_bus(
                "workflow:agent-output",
                serde_json::json!({
                    "run_id": run_id,
                    "node_id": node_id,
                    "pty_id": pty_id,
                }),
            );
            return Ok(true);
        }

        // Try fallback
        if agent.fallback_agent.is_some() {
            info!(
                "Activating fallback for node: run_id={}, node_id={}, fallback={:?}",
                run_id, node_id, agent.fallback_agent
            );
            let _ = runtime.transition(&agent_id, AgentState::Fallback);
            self.update_node_state(run_id, node_id, AgentState::Fallback, None)?;
            let node = workflow
                .nodes
                .iter()
                .find(|n| n.id == node_id)
                .ok_or_else(|| ReasonanceError::not_found("node", node_id))?;
            let fallback_llm = node
                .config
                .get("fallback")
                .and_then(|v| v.as_str())
                .unwrap_or("claude");
            let new_agent_id =
                runtime.create_agent(node_id, &format!("{}:{}", run_id, node_id), 0, None);
            let _ = runtime.transition(&new_agent_id, AgentState::Queued);
            let _ = runtime.transition(&new_agent_id, AgentState::Running);
            let pty_id = pty_manager.spawn(fallback_llm, &[], cwd, app.clone())?;
            runtime.set_pty(&new_agent_id, &pty_id)?;
            self.update_node_state(
                run_id,
                node_id,
                AgentState::Running,
                Some(new_agent_id.clone()),
            )?;
            self.emit_to_bus(
                "workflow:node-state",
                serde_json::json!({
                    "run_id": run_id,
                    "node_id": node_id,
                    "old_state": "failed",
                    "new_state": "running",
                }),
            );
            self.emit_to_bus(
                "workflow:agent-output",
                serde_json::json!({
                    "run_id": run_id,
                    "node_id": node_id,
                    "pty_id": pty_id,
                }),
            );
            return Ok(true);
        }

        Ok(false)
    }

    pub fn step_run(
        &self,
        run_id: &str,
        workflow: &Workflow,
        runtime: &AgentRuntime,
        pty_manager: &PtyManager,
        app: &AppHandle,
        cwd: &str,
        lock_manager: &ResourceLockManager,
    ) -> Result<Option<String>, ReasonanceError> {
        debug!("Stepping workflow run: run_id={}", run_id);
        {
            let mut runs = self.runs.lock().unwrap_or_else(|e| e.into_inner());
            let run = runs
                .get_mut(run_id)
                .ok_or_else(|| ReasonanceError::not_found("run", run_id))?;
            if run.status != RunStatus::Paused && run.status != RunStatus::Running {
                return Err(ReasonanceError::workflow(
                    run_id,
                    "",
                    format!("Cannot step run in {:?} state", run.status),
                ));
            }
            run.status = RunStatus::Running;
        }
        let node_states = {
            let runs = self.runs.lock().unwrap_or_else(|e| e.into_inner());
            runs.get(run_id)
                .ok_or_else(|| ReasonanceError::not_found("run", run_id))?
                .node_states
                .clone()
        };
        let ready = Self::get_ready_nodes(&workflow.edges, &node_states);

        if ready.is_empty() {
            let mut runs = self.runs.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(run) = runs.get_mut(run_id) {
                run.status = RunStatus::Paused;
            }
            return Ok(None);
        }

        let started = self.advance_run(
            run_id,
            workflow,
            runtime,
            pty_manager,
            app,
            cwd,
            lock_manager,
        )?;
        {
            let mut runs = self.runs.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(run) = runs.get_mut(run_id) {
                if run.status == RunStatus::Running {
                    run.status = RunStatus::Paused;
                }
            }
        }
        Ok(started.into_iter().next())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_store::*;

    fn make_workflow(nodes: Vec<(&str, NodeType)>, edges: Vec<(&str, &str)>) -> Workflow {
        Workflow {
            name: "Test".to_string(),
            version: "1.0".to_string(),
            schema_version: 1,
            description: None,
            created: None,
            modified: None,
            nodes: nodes
                .into_iter()
                .map(|(id, nt)| WorkflowNode {
                    id: id.to_string(),
                    node_type: nt,
                    label: id.to_string(),
                    config: serde_json::json!({}),
                    position: Position { x: 0.0, y: 0.0 },
                })
                .collect(),
            edges: edges
                .into_iter()
                .enumerate()
                .map(|(i, (from, to))| WorkflowEdge {
                    id: format!("e{}", i),
                    from: from.to_string(),
                    to: to.to_string(),
                    label: None,
                })
                .collect(),
            settings: WorkflowSettings::default(),
        }
    }

    #[test]
    fn test_topological_sort_linear() {
        let wf = make_workflow(
            vec![
                ("A", NodeType::Agent),
                ("B", NodeType::Agent),
                ("C", NodeType::Agent),
            ],
            vec![("A", "B"), ("B", "C")],
        );
        let sorted = WorkflowEngine::topological_sort(&wf).unwrap();
        assert_eq!(sorted, vec!["A", "B", "C"]);
    }

    #[test]
    fn test_topological_sort_fan_out() {
        let wf = make_workflow(
            vec![
                ("A", NodeType::Agent),
                ("B", NodeType::Agent),
                ("C", NodeType::Agent),
            ],
            vec![("A", "B"), ("A", "C")],
        );
        let sorted = WorkflowEngine::topological_sort(&wf).unwrap();
        assert_eq!(sorted[0], "A");
        assert!(sorted.contains(&"B".to_string()));
        assert!(sorted.contains(&"C".to_string()));
    }

    #[test]
    fn test_topological_sort_cycle() {
        let wf = make_workflow(
            vec![("A", NodeType::Agent), ("B", NodeType::Agent)],
            vec![("A", "B"), ("B", "A")],
        );
        let result = WorkflowEngine::topological_sort(&wf);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cycle"));
    }

    #[test]
    fn test_topological_sort_empty() {
        let wf = make_workflow(vec![], vec![]);
        let sorted = WorkflowEngine::topological_sort(&wf).unwrap();
        assert!(sorted.is_empty());
    }

    #[test]
    fn test_topological_sort_disconnected() {
        let wf = make_workflow(
            vec![
                ("A", NodeType::Agent),
                ("B", NodeType::Agent),
                ("C", NodeType::Agent),
            ],
            vec![],
        );
        let sorted = WorkflowEngine::topological_sort(&wf).unwrap();
        assert_eq!(sorted.len(), 3);
    }

    #[test]
    fn test_predecessors_and_successors() {
        let edges = vec![
            WorkflowEdge {
                id: "e1".into(),
                from: "A".into(),
                to: "B".into(),
                label: None,
            },
            WorkflowEdge {
                id: "e2".into(),
                from: "A".into(),
                to: "C".into(),
                label: None,
            },
            WorkflowEdge {
                id: "e3".into(),
                from: "B".into(),
                to: "D".into(),
                label: None,
            },
            WorkflowEdge {
                id: "e4".into(),
                from: "C".into(),
                to: "D".into(),
                label: None,
            },
        ];

        let preds_d = WorkflowEngine::get_predecessors("D", &edges);
        assert_eq!(preds_d.len(), 2);
        assert!(preds_d.contains(&"B".to_string()));
        assert!(preds_d.contains(&"C".to_string()));

        let succs_a = WorkflowEngine::get_successors("A", &edges);
        assert_eq!(succs_a.len(), 2);

        let preds_a = WorkflowEngine::get_predecessors("A", &edges);
        assert!(preds_a.is_empty());
    }

    #[test]
    fn test_ready_nodes() {
        let edges = vec![
            WorkflowEdge {
                id: "e1".into(),
                from: "A".into(),
                to: "C".into(),
                label: None,
            },
            WorkflowEdge {
                id: "e2".into(),
                from: "B".into(),
                to: "C".into(),
                label: None,
            },
        ];
        let mut states = HashMap::new();
        states.insert(
            "A".to_string(),
            NodeRunState {
                node_id: "A".into(),
                agent_id: None,
                state: AgentState::Success,
            },
        );
        states.insert(
            "B".to_string(),
            NodeRunState {
                node_id: "B".into(),
                agent_id: None,
                state: AgentState::Idle,
            },
        );
        states.insert(
            "C".to_string(),
            NodeRunState {
                node_id: "C".into(),
                agent_id: None,
                state: AgentState::Idle,
            },
        );

        // C is not ready because B is still Idle
        let ready = WorkflowEngine::get_ready_nodes(&edges, &states);
        assert_eq!(ready, vec!["B".to_string()]);

        // After B completes, C becomes ready
        states.get_mut("B").unwrap().state = AgentState::Success;
        let ready = WorkflowEngine::get_ready_nodes(&edges, &states);
        assert_eq!(ready, vec!["C".to_string()]);
    }

    #[test]
    fn test_create_run() {
        let engine = WorkflowEngine::new();
        let wf = make_workflow(
            vec![("A", NodeType::Agent), ("R", NodeType::Resource)],
            vec![("R", "A")],
        );
        let run_id = engine.create_run(&wf, "test.json").unwrap();
        let run = engine.get_run(&run_id).unwrap();

        assert_eq!(run.status, RunStatus::Running);
        assert_eq!(run.node_states.len(), 2);
        // Resource nodes start as Success
        assert_eq!(run.node_states["R"].state, AgentState::Success);
        // Agent nodes start as Idle
        assert_eq!(run.node_states["A"].state, AgentState::Idle);
    }

    #[test]
    fn test_create_run_cycle_rejected() {
        let engine = WorkflowEngine::new();
        let wf = make_workflow(
            vec![("A", NodeType::Agent), ("B", NodeType::Agent)],
            vec![("A", "B"), ("B", "A")],
        );
        let result = engine.create_run(&wf, "cyclic.json");
        assert!(result.is_err());
    }

    #[test]
    fn test_pause_resume_stop() {
        let engine = WorkflowEngine::new();
        let wf = make_workflow(vec![("A", NodeType::Agent)], vec![]);
        let run_id = engine.create_run(&wf, "test.json").unwrap();

        engine.pause_run(&run_id).unwrap();
        assert_eq!(engine.get_run(&run_id).unwrap().status, RunStatus::Paused);

        engine.resume_run(&run_id).unwrap();
        assert_eq!(engine.get_run(&run_id).unwrap().status, RunStatus::Running);

        engine.stop_run(&run_id).unwrap();
        assert_eq!(engine.get_run(&run_id).unwrap().status, RunStatus::Stopped);
    }

    #[test]
    fn test_finalize_run_success() {
        let engine = WorkflowEngine::new();
        let wf = make_workflow(vec![("A", NodeType::Agent)], vec![]);
        let run_id = engine.create_run(&wf, "test.json").unwrap();

        engine
            .update_node_state(&run_id, "A", AgentState::Success, None)
            .unwrap();
        let status = engine.finalize_run(&run_id).unwrap();
        assert_eq!(status, RunStatus::Completed);
    }

    #[test]
    fn test_finalize_run_with_errors() {
        let engine = WorkflowEngine::new();
        let wf = make_workflow(vec![("A", NodeType::Agent), ("B", NodeType::Agent)], vec![]);
        let run_id = engine.create_run(&wf, "test.json").unwrap();

        engine
            .update_node_state(&run_id, "A", AgentState::Success, None)
            .unwrap();
        engine
            .update_node_state(&run_id, "B", AgentState::Error, None)
            .unwrap();
        let status = engine.finalize_run(&run_id).unwrap();
        assert_eq!(status, RunStatus::Failed);
    }

    #[test]
    fn test_check_run_complete() {
        let engine = WorkflowEngine::new();
        let wf = make_workflow(vec![("A", NodeType::Agent), ("B", NodeType::Agent)], vec![]);
        let run_id = engine.create_run(&wf, "test.json").unwrap();

        assert!(!engine.check_run_complete(&run_id).unwrap());

        engine
            .update_node_state(&run_id, "A", AgentState::Success, None)
            .unwrap();
        assert!(!engine.check_run_complete(&run_id).unwrap());

        engine
            .update_node_state(&run_id, "B", AgentState::Error, None)
            .unwrap();
        assert!(engine.check_run_complete(&run_id).unwrap());
    }

    #[test]
    fn test_all_predecessors_complete_partial() {
        // Diamond: A -> C, B -> C. C is only ready when both A and B succeed.
        let edges = vec![
            WorkflowEdge {
                id: "e1".into(),
                from: "A".into(),
                to: "C".into(),
                label: None,
            },
            WorkflowEdge {
                id: "e2".into(),
                from: "B".into(),
                to: "C".into(),
                label: None,
            },
        ];
        let mut states = HashMap::new();
        states.insert(
            "A".to_string(),
            NodeRunState {
                node_id: "A".into(),
                agent_id: None,
                state: AgentState::Success,
            },
        );
        states.insert(
            "B".to_string(),
            NodeRunState {
                node_id: "B".into(),
                agent_id: None,
                state: AgentState::Running,
            },
        );
        states.insert(
            "C".to_string(),
            NodeRunState {
                node_id: "C".into(),
                agent_id: None,
                state: AgentState::Idle,
            },
        );

        // C not ready: B is still Running
        assert!(!WorkflowEngine::all_predecessors_complete(
            "C", &edges, &states
        ));

        // A node with no predecessors is always "ready"
        assert!(WorkflowEngine::all_predecessors_complete(
            "A", &edges, &states
        ));

        // After B completes, C becomes ready
        states.get_mut("B").unwrap().state = AgentState::Success;
        assert!(WorkflowEngine::all_predecessors_complete(
            "C", &edges, &states
        ));
    }
}
