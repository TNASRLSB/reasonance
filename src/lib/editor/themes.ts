import { EditorView } from 'codemirror';
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { tags } from '@lezer/highlight';
import type { Extension } from '@codemirror/state';

// Dracula theme
const draculaColors = EditorView.theme({
  '&': { backgroundColor: '#282a36', color: '#f8f8f2' },
  '.cm-content': { fontFamily: "'Atkinson Hyperlegible Mono', monospace", fontSize: '14px', caretColor: '#f8f8f0' },
  '.cm-cursor': { borderLeftColor: '#f8f8f0' },
  '.cm-gutters': { backgroundColor: '#282a36', color: '#6272a4', borderRight: '2px solid #44475a' },
  '.cm-activeLineGutter': { backgroundColor: '#44475a', color: '#f8f8f2' },
  '.cm-activeLine': { backgroundColor: 'rgba(68, 71, 90, 0.4)' },
  '.cm-selectionBackground': { backgroundColor: 'rgba(68, 71, 90, 0.6) !important' },
  '&.cm-focused .cm-selectionBackground': { backgroundColor: 'rgba(68, 71, 90, 0.8) !important' },
  '.cm-selectionMatch': { backgroundColor: 'rgba(68, 71, 90, 0.3)' },
  '.cm-foldGutter .cm-gutterElement': { color: '#6272a4' },
}, { dark: true });

const draculaHighlight = HighlightStyle.define([
  { tag: tags.keyword, color: '#ff79c6' },
  { tag: tags.comment, color: '#6272a4', fontStyle: 'italic' },
  { tag: tags.string, color: '#f1fa8c' },
  { tag: tags.number, color: '#bd93f9' },
  { tag: tags.function(tags.variableName), color: '#50fa7b' },
  { tag: tags.typeName, color: '#8be9fd', fontStyle: 'italic' },
  { tag: tags.operator, color: '#ff79c6' },
  { tag: tags.bool, color: '#bd93f9' },
  { tag: tags.propertyName, color: '#66d9ef' },
  { tag: tags.variableName, color: '#f8f8f2' },
  { tag: tags.definition(tags.variableName), color: '#50fa7b' },
  { tag: tags.tagName, color: '#ff79c6' },
  { tag: tags.attributeName, color: '#50fa7b' },
]);

export const dracula = [draculaColors, syntaxHighlighting(draculaHighlight)];

// Solarized Dark theme
const solarizedColors = EditorView.theme({
  '&': { backgroundColor: '#002b36', color: '#839496' },
  '.cm-content': { fontFamily: "'Atkinson Hyperlegible Mono', monospace", fontSize: '14px', caretColor: '#839496' },
  '.cm-cursor': { borderLeftColor: '#839496' },
  '.cm-gutters': { backgroundColor: '#002b36', color: '#586e75', borderRight: '2px solid #073642' },
  '.cm-activeLineGutter': { backgroundColor: '#073642', color: '#93a1a1' },
  '.cm-activeLine': { backgroundColor: 'rgba(7, 54, 66, 0.5)' },
  '.cm-selectionBackground': { backgroundColor: 'rgba(7, 54, 66, 0.8) !important' },
  '&.cm-focused .cm-selectionBackground': { backgroundColor: 'rgba(38, 139, 210, 0.3) !important' },
  '.cm-selectionMatch': { backgroundColor: 'rgba(7, 54, 66, 0.4)' },
  '.cm-foldGutter .cm-gutterElement': { color: '#586e75' },
}, { dark: true });

const solarizedHighlight = HighlightStyle.define([
  { tag: tags.keyword, color: '#859900' },
  { tag: tags.comment, color: '#586e75', fontStyle: 'italic' },
  { tag: tags.string, color: '#2aa198' },
  { tag: tags.number, color: '#d33682' },
  { tag: tags.function(tags.variableName), color: '#268bd2' },
  { tag: tags.typeName, color: '#b58900' },
  { tag: tags.operator, color: '#859900' },
  { tag: tags.bool, color: '#d33682' },
  { tag: tags.propertyName, color: '#268bd2' },
  { tag: tags.variableName, color: '#839496' },
  { tag: tags.definition(tags.variableName), color: '#268bd2' },
  { tag: tags.tagName, color: '#268bd2' },
  { tag: tags.attributeName, color: '#93a1a1' },
]);

export const solarizedDark = [solarizedColors, syntaxHighlighting(solarizedHighlight)];

// GitHub Dark theme
const githubDarkColors = EditorView.theme({
  '&': { backgroundColor: '#0d1117', color: '#c9d1d9' },
  '.cm-content': { fontFamily: "'Atkinson Hyperlegible Mono', monospace", fontSize: '14px', caretColor: '#c9d1d9' },
  '.cm-cursor': { borderLeftColor: '#c9d1d9' },
  '.cm-gutters': { backgroundColor: '#0d1117', color: '#484f58', borderRight: '2px solid #21262d' },
  '.cm-activeLineGutter': { backgroundColor: '#161b22', color: '#8b949e' },
  '.cm-activeLine': { backgroundColor: 'rgba(22, 27, 34, 0.5)' },
  '.cm-selectionBackground': { backgroundColor: 'rgba(56, 139, 253, 0.2) !important' },
  '&.cm-focused .cm-selectionBackground': { backgroundColor: 'rgba(56, 139, 253, 0.3) !important' },
  '.cm-selectionMatch': { backgroundColor: 'rgba(56, 139, 253, 0.1)' },
  '.cm-foldGutter .cm-gutterElement': { color: '#484f58' },
}, { dark: true });

const githubDarkHighlight = HighlightStyle.define([
  { tag: tags.keyword, color: '#ff7b72' },
  { tag: tags.comment, color: '#8b949e', fontStyle: 'italic' },
  { tag: tags.string, color: '#a5d6ff' },
  { tag: tags.number, color: '#79c0ff' },
  { tag: tags.function(tags.variableName), color: '#d2a8ff' },
  { tag: tags.typeName, color: '#ffa657' },
  { tag: tags.operator, color: '#ff7b72' },
  { tag: tags.bool, color: '#79c0ff' },
  { tag: tags.propertyName, color: '#79c0ff' },
  { tag: tags.variableName, color: '#c9d1d9' },
  { tag: tags.definition(tags.variableName), color: '#ffa657' },
  { tag: tags.tagName, color: '#7ee787' },
  { tag: tags.attributeName, color: '#79c0ff' },
]);

export const githubDark = [githubDarkColors, syntaxHighlighting(githubDarkHighlight)];

// Material Dark theme
const materialColors = EditorView.theme({
  '&': { backgroundColor: '#263238', color: '#eeffff' },
  '.cm-content': { fontFamily: "'Atkinson Hyperlegible Mono', monospace", fontSize: '14px', caretColor: '#ffcc00' },
  '.cm-cursor': { borderLeftColor: '#ffcc00' },
  '.cm-gutters': { backgroundColor: '#263238', color: '#546e7a', borderRight: '2px solid #37474f' },
  '.cm-activeLineGutter': { backgroundColor: '#37474f', color: '#eeffff' },
  '.cm-activeLine': { backgroundColor: 'rgba(55, 71, 79, 0.4)' },
  '.cm-selectionBackground': { backgroundColor: 'rgba(128, 203, 196, 0.2) !important' },
  '&.cm-focused .cm-selectionBackground': { backgroundColor: 'rgba(128, 203, 196, 0.3) !important' },
  '.cm-selectionMatch': { backgroundColor: 'rgba(128, 203, 196, 0.1)' },
  '.cm-foldGutter .cm-gutterElement': { color: '#546e7a' },
}, { dark: true });

const materialHighlight = HighlightStyle.define([
  { tag: tags.keyword, color: '#c792ea' },
  { tag: tags.comment, color: '#546e7a', fontStyle: 'italic' },
  { tag: tags.string, color: '#c3e88d' },
  { tag: tags.number, color: '#f78c6c' },
  { tag: tags.function(tags.variableName), color: '#82aaff' },
  { tag: tags.typeName, color: '#ffcb6b' },
  { tag: tags.operator, color: '#89ddff' },
  { tag: tags.bool, color: '#f78c6c' },
  { tag: tags.propertyName, color: '#82aaff' },
  { tag: tags.variableName, color: '#eeffff' },
  { tag: tags.definition(tags.variableName), color: '#82aaff' },
  { tag: tags.tagName, color: '#f07178' },
  { tag: tags.attributeName, color: '#c792ea' },
]);

export const materialDark = [materialColors, syntaxHighlighting(materialHighlight)];

// Theme registry
export const editorThemes: Record<string, { label: string; extensions: Extension[]; isDark: boolean }> = {
  'forge-dark': { label: 'Forge Dark', extensions: [], isDark: true },
  'forge-light': { label: 'Forge Light', extensions: [], isDark: false },
  'one-dark': { label: 'One Dark', extensions: [], isDark: true },
  'dracula': { label: 'Dracula', extensions: dracula, isDark: true },
  'solarized-dark': { label: 'Solarized Dark', extensions: solarizedDark, isDark: true },
  'github-dark': { label: 'GitHub Dark', extensions: githubDark, isDark: true },
  'material-dark': { label: 'Material Dark', extensions: materialDark, isDark: true },
};
