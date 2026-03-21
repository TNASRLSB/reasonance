const ICON_MAP: Record<string, string> = {
  ts: '🟦', tsx: '🟦', js: '🟨', jsx: '🟨',
  rs: '🦀', py: '🐍', go: '🔷', java: '☕',
  html: '🌐', css: '🎨', scss: '🎨',
  json: '📋', toml: '📋', yaml: '📋', yml: '📋',
  md: '📝', txt: '📄',
  png: '🖼️', jpg: '🖼️', svg: '🖼️', gif: '🖼️',
  sh: '🐚', bash: '🐚',
};

export function getFileIcon(name: string, isDir: boolean): string {
  if (isDir) return '📁';
  const ext = name.split('.').pop()?.toLowerCase() ?? '';
  return ICON_MAP[ext] ?? '📄';
}
