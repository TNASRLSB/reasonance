import { writable, get } from 'svelte/store';
import type { NegotiatedCapabilities, CliVersionInfo } from '$lib/types/capability';

export const providerCapabilities = writable<Record<string, NegotiatedCapabilities>>({});
export const cliVersions = writable<CliVersionInfo[]>([]);

export function setCapabilities(caps: Record<string, NegotiatedCapabilities>) {
  providerCapabilities.set(caps);
}

export function updateProviderCapabilities(provider: string, caps: NegotiatedCapabilities) {
  providerCapabilities.update((current) => ({ ...current, [provider]: caps }));
}

export function setCliVersions(versions: CliVersionInfo[]) {
  cliVersions.set(versions);
}

export function isFeatureSupported(provider: string, feature: string): boolean {
  const caps = get(providerCapabilities);
  const providerCaps = caps[provider];
  if (!providerCaps) return false;
  const featureSupport = providerCaps.features[feature];
  return featureSupport?.level === 'full' || featureSupport?.level === 'partial';
}
