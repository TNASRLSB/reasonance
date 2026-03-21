import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
  fileTreeWidth,
  terminalWidth,
  activeEditorTab,
  showSettings,
  yoloMode,
  showDiff,
  fontFamily,
  fontSize,
  enhancedReadability,
  editorTheme,
  showSwarmCanvas,
  swarmViewMode,
  selectedNodeId,
} from '$lib/stores/ui';

describe('ui store', () => {
  beforeEach(() => {
    fileTreeWidth.set(200);
    terminalWidth.set(300);
    activeEditorTab.set(null);
    showSettings.set(false);
    yoloMode.set(false);
    showDiff.set(false);
    fontFamily.set("'Atkinson Hyperlegible Mono', monospace");
    fontSize.set(14);
    enhancedReadability.set(false);
    editorTheme.set('forge-dark');
    showSwarmCanvas.set(false);
    swarmViewMode.set('visual');
    selectedNodeId.set(null);
  });

  it('has correct default values', () => {
    expect(get(fileTreeWidth)).toBe(200);
    expect(get(terminalWidth)).toBe(300);
    expect(get(activeEditorTab)).toBeNull();
    expect(get(showSettings)).toBe(false);
    expect(get(yoloMode)).toBe(false);
    expect(get(showDiff)).toBe(false);
    expect(get(fontSize)).toBe(14);
    expect(get(enhancedReadability)).toBe(false);
    expect(get(editorTheme)).toBe('forge-dark');
  });

  it('can toggle yoloMode', () => {
    expect(get(yoloMode)).toBe(false);
    yoloMode.set(true);
    expect(get(yoloMode)).toBe(true);
    yoloMode.update((v) => !v);
    expect(get(yoloMode)).toBe(false);
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

  it('swarm canvas defaults to hidden with visual mode and no selection', () => {
    expect(get(showSwarmCanvas)).toBe(false);
    expect(get(swarmViewMode)).toBe('visual');
    expect(get(selectedNodeId)).toBeNull();
  });

  it('can show swarm canvas and switch view mode', () => {
    showSwarmCanvas.set(true);
    expect(get(showSwarmCanvas)).toBe(true);

    swarmViewMode.set('code');
    expect(get(swarmViewMode)).toBe('code');

    swarmViewMode.set('split');
    expect(get(swarmViewMode)).toBe('split');
  });

  it('can select a swarm node', () => {
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
