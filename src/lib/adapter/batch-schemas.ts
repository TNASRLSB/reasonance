/**
 * Zod schema registry for batch-validated IPC commands.
 *
 * Coverage scope:
 * - All data-fetching commands (read, list, get, search) → validated
 * - All state-mutation commands (save, record, set) → validated
 * - Long-running/streaming commands (agentSend, testProviderConnection) → inline z.parse()
 * - Tauri plugin commands (clipboard, dialog, window, notification) → external APIs, not validated
 *
 * Commands using inline z.parse() instead of batch schemas are:
 * 1. agent_send — long-running LLM call, may take minutes
 * 2. agent_stop — terminates an active streaming session
 * 3. agent_get_events — returns full session event history
 * 4. test_provider_connection — long-running, emits step events
 * 5. reload_normalizers — reloads all providers from disk
 * 6. start_watching — sets up persistent filesystem watcher
 *
 * These schemas mirror the TypeScript types used by the Adapter interface so that
 * batch_invoke responses can be validated at the frontend boundary.
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

// === Discovery ===

export const DiscoveredLlmSchema = z.object({
  name: z.string(),
  command: z.string(),
  found: z.boolean(),
});

const CapabilityProfileSchema = z.object({
  read_file: z.boolean(),
  write_file: z.boolean(),
  execute_command: z.boolean(),
  web_search: z.boolean(),
  image_input: z.boolean(),
  long_context: z.boolean(),
});

export const DiscoveredAgentSchema = z.object({
  name: z.string(),
  source: z.enum(['cli', 'api', 'manual']),
  command: z.string().nullable(),
  endpoint: z.string().nullable(),
  models: z.array(z.string()),
  capabilities: CapabilityProfileSchema,
  max_context: z.number().nullable(),
  available: z.boolean(),
});

// === Workflow YAML ===

const WorkflowNodeSchema = z.object({
  id: z.string(),
  type: z.enum(['agent', 'resource', 'logic']),
  label: z.string(),
  config: z.record(z.string(), z.unknown()),
  position: z.object({ x: z.number(), y: z.number() }),
});

const WorkflowEdgeSchema = z.object({
  id: z.string().optional(),
  from: z.string(),
  to: z.string(),
  label: z.string().optional(),
});

const WorkflowSettingsSchema = z.object({
  max_concurrent_agents: z.number(),
  default_retry: z.number(),
  timeout: z.number(),
  permissionLevel: z.enum(['supervised', 'trusted', 'dry-run']),
});

export const WorkflowSchema = z.object({
  name: z.string(),
  version: z.string(),
  schemaVersion: z.number(),
  description: z.string().optional(),
  created: z.string().optional(),
  modified: z.string().optional(),
  nodes: z.array(WorkflowNodeSchema),
  edges: z.array(WorkflowEdgeSchema),
  settings: WorkflowSettingsSchema,
});

// === Agent Runtime ===

export const AgentInstanceSchema = z.object({
  id: z.string(),
  node_id: z.string(),
  workflow_path: z.string(),
  state: AgentStateSchema,
  pty_id: z.string().nullable(),
  retry_count: z.number(),
  max_retries: z.number(),
  fallback_agent: z.string().nullable(),
  started_at: z.string().nullable(),
  finished_at: z.string().nullable(),
  error_message: z.string().nullable(),
  output_buffer: z.array(z.string()),
});

export const AgentMessageSchema = z.object({
  from: z.string(),
  to: z.string(),
  payload: z.unknown(),
  timestamp: z.string(),
});

// === Agent Memory (v1 format from get_agent_memory) ===

export const MemoryEntryV1Schema = z.object({
  id: z.string().optional(),
  node_id: z.string().optional(),
  project_id: z.string().nullable().optional(),
  session_id: z.string().nullable().optional(),
  run_id: z.string(),
  timestamp: z.string(),
  input_summary: z.string(),
  output_summary: z.string(),
  outcome: z.string(),
  importance: z.number().optional(),
  tags: z.string().optional(),
  context: z.unknown(),
});

// === Capability & Health ===

const WorkaroundMethodSchema: z.ZodType = z.union([
  z.literal('inline_in_prompt'),
  z.literal('simulate_from_batch'),
  z.object({ fallback_flag: z.string() }),
  z.literal('skip_silently'),
]);

const WorkaroundSchema = z.object({
  description: z.string(),
  method: WorkaroundMethodSchema,
});

const FeatureSupportSchema: z.ZodType = z.union([
  z.object({ level: z.literal('full') }),
  z.object({ level: z.literal('partial'), limitations: z.array(z.string()), workaround: WorkaroundSchema.optional() }),
  z.object({ level: z.literal('unsupported'), alternative: WorkaroundSchema.optional() }),
]);

export const NegotiatedCapabilitiesSchema = z.object({
  provider: z.string(),
  cli_version: z.string(),
  cli_mode: z.enum(['structured', 'basic_print', 'pty_only', 'direct_api']),
  features: z.record(z.string(), FeatureSupportSchema),
  negotiated_at: z.number(),
});

export const CliVersionInfoSchema = z.object({
  provider: z.string(),
  current_version: z.string().nullable(),
  last_checked: z.number().nullable(),
  auto_update: z.boolean(),
  version_command: z.array(z.string()),
  update_command: z.array(z.string()),
});

export const VersionEntrySchema = z.object({
  id: z.string(),
  provider: z.string(),
  timestamp: z.number(),
  checksum: z.string(),
});

const TestCaseResultSchema = z.object({
  name: z.string(),
  passed: z.boolean(),
  failure_reason: z.string().nullable(),
});

const HealthStatusSchema: z.ZodType = z.union([
  z.object({ type: z.literal('healthy') }),
  z.object({ type: z.literal('degraded'), failing_tests: z.array(z.string()) }),
  z.object({ type: z.literal('broken'), error: z.string() }),
]);

export const HealthReportSchema = z.object({
  provider: z.string(),
  status: HealthStatusSchema,
  results: z.array(TestCaseResultSchema),
  capabilities_confirmed: z.array(z.string()),
  capabilities_missing: z.array(z.string()),
  capabilities_broken: z.array(z.string()),
  tested_at: z.number(),
  cli_version: z.string(),
});

// === Analytics (session-level) ===

const ErrorRecordSchema = z.object({
  timestamp: z.number(),
  code: z.string(),
  severity: z.string(),
  recovered: z.boolean(),
});

export const SessionMetricsSchema = z.object({
  session_id: z.string(),
  provider: z.string(),
  model: z.string(),
  started_at: z.number(),
  ended_at: z.number().nullable(),
  input_tokens: z.number(),
  output_tokens: z.number(),
  cache_creation_tokens: z.number(),
  cache_read_tokens: z.number(),
  duration_ms: z.number().nullable(),
  duration_api_ms: z.number().nullable(),
  num_turns: z.number(),
  tools_used: z.record(z.string(), z.number()),
  stop_reason: z.string().nullable(),
  peak_context_usage: z.number().nullable(),
  max_context_tokens: z.number().nullable(),
  total_cost_usd: z.number().nullable(),
  errors: z.array(ErrorRecordSchema),
});

// === Workspace Trust ===

const TrustLevelSchema = z.enum(['trusted', 'read_only', 'blocked']);

const FolderInfoSchema = z.object({
  name: z.string(),
  path: z.string(),
  has_git: z.boolean(),
  file_count: z.number(),
});

export const TrustCheckResultSchema = z.object({
  level: TrustLevelSchema.nullable(),
  needs_prompt: z.boolean(),
  folder_info: FolderInfoSchema.nullable(),
  rename_hint: z.string().nullable(),
});

export const TrustEntrySchema = z.object({
  hash: z.string(),
  path: z.string(),
  level: TrustLevelSchema,
  trusted_at: z.string(),
});

export const NormalizerConfigSchema = z.object({
  binary: z.string(),
  programmatic_args: z.array(z.string()),
  resume_args: z.array(z.string()),
  permission_args: z.array(z.string()),
  image_mode: z.string().nullable(),
  transport_mode: z.string().nullable(),
  rules_count: z.number(),
});

// === Session ===

export const SessionHandleSchema = z.object({
  id: z.string(),
  provider: z.string(),
  model: z.string(),
  cli_session_id: z.string().nullable(),
  status: SessionStatusSchema,
  title: z.string(),
  created_at: z.number(),
  last_active_at: z.number(),
  event_count: z.number(),
  view_mode: z.enum(['chat', 'terminal']),
  source: SessionSourceSchema,
  forked_from: z.object({
    parent_session_id: z.string(),
    fork_event_index: z.number(),
  }).nullable(),
});

// === Agent Events ===

const DiffHunkSchema = z.object({
  old_start: z.number(),
  new_start: z.number(),
  old_lines: z.array(z.string()),
  new_lines: z.array(z.string()),
});

const EventContentSchema: z.ZodType = z.union([
  z.object({ type: z.literal('text'), value: z.string() }),
  z.object({ type: z.literal('code'), language: z.string(), source: z.string() }),
  z.object({ type: z.literal('diff'), file_path: z.string(), hunks: z.array(DiffHunkSchema) }),
  z.object({ type: z.literal('file_ref'), path: z.string(), action: z.enum(['read', 'write', 'create', 'delete']) }),
  z.object({ type: z.literal('json'), value: z.unknown() }),
]);

const StreamMetricsSchema = z.object({
  tokens_so_far: z.number(),
  elapsed_ms: z.number(),
  tokens_per_second: z.number(),
});

const AgentEventMetadataSchema = z.object({
  session_id: z.string().nullable(),
  input_tokens: z.number().nullable(),
  output_tokens: z.number().nullable(),
  tool_name: z.string().nullable(),
  model: z.string().nullable(),
  provider: z.string(),
  error_severity: ErrorSeveritySchema.nullable(),
  error_code: z.string().nullable(),
  stream_metrics: StreamMetricsSchema.nullable(),
});

export const AgentEventSchema = z.object({
  id: z.string(),
  parent_id: z.string().nullable(),
  event_type: z.enum(['text', 'tool_use', 'tool_result', 'thinking', 'error', 'status', 'usage', 'metrics', 'permission_denial', 'done']),
  content: EventContentSchema,
  timestamp: z.number(),
  metadata: AgentEventMetadataSchema,
});

// === Theme ===

export const ThemePreferencesSchema = z.object({
  activeTheme: z.string(),
  activeModifiers: z.array(z.string()),
});

// === Schema map (command name → result schema) ===

export const batchSchemas: Record<string, z.ZodType> = {
  // Filesystem
  read_file: z.string(),
  write_file: z.null(),
  list_dir: z.array(FileEntrySchema),
  grep_files: z.array(GrepResultSchema),
  get_git_status: z.record(z.string(), z.string()),
  set_project_root: z.null(),
  write_config: z.null(),
  read_config: z.string(),
  store_shadow: z.null(),
  get_shadow: z.string().nullable(),

  // System
  open_external: z.null(),
  discover_llms: z.array(DiscoveredLlmSchema),

  // PTY
  spawn_process: z.string(),
  kill_process: z.null(),
  resize_pty: z.null(),
  write_pty: z.null(),
  sweep_ptys: z.array(z.string()),
  reconnect_pty: z.string(),
  kill_all_ptys: z.number(),
  kill_project_ptys: z.array(z.string()),

  // Discovery
  discover_agents: z.array(DiscoveredAgentSchema),
  get_discovered_agents: z.array(DiscoveredAgentSchema),

  // Workflows
  load_workflow: WorkflowSchema,
  save_workflow: z.null(),
  list_workflows: z.array(z.string()),
  delete_workflow: z.null(),
  create_workflow: WorkflowSchema,
  get_workflow: WorkflowSchema.nullable(),
  duplicate_workflow: WorkflowSchema,
  save_to_global: z.string(),
  list_global_workflows: z.array(z.string()),

  // Agent Runtime
  create_agent: z.string(),
  transition_agent: AgentStateSchema,
  set_agent_pty: z.null(),
  set_agent_error: z.null(),
  get_agent: AgentInstanceSchema.nullable(),
  get_workflow_agents: z.array(AgentInstanceSchema),
  remove_agent: z.null(),
  stop_workflow_agents: z.null(),
  send_agent_message: z.null(),
  get_agent_messages: z.array(AgentMessageSchema),
  get_agent_memory: z.array(MemoryEntryV1Schema),

  // Workflow Engine
  play_workflow: z.string(),
  pause_workflow: z.null(),
  resume_workflow: z.null(),
  stop_workflow: z.null(),
  step_workflow: z.string().nullable(),
  get_run_status: WorkflowRunSchema.nullable(),
  notify_node_completed: z.null(),

  // Session Management
  session_create: z.string(),
  session_restore: SessionHandleSchema,
  session_get_events: z.array(AgentEventSchema),
  session_list: z.array(SessionSummarySchema),
  session_delete: z.null(),
  session_rename: z.null(),
  session_fork: z.string(),
  session_set_view_mode: z.null(),

  // Capability & Health
  get_capabilities: z.record(z.string(), NegotiatedCapabilitiesSchema),
  get_provider_capabilities: NegotiatedCapabilitiesSchema,
  get_cli_versions: z.array(CliVersionInfoSchema),
  get_normalizer_versions: z.array(VersionEntrySchema),
  rollback_normalizer: z.string(),
  get_health_report: HealthReportSchema,
  get_all_health_reports: z.record(z.string(), HealthReportSchema),

  // Analytics
  analytics_daily: z.array(DailyStatsSchema),
  analytics_compare: z.array(ProviderAnalyticsSchema),
  analytics_model_breakdown: z.array(ModelAnalyticsSchema),
  analytics_provider: ProviderAnalyticsSchema,
  analytics_session: SessionMetricsSchema.nullable(),
  analytics_active: z.array(SessionMetricsSchema),

  // Workspace Trust
  check_workspace_trust: TrustCheckResultSchema,
  set_workspace_trust: z.null(),
  revoke_workspace_trust: z.null(),
  list_workspace_trust: z.array(TrustEntrySchema),
  get_normalizer_config: NormalizerConfigSchema.nullable(),

  // App State
  get_app_state: AppStateSchema,
  save_app_state: z.null(),
  get_project_state: ProjectStateSchema,
  save_project_state: z.null(),

  // File Operations
  file_ops_set_project: z.null(),
  file_ops_delete: z.null(),
  file_ops_undo: z.string().nullable(),
  file_ops_record_create: z.null(),
  file_ops_record_rename: z.null(),
  file_ops_move: z.null(),

  // Permissions
  record_permission_decision: z.null(),
  lookup_permission_decision: z.unknown().nullable(),
  clear_permission_session: z.null(),

  // Agent Memory v2
  memory_add_entry: z.string(),
  memory_search: z.array(MemoryEntryV2Schema),
  memory_list: z.array(MemoryEntryV2Schema),
  memory_get: MemoryEntryV2Schema.nullable(),

  // Layered Settings
  get_setting: z.unknown().nullable(),
  set_setting: z.null(),
  get_all_settings: z.record(z.string(), z.unknown()),
  reload_settings: z.null(),

  // Model Slots
  get_model_for_slot: z.string().nullable(),
  set_model_slot: z.null(),
  list_model_slots: z.array(ModelSlotEntrySchema),

  // Multi-project
  add_project: z.null(),
  remove_project: z.null(),
  set_active_project: z.null(),
  get_project_root: z.string(),

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
