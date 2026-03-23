/**
 * session.ts — Session persistence utilities for Reasonance IDE.
 *
 * Handles save/restore of open files, active file, theme, font settings,
 * locale, project root, recent projects, and terminal tab metadata via
 * @tauri-apps/plugin-store.
 */

import { get } from 'svelte/store';
import { load } from '@tauri-apps/plugin-store';
import { openFiles, activeFilePath, projectRoot, recentProjects, addOpenFile } from '$lib/stores/files';
import { themeMode } from '$lib/stores/theme';
import { fontFamily, fontSize, enhancedReadability } from '$lib/stores/ui';
import { terminalInstances } from '$lib/stores/terminals';
import { locale, loadLocale } from '$lib/i18n/index';
import type { Adapter } from '$lib/adapter/index';

export type { ThemeMode } from '$lib/stores/theme';

// ── Save ──────────────────────────────────────────────────────────────────────

export async function saveSession(): Promise<void> {
  try {
    const store = await load('session.json', { defaults: {}, autoSave: false });

    const currentFiles = get(openFiles);
    await store.set('openFiles', currentFiles.map((f) => f.path));
    await store.set('activeFile', get(activeFilePath));
    await store.set('theme', get(themeMode));
    await store.set('fontFamily', get(fontFamily));
    await store.set('fontSize', get(fontSize));
    await store.set('enhancedReadability', get(enhancedReadability));
    await store.set('projectRoot', get(projectRoot));
    await store.set('recentProjects', get(recentProjects));

    const currentLocale = get(locale);
    await store.set('locale', currentLocale);

    // Terminal instances: only metadata (provider, instance count) — PTY sessions can't survive restart
    const instances = get(terminalInstances);
    await store.set(
      'terminalInstances',
      instances.map((i) => ({ provider: i.provider, instanceCount: 1 }))
    );

    await store.save();
  } catch {
    // Session save failures are non-fatal
  }
}

// ── Restore ───────────────────────────────────────────────────────────────────

/**
 * Restores the previous session from plugin-store.
 *
 * @param adapter        Used to read open files and set the project root.
 * @param onProjectFound Called when a saved project root is found, so the
 *                       caller can hide the welcome screen.
 */
export async function restoreSession(
  adapter: Adapter,
  onProjectFound: () => void
): Promise<void> {
  try {
    const store = await load('session.json', { defaults: {}, autoSave: false });

    // Restore theme
    const savedTheme = await store.get<string>('theme');
    if (savedTheme && ['light', 'dark', 'system'].includes(savedTheme)) {
      themeMode.set(savedTheme as import('$lib/stores/theme').ThemeMode);
    }

    // Restore font settings — validate that saved font is a monospace font.
    // A previous bug could save the UI font (sans-serif) instead of mono.
    const savedFontFamily = await store.get<string>('fontFamily');
    if (savedFontFamily && /mono/i.test(savedFontFamily)) {
      fontFamily.set(savedFontFamily);
    }

    const savedFontSize = await store.get<number>('fontSize');
    if (savedFontSize && savedFontSize > 0) fontSize.set(savedFontSize);

    const savedEnhancedReadability = await store.get<boolean>('enhancedReadability');
    if (savedEnhancedReadability !== null && savedEnhancedReadability !== undefined) {
      enhancedReadability.set(savedEnhancedReadability);
    }

    // Restore locale
    const savedLocale = await store.get<string>('locale');
    if (savedLocale && ['en', 'it', 'de', 'es', 'fr', 'pt', 'zh', 'hi', 'ar'].includes(savedLocale)) {
      await loadLocale(savedLocale as import('$lib/i18n/index').Locale);
      locale.set(savedLocale as import('$lib/i18n/index').Locale);
    }

    // Restore project root and recent projects
    const savedRoot = await store.get<string>('projectRoot');
    if (savedRoot) {
      projectRoot.set(savedRoot);
      onProjectFound();
      try { await adapter.setProjectRoot(savedRoot); } catch { /* non-fatal */ }
    }

    const savedRecent = await store.get<string[]>('recentProjects');
    if (savedRecent) recentProjects.set(savedRecent);

    // Restore open files
    const savedPaths = await store.get<string[]>('openFiles');
    const savedActive = await store.get<string | null>('activeFile');

    if (savedPaths && savedPaths.length > 0) {
      for (const path of savedPaths) {
        try {
          const content = await adapter.readFile(path);
          const name = path.split('/').pop() ?? path;
          addOpenFile({ path, name, content, isDirty: false, isDeleted: false });
        } catch {
          // File may have been deleted since last session — skip silently
        }
      }
    }

    // Set active file after all are loaded (addOpenFile already sets the last one)
    if (savedActive) {
      activeFilePath.set(savedActive);
    }
  } catch {
    // Restore failures are non-fatal — start fresh
  }
}

// ── Shadow tracking ───────────────────────────────────────────────────────────

/**
 * Subscribes to openFiles and stores a shadow copy whenever a new file is
 * opened for the first time. Returns an unsubscribe function.
 */
export function initShadowTracking(adapter: Adapter): () => void {
  const knownPaths = new Set<string>();

  return openFiles.subscribe(async (files) => {
    for (const file of files) {
      if (!knownPaths.has(file.path)) {
        knownPaths.add(file.path);
        try {
          await adapter.storeShadow(file.path, file.content);
        } catch {
          // Shadow store failures are non-fatal
        }
      }
    }
  });
}
