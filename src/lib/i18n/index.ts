import { writable, derived, get } from 'svelte/store';

export type Locale = 'en' | 'it' | 'de' | 'es' | 'fr' | 'pt' | 'zh' | 'hi' | 'ar';

export const locale = writable<Locale>('en');
export const isRTL = derived(locale, ($l) => $l === 'ar');

import en from './en.json';

const translations: Record<string, Record<string, string>> = { en };

const localeLoaders: Record<string, () => Promise<{ default: Record<string, string> }>> = {
  it: () => import('./it.json'),
  de: () => import('./de.json'),
  es: () => import('./es.json'),
  fr: () => import('./fr.json'),
  pt: () => import('./pt.json'),
  zh: () => import('./zh.json'),
  hi: () => import('./hi.json'),
  ar: () => import('./ar.json'),
};

export async function loadLocale(loc: Locale): Promise<void> {
  if (translations[loc]) return;
  const loader = localeLoaders[loc];
  if (!loader) {
    console.warn(`Unknown locale: ${loc}`);
    return;
  }
  try {
    const mod = await loader();
    translations[loc] = mod.default;
  } catch (e) {
    console.warn(`Failed to load locale ${loc}:`, e);
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
    document.documentElement.lang = l;
  });
}
