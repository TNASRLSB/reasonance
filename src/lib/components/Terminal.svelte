<script lang="ts">
  import { onMount } from 'svelte';
  import { Terminal } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import { WebLinksAddon } from '@xterm/addon-web-links';
  // WebGL addon loaded dynamically — static import crashes WebKitGTK if WebGL unavailable
  import { SearchAddon } from '@xterm/addon-search';
  import { SerializeAddon } from '@xterm/addon-serialize';
  import { ImageAddon } from '@xterm/addon-image';
  import '@xterm/xterm/css/xterm.css';
  import type { Adapter } from '$lib/adapter/index';
  import { terminalTabs } from '$lib/stores/terminals';
  import { enhancedReadability, fontFamily, fontSize } from '$lib/stores/ui';
  import { isDark } from '$lib/stores/theme';

  const darkXtermTheme = {
    background: '#161616', foreground: '#d4d4d4', cursor: '#f0f0f0', cursorAccent: '#161616',
    selectionBackground: 'rgba(29, 78, 216, 0.4)', selectionForeground: '#ffffff',
    black: '#121212', red: '#dc2626', green: '#16a34a', yellow: '#ca8a04',
    blue: '#1d4ed8', magenta: '#a855f7', cyan: '#06b6d4', white: '#d4d4d4',
    brightBlack: '#333333', brightRed: '#ef4444', brightGreen: '#22c55e',
    brightYellow: '#eab308', brightBlue: '#3b82f6', brightMagenta: '#c084fc',
    brightCyan: '#22d3ee', brightWhite: '#f0f0f0',
  };

  const lightXtermTheme = {
    background: '#ffffff', foreground: '#1a1a1a', cursor: '#0a0a0a', cursorAccent: '#ffffff',
    selectionBackground: 'rgba(29, 78, 216, 0.25)', selectionForeground: '#000000',
    black: '#1a1a1a', red: '#b91c1c', green: '#15803d', yellow: '#a16207',
    blue: '#1d4ed8', magenta: '#7e22ce', cyan: '#0e7490', white: '#e5e5e5',
    brightBlack: '#525252', brightRed: '#dc2626', brightGreen: '#16a34a',
    brightYellow: '#ca8a04', brightBlue: '#3b82f6', brightMagenta: '#a855f7',
    brightCyan: '#06b6d4', brightWhite: '#fafafa',
  };

  let { adapter, ptyId }: { adapter: Adapter; ptyId: string } = $props();

  let containerEl: HTMLDivElement;
  let term: Terminal;
  let fitAddon: FitAddon;
  let searchAddon: SearchAddon;
  let cleanups: Array<() => void> = [];
  let searchVisible = $state(false);
  let searchQuery = $state('');

  onMount(() => {
    term = new Terminal({
      fontFamily: $fontFamily,
      fontSize: $fontSize,
      lineHeight: 1.3,
      cursorBlink: false,
      cursorStyle: 'block',
      cursorInactiveStyle: 'bar',
      theme: $isDark ? darkXtermTheme : lightXtermTheme,
      allowProposedApi: true,
    });

    fitAddon = new FitAddon();
    searchAddon = new SearchAddon();
    const webLinksAddon = new WebLinksAddon();
    const serializeAddon = new SerializeAddon();
    const imageAddon = new ImageAddon();
    term.loadAddon(fitAddon);
    term.loadAddon(webLinksAddon);
    term.loadAddon(searchAddon);
    term.loadAddon(serializeAddon);
    term.loadAddon(imageAddon);
    term.open(containerEl);

    // Try WebGL renderer for GPU-accelerated rendering, fall back to DOM
    (async () => {
      try {
        const { WebglAddon } = await import('@xterm/addon-webgl');
        const webglAddon = new WebglAddon();
        webglAddon.onContextLoss(() => {
          webglAddon.dispose();
        });
        term.loadAddon(webglAddon);
      } catch {
        // WebGL not available, DOM renderer is fine
      }
    })();

    fitAddon.fit();
    term.focus();

    // Retry focus to ensure cursor renders
    requestAnimationFrame(() => {
      term.focus();
      fitAddon.fit();
    });

    // Handle Ctrl+V paste from clipboard
    term.attachCustomKeyEventHandler((event: KeyboardEvent) => {
      if (event.type === 'keydown' && event.ctrlKey && event.key === 'v') {
        adapter.getClipboard().then((text) => {
          adapter.writePty(ptyId, text);
        }).catch((e) => console.warn('Clipboard paste failed:', e));
        return false;
      }
      // Handle Ctrl+C for copy when there's a selection
      if (event.type === 'keydown' && event.ctrlKey && event.key === 'c' && term.hasSelection()) {
        adapter.setClipboard(term.getSelection()).catch((e) => console.warn('Clipboard copy failed:', e));
        return false;
      }
      // Handle Ctrl+F for find in terminal
      if (event.type === 'keydown' && event.ctrlKey && event.key === 'f') {
        searchVisible = !searchVisible;
        return false;
      }
      return true;
    });

    // Wire input: terminal → PTY
    const onDataDispose = term.onData((data) => {
      adapter.writePty(ptyId, data);
    });

    // Wire resize: terminal → PTY
    const onResizeDispose = term.onResize(({ cols, rows }) => {
      adapter.resizePty(ptyId, cols, rows);
    });

    // Strip ANSI escape sequences from terminal output
    function stripAnsi(str: string): string {
      return str.replace(/\x1b\[[0-9;]*[a-zA-Z]/g, '');
    }

    // Parse context/token/model/messages/reset data from LLM CLI output
    function parseContextToken(data: string) {
      const clean = stripAnsi(data);

      const ctxMatch = clean.match(/(?:ctx|context)[^\d]*?(\d{1,3})%/i);
      const tokenMatch = clean.match(/([\d.]+[km]?)\s*tokens/i);
      const modelMatch = clean.match(/model:\s*([\w-]+)/i);
      const msgMatch = clean.match(/(?:messages?\s*left|remaining):\s*(\d+)/i);
      const resetMatch = clean.match(/(?:reset(?:s)?\s*in|resets?\s*in):\s*([\dhm\s]+)/i);

      if (ctxMatch || tokenMatch || modelMatch || msgMatch || resetMatch) {
        terminalTabs.update(tabs => tabs.map(tab => ({
          ...tab,
          instances: tab.instances.map(inst => {
            if (inst.id !== ptyId) return inst;
            return {
              ...inst,
              ...(ctxMatch ? { contextPercent: parseInt(ctxMatch[1], 10) } : {}),
              ...(tokenMatch ? { tokenCount: tokenMatch[1] } : {}),
              ...(modelMatch ? { modelName: modelMatch[1] } : {}),
              ...(msgMatch ? { messagesLeft: parseInt(msgMatch[1], 10) } : {}),
              ...(resetMatch ? { resetTimer: resetMatch[1].trim() } : {}),
            };
          })
        })));
      }
    }

    // Parse ConEmu progress sequences (ESC ] 9 ; 4 ; state ; value BEL)
    function parseProgress(data: string) {
      const match = data.match(/\x1b\]9;4;(\d);(\d{0,3})\x07/);
      if (match) {
        const state = parseInt(match[1], 10);
        const value = parseInt(match[2], 10) || 0;
        terminalTabs.update(tabs => tabs.map(tab => ({
          ...tab,
          instances: tab.instances.map(inst => {
            if (inst.id !== ptyId) return inst;
            return { ...inst, progressState: state, progressValue: value };
          })
        })));
      }
    }

    // Listen to PTY data → write to terminal
    adapter.onPtyData(ptyId, (data) => {
      term.write(data);
      parseContextToken(data);
      parseProgress(data);
    }).then((unlisten) => {
      cleanups.push(unlisten);
    });

    // Listen to PTY exit
    adapter.onPtyExit(ptyId, (code) => {
      term.write(`\r\n\x1b[90m[Process exited with code ${code}]\x1b[0m\r\n`);
    }).then((unlisten) => {
      cleanups.push(unlisten);
    });

    // ResizeObserver for auto-fit
    const resizeObserver = new ResizeObserver(() => {
      try {
        fitAddon.fit();
      } catch {
        // ignore if terminal not yet ready
      }
    });
    resizeObserver.observe(containerEl);

    cleanups.push(() => {
      onDataDispose.dispose();
      onResizeDispose.dispose();
      resizeObserver.disconnect();
      term.dispose();
    });

    return () => {
      for (const cleanup of cleanups) {
        try { cleanup(); } catch { /* ignore */ }
      }
    };
  });

  // React to Enhanced Readability font size changes
  $effect(() => {
    const on = $enhancedReadability;
    if (term) {
      term.options.fontSize = on ? 16 : $fontSize;
      fitAddon?.fit();
    }
  });

  // React to font family/size store changes
  $effect(() => {
    const ff = $fontFamily;
    const fs = $fontSize;
    if (term) {
      term.options.fontFamily = ff;
      if (!$enhancedReadability) {
        term.options.fontSize = fs;
      }
      fitAddon?.fit();
    }
  });

  // React to theme changes (dark/light)
  $effect(() => {
    const dark = $isDark;
    if (term) {
      term.options.theme = dark ? darkXtermTheme : lightXtermTheme;
    }
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="terminal-wrapper">
  {#if searchVisible}
    <div class="terminal-search">
      <input
        type="text"
        placeholder="Find in terminal..."
        bind:value={searchQuery}
        oninput={() => searchAddon?.findNext(searchQuery)}
        onkeydown={(e) => {
          if (e.key === 'Enter') {
            if (e.shiftKey) searchAddon?.findPrevious(searchQuery);
            else searchAddon?.findNext(searchQuery);
          }
          if (e.key === 'Escape') { searchVisible = false; term?.focus(); }
        }}
      />
      <button onclick={() => searchAddon?.findPrevious(searchQuery)} aria-label="Previous match">&#9650;</button>
      <button onclick={() => searchAddon?.findNext(searchQuery)} aria-label="Next match">&#9660;</button>
      <button onclick={() => { searchVisible = false; term?.focus(); }} aria-label="Close search">&#10005;</button>
    </div>
  {/if}
  <div class="terminal-container" bind:this={containerEl} onclick={() => term?.focus()}></div>
</div>

<style>
  .terminal-wrapper {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .terminal-search {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    background: var(--bg-secondary);
    border-bottom: 2px solid var(--border);
    flex-shrink: 0;
  }

  .terminal-search input {
    flex: 1;
    background: var(--bg-primary);
    border: 2px solid var(--border);
    border-radius: 0;
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: var(--font-size-small);
    padding: 2px 8px;
    outline: none;
  }

  .terminal-search input:focus {
    border-color: var(--accent);
  }

  .terminal-search button {
    background: var(--bg-tertiary);
    border: 2px solid var(--border);
    border-radius: 0;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 10px;
    padding: 2px 6px;
    font-weight: 800;
  }

  .terminal-search button:hover {
    background: var(--accent);
    color: #fff;
  }

  .terminal-container {
    width: 100%;
    flex: 1;
    overflow: hidden;
    background: var(--bg-surface);
    min-height: 0;
  }

  :global(.terminal-container .xterm) {
    height: 100%;
  }

  :global(.terminal-container .xterm-viewport) {
    overflow-y: auto !important;
  }
</style>
