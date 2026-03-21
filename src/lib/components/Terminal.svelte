<script lang="ts">
  import { onMount } from 'svelte';
  import { Terminal } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import { WebLinksAddon } from '@xterm/addon-web-links';
  import '@xterm/xterm/css/xterm.css';
  import type { Adapter } from '$lib/adapter/index';

  let { adapter, ptyId }: { adapter: Adapter; ptyId: string } = $props();

  let containerEl: HTMLDivElement;
  let term: Terminal;
  let fitAddon: FitAddon;
  let cleanups: Array<() => void> = [];

  onMount(() => {
    term = new Terminal({
      fontFamily: '"JetBrains Mono", "Fira Code", monospace',
      fontSize: 13,
      lineHeight: 1.2,
      cursorBlink: true,
      theme: {
        background: '#1a1a2e',
        foreground: '#e0e0e0',
        cursor: '#e0e0e0',
        black: '#1a1a2e',
        red: '#ff5555',
        green: '#50fa7b',
        yellow: '#f1fa8c',
        blue: '#6272a4',
        magenta: '#ff79c6',
        cyan: '#8be9fd',
        white: '#f8f8f2',
        brightBlack: '#44475a',
        brightRed: '#ff6e6e',
        brightGreen: '#69ff94',
        brightYellow: '#ffffa5',
        brightBlue: '#d6acff',
        brightMagenta: '#ff92df',
        brightCyan: '#a4ffff',
        brightWhite: '#ffffff',
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

    // Listen to PTY data → write to terminal
    adapter.onPtyData(ptyId, (data) => {
      term.write(data);
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
    background: #1a1a2e;
  }

  :global(.terminal-container .xterm) {
    height: 100%;
  }

  :global(.terminal-container .xterm-viewport) {
    overflow-y: auto !important;
  }
</style>
