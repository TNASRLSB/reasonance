/**
 * config-bootstrap.ts — TOML config loading and LLM auto-discovery for
 * Reasonance IDE.
 *
 * loadInitialConfig  — Reads forge.toml, parses LLM entries and app settings.
 * discoverAndApplyLlms — Auto-discovers installed LLM CLIs when no config exists.
 */

import { get } from 'svelte/store';
import { parse } from 'smol-toml';
import { llmConfigs, appSettings } from '$lib/stores/config';
import { showToast } from '$lib/stores/toast';
import { parseLlmConfig } from '$lib/utils/config-parser';
import { t } from '$lib/i18n/index';
import type { Adapter } from '$lib/adapter/index';
import type { LlmConfig } from '$lib/stores/config';

// ── Config loading ────────────────────────────────────────────────────────────

export async function loadInitialConfig(adapter: Adapter): Promise<void> {
  try {
    const raw = await adapter.readConfig();
    if (!raw || !raw.trim()) return;

    const parsed = parse(raw) as {
      settings?: { default?: string; context_menu_llm?: string };
      llm?: Array<Record<string, unknown>>;
    };

    const rawLlms = parsed.llm ?? [];
    llmConfigs.set(parseLlmConfig(rawLlms));

    const s = parsed.settings ?? {};
    appSettings.set({
      default: s.default !== undefined ? String(s.default) : undefined,
      contextMenuLlm:
        s.context_menu_llm !== undefined ? String(s.context_menu_llm) : undefined,
    });
  } catch (err) {
    // Config load failures are non-fatal — continue with defaults
    showToast('error', 'Config parse error', String(err));
  }
}

// ── LLM auto-discovery ────────────────────────────────────────────────────────

/**
 * Discovers installed LLM CLIs and writes them to both the in-memory store
 * and the TOML config file. Only runs when no LLMs are currently configured.
 */
export async function discoverAndApplyLlms(adapter: Adapter): Promise<void> {
  const configs = get(llmConfigs);
  if (configs.length > 0) return;

  try {
    const discovered = await adapter.discoverLlms();
    const found = discovered.filter((d) => d.found);
    if (found.length === 0) return;

    const newConfigs: LlmConfig[] = found.map((d) => ({
      name: d.name,
      type: 'cli' as const,
      command: d.command,
      args: [],
      yoloFlag: d.command === 'claude' ? '--dangerously-skip-permissions' : undefined,
      imageMode: 'path' as const,
    }));

    // If Ollama was found, also add it as an API-type LLM (OpenAI-compatible)
    if (found.some((d) => d.command === 'ollama')) {
      newConfigs.push({
        name: 'Ollama (API)',
        type: 'api',
        provider: 'openai',
        endpoint: 'http://localhost:11434/v1',
        model: 'llama3',
        imageMode: 'none',
      });
    }

    llmConfigs.set(newConfigs);

    // Persist discovered LLMs to TOML so Settings can find them
    try {
      const { stringify } = await import('smol-toml');
      const tomlObj: Record<string, unknown> = {
        settings: { default: '', context_menu_llm: '' },
        llm: newConfigs.map((l) => {
          const entry: Record<string, unknown> = { name: l.name, type: l.type };
          if (l.type === 'cli') {
            entry.command = l.command ?? '';
            entry.args = l.args ?? [];
            if (l.yoloFlag) entry.yolo_flag = l.yoloFlag;
            entry.image_mode = l.imageMode ?? 'path';
          } else {
            entry.provider = l.provider ?? '';
            entry.model = l.model ?? '';
            if (l.endpoint) entry.endpoint = l.endpoint;
          }
          return entry;
        }),
      };
      await adapter.writeConfig(stringify(tomlObj));
    } catch { /* non-fatal */ }

    showToast(
      'success',
      t('toast.llmFound'),
      t('toast.detected', { names: found.map((d) => d.name).join(', ') })
    );
  } catch {
    // Silently ignore discovery errors
  }
}
