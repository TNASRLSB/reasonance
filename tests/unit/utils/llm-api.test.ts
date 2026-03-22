import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock @tauri-apps/api/core before importing the module under test
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue('test-api-key'),
}));

import { callLlm } from '$lib/utils/llm-api';
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

describe('callLlm', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    globalThis.fetch = vi.fn();
  });

  describe('Anthropic provider', () => {
    it('calls the correct Anthropic endpoint', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        json: async () => ({ content: [{ text: 'Hello from Claude' }] }),
      });

      await callLlm(anthropicConfig, 'Hello');

      expect(globalThis.fetch).toHaveBeenCalledWith(
        'https://api.anthropic.com/v1/messages',
        expect.objectContaining({ method: 'POST' })
      );
    });

    it('sends correct Anthropic headers', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        json: async () => ({ content: [{ text: 'Hi' }] }),
      });

      await callLlm(anthropicConfig, 'Hello');

      const [, options] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
      const headers = options.headers as Record<string, string>;
      expect(headers['Content-Type']).toBe('application/json');
      expect(headers['x-api-key']).toBe('test-api-key');
      expect(headers['anthropic-version']).toBe('2023-06-01');
    });

    it('sends correct Anthropic request body', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        json: async () => ({ content: [{ text: 'Response' }] }),
      });

      await callLlm(anthropicConfig, 'Test prompt');

      const [, options] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
      const body = JSON.parse(options.body);
      expect(body.model).toBe('claude-sonnet-4-6');
      expect(body.max_tokens).toBe(4096);
      expect(body.messages).toEqual([{ role: 'user', content: 'Test prompt' }]);
    });

    it('extracts content from Anthropic response', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        json: async () => ({ content: [{ text: 'Extracted text' }] }),
      });

      const result = await callLlm(anthropicConfig, 'Hello');
      expect(result.content).toBe('Extracted text');
      expect(result.error).toBeUndefined();
    });

    it('returns empty string when Anthropic response has no content', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        json: async () => ({}),
      });

      const result = await callLlm(anthropicConfig, 'Hello');
      expect(result.content).toBe('');
    });
  });

  describe('OpenAI provider', () => {
    it('calls the default OpenAI endpoint', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        json: async () => ({ choices: [{ message: { content: 'Hi from GPT' } }] }),
      });

      await callLlm(openaiConfig, 'Hello');

      expect(globalThis.fetch).toHaveBeenCalledWith(
        'https://api.openai.com/v1/chat/completions',
        expect.objectContaining({ method: 'POST' })
      );
    });

    it('uses custom endpoint when provided', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        json: async () => ({ choices: [{ message: { content: 'Hi' } }] }),
      });

      await callLlm(openaiCustomEndpointConfig, 'Hello');

      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:11434/v1/chat/completions',
        expect.anything()
      );
    });

    it('sends Bearer token in Authorization header', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        json: async () => ({ choices: [{ message: { content: 'Hi' } }] }),
      });

      await callLlm(openaiConfig, 'Hello');

      const [, options] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
      const headers = options.headers as Record<string, string>;
      expect(headers['Authorization']).toBe('Bearer test-api-key');
    });

    it('sends correct OpenAI request body', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        json: async () => ({ choices: [{ message: { content: 'Hi' } }] }),
      });

      await callLlm(openaiConfig, 'Test prompt');

      const [, options] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
      const body = JSON.parse(options.body);
      expect(body.model).toBe('gpt-4o');
      expect(body.messages).toEqual([{ role: 'user', content: 'Test prompt' }]);
    });

    it('extracts content from OpenAI response', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        json: async () => ({ choices: [{ message: { content: 'GPT says hi' } }] }),
      });

      const result = await callLlm(openaiConfig, 'Hello');
      expect(result.content).toBe('GPT says hi');
      expect(result.error).toBeUndefined();
    });

    it('returns empty string when OpenAI response has no choices', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        json: async () => ({}),
      });

      const result = await callLlm(openaiConfig, 'Hello');
      expect(result.content).toBe('');
    });
  });

  describe('error handling', () => {
    it('returns error object on non-OK HTTP response', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: false,
        status: 401,
        text: async () => 'Unauthorized',
      });

      const result = await callLlm(anthropicConfig, 'Hello');
      expect(result.content).toBe('');
      expect(result.error).toBe('401: Unauthorized');
    });

    it('returns error object when fetch throws', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockRejectedValueOnce(
        new Error('Network error')
      );

      const result = await callLlm(anthropicConfig, 'Hello');
      expect(result.content).toBe('');
      expect(result.error).toBe('Network error');
    });

    it('omits Authorization header when no API key env is set', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        json: async () => ({ choices: [{ message: { content: 'Hi' } }] }),
      });

      // Config with no apiKeyEnv — invoke returns '' from mock default
      const { invoke } = await import('@tauri-apps/api/core');
      (invoke as ReturnType<typeof vi.fn>).mockResolvedValueOnce(null);

      await callLlm(openaiCustomEndpointConfig, 'Hello');

      const [, options] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
      const headers = options.headers as Record<string, string>;
      expect(headers['Authorization']).toBeUndefined();
    });
  });
});
