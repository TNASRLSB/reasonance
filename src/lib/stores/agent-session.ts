import { writable, derived, get } from 'svelte/store';
import type { CliMode, SessionStatus, ViewMode, SessionSummary } from '$lib/types/agent-event';

export interface AgentSessionState {
  id: string;
  provider: string;
  model: string;
  status: SessionStatus;
  viewMode: ViewMode;
  cliMode: CliMode;
  title: string;
  totalInputTokens: number;
  totalOutputTokens: number;
  currentSpeed: number;  // tokens/second during active streaming (Phase 5: ChatHeader uses this)
  elapsed: number;       // ms since session started (Phase 5: ChatHeader uses this)
}

// All known sessions (both active and restored)
export const agentSessions = writable<Map<string, AgentSessionState>>(new Map());

// Currently active/viewed session ID
export const activeAgentSessionId = writable<string | null>(null);

// Derived: current session state
export const activeAgentSession = derived(
  [agentSessions, activeAgentSessionId],
  ([$sessions, $id]) => $id ? $sessions.get($id) ?? null : null
);

// Helper: add or update a session
export function upsertSession(session: AgentSessionState): void {
  agentSessions.update((map) => {
    const next = new Map(map);
    next.set(session.id, session);
    return next;
  });
}

// Helper: remove a session
export function removeSession(sessionId: string): void {
  agentSessions.update((map) => {
    const next = new Map(map);
    next.delete(sessionId);
    return next;
  });
}

// Helper: update session status
export function updateSessionStatus(sessionId: string, status: SessionStatus): void {
  agentSessions.update((map) => {
    const session = map.get(sessionId);
    if (!session) return map;
    const next = new Map(map);
    next.set(sessionId, { ...session, status });
    return next;
  });
}

// Helper: update view mode
export function updateViewMode(sessionId: string, viewMode: ViewMode): void {
  agentSessions.update((map) => {
    const session = map.get(sessionId);
    if (!session) return map;
    const next = new Map(map);
    next.set(sessionId, { ...session, viewMode });
    return next;
  });
}

// Helper: update token counts
export function updateTokens(sessionId: string, inputTokens: number, outputTokens: number): void {
  agentSessions.update((map) => {
    const session = map.get(sessionId);
    if (!session) return map;
    const next = new Map(map);
    next.set(sessionId, {
      ...session,
      totalInputTokens: session.totalInputTokens + inputTokens,
      totalOutputTokens: session.totalOutputTokens + outputTokens,
    });
    return next;
  });
}

// Helper: update streaming metrics (speed, elapsed)
export function updateMetrics(sessionId: string, currentSpeed: number, elapsed: number): void {
  agentSessions.update((map) => {
    const session = map.get(sessionId);
    if (!session) return map;
    const next = new Map(map);
    next.set(sessionId, { ...session, currentSpeed, elapsed });
    return next;
  });
}

// Helper: create session state from summary
export function sessionFromSummary(summary: SessionSummary): AgentSessionState {
  return {
    id: summary.id,
    provider: summary.provider,
    model: summary.model,
    status: summary.status,
    viewMode: 'chat',
    cliMode: 'structured',
    title: summary.title,
    totalInputTokens: 0,
    totalOutputTokens: 0,
    currentSpeed: 0,
    elapsed: 0,
  };
}
