/**
 * config-bootstrap.ts — TOML config loading and LLM auto-discovery for
 * Reasonance IDE.
 *
 * loadInitialConfig  — Reads forge.toml, parses LLM entries and app settings.
 * discoverAndApplyLlms — Auto-discovers installed LLM CLIs and merges new ones.
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
 * Discovers installed LLM CLIs and merges newly found ones into the
 * existing config. Runs on every startup to pick up newly installed CLIs.
 */
export async function discoverAndApplyLlms(adapter: Adapter): Promise<void> {
  try {
    const discovered = await adapter.discoverLlms();
    const found = discovered.filter((d) => d.found);
    if (found.length === 0) return;

    const existing = get(llmConfigs);
    // Index existing configs by command (CLI) or name (API) for fast lookup
    const existingCommands = new Set(existing.filter((c) => c.command).map((c) => c.command));
    const existingNames = new Set(existing.map((c) => c.name));

    // Build list of newly discovered CLIs not already in config
    const brandNew: LlmConfig[] = found
      .filter((d) => !existingCommands.has(d.command) && !existingNames.has(d.name))
      .map((d) => {
        const isClaude = d.command.endsWith('/claude') || d.command === 'claude';
        return {
          name: d.name,
          type: 'cli' as const,
          command: d.command,
          // Claude needs --dangerously-skip-permissions by default to work in embedded terminal
          args: isClaude ? ['--dangerously-skip-permissions'] : [],
          imageMode: 'path' as const,
        };
      });

    // If Ollama was found and no API entry exists yet, add it
    if (
      found.some((d) => d.command === 'ollama') &&
      !existingNames.has('Ollama (API)')
    ) {
      brandNew.push({
        name: 'Ollama (API)',
        type: 'api',
        provider: 'openai',
        endpoint: 'http://localhost:11434/v1',
        model: 'llama3',
        imageMode: 'none',
      });
    }

    // Ensure existing Claude entries always have --dangerously-skip-permissions
    // (needed for embedded terminal — Claude prompts for permissions otherwise)
    let patched = false;
    const updated = existing.map((c) => {
      if (c.type !== 'cli') return c;
      const cmd = c.command ?? '';
      const isClaude = cmd.endsWith('/claude') || cmd === 'claude';
      if (!isClaude) return c;
      const args = c.args ?? [];
      if (args.includes('--dangerously-skip-permissions')) return c;
      patched = true;
      return { ...c, args: ['--dangerously-skip-permissions', ...args] };
    });

    if (brandNew.length === 0 && !patched) return;

    const merged = [...updated, ...brandNew];
    llmConfigs.set(merged);

    // Persist merged config to TOML
    try {
      const { stringify } = await import('smol-toml');
      const tomlObj: Record<string, unknown> = {
        settings: {
          default: get(appSettings).default ?? '',
          context_menu_llm: get(appSettings).contextMenuLlm ?? '',
        },
        llm: merged.map((l) => {
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
      t('toast.detected', { names: brandNew.map((d) => d.name).join(', ') })
    );
  } catch {
    // Silently ignore discovery errors
  }
}
