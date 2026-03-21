<script lang="ts">
  import { onMount } from 'svelte';
  import { Terminal } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import { WebLinksAddon } from '@xterm/addon-web-links';
  import '@xterm/xterm/css/xterm.css';
  import type { Adapter } from '$lib/adapter/index';
  import { terminalTabs } from '$lib/stores/terminals';

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
      theme: {
        background: '#121212',
        foreground: '#d4d4d4',
        cursor: '#f0f0f0',
        cursorAccent: '#121212',
        selectionBackground: 'rgba(29, 78, 216, 0.4)',
        selectionForeground: '#ffffff',
        black: '#121212',
        red: '#dc2626',
        green: '#16a34a',
        yellow: '#ca8a04',
        blue: '#1d4ed8',
        magenta: '#a855f7',
        cyan: '#06b6d4',
        white: '#d4d4d4',
        brightBlack: '#333333',
        brightRed: '#ef4444',
        brightGreen: '#22c55e',
        brightYellow: '#eab308',
        brightBlue: '#3b82f6',
        brightMagenta: '#c084fc',
        brightCyan: '#22d3ee',
        brightWhite: '#f0f0f0',
      },
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

    // Parse context/token data from LLM CLI output
    // Claude Code outputs patterns like: "ctx ████░░░░ 42%" and "38.2k tokens"
    function parseContextToken(data: string) {
      const ctxMatch = data.match(/(?:ctx|context)[^\d]*?(\d{1,3})%/i);
      const tokenMatch = data.match(/([\d.]+[km]?)\s*tokens/i);

      if (ctxMatch || tokenMatch) {
        terminalTabs.update(tabs => tabs.map(tab => ({
          ...tab,
          instances: tab.instances.map(inst => {
            if (inst.id !== ptyId) return inst;
            return {
              ...inst,
              ...(ctxMatch ? { contextPercent: parseInt(ctxMatch[1], 10) } : {}),
              ...(tokenMatch ? { tokenCount: tokenMatch[1] } : {}),
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
    background: #121212;
  }

  :global(.terminal-container .xterm) {
    height: 100%;
  }

  :global(.terminal-container .xterm-viewport) {
    overflow-y: auto !important;
  }
</style>
