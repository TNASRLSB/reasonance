/**
 * config-bootstrap.ts — TOML config loading and LLM auto-discovery for
 * Reasonance IDE.
 *
 * loadInitialConfig  — Reads llms.toml, parses LLM entries and app settings.
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
      settings?: {
        default?: string;
        context_menu_llm?: string;
        default_permission_level?: string;
        keybindings?: { cycle_permission?: string };
      };
      llm?: Array<Record<string, unknown>>;
    };

    const rawLlms = parsed.llm ?? [];
    llmConfigs.set(parseLlmConfig(rawLlms));

    const s = parsed.settings ?? {};
    const dpl = String(s.default_permission_level ?? '');
    const validDpl = ['yolo', 'ask', 'locked'].includes(dpl) ? dpl : undefined;
    appSettings.set({
      default: s.default !== undefined ? String(s.default) : undefined,
      contextMenuLlm:
        s.context_menu_llm !== undefined ? String(s.context_menu_llm) : undefined,
      defaultPermissionLevel: validDpl as 'yolo' | 'ask' | 'locked' | undefined,
      keybindings: s.keybindings ? {
        cycle_permission: s.keybindings.cycle_permission ? String(s.keybindings.cycle_permission) : undefined,
      } : undefined,
    });

    // Migration: if defaultPermissionLevel was not set, this is first run after upgrade
    if (!validDpl) {
      showToast('info', t('toast.permissionDefault'), t('toast.permissionDefaultBody'));
    }
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
        return {
          name: d.name,
          type: 'cli' as const,
          command: d.command,
          args: [], // Permission args now handled by trust + transport
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

    // Strip legacy permission args from config — now handled by transport
    const knownPermissionArgs = ['--dangerously-skip-permissions', '--sandbox=none', '--full-auto'];
    let patched = false;
    const cleaned = existing.map((c) => {
      if (c.type !== 'cli' || !c.args?.length) return c;
      const cleanedArgs = c.args.filter((a) => !knownPermissionArgs.includes(a));
      if (cleanedArgs.length === c.args.length) return c;
      patched = true;
      return { ...c, args: cleanedArgs };
    });

    if (brandNew.length === 0 && !patched) return;

    const merged = [...cleaned, ...brandNew];
    llmConfigs.set(merged);

    // Persist merged config to TOML
    try {
      const { stringify } = await import('smol-toml');
      const tomlObj: Record<string, unknown> = {
        settings: {
          default: get(appSettings).default ?? '',
          context_menu_llm: get(appSettings).contextMenuLlm ?? '',
          default_permission_level: get(appSettings).defaultPermissionLevel ?? 'yolo',
        },
        llm: merged.map((l) => {
          const entry: Record<string, unknown> = { name: l.name, type: l.type };
          if (l.type === 'cli') {
            entry.command = l.command ?? '';
            entry.args = l.args ?? [];
            if (l.yoloFlag) entry.yolo_flag = l.yoloFlag;
            entry.image_mode = l.imageMode ?? 'path';
            if (l.permissionLevel) entry.permission_level = l.permissionLevel;
            if (l.allowedTools?.length) entry.allowed_tools = l.allowedTools;
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
