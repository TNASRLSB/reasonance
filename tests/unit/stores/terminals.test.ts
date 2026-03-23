import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
  terminalInstances,
  activeInstanceId,
  activeInstance,
  computedLabels,
  addInstance,
  removeInstance,
  updateInstance,
} from '$lib/stores/terminals';
import type { TerminalInstance } from '$lib/stores/terminals';

const makeInstance = (id: string, provider: string, label: string): TerminalInstance => ({
  id,
  provider,
  label,
});

describe('terminals store', () => {
  beforeEach(() => {
    terminalInstances.set([]);
    activeInstanceId.set(null);
  });

  it('starts empty', () => {
    expect(get(terminalInstances)).toEqual([]);
    expect(get(activeInstanceId)).toBeNull();
  });

  it('can add a terminal instance', () => {
    const inst = makeInstance('pty-1', 'claude', 'inst. 1');
    addInstance(inst);

    const instances = get(terminalInstances);
    expect(instances).toHaveLength(1);
    expect(instances[0].provider).toBe('claude');
    expect(instances[0].id).toBe('pty-1');
  });

  it('can track active instance', () => {
    activeInstanceId.set('pty-1');
    expect(get(activeInstanceId)).toBe('pty-1');
  });

  it('activeInstance is null when no active id', () => {
    terminalInstances.set([makeInstance('pty-1', 'claude', 'inst. 1')]);
    activeInstanceId.set(null);
    expect(get(activeInstance)).toBeNull();
  });

  it('activeInstance resolves to correct instance', () => {
    terminalInstances.set([
      makeInstance('pty-1', 'claude', 'inst. 1'),
      makeInstance('pty-2', 'gemini', 'inst. 1'),
    ]);
    activeInstanceId.set('pty-2');
    expect(get(activeInstance)?.provider).toBe('gemini');
  });

  it('can add multiple instances for different providers', () => {
    terminalInstances.set([
      makeInstance('pty-1', 'claude', 'inst. 1'),
      makeInstance('pty-2', 'gemini', 'inst. 1'),
    ]);

    const instances = get(terminalInstances);
    expect(instances).toHaveLength(2);
    expect(instances.map((i) => i.provider)).toEqual(['claude', 'gemini']);
  });

  it('can add multiple instances for the same provider', () => {
    terminalInstances.set([
      makeInstance('pty-1', 'claude', 'inst. 1'),
      makeInstance('pty-2', 'claude', 'inst. 2'),
    ]);

    const instances = get(terminalInstances);
    expect(instances).toHaveLength(2);
    expect(instances[0].id).toBe('pty-1');
    expect(instances[1].id).toBe('pty-2');
  });

  it('instance supports optional metadata fields', () => {
    const instance: TerminalInstance = {
      id: 'pty-1',
      provider: 'claude',
      label: 'inst. 1',
      contextPercent: 42,
      tokenCount: '12k',
      activeMode: 'code',
      modelName: 'claude-opus-4',
      messagesLeft: 8,
      resetTimer: '2h 30m',
      progressState: 1,
      progressValue: 65,
    };

    terminalInstances.set([instance]);

    const saved = get(terminalInstances)[0];
    expect(saved.contextPercent).toBe(42);
    expect(saved.tokenCount).toBe('12k');
    expect(saved.modelName).toBe('claude-opus-4');
    expect(saved.progressState).toBe(1);
    expect(saved.progressValue).toBe(65);
  });

  it('can remove an instance by id', () => {
    terminalInstances.set([
      makeInstance('pty-1', 'claude', 'inst. 1'),
      makeInstance('pty-2', 'gemini', 'inst. 1'),
    ]);

    removeInstance('pty-1');

    const instances = get(terminalInstances);
    expect(instances).toHaveLength(1);
    expect(instances[0].id).toBe('pty-2');
  });

  it('can update an instance by id', () => {
    terminalInstances.set([makeInstance('pty-1', 'claude', 'inst. 1')]);

    updateInstance('pty-1', { contextPercent: 75, tokenCount: '20k' });

    const inst = get(terminalInstances)[0];
    expect(inst.contextPercent).toBe(75);
    expect(inst.tokenCount).toBe('20k');
    expect(inst.provider).toBe('claude'); // unchanged field preserved
  });

  it('clearing active instance id resets to null', () => {
    activeInstanceId.set('pty-1');
    activeInstanceId.set(null);
    expect(get(activeInstanceId)).toBeNull();
  });

  it('computedLabels uses modelName when resolved', () => {
    terminalInstances.set([
      { id: 'pty-1', provider: 'claude', label: '', modelName: 'claude-opus-4' },
    ]);
    const labels = get(computedLabels);
    expect(labels.get('pty-1')).toBe('claude-opus-4');
  });

  it('computedLabels appends ... suffix when modelName not yet resolved', () => {
    terminalInstances.set([
      { id: 'pty-1', provider: 'claude', label: '' },
    ]);
    const labels = get(computedLabels);
    expect(labels.get('pty-1')).toBe('claude ...');
  });

  it('computedLabels deduplicates by appending index for same model', () => {
    terminalInstances.set([
      { id: 'pty-1', provider: 'claude', label: '', modelName: 'claude-opus-4' },
      { id: 'pty-2', provider: 'claude', label: '', modelName: 'claude-opus-4' },
    ]);
    const labels = get(computedLabels);
    expect(labels.get('pty-1')).toBe('claude-opus-4 1');
    expect(labels.get('pty-2')).toBe('claude-opus-4 2');
  });
});
