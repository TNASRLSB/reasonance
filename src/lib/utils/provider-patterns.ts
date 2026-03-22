// src/lib/utils/provider-patterns.ts
import type { ProviderVisual } from '$lib/types/analytics';

export const PROVIDER_VISUALS: Record<string, ProviderVisual> = {
  claude: {
    color: '#1d4ed8',
    pattern: '#1d4ed8',
    patternLabel: 'solid blue',
    contrastColor: '#ffffff',
  },
  gemini: {
    color: '#16a34a',
    pattern: 'repeating-linear-gradient(90deg, #16a34a 0px, #16a34a 4px, transparent 4px, transparent 6px)',
    patternLabel: 'green vertical stripes',
    contrastColor: '#ffffff',
  },
  qwen: {
    color: '#ca8a04',
    pattern: 'repeating-linear-gradient(45deg, #ca8a04 0px, #ca8a04 3px, transparent 3px, transparent 6px)',
    patternLabel: 'amber diagonal',
    contrastColor: '#000000',
  },
  kimi: {
    color: '#9333ea',
    pattern: 'radial-gradient(circle, #9333ea 1px, transparent 1px)',
    patternLabel: 'purple dotted',
    contrastColor: '#ffffff',
    backgroundSize: '4px 4px',
  },
  codex: {
    color: '#dc2626',
    pattern: 'repeating-linear-gradient(0deg, #dc2626 0px, #dc2626 3px, transparent 3px, transparent 6px)',
    patternLabel: 'red horizontal stripes',
    contrastColor: '#ffffff',
  },
};

const FALLBACK_VISUAL: ProviderVisual = {
  color: '#6b7280',
  pattern: '#6b7280',
  patternLabel: 'gray solid',
  contrastColor: '#ffffff',
};

export function getProviderVisual(provider: string): ProviderVisual {
  return PROVIDER_VISUALS[provider.toLowerCase()] ?? FALLBACK_VISUAL;
}

export function barStyle(provider: string, widthPercent: number): string {
  const v = getProviderVisual(provider);
  const bg = v.pattern.includes('gradient') || v.pattern.includes('radial')
    ? `background: ${v.pattern}`
    : `background-color: ${v.pattern}`;
  const size = v.backgroundSize ? `; background-size: ${v.backgroundSize}` : '';
  return `${bg}${size}; width: ${widthPercent}%; height: 100%`;
}
