<script lang="ts">
  import { tr } from '$lib/i18n/index';
  import { recentProjects } from '$lib/stores/files';

  let { onOpenFolder, onSelectProject }: {
    onOpenFolder: () => void;
    onSelectProject: (path: string) => void;
  } = $props();
</script>

<div class="welcome">
  <div class="welcome-content">
    <h1 class="welcome-logo">REASONANCE</h1>
    <p class="welcome-subtitle">IDE for Vibecoders</p>

    <button class="welcome-btn primary" onclick={onOpenFolder}>
      {$tr('welcome.openFolder')}
    </button>

    <div class="recent-section">
      <h2 class="recent-title">{$tr('welcome.recentProjects')}</h2>
      {#if $recentProjects.length === 0}
        <p class="no-recent">{$tr('welcome.noRecent')}</p>
      {:else}
        <ul class="recent-list">
          {#each $recentProjects as project}
            <li>
              <button class="recent-item" onclick={() => onSelectProject(project)}>
                {project}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </div>
</div>

<style>
  .welcome {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    background: var(--bg-primary);
    font-family: var(--font-ui);
  }

  .welcome-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    max-width: 480px;
    width: 100%;
    padding: 32px;
  }

  .welcome-logo {
    font-weight: 800;
    font-size: 32px;
    color: var(--text-primary);
    letter-spacing: -0.02em;
    margin: 0;
    font-family: var(--font-ui);
  }

  .welcome-subtitle {
    color: var(--text-muted);
    font-size: 14px;
    margin: 0 0 24px;
    font-family: var(--font-ui);
  }

  .welcome-btn.primary {
    padding: 12px 32px;
    font-family: var(--font-ui);
    font-size: 14px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    border: 2px solid var(--accent);
    border-radius: 0;
    background: transparent;
    color: var(--accent);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .welcome-btn.primary:hover {
    background: var(--accent);
    color: var(--bg-primary);
  }

  .recent-section {
    width: 100%;
    margin-top: 32px;
  }

  .recent-title {
    font-size: 12px;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: var(--text-muted);
    margin: 0 0 12px;
    font-family: var(--font-ui);
  }

  .no-recent {
    color: var(--text-muted);
    font-size: 13px;
    margin: 0;
    font-family: var(--font-ui);
  }

  .recent-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .recent-item {
    display: block;
    width: 100%;
    padding: 8px 12px;
    font-family: var(--font-ui);
    font-size: 13px;
    text-align: left;
    color: var(--text-body);
    background: none;
    border: none;
    border-radius: 0;
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .recent-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
</style>
