import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
  agentEvents,
  streamingSessionIds,
  processAgentEvent,
  setSessionEvents,
  setStreaming,
} from '$lib/stores/agent-events';

/** Get events for a session (helper since getSessionEvents was removed) */
function getSessionEvents(sessionId: string) {
  return get(agentEvents).get(sessionId) ?? [];
}

/** Check if session is streaming (helper since isStreaming was removed) */
function isStreaming(sessionId: string) {
  return get(streamingSessionIds).has(sessionId);
}
import type { AgentEvent, AgentEventPayload } from '$lib/types/agent-event';

/** Build a minimal AgentEvent for testing. */
function makeEvent(overrides: Partial<AgentEvent> = {}): AgentEvent {
  return {
    id: crypto.randomUUID(),
    parent_id: null,
    event_type: 'text',
    content: { type: 'text', value: 'hello' },
    timestamp: Date.now(),
    metadata: {
      session_id: null,
      input_tokens: null,
      output_tokens: null,
      tool_name: null,
      model: null,
      provider: 'claude',
      error_severity: null,
      error_code: null,
      stream_metrics: null,
    },
    ...overrides,
  };
}

describe('agent-events store', () => {
  beforeEach(() => {
    agentEvents.set(new Map());
    streamingSessionIds.set(new Set());
  });

  // ─── Basic store operations ───────────────────────────────

  it('starts empty', () => {
    expect(get(agentEvents).size).toBe(0);
    expect(get(streamingSessionIds).size).toBe(0);
  });

  it('setSessionEvents populates events for a session', () => {
    const events = [makeEvent(), makeEvent()];
    setSessionEvents('s1', events);
    expect(getSessionEvents('s1')).toHaveLength(2);
  });

  it('setSessionEvents with empty array clears events for a session', () => {
    setSessionEvents('s1', [makeEvent()]);
    setSessionEvents('s1', []);
    expect(getSessionEvents('s1')).toHaveLength(0);
  });

  it('setStreaming toggles streaming state', () => {
    setStreaming('s1', true);
    expect(isStreaming('s1')).toBe(true);
    setStreaming('s1', false);
    expect(isStreaming('s1')).toBe(false);
  });

  // ─── processAgentEvent — the critical path ───────────────

  it('processAgentEvent appends event to session', () => {
    const event = makeEvent({ event_type: 'text' });
    processAgentEvent({ session_id: 's1', event });
    expect(getSessionEvents('s1')).toHaveLength(1);
  });

  it('processAgentEvent creates new Map reference (Svelte 5 reactivity)', () => {
    const mapBefore = get(agentEvents);
    processAgentEvent({ session_id: 's1', event: makeEvent() });
    const mapAfter = get(agentEvents);
    expect(mapAfter).not.toBe(mapBefore);
  });

  it('processAgentEvent creates new array reference (Svelte 5 reactivity)', () => {
    // Prime the store with one event
    processAgentEvent({ session_id: 's1', event: makeEvent() });
    const arrayBefore = get(agentEvents).get('s1');

    // Add second event
    processAgentEvent({ session_id: 's1', event: makeEvent() });
    const arrayAfter = get(agentEvents).get('s1');

    // Array reference MUST differ for $derived to detect the change
    expect(arrayAfter).not.toBe(arrayBefore);
    expect(arrayAfter).toHaveLength(2);
  });

  // ─── Simulate full chat flow ──────────────────────────────

  it('full chat flow: user msg → agent text → usage → done', () => {
    const sessionId = 'chat-session-1';

    // 1. User sends message (synthetic event added by ChatView)
    const userEvent = makeEvent({
      event_type: 'text',
      content: { type: 'text', value: 'ciao' },
      metadata: {
        session_id: sessionId,
        input_tokens: null,
        output_tokens: null,
        tool_name: null,
        model: null,
        provider: 'user',
        error_severity: null,
        error_code: null,
        stream_metrics: null,
      },
    });
    // ChatView adds user events with new Map + spread
    agentEvents.update((map) => {
      const next = new Map(map);
      next.set(sessionId, [userEvent]);
      return next;
    });
    setStreaming(sessionId, true);

    // 2. Backend emits text event (via processAgentEvent)
    const agentText = makeEvent({
      event_type: 'text',
      content: { type: 'text', value: 'Ciao! Come posso aiutarti?' },
    });
    processAgentEvent({ session_id: sessionId, event: agentText });

    // Verify: 2 events (user + agent), streaming is ON
    expect(getSessionEvents(sessionId)).toHaveLength(2);
    expect(isStreaming(sessionId)).toBe(true);

    // 3. Backend emits usage event
    const usageEvent = makeEvent({
      event_type: 'usage',
      metadata: {
        session_id: sessionId,
        input_tokens: 10,
        output_tokens: 15,
        tool_name: null,
        model: null,
        provider: 'claude',
        error_severity: null,
        error_code: null,
        stream_metrics: null,
      },
    });
    processAgentEvent({ session_id: sessionId, event: usageEvent });

    // 4. Backend emits done event
    const doneEvent = makeEvent({ event_type: 'done' });
    processAgentEvent({ session_id: sessionId, event: doneEvent });

    // Final state: 4 events, streaming OFF
    const events = getSessionEvents(sessionId);
    expect(events).toHaveLength(4);
    expect(events[0].metadata.provider).toBe('user');
    expect(events[1].event_type).toBe('text');
    expect(events[2].event_type).toBe('usage');
    expect(events[3].event_type).toBe('done');
    expect(isStreaming(sessionId)).toBe(false);
  });

  it('error event stops streaming', () => {
    const sessionId = 'err-session';
    setStreaming(sessionId, true);

    processAgentEvent({
      session_id: sessionId,
      event: makeEvent({
        event_type: 'error',
        content: { type: 'text', value: 'Rate limit exceeded' },
        metadata: {
          session_id: sessionId,
          input_tokens: null,
          output_tokens: null,
          tool_name: null,
          model: null,
          provider: 'claude',
          error_severity: 'fatal',
          error_code: 'rate_limit',
          stream_metrics: null,
        },
      }),
    });

    expect(isStreaming(sessionId)).toBe(false);
  });

  // ─── Reactivity under rapid updates ───────────────────────

  it('10 rapid events all produce distinct Map references', () => {
    const sessionId = 'rapid-session';
    const refs: Map<string, AgentEvent[]>[] = [];

    // Subscribe to capture every store update
    const unsub = agentEvents.subscribe((map) => {
      refs.push(map);
    });

    for (let i = 0; i < 10; i++) {
      processAgentEvent({
        session_id: sessionId,
        event: makeEvent({
          content: { type: 'text', value: `msg-${i}` },
        }),
      });
    }

    unsub();

    // refs[0] is the initial empty Map, then 10 updates
    expect(refs).toHaveLength(11);

    // Every update must have a unique Map reference
    for (let i = 1; i < refs.length; i++) {
      expect(refs[i]).not.toBe(refs[i - 1]);
    }

    // Final events count
    expect(getSessionEvents(sessionId)).toHaveLength(10);
  });

  // ─── Multiple concurrent sessions ────────────────────────

  it('events for different sessions are isolated', () => {
    processAgentEvent({ session_id: 's1', event: makeEvent() });
    processAgentEvent({ session_id: 's1', event: makeEvent() });
    processAgentEvent({ session_id: 's2', event: makeEvent() });

    expect(getSessionEvents('s1')).toHaveLength(2);
    expect(getSessionEvents('s2')).toHaveLength(1);
    expect(getSessionEvents('s3')).toHaveLength(0);
  });

  // ─── Pruning ─────────────────────────────────────────────

  it('prunes events over MAX limit', () => {
    const sessionId = 'prune-session';

    // Push 5010 events (limit is 5000)
    for (let i = 0; i < 5010; i++) {
      processAgentEvent({
        session_id: sessionId,
        event: makeEvent({ content: { type: 'text', value: `evt-${i}` } }),
      });
    }

    const events = getSessionEvents(sessionId);
    expect(events.length).toBeLessThanOrEqual(5000);
    // Most recent event should be the last one pushed
    const last = events[events.length - 1];
    expect(last.content).toEqual({ type: 'text', value: 'evt-5009' });
  });
});
