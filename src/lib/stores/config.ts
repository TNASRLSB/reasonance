import { writable } from 'svelte/store';

export interface LlmMode {
  name: string;
  label: string;
  description: string;
}

export interface SlashCommand {
  command: string;
  description: string;
}

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
  modes?: LlmMode[];
  slashCommands?: SlashCommand[];
}

export interface AppSettings {
  default?: string;
  contextMenuLlm?: string;
}

export const llmConfigs = writable<LlmConfig[]>([]);
export const appSettings = writable<AppSettings>({});
