import { invoke } from '@tauri-apps/api/core';
import { z } from 'zod';
import type { LlmConfig } from '$lib/stores/config';

interface LlmResponse {
  content: string;
  error?: string;
}

export async function callLlm(config: LlmConfig, prompt: string): Promise<LlmResponse> {
  try {
    const result = await invoke<string>('call_llm_api', {
      provider: config.provider ?? 'anthropic',
      model: config.model ?? '',
      prompt,
      apiKeyEnv: config.apiKeyEnv ?? '',
      endpoint: config.endpoint ?? '',
    });
    const validated = z.string().parse(result);
    return JSON.parse(validated);
  } catch (e: any) {
    return { content: '', error: String(e) };
  }
}
