<script lang="ts">
  import { get } from 'svelte/store';
  import { tr } from '$lib/i18n/index';
  import { activeThemeName, loadBuiltinTheme, toggleModifier } from '$lib/stores/theme';
  import { showThemeEditor } from '$lib/stores/ui';
  import { activeInstanceId } from '$lib/stores/terminals';
  import { recentProjectsList, addProject, removeProject, activeProjectId } from '$lib/stores/projects';
  import type { Adapter } from '$lib/adapter/index';
  import type { MenuItemDef } from '$lib/types/menu';
  import MenuItem from './MenuItem.svelte';

  let { adapter }: { adapter: Adapter } = $props();

  let openIndex = $state<number | null>(null);
  let menuBarEl: HTMLDivElement | undefined = $state();

  async function sendGitCommand(command: string) {
    const id = get(activeInstanceId);
    if (!id) return;
    await adapter.writePty(id, command);
  }

  function closeAll() {
    openIndex = null;
  }

  function handleClickOutside(e: MouseEvent) {
    if (openIndex !== null && menuBarEl && !menuBarEl.contains(e.target as Node)) {
      closeAll();
    }
  }

  function handleGlobalKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && openIndex !== null) {
      closeAll();
    }
  }

  $effect(() => {
    document.addEventListener('click', handleClickOutside, true);
    document.addEventListener('keydown', handleGlobalKeydown);
    return () => {
      document.removeEventListener('click', handleClickOutside, true);
      document.removeEventListener('keydown', handleGlobalKeydown);
    };
  });

  const menus: Array<{ key: string; itemsFn: () => MenuItemDef[] }> = [
    {
      key: 'menu.file',
      itemsFn: () => [
        {
          label: $tr('menu.file.openFolder'),
          action: () => dispatchEvent(new CustomEvent('reasonance:openFolder')),
        },
        {
          label: $tr('menu.file.recent'),
          submenu: $recentProjectsList.length > 0
            ? $recentProjectsList.map(item => ({ label: item.label, action: () => addProject(item.path) }))
            : [{ label: '(none)', action: () => {} }],
        },
        { divider: true },
        { label: $tr('menu.file.save'), shortcut: 'Ctrl+S', action: () => document.dispatchEvent(new CustomEvent('reasonance:save')) },
        { label: $tr('menu.file.saveAll'), shortcut: 'Ctrl+Shift+S', action: () => document.dispatchEvent(new CustomEvent('reasonance:saveAll')) },
        { label: $tr('menu.file.closeFile'), shortcut: 'Ctrl+W', action: () => document.dispatchEvent(new CustomEvent('reasonance:closeFile')) },
        { label: $tr('menu.file.closeProject'), action: () => document.dispatchEvent(new CustomEvent('reasonance:closeProject')) },
        { divider: true },
        { label: $tr('menu.file.exit'), action: () => window.close() },
      ],
    },
    {
      key: 'menu.edit',
      itemsFn: () => [
        { label: $tr('menu.edit.undo'), shortcut: 'Ctrl+Z', action: () => document.execCommand('undo') },
        { label: $tr('menu.edit.redo'), shortcut: 'Ctrl+Shift+Z', action: () => document.execCommand('redo') },
        { divider: true },
        { label: $tr('menu.edit.cut'), shortcut: 'Ctrl+X', action: () => document.execCommand('cut') },
        { label: $tr('menu.edit.copy'), shortcut: 'Ctrl+C', action: () => document.execCommand('copy') },
        { label: $tr('menu.edit.paste'), shortcut: 'Ctrl+V', action: () => adapter.getClipboard().then(t => document.execCommand('insertText', false, t)).catch((e) => console.warn('Clipboard paste failed:', e)) },
        { divider: true },
        { label: $tr('menu.edit.find'), shortcut: 'Ctrl+F', action: () => document.dispatchEvent(new CustomEvent('reasonance:findInFile')) },
        { label: $tr('menu.edit.findInFiles'), shortcut: 'Ctrl+Shift+F', action: () => document.dispatchEvent(new CustomEvent('reasonance:findInFiles')) },
      ],
    },
    {
      key: 'menu.view',
      itemsFn: () => [
        {
          label: $tr('menu.view.theme'),
          submenu: [
            { label: 'Reasonance Dark', action: () => loadBuiltinTheme('reasonance-dark') },
            { label: 'Reasonance Light', action: () => loadBuiltinTheme('reasonance-light') },
          ],
        },
        {
          label: $tr('menu.view.modifiers'),
          submenu: [
            { label: $tr('menu.view.readability'), action: () => toggleModifier('enhanced-readability') },
          ],
        },
        { divider: true },
        { label: $tr('menu.view.themeEditor'), action: () => showThemeEditor.set(true) },
        { divider: true },
        { label: $tr('menu.view.filePanel'), action: () => document.dispatchEvent(new CustomEvent('reasonance:toggleFilePanel')) },
        { label: $tr('menu.view.terminalPanel'), action: () => document.dispatchEvent(new CustomEvent('reasonance:toggleTerminalPanel')) },
        { divider: true },
        { label: $tr('menu.view.zoomIn'), shortcut: 'Ctrl++', action: () => document.dispatchEvent(new CustomEvent('reasonance:zoomIn')) },
        { label: $tr('menu.view.zoomOut'), shortcut: 'Ctrl+-', action: () => document.dispatchEvent(new CustomEvent('reasonance:zoomOut')) },
      ],
    },
    {
      key: 'menu.terminal',
      itemsFn: () => [
        {
          label: $tr('menu.terminal.newLLM'),
          action: () => document.dispatchEvent(new CustomEvent('reasonance:newTerminal')),
        },
        { label: $tr('menu.terminal.close'), action: () => document.dispatchEvent(new CustomEvent('reasonance:closeTerminal')) },
        { divider: true },
        { label: $tr('menu.terminal.detectLLM'), action: () => document.dispatchEvent(new CustomEvent('reasonance:detectLLMs')) },
      ],
    },
    {
      key: 'menu.git',
      itemsFn: () => [
        { label: $tr('menu.git.status'), action: () => sendGitCommand('git status\n') },
        { label: $tr('menu.git.commit'), action: () => sendGitCommand('git commit -m ""') },
        { label: $tr('menu.git.push'), action: () => { if (confirm('Push to remote?\n\nThis will run: git push\n\nProceed?')) sendGitCommand('git push\n'); } },
        { label: $tr('menu.git.pull'), action: () => sendGitCommand('git pull\n') },
        { label: $tr('menu.git.log'), action: () => sendGitCommand('git log --oneline -20\n') },
      ],
    },
    {
      key: 'menu.help',
      itemsFn: () => [
        { label: $tr('menu.help.docs'), shortcut: 'F1', action: () => document.dispatchEvent(new CustomEvent('reasonance:help')) },
        { label: $tr('menu.help.shortcuts'), action: () => document.dispatchEvent(new CustomEvent('reasonance:shortcuts')) },
        { divider: true },
        { label: $tr('menu.help.about'), action: () => document.dispatchEvent(new CustomEvent('reasonance:about')) },
      ],
    },
  ];
</script>

<div class="menu-bar" role="menubar" bind:this={menuBarEl}>
  {#each menus as menu, i}
    <MenuItem
      label={$tr(menu.key)}
      items={menu.itemsFn()}
      open={openIndex === i}
      onOpen={() => openIndex = openIndex === i ? null : i}
      onClose={closeAll}
      onHover={() => { if (openIndex !== null) openIndex = i; }}
    />
  {/each}
</div>

<style>
  .menu-bar {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 0;
    flex-wrap: nowrap;
    flex-shrink: 1;
    min-width: 0;
    height: 100%;
  }
</style>
