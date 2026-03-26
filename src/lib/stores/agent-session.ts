import { writable, get } from 'svelte/store';
import type { CliMode, SessionStatus, ViewMode } from '$lib/types/agent-event';

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
  currentSpeed: number;  // tokens/second during active streaming (shown in footer metrics)
  elapsed: number;       // ms since session started (shown in footer metrics)
  turnCount: number;
  projectId: string;     // which project this session belongs to
}

// All known sessions (both active and restored)
export const agentSessions = writable<Map<string, AgentSessionState>>(new Map());

// Helper: add or update a session
export function upsertSession(session: AgentSessionState): void {
  agentSessions.update((map) => {
    const next = new Map(map);
    next.set(session.id, session);
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

// Helper: increment turn count
export function incrementTurnCount(sessionId: string): void {
  agentSessions.update((map) => {
    const session = map.get(sessionId);
    if (!session) return map;
    const next = new Map(map);
    next.set(sessionId, { ...session, turnCount: session.turnCount + 1 });
    return next;
  });
}

