import type { Extension } from '@codemirror/state';

/**
 * Dynamically loads the CM6 language extension for a given filename.
 * Returns an empty array for unknown extensions (plain text mode).
 * All language packages are loaded on demand — none are bundled at startup.
 */
export async function getLangAsync(fileName: string): Promise<Extension[]> {
  const ext = fileName.split('.').pop()?.toLowerCase() ?? '';

  switch (ext) {
    case 'js':
      return [(await import('@codemirror/lang-javascript')).javascript()];
    case 'jsx':
      return [(await import('@codemirror/lang-javascript')).javascript({ jsx: true })];
    case 'ts':
      return [(await import('@codemirror/lang-javascript')).javascript({ typescript: true })];
    case 'tsx':
      return [(await import('@codemirror/lang-javascript')).javascript({ typescript: true, jsx: true })];
    case 'html':
    case 'svelte':
    case 'vue':
      return [(await import('@codemirror/lang-html')).html()];
    case 'css':
    case 'scss':
    case 'sass':
      return [(await import('@codemirror/lang-css')).css()];
    case 'py':
      return [(await import('@codemirror/lang-python')).python()];
    case 'rs':
      return [(await import('@codemirror/lang-rust')).rust()];
    case 'json':
      return [(await import('@codemirror/lang-json')).json()];
    case 'md':
      return [(await import('@codemirror/lang-markdown')).markdown()];
    case 'yaml':
    case 'yml':
      return [(await import('@codemirror/lang-yaml')).yaml()];
    case 'sql':
      return [(await import('@codemirror/lang-sql')).sql()];
    case 'go':
      return [(await import('@codemirror/lang-go')).go()];
    case 'c':
    case 'cpp':
    case 'cc':
    case 'h':
    case 'hpp':
      return [(await import('@codemirror/lang-cpp')).cpp()];
    case 'xml':
    case 'svg':
      return [(await import('@codemirror/lang-xml')).xml()];
    default:
      return [];
  }
}
