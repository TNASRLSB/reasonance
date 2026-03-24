import { THEME_SECTIONS, type ThemeFile } from './theme-types';

type VariableMap = Record<string, string | number>;

/**
 * Extract all CSS variables from a theme, flattening all sections.
 * Skips 'meta', 'when-dark', 'when-light'.
 */
export function extractVariables(theme: ThemeFile): VariableMap {
  const vars: VariableMap = {};

  for (const section of THEME_SECTIONS) {
    const data = theme[section];
    if (data && typeof data === 'object') {
      Object.assign(vars, data);
    }
  }

  return vars;
}

/**
 * Merge a modifier on top of base variables.
 * Resolves when-dark/when-light based on colorScheme.
 */
export function mergeModifier(
  base: VariableMap,
  modifier: ThemeFile,
  colorScheme: 'dark' | 'light'
): VariableMap {
  const result = { ...base };

  // Apply direct section overrides
  for (const section of THEME_SECTIONS) {
    const data = modifier[section];
    if (data && typeof data === 'object') {
      Object.assign(result, data);
    }
  }

  // Apply conditional overrides
  const conditional = colorScheme === 'dark' ? modifier['when-dark'] : modifier['when-light'];
  if (conditional && typeof conditional === 'object') {
    for (const sectionData of Object.values(conditional)) {
      if (sectionData && typeof sectionData === 'object') {
        Object.assign(result, sectionData);
      }
    }
  }

  return result;
}

/**
 * Build a CSS string from a variable map.
 */
export function buildCssString(vars: VariableMap): string {
  const lines = Object.entries(vars)
    .map(([key, value]) => `  ${key}: ${value};`)
    .join('\n');
  return `:root {\n${lines}\n}`;
}

/**
 * Inject a CSS string into a <style> element in the document head.
 * Creates the element if it doesn't exist.
 */
export function injectStyles(id: string, css: string): void {
  let el = document.getElementById(id) as HTMLStyleElement | null;
  if (!el) {
    el = document.createElement('style');
    el.id = id;
    document.head.appendChild(el);
  }
  el.textContent = css;
}

/**
 * Apply color-scheme to document root.
 */
export function applyColorScheme(colorScheme: 'dark' | 'light'): void {
  document.documentElement.style.colorScheme = colorScheme;
}

/**
 * Full theme application: extract variables, merge modifiers, inject CSS.
 */
export function applyTheme(
  theme: ThemeFile,
  modifiers: ThemeFile[] = []
): VariableMap {
  const colorScheme = theme.meta.colorScheme ?? 'dark';

  let vars = extractVariables(theme);

  for (const modifier of modifiers) {
    vars = mergeModifier(vars, modifier, colorScheme);
  }

  const themeCss = buildCssString(vars);
  injectStyles('reasonance-theme', themeCss);
  applyColorScheme(colorScheme);

  return vars;
}
