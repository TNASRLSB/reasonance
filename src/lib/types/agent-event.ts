// TypeScript mirrors of Rust AgentEvent types (src-tauri/src/agent_event.rs)
// These must match the serde serialization format from the backend.

export type AgentEventType =
  | 'text'
  | 'tool_use'
  | 'tool_result'
  | 'thinking'
  | 'error'
  | 'status'
  | 'usage'
  | 'metrics'
  | 'permission_denial'
  | 'done';

export type ErrorSeverity = 'recoverable' | 'degraded' | 'fatal';

export type EventContent =
  | { type: 'text'; value: string }
  | { type: 'code'; language: string; source: string }
  | { type: 'diff'; file_path: string; hunks: DiffHunk[] }
  | { type: 'file_ref'; path: string; action: FileAction }
  | { type: 'json'; value: unknown };

export type FileAction = 'read' | 'write' | 'create' | 'delete';

export interface DiffHunk {
  old_start: number;
  new_start: number;
  old_lines: string[];
  new_lines: string[];
}

export interface StreamMetrics {
  tokens_so_far: number;
  elapsed_ms: number;
  tokens_per_second: number;
}

export interface AgentEventMetadata {
  session_id: string | null;
  input_tokens: number | null;
  output_tokens: number | null;
  tool_name: string | null;
  model: string | null;
  provider: string;
  error_severity: ErrorSeverity | null;
  error_code: string | null;
  stream_metrics: StreamMetrics | null;
  images?: Array<{ mimeType: string; data: string; name: string }> | null;
}

export interface AgentEvent {
  id: string;
  parent_id: string | null;
  event_type: AgentEventType;
  content: EventContent;
  timestamp: number;
  metadata: AgentEventMetadata;
}

// Session types (from transport/session_handle.rs)
export type SessionStatus =
  | 'active'
  | 'idle'
  | 'resumable'
  | 'terminated'
  | { error: { severity: ErrorSeverity } };

export type ViewMode = 'chat' | 'terminal';

export type CliMode = 'structured' | 'basic_print' | 'pty_only' | 'direct_api';

export type SessionSource =
  | 'user'
  | { workflow: { run_id: string; node_id: string } };

export interface ForkInfo {
  parent_session_id: string;
  fork_event_index: number;
}

export interface SessionHandle {
  id: string;
  provider: string;
  model: string;
  cli_session_id: string | null;
  status: SessionStatus;
  title: string;
  created_at: number;
  last_active_at: number;
  event_count: number;
  view_mode: ViewMode;
  source: SessionSource;
  forked_from: ForkInfo | null;
}

export interface SessionSummary {
  id: string;
  provider: string;
  model: string;
  title: string;
  status: SessionStatus;
  created_at: number;
  last_active_at: number;
  event_count: number;
  source: SessionSource;
}

// Tauri event payload (from event_bus.rs TauriFrontendBridge)
export interface AgentEventPayload {
  session_id: string;
  event: AgentEvent;
}
