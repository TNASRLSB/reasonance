import { writable, derived, get } from 'svelte/store';

export type Locale = 'en' | 'it' | 'de' | 'es' | 'fr' | 'pt' | 'zh' | 'hi' | 'ar';

export const locale = writable<Locale>('en');
export const isRTL = derived(locale, ($l) => $l === 'ar');

import en from './en.json';
import it from './it.json';
import de from './de.json';
import es from './es.json';
import fr from './fr.json';
import pt from './pt.json';
import zh from './zh.json';
import hi from './hi.json';
import ar from './ar.json';

const translations: Record<string, Record<string, string>> = {
  en, it, de, es, fr, pt, zh, hi, ar,
};

export async function loadLocale(loc: Locale): Promise<void> {
  // All locales are statically imported — this is a no-op kept for API compatibility
  if (!translations[loc]) {
    console.warn(`Unknown locale: ${loc}`);
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
