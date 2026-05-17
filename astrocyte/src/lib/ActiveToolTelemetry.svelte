<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';

  type ToolRow = {
    callId: string;
    toolName: string;
    startedAtClientMs: number;
    status: 'running' | 'done';
    displaySec: number;
    finalStatusLabel: string;
    fading: boolean;
  };

  const LABELS: Record<string, string> = {
    search_vault: 'Searching vault',
    web_search: 'Searching the web',
    read_vault_file: 'Reading vault file',
    search_vault_attribute: 'Searching vault metadata',
    obsidian_graph_query: 'Graph query',
    arxiv_miner: 'arXiv miner',
    daily_paper_pipeline: 'Daily pipeline',
    check_task_status: 'Task status',
  };

  function friendlyLabel(toolName: string): string {
    return LABELS[toolName] ?? toolName.replace(/_/g, ' ');
  }

  let rowsById = $state<Record<string, ToolRow>>({});
  let unStart: UnlistenFn | null = null;
  let unDone: UnlistenFn | null = null;
  let ticker: ReturnType<typeof setInterval> | null = null;
  const removeTimers: Record<string, ReturnType<typeof setTimeout>> = {};

  const ordered = $derived(
    Object.values(rowsById).sort((a, b) => a.startedAtClientMs - b.startedAtClientMs)
  );

  function pulseRunning(): void {
    const now = Date.now();
    let next = { ...rowsById };
    let touched = false;
    for (const id of Object.keys(rowsById)) {
      const r = rowsById[id];
      if (r.status === 'running') {
        const sec = Math.max(0, (now - r.startedAtClientMs) / 1000);
        next[id] = { ...r, displaySec: sec };
        touched = true;
      }
    }
    if (touched) rowsById = next;
  }

  function scheduleRemove(callId: string): void {
    const prev = removeTimers[callId];
    if (prev !== undefined) clearTimeout(prev);

    const row = rowsById[callId];
    if (row) {
      rowsById = { ...rowsById, [callId]: { ...row, fading: true } };
    }

    removeTimers[callId] = setTimeout(() => {
      const rest = { ...rowsById };
      delete rest[callId];
      rowsById = rest;
      delete removeTimers[callId];
    }, 1000);
  }

  onMount(async () => {
    unStart = await listen<Record<string, unknown>>('bb-tool-start', (ev) => {
      const p = ev.payload;
      if (!p || typeof p.call_id !== 'string' || typeof p.tool_name !== 'string') return;
      const now = Date.now();
      rowsById = {
        ...rowsById,
        [p.call_id]: {
          callId: p.call_id,
          toolName: p.tool_name,
          startedAtClientMs: now,
          status: 'running',
          displaySec: 0,
          finalStatusLabel: '',
          fading: false,
        },
      };
    });

    unDone = await listen<Record<string, unknown>>('bb-tool-done', (ev) => {
      const p = ev.payload;
      if (!p || typeof p.call_id !== 'string') return;
      const st = typeof p.status === 'string' ? p.status : '?';
      const prev = rowsById[p.call_id];
      const now = Date.now();
      const startMs = prev?.startedAtClientMs ?? now;
      const displaySec = Math.max(0, (now - startMs) / 1000);
      rowsById = {
        ...rowsById,
        [p.call_id]: {
          callId: p.call_id,
          toolName: prev?.toolName ?? (typeof p.tool_name === 'string' ? p.tool_name : ''),
          startedAtClientMs: startMs,
          status: 'done',
          displaySec,
          finalStatusLabel: st,
          fading: false,
        },
      };
      scheduleRemove(p.call_id);
    });

    ticker = setInterval(pulseRunning, 100);
  });

  onDestroy(() => {
    unStart?.();
    unDone?.();
    if (ticker !== null) clearInterval(ticker);
    for (const t of Object.values(removeTimers)) {
      clearTimeout(t);
    }
  });
</script>

{#if ordered.length > 0}
  <section class="active-tool-telemetry" aria-live="polite">
    {#each ordered as row (row.callId)}
      <div
        class="tool-tel-row"
        class:tool-tel-row--done={row.status === 'done'}
        class:tool-tel-row--fading={row.fading}
      >
        <span class="tool-tel-label">{friendlyLabel(row.toolName)}</span>
        <span class="tool-tel-meta">{row.status === 'running' ? 'running' : row.finalStatusLabel}</span>
        <span class="tool-tel-elapsed">{row.displaySec.toFixed(1)}s</span>
      </div>
    {/each}
  </section>
{/if}

<style>
  .tool-tel-row {
    display: grid;
    grid-template-columns: 1fr auto auto;
    gap: var(--space-2);
    align-items: baseline;
    padding: var(--space-1) 0 var(--space-1) var(--space-2);
    border-left: 2px solid var(--astrocyte-neural-purple);
    transition: opacity 0.35s ease;
  }

  .tool-tel-row--done {
    border-left-color: var(--feedback-good);
    opacity: 0.88;
  }

  .tool-tel-row--fading {
    opacity: 0;
  }

  .tool-tel-label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--astrocyte-read-fg);
    font-size: var(--font-sm);
  }

  .tool-tel-meta {
    font-family: var(--font-mono);
    font-size: var(--font-xs);
    color: var(--astrocyte-purple-a-86);
    text-transform: lowercase;
    white-space: nowrap;
  }

  .tool-tel-elapsed {
    font-family: var(--font-mono);
    font-size: var(--font-sm);
    font-variant-numeric: tabular-nums;
    color: var(--astrocyte-neural-purple);
    white-space: nowrap;
  }
</style>
