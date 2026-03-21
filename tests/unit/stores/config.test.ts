import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { llmConfigs, appSettings } from '$lib/stores/config';
import type { LlmConfig, AppSettings } from '$lib/stores/config';

describe('config store', () => {
  beforeEach(() => {
    llmConfigs.set([]);
    appSettings.set({});
  });

  it('has empty default state', () => {
    expect(get(llmConfigs)).toEqual([]);
    expect(get(appSettings)).toEqual({});
  });

  it('can set llmConfigs array', () => {
    const configs: LlmConfig[] = [
      {
        name: 'claude',
        type: 'cli',
        command: 'claude',
        args: ['--dangerously-skip-permissions'],
        yoloFlag: '--yolo',
        imageMode: 'path',
      },
      {
        name: 'gemini',
        type: 'cli',
        command: 'gemini',
        imageMode: 'none',
      },
    ];

    llmConfigs.set(configs);

    const result = get(llmConfigs);
    expect(result).toHaveLength(2);
    expect(result[0].name).toBe('claude');
    expect(result[0].type).toBe('cli');
    expect(result[1].name).toBe('gemini');
  });

  it('can set a CLI LlmConfig with all optional fields', () => {
    const config: LlmConfig = {
      name: 'openai',
      type: 'api',
      provider: 'openai',
      apiKeyEnv: 'OPENAI_API_KEY',
      model: 'gpt-4o',
      endpoint: 'https://api.openai.com/v1',
      modes: [{ name: 'chat', label: 'Chat', description: 'Standard chat' }],
      slashCommands: [{ command: '/help', description: 'Show help' }],
    };

    llmConfigs.set([config]);

    const result = get(llmConfigs)[0];
    expect(result.provider).toBe('openai');
    expect(result.apiKeyEnv).toBe('OPENAI_API_KEY');
    expect(result.model).toBe('gpt-4o');
    expect(result.modes).toHaveLength(1);
    expect(result.slashCommands).toHaveLength(1);
  });

  it('can set appSettings with default LLM', () => {
    const settings: AppSettings = { default: 'claude' };
    appSettings.set(settings);

    expect(get(appSettings).default).toBe('claude');
  });

  it('can set appSettings with contextMenuLlm', () => {
    const settings: AppSettings = {
      default: 'claude',
      contextMenuLlm: 'gemini',
    };
    appSettings.set(settings);

    const result = get(appSettings);
    expect(result.default).toBe('claude');
    expect(result.contextMenuLlm).toBe('gemini');
  });

  it('can update appSettings independently from llmConfigs', () => {
    llmConfigs.set([{ name: 'claude', type: 'cli' }]);
    appSettings.set({ default: 'claude' });

    appSettings.update((s) => ({ ...s, contextMenuLlm: 'claude' }));

    expect(get(llmConfigs)).toHaveLength(1);
    expect(get(appSettings).contextMenuLlm).toBe('claude');
  });
});
