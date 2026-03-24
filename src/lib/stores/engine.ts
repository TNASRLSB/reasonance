import { writable, derived } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';
import type { WorkflowRun, RunStatus, AgentState, NodeRunState } from '$lib/adapter/index';

export interface AgentLogEntry {
  node_id: string;
  line: string;
  timestamp: number;
}

export const agentOutputLog = writable<AgentLogEntry[]>([]);
const MAX_LOG_LINES = 500;

export const currentRun = writable<WorkflowRun | null>(null);
export const currentRunId = writable<string | null>(null);
export const pendingApprovals = writable<Array<{ run_id: string; node_id: string; agent_label: string }>>([]);

export const runStatus = derived(currentRun, ($run): RunStatus => $run?.status ?? 'idle');

export const nodeStates = derived(currentRun, ($run): NodeRunState[] =>
  $run ? Object.values($run.node_states) : []
);

export const completedNodeCount = derived(nodeStates, ($states) =>
  $states.filter(s => s.state === 'success' || s.state === 'skipped').length
);

export const skippedNodeCount = derived(nodeStates, ($states) =>
  $states.filter(s => s.state === 'skipped').length
);

export const totalNodeCount = derived(nodeStates, ($states) => $states.length);

export const activeNodeCount = derived(nodeStates, ($states) =>
  $states.filter(s => s.state === 'running').length
);

export const errorNodeCount = derived(nodeStates, ($states) =>
  $states.filter(s => s.state === 'error').length
);

export const statusSummary = derived(
  [completedNodeCount, totalNodeCount, activeNodeCount, errorNodeCount],
  ([$completed, $total, $active, $errors]) => {
    const parts: string[] = [];
    parts.push(`${$completed}/${$total} complete`);
    if ($active > 0) parts.push(`${$active} active`);
    if ($errors > 0) parts.push(`${$errors} error${$errors > 1 ? 's' : ''}`);
    return parts.join(', ');
  }
);

export async function setupHiveEventListeners() {
  await listen<{ run_id: string; node_id: string; old_state: string; new_state: string }>(
    'hive://node-state-changed',
    (event) => {
      currentRun.update(run => {
        if (!run || run.id !== event.payload.run_id) return run;
        const ns = { ...run.node_states };
        if (ns[event.payload.node_id]) {
          ns[event.payload.node_id] = {
            ...ns[event.payload.node_id],
            state: event.payload.new_state as AgentState,
          };
        }
        return { ...run, node_states: ns };
      });
    }
  );

  await listen<{ run_id: string; old_status: string; new_status: string }>(
    'hive://run-status-changed',
    (event) => {
      currentRun.update(run => {
        if (!run || run.id !== event.payload.run_id) return run;
        return { ...run, status: event.payload.new_status as RunStatus };
      });
    }
  );

  await listen<{ run_id: string; node_id: string; agent_label: string }>(
    'hive://permission-request',
    (event) => {
      pendingApprovals.update(list => [...list, event.payload]);
    }
  );

  await listen<{ run_id: string; status: string }>(
    'hive://run-completed',
    (event) => {
      currentRun.update(run => {
        if (!run || run.id !== event.payload.run_id) return run;
        return { ...run, status: event.payload.status as RunStatus };
      });
    }
  );

  await listen<{ run_id: string; node_id: string; pty_id: string }>(
    'hive://agent-output',
    (event) => {
      const { node_id, pty_id } = event.payload;
      listen<string>(`pty-data-${pty_id}`, (ptyEvent) => {
        agentOutputLog.update(log => {
          const newLog = [...log, { node_id, line: ptyEvent.payload, timestamp: Date.now() }];
          return newLog.slice(-MAX_LOG_LINES);
        });
      });
    }
  );
}
