import { describe, it, expect } from 'vitest';
import en from '$lib/i18n/en.json';
import it_locale from '$lib/i18n/it.json';
import de from '$lib/i18n/de.json';
import es from '$lib/i18n/es.json';
import fr from '$lib/i18n/fr.json';
import pt from '$lib/i18n/pt.json';
import zh from '$lib/i18n/zh.json';
import hi from '$lib/i18n/hi.json';
import ar from '$lib/i18n/ar.json';

const locales = { it: it_locale, de, es, fr, pt, zh, hi, ar };
const enKeys = Object.keys(en).sort();

describe('i18n key coverage', () => {
  for (const [name, translations] of Object.entries(locales)) {
    describe(`${name} locale`, () => {
      it('has all English keys', () => {
        const localeKeys = Object.keys(translations).sort();
        const missing = enKeys.filter(k => !localeKeys.includes(k));
        expect(missing, `Missing keys in ${name}: ${missing.join(', ')}`).toEqual([]);
      });

      it('has no extra keys not in English', () => {
        const localeKeys = Object.keys(translations).sort();
        const extra = localeKeys.filter(k => !enKeys.includes(k));
        expect(extra, `Extra keys in ${name}: ${extra.join(', ')}`).toEqual([]);
      });
    });
  }
});
