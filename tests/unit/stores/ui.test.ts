import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
  fileTreeWidth,
  terminalWidth,
  activeEditorTab,
  showSettings,
  showDiff,
  fontFamily,
  fontSize,
  enhancedReadability,
  editorTheme,
  showHiveCanvas,
  hiveViewMode,
  selectedNodeId,
} from '$lib/stores/ui';

describe('ui store', () => {
  beforeEach(() => {
    fileTreeWidth.set(200);
    terminalWidth.set(300);
    activeEditorTab.set(null);
    showSettings.set(false);
    showDiff.set(false);
    fontFamily.set("'Atkinson Hyperlegible Mono', monospace");
    fontSize.set(14);
    enhancedReadability.set(false);
    editorTheme.set('forge-dark');
    showHiveCanvas.set(false);
    hiveViewMode.set('visual');
    selectedNodeId.set(null);
  });

  it('has correct default values', () => {
    expect(get(fileTreeWidth)).toBe(200);
    expect(get(terminalWidth)).toBe(300);
    expect(get(activeEditorTab)).toBeNull();
    expect(get(showSettings)).toBe(false);
    expect(get(showDiff)).toBe(false);
    expect(get(fontSize)).toBe(14);
    expect(get(enhancedReadability)).toBe(false);
    expect(get(editorTheme)).toBe('forge-dark');
  });

  it('can toggle showSettings', () => {
    expect(get(showSettings)).toBe(false);
    showSettings.set(true);
    expect(get(showSettings)).toBe(true);
    showSettings.update((v) => !v);
    expect(get(showSettings)).toBe(false);
  });

  it('can update fontSize', () => {
    fontSize.set(16);
    expect(get(fontSize)).toBe(16);

    fontSize.update((n) => n + 2);
    expect(get(fontSize)).toBe(18);
  });

  it('can update fileTreeWidth', () => {
    fileTreeWidth.set(280);
    expect(get(fileTreeWidth)).toBe(280);
  });

  it('can set activeEditorTab', () => {
    activeEditorTab.set('/project/src/main.ts');
    expect(get(activeEditorTab)).toBe('/project/src/main.ts');
  });

  it('hive canvas defaults to hidden with visual mode and no selection', () => {
    expect(get(showHiveCanvas)).toBe(false);
    expect(get(hiveViewMode)).toBe('visual');
    expect(get(selectedNodeId)).toBeNull();
  });

  it('can show hive canvas and switch view mode', () => {
    showHiveCanvas.set(true);
    expect(get(showHiveCanvas)).toBe(true);

    hiveViewMode.set('code');
    expect(get(hiveViewMode)).toBe('code');

    hiveViewMode.set('split');
    expect(get(hiveViewMode)).toBe('split');
  });

  it('can select a hive node', () => {
    selectedNodeId.set('node-42');
    expect(get(selectedNodeId)).toBe('node-42');

    selectedNodeId.set(null);
    expect(get(selectedNodeId)).toBeNull();
  });

  it('can set fontFamily', () => {
    fontFamily.set("'JetBrains Mono', monospace");
    expect(get(fontFamily)).toBe("'JetBrains Mono', monospace");
  });

  it('can switch editorTheme', () => {
    editorTheme.set('forge-light');
    expect(get(editorTheme)).toBe('forge-light');
  });

  it('can toggle showDiff', () => {
    showDiff.set(true);
    expect(get(showDiff)).toBe(true);
  });

  it('can toggle enhancedReadability', () => {
    enhancedReadability.set(true);
    expect(get(enhancedReadability)).toBe(true);
  });
});
