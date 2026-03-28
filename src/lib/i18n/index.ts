import { writable, derived, get } from 'svelte/store';

export type Locale = 'en' | 'it' | 'de' | 'es' | 'fr' | 'pt' | 'zh' | 'hi' | 'ar';

export const locale = writable<Locale>('en');

import en from './en.json';

type PluralEntry = { zero?: string; one?: string; two?: string; few?: string; many?: string; other: string };
type TranslationValue = string | PluralEntry;
type TranslationDict = Record<string, TranslationValue>;

const translations: Record<string, TranslationDict> = { en: en as TranslationDict };

const localeLoaders: Record<string, () => Promise<{ default: TranslationDict }>> = {
  it: () => import('./it.json') as Promise<{ default: TranslationDict }>,
  de: () => import('./de.json') as Promise<{ default: TranslationDict }>,
  es: () => import('./es.json') as Promise<{ default: TranslationDict }>,
  fr: () => import('./fr.json') as Promise<{ default: TranslationDict }>,
  pt: () => import('./pt.json') as Promise<{ default: TranslationDict }>,
  zh: () => import('./zh.json') as Promise<{ default: TranslationDict }>,
  hi: () => import('./hi.json') as Promise<{ default: TranslationDict }>,
  ar: () => import('./ar.json') as Promise<{ default: TranslationDict }>,
};

let pluralRules = new Intl.PluralRules('en');

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

function substitute(template: string, params?: Record<string, string | number>): string {
  if (!params) return template;
  let result = template;
  for (const [k, v] of Object.entries(params)) {
    result = result.replace(`{${k}}`, String(v));
  }
  return result;
}

function resolveEntry(entry: TranslationValue, params?: Record<string, string | number>): string {
  if (typeof entry === 'string') {
    return substitute(entry, params);
  }
  if (typeof entry === 'object' && entry !== null && params?.count !== undefined) {
    const count = Number(params.count);
    const form = pluralRules.select(count) as keyof PluralEntry;
    const template = entry[form] ?? entry['other'];
    return substitute(template, params);
  }
  return substitute(entry['other'] ?? '', params);
}

export function t(key: string, params?: Record<string, string | number>): string {
  const loc = get(locale);
  const dict = translations[loc] ?? translations['en'];
  const entry = dict[key] ?? translations['en'][key];
  if (entry === undefined) return key;
  return resolveEntry(entry, params);
}

export const tr = derived(locale, ($loc) => {
  return (key: string, params?: Record<string, string | number>): string => {
    const dict = translations[$loc] ?? translations['en'];
    const entry = dict[key] ?? translations['en'][key];
    if (entry === undefined) return key;
    return resolveEntry(entry, params);
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
  pluralRules = new Intl.PluralRules(loc);
  locale.subscribe((l) => {
    document.documentElement.dir = l === 'ar' ? 'rtl' : 'ltr';
    document.documentElement.lang = l;
    pluralRules = new Intl.PluralRules(l);
  });
}
