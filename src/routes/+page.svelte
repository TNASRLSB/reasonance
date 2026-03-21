<script lang="ts">
  import App from '$lib/components/App.svelte';
  import FileTree from '$lib/components/FileTree.svelte';
  import { TauriAdapter } from '$lib/adapter/tauri';
  import { initTheme } from '$lib/stores/theme';
  import { onMount } from 'svelte';
  import '../app.css';

  const adapter = new TauriAdapter();

  onMount(() => {
    initTheme();
  });
</script>

<App>
  {#snippet fileTree()}
    <FileTree {adapter} />
  {/snippet}
</App>
