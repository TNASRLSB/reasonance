export interface FileEntry {
  name: string;
  path: string;
  isDir: boolean;
  size: number;
  modified: number; // unix timestamp ms
}

export interface PtyHandle {
  id: string;
}

export interface Adapter {
  // Filesystem
  readFile(path: string): Promise<string>;
  writeFile(path: string, content: string): Promise<void>;
  listDir(path: string, respectGitignore?: boolean): Promise<FileEntry[]>;
  watchFiles(path: string, callback: (event: FsEvent) => void): Promise<() => void>;

  // System
  openExternal(path: string): Promise<void>;
  getClipboard(): Promise<string>;
  setClipboard(text: string): Promise<void>;
  showNotification(title: string, body: string): Promise<void>;

  // PTY
  spawnProcess(command: string, args: string[], cwd: string): Promise<PtyHandle>;
  killProcess(id: string): Promise<void>;
  resizePty(id: string, cols: number, rows: number): Promise<void>;
  writePty(id: string, data: string): Promise<void>;
  onPtyData(id: string, callback: (data: string) => void): Promise<() => void>;
  onPtyExit(id: string, callback: (code: number) => void): Promise<() => void>;

  // Config
  readConfig(): Promise<string>;
  writeConfig(content: string): Promise<void>;

  // Shadow copies
  storeShadow(path: string, content: string): Promise<void>;
  getShadow(path: string): Promise<string | null>;
}

export interface FsEvent {
  type: 'create' | 'modify' | 'remove';
  path: string;
}
