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
import { setupTestProject, resetProjectState, TEST_PROJECT_ID } from '../../helpers/project-setup';

const makeInstance = (id: string, provider: string, label: string): TerminalInstance => ({
  id,
  provider,
  label,
  projectId: TEST_PROJECT_ID,
});

describe('terminals store', () => {
  beforeEach(() => {
    resetProjectState();
  });

  it('starts empty', () => {
    expect(get(terminalInstances)).toEqual([]);
    expect(get(activeInstanceId)).toBeNull();
  });

  it('can add a terminal instance', () => {
    setupTestProject();
    const inst = makeInstance('pty-1', 'claude', 'inst. 1');
    addInstance(inst);

    const instances = get(terminalInstances);
    expect(instances).toHaveLength(1);
    expect(instances[0].provider).toBe('claude');
    expect(instances[0].id).toBe('pty-1');
  });

  it('can track active instance', () => {
    setupTestProject({
      terminalInstances: [makeInstance('pty-1', 'claude', 'inst. 1')],
      activeTerminalId: 'pty-1',
    });
    expect(get(activeInstanceId)).toBe('pty-1');
  });

  it('activeInstance is null when no active id', () => {
    setupTestProject({
      terminalInstances: [makeInstance('pty-1', 'claude', 'inst. 1')],
      activeTerminalId: null,
    });
    expect(get(activeInstance)).toBeNull();
  });

  it('activeInstance resolves to correct instance', () => {
    setupTestProject({
      terminalInstances: [
        makeInstance('pty-1', 'claude', 'inst. 1'),
        makeInstance('pty-2', 'gemini', 'inst. 1'),
      ],
      activeTerminalId: 'pty-2',
    });
    expect(get(activeInstance)?.provider).toBe('gemini');
  });

  it('can add multiple instances for different providers', () => {
    setupTestProject({
      terminalInstances: [
        makeInstance('pty-1', 'claude', 'inst. 1'),
        makeInstance('pty-2', 'gemini', 'inst. 1'),
      ],
    });

    const instances = get(terminalInstances);
    expect(instances).toHaveLength(2);
    expect(instances.map((i) => i.provider)).toEqual(['claude', 'gemini']);
  });

  it('can add multiple instances for the same provider', () => {
    setupTestProject({
      terminalInstances: [
        makeInstance('pty-1', 'claude', 'inst. 1'),
        makeInstance('pty-2', 'claude', 'inst. 2'),
      ],
    });

    const instances = get(terminalInstances);
    expect(instances).toHaveLength(2);
    expect(instances[0].id).toBe('pty-1');
    expect(instances[1].id).toBe('pty-2');
  });

  it('instance supports optional metadata fields', () => {
    setupTestProject({
      terminalInstances: [{
        id: 'pty-1',
        provider: 'claude',
        label: 'inst. 1',
        projectId: TEST_PROJECT_ID,
        contextPercent: 42,
        tokenCount: '12k',
        activeMode: 'code',
        modelName: 'claude-opus-4',
        messagesLeft: 8,
        resetTimer: '2h 30m',
        progressState: 1,
        progressValue: 65,
      }],
    });

    const saved = get(terminalInstances)[0];
    expect(saved.contextPercent).toBe(42);
    expect(saved.tokenCount).toBe('12k');
    expect(saved.modelName).toBe('claude-opus-4');
    expect(saved.progressState).toBe(1);
    expect(saved.progressValue).toBe(65);
  });

  it('can remove an instance by id', () => {
    setupTestProject({
      terminalInstances: [
        makeInstance('pty-1', 'claude', 'inst. 1'),
        makeInstance('pty-2', 'gemini', 'inst. 1'),
      ],
    });

    removeInstance('pty-1');

    const instances = get(terminalInstances);
    expect(instances).toHaveLength(1);
    expect(instances[0].id).toBe('pty-2');
  });

  it('can update an instance by id', () => {
    setupTestProject({
      terminalInstances: [makeInstance('pty-1', 'claude', 'inst. 1')],
    });

    updateInstance('pty-1', { contextPercent: 75, tokenCount: '20k' });

    const inst = get(terminalInstances)[0];
    expect(inst.contextPercent).toBe(75);
    expect(inst.tokenCount).toBe('20k');
    expect(inst.provider).toBe('claude'); // unchanged field preserved
  });

  it('clearing active instance id resets to null', () => {
    setupTestProject({
      terminalInstances: [makeInstance('pty-1', 'claude', 'inst. 1')],
      activeTerminalId: 'pty-1',
    });
    expect(get(activeInstanceId)).toBe('pty-1');

    // Remove the instance — activeTerminalId should become null
    removeInstance('pty-1');
    expect(get(activeInstanceId)).toBeNull();
  });

  it('computedLabels uses modelName when resolved', () => {
    setupTestProject({
      terminalInstances: [
        { id: 'pty-1', provider: 'claude', label: '', modelName: 'claude-opus-4', projectId: TEST_PROJECT_ID },
      ],
    });
    const labels = get(computedLabels);
    expect(labels.get('pty-1')).toBe('claude-opus-4');
  });

  it('computedLabels appends ... suffix when modelName not yet resolved', () => {
    setupTestProject({
      terminalInstances: [
        { id: 'pty-1', provider: 'claude', label: '', projectId: TEST_PROJECT_ID },
      ],
    });
    const labels = get(computedLabels);
    expect(labels.get('pty-1')).toBe('claude ...');
  });

  it('computedLabels deduplicates by appending index for same model', () => {
    setupTestProject({
      terminalInstances: [
        { id: 'pty-1', provider: 'claude', label: '', modelName: 'claude-opus-4', projectId: TEST_PROJECT_ID },
        { id: 'pty-2', provider: 'claude', label: '', modelName: 'claude-opus-4', projectId: TEST_PROJECT_ID },
      ],
    });
    const labels = get(computedLabels);
    expect(labels.get('pty-1')).toBe('claude-opus-4 1');
    expect(labels.get('pty-2')).toBe('claude-opus-4 2');
  });
});
