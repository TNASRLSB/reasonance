// src/lib/data/model-info.ts
import type { ModelInfo } from '$lib/types/analytics';

export const MODEL_INFO: ModelInfo[] = [
  // Claude
  { id: 'claude-opus-4-6', provider: 'claude', name: 'Claude Opus 4.6', cost_per_1m_input: 15, cost_per_1m_output: 75, context_window: 200_000 },
  { id: 'claude-sonnet-4-6', provider: 'claude', name: 'Claude Sonnet 4.6', cost_per_1m_input: 3, cost_per_1m_output: 15, context_window: 200_000 },
  { id: 'claude-haiku-4-5', provider: 'claude', name: 'Claude Haiku 4.5', cost_per_1m_input: 0.25, cost_per_1m_output: 1.25, context_window: 200_000 },
  // Gemini
  { id: 'gemini-2.5-pro', provider: 'gemini', name: 'Gemini 2.5 Pro', cost_per_1m_input: 1.25, cost_per_1m_output: 10, context_window: 1_000_000 },
  { id: 'gemini-2.5-flash', provider: 'gemini', name: 'Gemini 2.5 Flash', cost_per_1m_input: 0.15, cost_per_1m_output: 0.60, context_window: 1_000_000 },
  // Qwen
  { id: 'qwen3-coder', provider: 'qwen', name: 'Qwen3 Coder', cost_per_1m_input: 0.16, cost_per_1m_output: 0.16, context_window: 128_000 },
  // Kimi
  { id: 'kimi-k2', provider: 'kimi', name: 'Kimi K2', cost_per_1m_input: 0.60, cost_per_1m_output: 2.40, context_window: 128_000 },
  // Codex
  { id: 'codex-mini', provider: 'codex', name: 'Codex Mini', cost_per_1m_input: 1.50, cost_per_1m_output: 6, context_window: 200_000 },
];

export function getModelsForProvider(provider: string): ModelInfo[] {
  return MODEL_INFO.filter(m => m.provider === provider);
}

export function getModelInfo(modelId: string): ModelInfo | undefined {
  return MODEL_INFO.find(m => m.id === modelId);
}

export function getCheapestModel(provider: string): ModelInfo | undefined {
  const models = getModelsForProvider(provider);
  return models.reduce((cheapest, m) =>
    !cheapest || m.cost_per_1m_input < cheapest.cost_per_1m_input ? m : cheapest,
    undefined as ModelInfo | undefined
  );
}
