import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock @tauri-apps/api/core before importing the module under test
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { callLlm } from '$lib/utils/llm-api';
import { invoke } from '@tauri-apps/api/core';
import type { LlmConfig } from '$lib/stores/config';

const anthropicConfig: LlmConfig = {
  name: 'claude',
  type: 'api',
  provider: 'anthropic',
  apiKeyEnv: 'ANTHROPIC_API_KEY',
  model: 'claude-sonnet-4-6',
};

const openaiConfig: LlmConfig = {
  name: 'gpt',
  type: 'api',
  provider: 'openai',
  apiKeyEnv: 'OPENAI_API_KEY',
  model: 'gpt-4o',
};

const openaiCustomEndpointConfig: LlmConfig = {
  name: 'ollama',
  type: 'api',
  provider: 'openai',
  endpoint: 'http://localhost:11434/v1',
  model: 'llama3',
};

const mockedInvoke = invoke as ReturnType<typeof vi.fn>;

describe('callLlm', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Anthropic provider', () => {
    it('calls invoke with correct provider and model', async () => {
      mockedInvoke.mockResolvedValueOnce(
        JSON.stringify({ content: 'Hello from Claude', error: null })
      );

      await callLlm(anthropicConfig, 'Hello');

      expect(mockedInvoke).toHaveBeenCalledWith('call_llm_api', {
        provider: 'anthropic',
        model: 'claude-sonnet-4-6',
        prompt: 'Hello',
        apiKeyEnv: 'ANTHROPIC_API_KEY',
        endpoint: '',
      });
    });

    it('extracts content from response', async () => {
      mockedInvoke.mockResolvedValueOnce(
        JSON.stringify({ content: 'Extracted text', error: null })
      );

      const result = await callLlm(anthropicConfig, 'Hello');
      expect(result.content).toBe('Extracted text');
      expect(result.error).toBeFalsy();
    });

    it('returns empty string when content is null', async () => {
      mockedInvoke.mockResolvedValueOnce(
        JSON.stringify({ content: null, error: null })
      );

      const result = await callLlm(anthropicConfig, 'Hello');
      expect(result.content).toBeNull();
    });
  });

  describe('OpenAI provider', () => {
    it('passes openai as provider', async () => {
      mockedInvoke.mockResolvedValueOnce(
        JSON.stringify({ content: 'Hi from GPT', error: null })
      );

      await callLlm(openaiConfig, 'Hello');

      expect(mockedInvoke).toHaveBeenCalledWith('call_llm_api', {
        provider: 'openai',
        model: 'gpt-4o',
        prompt: 'Hello',
        apiKeyEnv: 'OPENAI_API_KEY',
        endpoint: '',
      });
    });

    it('passes custom endpoint when provided', async () => {
      mockedInvoke.mockResolvedValueOnce(
        JSON.stringify({ content: 'Hi', error: null })
      );

      await callLlm(openaiCustomEndpointConfig, 'Hello');

      expect(mockedInvoke).toHaveBeenCalledWith('call_llm_api', {
        provider: 'openai',
        model: 'llama3',
        prompt: 'Hello',
        apiKeyEnv: '',
        endpoint: 'http://localhost:11434/v1',
      });
    });

    it('extracts content from response', async () => {
      mockedInvoke.mockResolvedValueOnce(
        JSON.stringify({ content: 'GPT says hi', error: null })
      );

      const result = await callLlm(openaiConfig, 'Hello');
      expect(result.content).toBe('GPT says hi');
      expect(result.error).toBeFalsy();
    });
  });

  describe('error handling', () => {
    it('returns error from backend response', async () => {
      mockedInvoke.mockResolvedValueOnce(
        JSON.stringify({ content: null, error: '401: Unauthorized' })
      );

      const result = await callLlm(anthropicConfig, 'Hello');
      expect(result.error).toBe('401: Unauthorized');
    });

    it('returns error when invoke throws', async () => {
      mockedInvoke.mockRejectedValueOnce(new Error('Network error'));

      const result = await callLlm(anthropicConfig, 'Hello');
      expect(result.content).toBe('');
      expect(result.error).toBe('Error: Network error');
    });

    it('passes empty apiKeyEnv when not set in config', async () => {
      mockedInvoke.mockResolvedValueOnce(
        JSON.stringify({ content: 'Hi', error: null })
      );

      await callLlm(openaiCustomEndpointConfig, 'Hello');

      const call = mockedInvoke.mock.calls[0];
      expect(call[1].apiKeyEnv).toBe('');
    });
  });
});
