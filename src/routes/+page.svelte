<script lang="ts">
  import App from '$lib/components/App.svelte';
  import { initTheme } from '$lib/stores/theme';
  import { onMount } from 'svelte';
  import '../app.css';

  onMount(() => {
    initTheme();
  });
</script>

<App />
