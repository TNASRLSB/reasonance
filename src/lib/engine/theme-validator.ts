import { THEME_SECTIONS, type ThemeFile } from './theme-types';

export interface ValidationResult {
  valid: boolean;
  errors: string[];
}

const CURRENT_SCHEMA_VERSION = 2;

export function validateTheme(data: unknown): ValidationResult {
  const errors: string[] = [];

  if (!data || typeof data !== 'object') {
    return { valid: false, errors: ['Theme must be an object'] };
  }

  const theme = data as Record<string, unknown>;

  // Meta validation
  if (!theme.meta || typeof theme.meta !== 'object') {
    return { valid: false, errors: ['Missing required field: meta'] };
  }

  const meta = theme.meta as Record<string, unknown>;

  if (!meta.name || typeof meta.name !== 'string') {
    errors.push('meta.name is required and must be a string');
  }

  if (!meta.type || (meta.type !== 'theme' && meta.type !== 'modifier')) {
    errors.push('meta.type must be "theme" or "modifier"');
  }

  if (meta.schemaVersion !== CURRENT_SCHEMA_VERSION) {
    errors.push(`Unsupported schemaVersion: ${meta.schemaVersion} (expected ${CURRENT_SCHEMA_VERSION})`);
  }

  if (meta.type === 'theme' && meta.colorScheme !== 'dark' && meta.colorScheme !== 'light') {
    errors.push('Theme meta.colorScheme must be "dark" or "light"');
  }

  if (errors.length > 0) {
    return { valid: false, errors };
  }

  // Section validation
  if (meta.type === 'theme') {
    for (const section of THEME_SECTIONS) {
      if (!theme[section] || typeof theme[section] !== 'object') {
        errors.push(`Missing required section: ${section}`);
      }
    }
  } else {
    // Modifier: at least one section (excluding meta, when-dark, when-light)
    const hasSections = THEME_SECTIONS.some((s) => theme[s] && typeof theme[s] === 'object');
    const hasConditionals =
      (theme['when-dark'] && typeof theme['when-dark'] === 'object') ||
      (theme['when-light'] && typeof theme['when-light'] === 'object');

    if (!hasSections && !hasConditionals) {
      errors.push('Modifier must have at least one section or conditional override');
    }
  }

  // Value type validation for present sections
  for (const section of THEME_SECTIONS) {
    if (theme[section] && typeof theme[section] === 'object') {
      const sectionData = theme[section] as Record<string, unknown>;
      for (const [key, value] of Object.entries(sectionData)) {
        if (!key.startsWith('--')) {
          errors.push(`${section}.${key}: variable names must start with --`);
        }
        if (typeof value !== 'string' && typeof value !== 'number') {
          errors.push(`${section}.${key}: value must be a string or number, got ${typeof value}`);
        }
      }
    }
  }

  return { valid: errors.length === 0, errors };
}
