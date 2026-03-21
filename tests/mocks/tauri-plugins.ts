// Catch-all mock for all @tauri-apps/plugin-* packages

// plugin-fs
export function readFile() { return Promise.resolve(''); }
export function writeFile() { return Promise.resolve(); }
export function readDir() { return Promise.resolve([]); }
export function exists() { return Promise.resolve(false); }
export function mkdir() { return Promise.resolve(); }
export function remove() { return Promise.resolve(); }
export function watch() { return Promise.resolve(() => {}); }
export function readTextFile() { return Promise.resolve(''); }
export function writeTextFile() { return Promise.resolve(); }

// plugin-store
export class Store {
  private data = new Map<string, unknown>();
  static async load(_path: string) { return new Store(); }
  async get(key: string) { return this.data.get(key) ?? null; }
  async set(key: string, value: unknown) { this.data.set(key, value); }
  async save() {}
  async delete(key: string) { this.data.delete(key); }
  async keys() { return [...this.data.keys()]; }
}
export function load(path: string) { return Store.load(path); }

// plugin-dialog
export function open() { return Promise.resolve(null); }
export function save() { return Promise.resolve(null); }
export function message() { return Promise.resolve(); }
export function ask() { return Promise.resolve(false); }
export function confirm() { return Promise.resolve(false); }

// plugin-clipboard-manager
export function writeText() { return Promise.resolve(); }
export function readText() { return Promise.resolve(''); }

// plugin-notification
export function sendNotification() { return Promise.resolve(); }
export function isPermissionGranted() { return Promise.resolve(true); }
export function requestPermission() { return Promise.resolve('granted'); }

// plugin-global-shortcut
export function register() { return Promise.resolve(); }
export function unregister() { return Promise.resolve(); }
export function isRegistered() { return Promise.resolve(false); }

// plugin-process
export function exit() { return Promise.resolve(); }
export function relaunch() { return Promise.resolve(); }

// plugin-deep-link
export function getCurrent() { return Promise.resolve([]); }
export function onOpenUrl() { return Promise.resolve(() => {}); }

// plugin-window-state
export function restoreStateCurrent() { return Promise.resolve(); }
export function saveWindowState() { return Promise.resolve(); }
export const StateFlags = { ALL: 0 };

// plugin-updater
export function check() { return Promise.resolve(null); }

// plugin-opener
export function openUrl() { return Promise.resolve(); }
export function openPath() { return Promise.resolve(); }

// Default export
export default {};
