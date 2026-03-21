<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Adapter } from '$lib/adapter/index';
  import { get } from 'svelte/store';
  import { llmConfigs } from '$lib/stores/config';

  let {
    adapter,
    instanceId,
    llmName,
    children,
  }: {
    adapter: Adapter;
    instanceId: string;
    llmName: string;
    children: Snippet;
  } = $props();

  let dragOver = $state(false);

  function getImageMode(): 'path' | 'base64' | 'none' {
    const configs = get(llmConfigs);
    const config = configs.find((c) => c.name === llmName);
    return config?.imageMode ?? 'path';
  }

  function onDragOver(e: DragEvent) {
    if (!e.dataTransfer) return;
    const hasImage = Array.from(e.dataTransfer.items).some(
      (item) => item.kind === 'file' && item.type.startsWith('image/')
    );
    if (!hasImage) return;
    e.preventDefault();
    dragOver = true;
  }

  function onDragLeave() {
    dragOver = false;
  }

  async function onDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;

    const files = Array.from(e.dataTransfer?.files ?? []).filter((f) =>
      f.type.startsWith('image/')
    );
    if (files.length === 0) return;

    const mode = getImageMode();
    if (mode === 'none') return;

    for (const file of files) {
      if (mode === 'path') {
        // In Tauri, File objects have a webkitRelativePath or we can get the path via file.path (Tauri exposes it)
        const filePath = (file as File & { path?: string }).path ?? file.name;
        await adapter.writePty(instanceId, filePath + ' ');
      } else if (mode === 'base64') {
        const base64 = await readFileAsBase64(file);
        await adapter.writePty(instanceId, base64 + ' ');
      }
    }
  }

  function readFileAsBase64(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => {
        const result = reader.result as string;
        // Strip the data URL prefix (e.g. "data:image/png;base64,")
        const base64 = result.split(',')[1] ?? result;
        resolve(base64);
      };
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(file);
    });
  }
</script>

<div
  class="image-drop-wrapper"
  class:drag-over={dragOver}
  ondragover={onDragOver}
  ondragleave={onDragLeave}
  ondrop={onDrop}
  role="region"
  aria-label="Terminal — drop images here"
>
  {@render children()}
  {#if dragOver}
    <div class="drop-overlay">Drop image to paste</div>
  {/if}
</div>

<style>
  .image-drop-wrapper {
    position: relative;
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
  }

  .drop-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(124, 106, 247, 0.15);
    border: 2px dashed var(--accent, #7c6af7);
    border-radius: 4px;
    font-size: 14px;
    font-weight: 600;
    color: var(--accent, #7c6af7);
    pointer-events: none;
    z-index: 10;
  }
</style>
