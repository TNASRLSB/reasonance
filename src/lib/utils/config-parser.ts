import type { LlmConfig } from '$lib/stores/config';

export function parseLlmConfig(rawLlms: Array<Record<string, unknown>>): LlmConfig[] {
  return rawLlms.map((l) => ({
    name: String(l.name ?? ''),
    type: (l.type === 'api' ? 'api' : 'cli') as 'cli' | 'api',
    command: l.command !== undefined ? String(l.command) : undefined,
    args: Array.isArray(l.args) ? l.args.map(String) : [],
    yoloFlag: l.yolo_flag !== undefined ? String(l.yolo_flag) : undefined,
    imageMode: (['path', 'base64', 'none'].includes(String(l.image_mode))
      ? l.image_mode
      : 'path') as 'path' | 'base64' | 'none',
    permissionLevel: (['yolo', 'ask', 'locked'].includes(String(l.permission_level))
      ? l.permission_level
      : undefined) as 'yolo' | 'ask' | 'locked' | undefined,
    allowedTools: Array.isArray(l.allowed_tools) ? l.allowed_tools.map(String) : undefined,
    provider: l.provider !== undefined ? String(l.provider) : undefined,
    apiKeyEnv: l.api_key_env !== undefined ? String(l.api_key_env) : undefined,
    model: l.model !== undefined ? String(l.model) : undefined,
    endpoint: l.endpoint !== undefined ? String(l.endpoint) : undefined,
  }));
}
