<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onDestroy, onMount } from 'svelte';

  let candidates = $state<string[]>([]);
  let interval: ReturnType<typeof setInterval>;

  async function refresh() {
    try {
      const r = await invoke<{ candidates: string[] }>('list_staging_candidates');
      candidates = r.candidates ?? [];
    } catch {}
  }

  async function promote(path: string) {
    try {
      await invoke('promote_staging_node', { stagingPath: path });
      await refresh();
    } catch (e) { console.error('[Staging] promote failed', e); }
  }

  async function reject(path: string) {
    try {
      await invoke('reject_staging_node', { stagingPath: path });
      await refresh();
    } catch (e) { console.error('[Staging] reject failed', e); }
  }

  onMount(() => { void refresh(); interval = setInterval(refresh, 5000); });
  onDestroy(() => clearInterval(interval));
</script>

{#if candidates.length > 0}
  <div class="staging-panel">
    <div class="staging-panel__header">Staging ({candidates.length})</div>
    {#each candidates as path (path)}
      <div class="staging-row">
        <span class="staging-name" title={path}>{path.split(/[\\/]/).pop()}</span>
        <button type="button" class="btn btn--icon" title="Promote to vault" on:click={() => promote(path)}>✓</button>
        <button type="button" class="btn btn--icon" title="Reject" on:click={() => reject(path)}>✗</button>
      </div>
    {/each}
  </div>
{/if}

<style>
  .staging-panel {
    border: 1px solid var(--border, #444);
    border-radius: 4px;
    padding: 6px 8px;
    margin: 4px 0;
    font-size: 0.75rem;
    background: var(--bg-secondary, #1a1a1a);
  }
  .staging-panel__header {
    font-weight: 600;
    margin-bottom: 4px;
    opacity: 0.7;
  }
  .staging-row {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 0;
  }
  .staging-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    opacity: 0.85;
  }
</style>
