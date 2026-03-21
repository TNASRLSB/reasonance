<script lang="ts">
  import { onMount } from 'svelte';
  import { Terminal } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import { WebLinksAddon } from '@xterm/addon-web-links';
  import '@xterm/xterm/css/xterm.css';
  import type { Adapter } from '$lib/adapter/index';
  import { terminalTabs } from '$lib/stores/terminals';
  import { enhancedReadability } from '$lib/stores/ui';
  import { get } from 'svelte/store';
  import { isDark } from '$lib/stores/theme';

  const darkXtermTheme = {
    background: '#121212', foreground: '#d4d4d4', cursor: '#f0f0f0', cursorAccent: '#121212',
    selectionBackground: 'rgba(29, 78, 216, 0.4)', selectionForeground: '#ffffff',
    black: '#121212', red: '#dc2626', green: '#16a34a', yellow: '#ca8a04',
    blue: '#1d4ed8', magenta: '#a855f7', cyan: '#06b6d4', white: '#d4d4d4',
    brightBlack: '#333333', brightRed: '#ef4444', brightGreen: '#22c55e',
    brightYellow: '#eab308', brightBlue: '#3b82f6', brightMagenta: '#c084fc',
    brightCyan: '#22d3ee', brightWhite: '#f0f0f0',
  };

  const lightXtermTheme = {
    background: '#fafafa', foreground: '#1a1a1a', cursor: '#0a0a0a', cursorAccent: '#fafafa',
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
  let cleanups: Array<() => void> = [];

  onMount(() => {
    term = new Terminal({
      fontFamily: "'Atkinson Hyperlegible Mono', 'JetBrains Mono', 'Fira Code', monospace",
      fontSize: 12,
      lineHeight: 1.3,
      cursorBlink: true,
      theme: get(isDark) ? darkXtermTheme : lightXtermTheme,
      allowProposedApi: true,
    });

    fitAddon = new FitAddon();
    const webLinksAddon = new WebLinksAddon();
    term.loadAddon(fitAddon);
    term.loadAddon(webLinksAddon);
    term.open(containerEl);
    fitAddon.fit();

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

    // Listen to PTY data → write to terminal
    adapter.onPtyData(ptyId, (data) => {
      term.write(data);
      parseContextToken(data);
    }).then((unlisten) => {
      cleanups.push(unlisten);
    });

    // Listen to PTY exit
    adapter.onPtyExit(ptyId, (code) => {
      term.write(`\r\n\x1b[90m[Process exited with code ${code}]\x1b[0m\r\n`);
    }).then((unlisten) => {
      cleanups.push(unlisten);
    });

    // React to Enhanced Readability font size changes
    const unsubER = enhancedReadability.subscribe((on) => {
      if (term) {
        term.options.fontSize = on ? 15 : 12;
        fitAddon.fit();
      }
    });
    cleanups.push(unsubER);

    // React to theme changes (dark/light)
    const unsubTheme = isDark.subscribe((dark) => {
      if (term) {
        term.options.theme = dark ? darkXtermTheme : lightXtermTheme;
      }
    });
    cleanups.push(unsubTheme);

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
</script>

<div class="terminal-container" bind:this={containerEl}></div>

<style>
  .terminal-container {
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: var(--bg-surface);
  }

  :global(.terminal-container .xterm) {
    height: 100%;
  }

  :global(.terminal-container .xterm-viewport) {
    overflow-y: auto !important;
  }
</style>
