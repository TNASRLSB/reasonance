<script lang="ts">
  let { text }: { text: string } = $props();
</script>

<!-- Phase 5: replace with proper markdown rendering (marked/remark) -->
<div class="text-block">
  {#each text.split('\n') as line, i}
    {#if i > 0}<br />{/if}
    {line}
  {/each}
</div>

<style>
  .text-block {
    font-family: var(--font-ui);
    font-size: var(--font-size-base);
    color: var(--text-body);
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
