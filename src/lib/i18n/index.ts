import { writable, derived, get } from 'svelte/store';

export type Locale = 'en' | 'it' | 'de' | 'es' | 'fr' | 'pt' | 'zh' | 'hi' | 'ar';

export const locale = writable<Locale>('en');
export const isRTL = derived(locale, ($l) => $l === 'ar');

const translations: Record<string, Record<string, string>> = {};

import en from './en.json';
translations['en'] = en;

export async function loadLocale(loc: Locale): Promise<void> {
  if (translations[loc]) return;
  try {
    const mod = await import(`./${loc}.json`);
    translations[loc] = mod.default;
  } catch {
    console.warn(`Failed to load locale: ${loc}`);
  }
}

export function t(key: string, params?: Record<string, string>): string {
  const loc = get(locale);
  const dict = translations[loc] ?? translations['en'];
  let text = dict[key] ?? translations['en'][key] ?? key;
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      text = text.replace(`{${k}}`, v);
    }
  }
  return text;
}

export const tr = derived(locale, ($loc) => {
  return (key: string, params?: Record<string, string>): string => {
    const dict = translations[$loc] ?? translations['en'];
    let text = dict[key] ?? translations['en'][key] ?? key;
    if (params) {
      for (const [k, v] of Object.entries(params)) {
        text = text.replace(`{${k}}`, v);
      }
    }
    return text;
  };
});

export function detectLocale(): Locale {
  const lang = navigator.language?.split('-')[0]?.toLowerCase() ?? 'en';
  const supported: Locale[] = ['en', 'it', 'de', 'es', 'fr', 'pt', 'zh', 'hi', 'ar'];
  return supported.includes(lang as Locale) ? (lang as Locale) : 'en';
}

export async function initI18n(): Promise<void> {
  const loc = detectLocale();
  await loadLocale(loc);
  locale.set(loc);
  locale.subscribe((l) => {
    document.documentElement.dir = l === 'ar' ? 'rtl' : 'ltr';
  });
}
