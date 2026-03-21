import { invoke } from '@tauri-apps/api/core';
import type { LlmConfig } from '$lib/stores/config';

interface LlmResponse {
  content: string;
  error?: string;
}

async function getApiKey(envVar?: string): Promise<string> {
  if (!envVar) return '';
  return (await invoke<string | null>('get_env_var', { name: envVar })) ?? '';
}

export async function callLlm(config: LlmConfig, prompt: string): Promise<LlmResponse> {
  const endpoint = getEndpoint(config);
  const headers = await getHeaders(config);
  const body = getBody(config, prompt);

  try {
    const res = await fetch(endpoint, { method: 'POST', headers, body: JSON.stringify(body) });
    if (!res.ok) {
      const text = await res.text();
      return { content: '', error: `${res.status}: ${text}` };
    }
    const data = await res.json();
    return { content: extractContent(config, data) };
  } catch (e: any) {
    return { content: '', error: e.message };
  }
}

function getEndpoint(config: LlmConfig): string {
  if (config.provider === 'anthropic') return 'https://api.anthropic.com/v1/messages';
  const base = config.endpoint ?? 'https://api.openai.com/v1';
  return `${base}/chat/completions`;
}

async function getHeaders(config: LlmConfig): Promise<Record<string, string>> {
  const apiKey = await getApiKey(config.apiKeyEnv);
  if (config.provider === 'anthropic') {
    return { 'Content-Type': 'application/json', 'x-api-key': apiKey, 'anthropic-version': '2023-06-01' };
  }
  return { 'Content-Type': 'application/json', ...(apiKey ? { Authorization: `Bearer ${apiKey}` } : {}) };
}

function getBody(config: LlmConfig, prompt: string): any {
  if (config.provider === 'anthropic') {
    return { model: config.model ?? 'claude-sonnet-4-6', max_tokens: 4096, messages: [{ role: 'user', content: prompt }] };
  }
  return { model: config.model ?? 'gpt-4o', messages: [{ role: 'user', content: prompt }] };
}

function extractContent(config: LlmConfig, data: any): string {
  if (config.provider === 'anthropic') return data.content?.[0]?.text ?? '';
  return data.choices?.[0]?.message?.content ?? '';
}
