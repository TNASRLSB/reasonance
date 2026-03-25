// src/lib/engine/theme-types.ts

export interface ThemeMeta {
  name: string;
  author?: string;
  version?: string;
  description?: string;
  type: 'theme' | 'modifier';
  colorScheme?: 'dark' | 'light';
  editorTheme?: string;
  schemaVersion: number;
  trigger?: 'manual' | string; // 'manual' or CSS media query string
}

/** A section is a record of CSS variable names to values */
export type ThemeSection = Record<string, string | number>;

/** Conditional overrides based on active theme's colorScheme */
export interface ConditionalOverrides {
  [sectionName: string]: ThemeSection;
}

export interface ThemeFile {
  meta: ThemeMeta;
  colors?: ThemeSection;
  hues?: ThemeSection;
  states?: ThemeSection;
  'ui-states'?: ThemeSection;
  typography?: ThemeSection;
  spacing?: ThemeSection;
  borders?: ThemeSection;
  focus?: ThemeSection;
  transitions?: ThemeSection;
  layout?: ThemeSection;
  layers?: ThemeSection;
  shadows?: ThemeSection;
  overlays?: ThemeSection;
  highlights?: ThemeSection;
  'when-dark'?: ConditionalOverrides;
  'when-light'?: ConditionalOverrides;
}

export interface ThemePreferences {
  activeTheme: string;
  activeModifiers: string[];
}

/** All section keys (excluding meta and when-* conditionals) */
export const THEME_SECTIONS = [
  'colors', 'hues', 'states', 'ui-states', 'typography',
  'spacing', 'borders', 'focus', 'transitions', 'layout', 'layers',
  'shadows', 'overlays', 'highlights'
] as const;

export type ThemeSectionKey = typeof THEME_SECTIONS[number];
