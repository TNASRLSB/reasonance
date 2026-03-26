import { z } from 'zod';

// Matches src-tauri/src/agent_event.rs ErrorSeverity
export const ErrorSeveritySchema = z.enum(['recoverable', 'degraded', 'fatal']);

// Matches src-tauri/src/transport/request.rs SessionStatus (serde rename_all = "snake_case")
// SessionStatus::Error { severity } serialises as { error: { severity: "..." } }
export const SessionStatusSchema = z.union([
  z.literal('active'),
  z.literal('idle'),
  z.literal('resumable'),
  z.literal('terminated'),
  z.object({ error: z.object({ severity: ErrorSeveritySchema }) }),
]);

// Matches src-tauri/src/transport/session_handle.rs ViewMode
export const ViewModeSchema = z.enum(['chat', 'terminal']);

// Matches src-tauri/src/transport/session_handle.rs SessionSource
export const SessionSourceSchema = z.union([
  z.literal('user'),
  z.object({ workflow: z.object({ run_id: z.string(), node_id: z.string() }) }),
]);

// Matches src-tauri/src/transport/session_handle.rs ForkInfo
export const ForkInfoSchema = z.object({
  parent_session_id: z.string(),
  fork_event_index: z.number(),
}).nullable();

// Matches src-tauri/src/transport/session_handle.rs SessionHandle
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
  view_mode: ViewModeSchema,
  source: SessionSourceSchema,
  forked_from: ForkInfoSchema,
});

// Matches src-tauri/src/transport/session_handle.rs SessionSummary
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

export type SessionHandle = z.infer<typeof SessionHandleSchema>;
export type SessionSummary = z.infer<typeof SessionSummarySchema>;
export type SessionStatus = z.infer<typeof SessionStatusSchema>;
export type SessionSource = z.infer<typeof SessionSourceSchema>;
export type ViewMode = z.infer<typeof ViewModeSchema>;
export type ForkInfo = z.infer<typeof ForkInfoSchema>;
