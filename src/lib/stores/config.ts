import { writable } from 'svelte/store';

export interface LlmConfig {
  name: string;
  type: 'cli' | 'api';
  command?: string;
  args?: string[];
  provider?: string;
  apiKeyEnv?: string;
  model?: string;
  endpoint?: string;
  yoloFlag?: string;
  imageMode?: 'path' | 'base64' | 'none';
}

export interface AppSettings {
  default?: string;
  contextMenuLlm?: string;
}

export const llmConfigs = writable<LlmConfig[]>([]);
export const appSettings = writable<AppSettings>({});
