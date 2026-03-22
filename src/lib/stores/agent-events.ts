import { writable, derived, get } from 'svelte/store';
import type { AgentEvent, AgentEventPayload } from '$lib/types/agent-event';
import { updateTokens, updateSessionStatus, updateMetrics } from './agent-session';

// Per-session event lists
export const agentEvents = writable<Map<string, AgentEvent[]>>(new Map());

// Per-session streaming state
export const streamingSessionIds = writable<Set<string>>(new Set());

// Helper: get events for a specific session
export function getSessionEvents(sessionId: string): AgentEvent[] {
  return get(agentEvents).get(sessionId) ?? [];
}

// Helper: set events for a session (e.g., on restore)
export function setSessionEvents(sessionId: string, events: AgentEvent[]): void {
  agentEvents.update((map) => {
    const next = new Map(map);
    next.set(sessionId, [...events]);
    return next;
  });
}

// Helper: clear events for a session
export function clearSessionEvents(sessionId: string): void {
  agentEvents.update((map) => {
    const next = new Map(map);
    next.delete(sessionId);
    return next;
  });
}

// Helper: mark session as streaming
export function setStreaming(sessionId: string, streaming: boolean): void {
  streamingSessionIds.update((set) => {
    const next = new Set(set);
    if (streaming) {
      next.add(sessionId);
    } else {
      next.delete(sessionId);
    }
    return next;
  });
}

// Helper: check if session is streaming
export function isStreaming(sessionId: string): boolean {
  return get(streamingSessionIds).has(sessionId);
}

/**
 * Process an incoming agent event from the Tauri event bus.
 * Appends to the event list and updates session state (tokens, status).
 */
export function processAgentEvent(payload: AgentEventPayload): void {
  const { session_id, event } = payload;

  // Append event
  agentEvents.update((map) => {
    const next = new Map(map);
    const events = next.get(session_id) ?? [];
    next.set(session_id, [...events, event]);
    return next;
  });

  // Handle event-type-specific side effects
  switch (event.event_type) {
    case 'text':
      setStreaming(session_id, true);
      break;
    case 'usage':
      if (event.metadata.input_tokens != null && event.metadata.output_tokens != null) {
        updateTokens(session_id, event.metadata.input_tokens, event.metadata.output_tokens);
      }
      break;
    case 'done':
      setStreaming(session_id, false);
      break;
    case 'error':
      setStreaming(session_id, false);
      if (event.metadata.error_severity === 'fatal') {
        updateSessionStatus(session_id, { error: { severity: 'fatal' } });
      }
      break;
    case 'metrics':
      if (event.metadata.stream_metrics) {
        updateMetrics(
          session_id,
          event.metadata.stream_metrics.tokens_per_second,
          event.metadata.stream_metrics.elapsed_ms,
        );
      }
      break;
  }
}
