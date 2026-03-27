import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { listen } from '@tauri-apps/api/event';
import { updateState } from '$lib/stores/update';
import { showToast } from '$lib/stores/toast';

export type UpdateMode = 'notify' | 'silent';

interface UpdateSettings {
  autoUpdate: boolean;
  updateMode: UpdateMode;
}

const DEFAULT_SETTINGS: UpdateSettings = {
  autoUpdate: true,
  updateMode: 'notify',
};

const CHECK_INTERVAL = 4 * 60 * 60 * 1000; // 4 hours
let unlisten: (() => void) | null = null;
let postponedUntil = 0;

export async function getUpdateSettings(): Promise<UpdateSettings> {
  try {
    const { load } = await import('@tauri-apps/plugin-store');
    const store = await load('settings.json');
    const autoUpdate = (await store.get<boolean>('autoUpdate')) ?? DEFAULT_SETTINGS.autoUpdate;
    const updateMode = (await store.get<UpdateMode>('updateMode')) ?? DEFAULT_SETTINGS.updateMode;
    return { autoUpdate, updateMode };
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export async function saveUpdateSettings(settings: Partial<UpdateSettings>): Promise<void> {
  const { load } = await import('@tauri-apps/plugin-store');
  const store = await load('settings.json');
  if (settings.autoUpdate !== undefined) await store.set('autoUpdate', settings.autoUpdate);
  if (settings.updateMode !== undefined) await store.set('updateMode', settings.updateMode);
  await store.save();
}

export async function checkForUpdate(manual = false): Promise<boolean> {
  if (!manual && Date.now() < postponedUntil) return false;

  try {
    const update = await check();
    if (!update) {
      if (manual) showToast('success', 'Up to date', 'You are running the latest version.');
      return false;
    }

    // Store the update info — StatusBar will display it
    updateState.set({
      newVersion: update.version,
      downloadProgress: null,
      updateHandle: update,
    });

    return true;
  } catch (err) {
    if (manual) showToast('error', 'Update check failed', String(err));
    return false;
  }
}

/** Called from StatusBar when user clicks install */
export async function installUpdate(): Promise<void> {
  let update: any;
  updateState.subscribe((s) => { update = s.updateHandle; })();

  if (!update) return;

  let downloaded = 0;
  let contentLength = 0;

  await update.downloadAndInstall((event: any) => {
    if (event.event === 'Started' && event.data.contentLength) {
      contentLength = event.data.contentLength;
    } else if (event.event === 'Progress') {
      downloaded += event.data.chunkLength;
      if (contentLength > 0) {
        updateState.update((s) => ({ ...s, downloadProgress: Math.round((downloaded / contentLength) * 100) }));
      }
    } else if (event.event === 'Finished') {
      updateState.update((s) => ({ ...s, downloadProgress: 100 }));
    }
  });

  await relaunch();
}

/** Called from StatusBar when user clicks "later" */
export function postponeUpdate(): void {
  postponedUntil = Date.now() + CHECK_INTERVAL;
  updateState.set({ newVersion: null, downloadProgress: null, updateHandle: null });
}

// The backend fires lifecycle:update-check via EventBus at the right intervals
// (5s initial delay, then every 4 hours). We listen instead of polling.
export async function startUpdateChecker(): Promise<void> {
  unlisten = await listen('lifecycle:update-check', async () => {
    const settings = await getUpdateSettings();
    if (settings.autoUpdate) {
      checkForUpdate();
    }
  });
}

export function stopUpdateChecker(): void {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
}
