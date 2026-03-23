const stateTokenMap: Record<string, string> = {
  idle: '--state-idle',
  queued: '--state-queued',
  running: '--state-running',
  success: '--state-success',
  failed: '--state-failed',
  retrying: '--state-retrying',
  fallback: '--state-retrying',
  error: '--state-failed',
};

export const stateIcons: Record<string, string> = {
  idle: '⏸',
  queued: '⏳',
  running: '↻',
  success: '✓',
  failed: '✗',
  retrying: '↻',
  fallback: '↻',
  error: '✗',
};

export function getStateColor(s: string): string {
  if (typeof document === 'undefined') return '';
  const token = stateTokenMap[s] || '--state-idle';
  return getComputedStyle(document.documentElement).getPropertyValue(token).trim();
}
