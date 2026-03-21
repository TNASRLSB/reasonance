import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
  terminalTabs,
  activeTerminalTab,
  activeInstanceId,
} from '$lib/stores/terminals';
import type { TerminalTab, TerminalInstance } from '$lib/stores/terminals';

const makeInstance = (id: string, llmName: string, label: string): TerminalInstance => ({
  id,
  llmName,
  label,
});

describe('terminals store', () => {
  beforeEach(() => {
    terminalTabs.set([]);
    activeTerminalTab.set(null);
    activeInstanceId.set(null);
  });

  it('starts empty', () => {
    expect(get(terminalTabs)).toEqual([]);
    expect(get(activeTerminalTab)).toBeNull();
    expect(get(activeInstanceId)).toBeNull();
  });

  it('can add a terminal tab', () => {
    const tab: TerminalTab = {
      llmName: 'claude',
      instances: [makeInstance('pty-1', 'claude', 'inst. 1')],
    };

    terminalTabs.update((tabs) => [...tabs, tab]);

    const tabs = get(terminalTabs);
    expect(tabs).toHaveLength(1);
    expect(tabs[0].llmName).toBe('claude');
    expect(tabs[0].instances).toHaveLength(1);
  });

  it('can track active tab', () => {
    activeTerminalTab.set('claude');
    expect(get(activeTerminalTab)).toBe('claude');
  });

  it('can track active instance', () => {
    activeInstanceId.set('pty-1');
    expect(get(activeInstanceId)).toBe('pty-1');
  });

  it('can add multiple instances to a tab', () => {
    const tab: TerminalTab = {
      llmName: 'claude',
      instances: [
        makeInstance('pty-1', 'claude', 'inst. 1'),
        makeInstance('pty-2', 'claude', 'inst. 2'),
      ],
    };

    terminalTabs.set([tab]);

    const tabs = get(terminalTabs);
    expect(tabs[0].instances).toHaveLength(2);
    expect(tabs[0].instances[0].id).toBe('pty-1');
    expect(tabs[0].instances[1].id).toBe('pty-2');
  });

  it('can add multiple tabs for different LLMs', () => {
    terminalTabs.set([
      { llmName: 'claude', instances: [makeInstance('pty-1', 'claude', 'inst. 1')] },
      { llmName: 'gemini', instances: [makeInstance('pty-2', 'gemini', 'inst. 1')] },
    ]);

    const tabs = get(terminalTabs);
    expect(tabs).toHaveLength(2);
    expect(tabs.map((t) => t.llmName)).toEqual(['claude', 'gemini']);
  });

  it('instance supports optional metadata fields', () => {
    const instance: TerminalInstance = {
      id: 'pty-1',
      llmName: 'claude',
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

    terminalTabs.set([{ llmName: 'claude', instances: [instance] }]);

    const saved = get(terminalTabs)[0].instances[0];
    expect(saved.contextPercent).toBe(42);
    expect(saved.tokenCount).toBe('12k');
    expect(saved.modelName).toBe('claude-opus-4');
    expect(saved.progressState).toBe(1);
    expect(saved.progressValue).toBe(65);
  });

  it('can remove a tab', () => {
    terminalTabs.set([
      { llmName: 'claude', instances: [] },
      { llmName: 'gemini', instances: [] },
    ]);

    terminalTabs.update((tabs) => tabs.filter((t) => t.llmName !== 'claude'));

    const tabs = get(terminalTabs);
    expect(tabs).toHaveLength(1);
    expect(tabs[0].llmName).toBe('gemini');
  });

  it('clearing active tab and instance resets to null', () => {
    activeTerminalTab.set('claude');
    activeInstanceId.set('pty-1');

    activeTerminalTab.set(null);
    activeInstanceId.set(null);

    expect(get(activeTerminalTab)).toBeNull();
    expect(get(activeInstanceId)).toBeNull();
  });
});
