/**
 * Zod schemas for batchable IPC command results.
 *
 * These mirror the TypeScript types used by the Adapter interface so that
 * batch_invoke responses can be validated at the frontend boundary.
 *
 * Complex/opaque payloads (session events, workflow YAML, etc.) are omitted —
 * those bypass Zod and trust the Rust serialization directly.
 */
import { z } from 'zod';

// === Filesystem ===

export const FileEntrySchema = z.object({
  name: z.string(),
  path: z.string(),
  isDir: z.boolean(),
  size: z.number(),
  modified: z.number(),
  isGitignored: z.boolean(),
});

export const GrepResultSchema = z.object({
  path: z.string(),
  line_number: z.number(),
  line: z.string(),
});

// === Sessions ===

const ErrorSeveritySchema = z.enum(['recoverable', 'degraded', 'fatal']);

const SessionStatusSchema: z.ZodType = z.union([
  z.enum(['active', 'idle', 'resumable', 'terminated']),
  z.object({ error: z.object({ severity: ErrorSeveritySchema }) }),
]);

const SessionSourceSchema: z.ZodType = z.union([
  z.literal('user'),
  z.object({ workflow: z.object({ run_id: z.string(), node_id: z.string() }) }),
]);

export const SessionSummarySchema = z.object({
  id: z.string(),
  provider: z.string(),
  model: z.string(),
  title: z.string(),
  status: SessionStatusSchema,
  created_at: z.number(),
  last_active_at: z.number(),
  event_count: z.number(),
  source: SessionSourceSchema,
});

// === Workflows ===

const RunStatusSchema = z.enum(['idle', 'running', 'paused', 'completed', 'failed', 'stopped']);

const AgentStateSchema = z.enum([
  'idle', 'queued', 'running', 'success', 'failed', 'retrying', 'fallback', 'error', 'skipped',
]);

const NodeRunStateSchema = z.object({
  node_id: z.string(),
  agent_id: z.string().nullable(),
  state: AgentStateSchema,
});

export const WorkflowRunSchema = z.object({
  id: z.string(),
  workflow_path: z.string(),
  status: RunStatusSchema,
  node_states: z.record(z.string(), NodeRunStateSchema),
  started_at: z.string().nullable(),
  finished_at: z.string().nullable(),
});

// === Analytics ===

export const DailyStatsSchema = z.object({
  date: z.string(),
  provider: z.string().nullable(),
  sessions: z.number(),
  input_tokens: z.number(),
  output_tokens: z.number(),
  errors: z.number(),
  avg_duration_ms: z.number(),
  total_cost_usd: z.number().optional(),
});

export const ProviderAnalyticsSchema = z.object({
  provider: z.string(),
  total_sessions: z.number(),
  total_input_tokens: z.number(),
  total_output_tokens: z.number(),
  total_cache_creation_tokens: z.number(),
  total_cache_read_tokens: z.number(),
  cache_hit_rate: z.number(),
  total_errors: z.number(),
  recovered_errors: z.number(),
  error_rate: z.number(),
  avg_duration_ms: z.number(),
  avg_tokens_per_second: z.number(),
  most_used_model: z.string(),
  total_tool_invocations: z.number(),
  total_cost_usd: z.number().nullable(),
});

export const ModelAnalyticsSchema = z.object({
  model: z.string(),
  provider: z.string(),
  session_count: z.number(),
  avg_input_tokens: z.number(),
  avg_output_tokens: z.number(),
  avg_duration_ms: z.number(),
  avg_tokens_per_second: z.number(),
  error_rate: z.number(),
});

// === App State ===

const WindowStateSchema = z.object({
  width: z.number(),
  height: z.number(),
  x: z.number(),
  y: z.number(),
  maximized: z.boolean(),
});

const RecentProjectEntrySchema = z.object({
  path: z.string(),
  label: z.string(),
  last_opened: z.number(),
});

export const AppStateSchema = z.object({
  last_active_project_id: z.string().nullable(),
  recent_projects: z.array(RecentProjectEntrySchema),
  window_state: WindowStateSchema.nullable(),
});

const OpenFileStateSchema = z.object({
  path: z.string(),
  cursor_line: z.number(),
  cursor_column: z.number(),
  scroll_offset: z.number(),
});

const PanelLayoutSchema = z.object({
  sidebar_visible: z.boolean(),
  sidebar_width: z.number(),
  bottom_panel_visible: z.boolean(),
  bottom_panel_height: z.number(),
});

export const ProjectStateSchema = z.object({
  active_session_id: z.string().nullable(),
  open_files: z.array(OpenFileStateSchema),
  active_file_path: z.string().nullable(),
  panel_layout: PanelLayoutSchema.nullable(),
  last_model_used: z.string().nullable(),
});

// === Agent Memory v2 ===

export const MemoryEntryV2Schema = z.object({
  id: z.string(),
  node_id: z.string(),
  project_id: z.string().nullable(),
  session_id: z.string().nullable(),
  run_id: z.string(),
  timestamp: z.string(),
  input_summary: z.string(),
  output_summary: z.string(),
  outcome: z.string(),
  importance: z.number(),
  tags: z.string(),
  context: z.unknown(),
});

// === Agent Communications (CommsBus) ===

const CommsChannelTypeSchema: z.ZodType = z.union([
  z.object({ type: z.literal('Direct'), value: z.object({ target_id: z.string() }) }),
  z.object({ type: z.literal('Broadcast'), value: z.object({ workflow_id: z.string() }) }),
  z.object({ type: z.literal('Topic'), value: z.object({ name: z.string() }) }),
]);

export const CommsMessageSchema = z.object({
  id: z.string(),
  from: z.string(),
  channel: CommsChannelTypeSchema,
  payload: z.unknown(),
  timestamp: z.string(),
  reply_to: z.string().nullable(),
  ttl_secs: z.number().nullable(),
});

// === Node Registry ===

export const NodeDescriptorSchema = z.object({
  type_id: z.string(),
  display_name: z.string(),
  description: z.string(),
  category: z.string(),
  config_schema: z.unknown(),
});

// === Self-Heal ===

export const HealNormalizerResultSchema = z.object({
  status: z.string(),
  message: z.string(),
  iterations: z.number(),
});

// === Model Slots ===

const ModelSlotSchema = z.enum(['chat', 'workflow', 'summary', 'quick']);

export const ModelSlotEntrySchema = z.tuple([ModelSlotSchema, z.string().nullable()]);

// === Schema map (command name → result schema) ===

export const batchSchemas: Record<string, z.ZodType> = {
  read_file: z.string(),
  write_file: z.null(),
  list_dir: z.array(FileEntrySchema),
  grep_files: z.array(GrepResultSchema),
  get_git_status: z.record(z.string(), z.string()),
  session_create: z.string(),
  session_list: z.array(SessionSummarySchema),
  get_app_state: AppStateSchema,
  get_project_state: ProjectStateSchema,
  save_app_state: z.null(),
  save_project_state: z.null(),
  get_run_status: WorkflowRunSchema.nullable(),
  list_workflows: z.array(z.string()),
  analytics_daily: z.array(DailyStatsSchema),
  analytics_compare: z.array(ProviderAnalyticsSchema),
  analytics_model_breakdown: z.array(ModelAnalyticsSchema),
  store_shadow: z.null(),
  get_shadow: z.string().nullable(),
  read_config: z.string(),
  get_setting: z.unknown().nullable(),
  record_permission_decision: z.null(),
  lookup_permission_decision: z.unknown().nullable(),
  clear_permission_session: z.null(),
  memory_add_entry: z.string(),
  memory_search: z.array(MemoryEntryV2Schema),
  memory_list: z.array(MemoryEntryV2Schema),
  memory_get: MemoryEntryV2Schema.nullable(),
  // Model Slots
  get_model_for_slot: z.string().nullable(),
  set_model_slot: z.null(),
  list_model_slots: z.array(ModelSlotEntrySchema),
  // PTY
  reconnect_pty: z.string(),
  // Node Registry
  get_node_types: z.array(NodeDescriptorSchema),
  // Agent Communications (CommsBus)
  agent_publish_message: z.string(),
  agent_get_messages: z.array(CommsMessageSchema),
  agent_get_topic_messages: z.array(CommsMessageSchema),
  agent_get_broadcast_messages: z.array(CommsMessageSchema),
  agent_sweep_messages: z.number(),
  agent_clear_workflow_messages: z.null(),
  // Self-Heal
  heal_normalizer: HealNormalizerResultSchema,
};
