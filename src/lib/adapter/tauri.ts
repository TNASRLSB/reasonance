import { invoke } from '@tauri-apps/api/core';
import type { Adapter, FileEntry, FsEvent, PtyHandle } from './index';

export class TauriAdapter implements Adapter {
  async readFile(path: string): Promise<string> {
    return invoke<string>('read_file', { path });
  }
  async writeFile(path: string, content: string): Promise<void> {
    return invoke<void>('write_file', { path, content });
  }
  async listDir(path: string, respectGitignore?: boolean): Promise<FileEntry[]> {
    return invoke<FileEntry[]>('list_dir', { path, respectGitignore: respectGitignore ?? true });
  }
  async watchFiles(path: string, callback: (event: FsEvent) => void): Promise<() => void> {
    throw new Error('Not implemented');
  }
  async openExternal(path: string): Promise<void> {
    return invoke<void>('open_external', { path });
  }
  async getClipboard(): Promise<string> {
    throw new Error('Not implemented');
  }
  async setClipboard(text: string): Promise<void> {
    throw new Error('Not implemented');
  }
  async showNotification(title: string, body: string): Promise<void> {
    throw new Error('Not implemented');
  }
  async spawnProcess(command: string, args: string[], cwd: string): Promise<PtyHandle> {
    throw new Error('Not implemented');
  }
  async killProcess(id: string): Promise<void> {
    throw new Error('Not implemented');
  }
  async resizePty(id: string, cols: number, rows: number): Promise<void> {
    throw new Error('Not implemented');
  }
  async writePty(id: string, data: string): Promise<void> {
    throw new Error('Not implemented');
  }
  async onPtyData(id: string, callback: (data: string) => void): Promise<() => void> {
    throw new Error('Not implemented');
  }
  async onPtyExit(id: string, callback: (code: number) => void): Promise<() => void> {
    throw new Error('Not implemented');
  }
  async readConfig(): Promise<string> {
    throw new Error('Not implemented');
  }
  async writeConfig(content: string): Promise<void> {
    throw new Error('Not implemented');
  }
  async storeShadow(path: string, content: string): Promise<void> {
    throw new Error('Not implemented');
  }
  async getShadow(path: string): Promise<string | null> {
    throw new Error('Not implemented');
  }
}
