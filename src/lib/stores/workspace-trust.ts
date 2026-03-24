import { writable } from 'svelte/store';

export type TrustLevel = 'trusted' | 'read_only' | 'blocked';

export interface TrustEntry {
  hash: string;
  path: string;
  level: TrustLevel;
  trusted_at: string;
}

export interface FolderInfo {
  name: string;
  path: string;
  has_git: boolean;
  file_count: number;
}

export interface TrustCheckResult {
  level: TrustLevel | null;
  needs_prompt: boolean;
  folder_info: FolderInfo | null;
  rename_hint: string | null;
}

// Reactive trust level for the current workspace
export const workspaceTrustLevel = writable<TrustLevel | null>(null);
