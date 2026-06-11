<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event';

  type SessionSummary = {
    id: string;
    timestamp: string;
    first_user_message_snippet: string;
  };
  type TimelineNode = {
    session: SessionSummary;
    gap: number;
  };

  const TOOLTIP_WIDTH = 200;
  const TOOLTIP_GAP = 10;
  const HOVER_BRIDGE_MS = 120;

  let isVisible = $state(false);
  let mouseInside = $state(false);
  let hideTimeout: ReturnType<typeof setTimeout> | null = null;
  let unlistenSessionListChanged: UnlistenFn | null = null;

  let sessionSummaries = $state<SessionSummary[]>([]);
  let timelineNodes = $derived(buildTimelineNodes(sessionSummaries));

  let scrollEl = $state<HTMLDivElement | null>(null);
  let hoveredSessionId = $state<string | null>(null);
  let tooltipOpen = $state(false);
  let tooltipLeft = $state(0);
  let tooltipTop = $state(0);
  let tooltipLeaveTimer: ReturnType<typeof setTimeout> | null = null;

  let tooltipSession = $derived(
    hoveredSessionId
      ? sessionSummaries.find((s) => s.id === hoveredSessionId) ?? null
      : null
  );

  const HIDE_DELAY_MS = 1000;
  const HIDE_ANIM_MS = 300;

  function onWindowFocus() {
    cancelHideTimer();
    isVisible = true;
    if (!mouseInside) startHideTimer();
    void refreshSessions();
  }

  function startHideTimer() {
    if (hideTimeout) clearTimeout(hideTimeout);
    hideTimeout = setTimeout(() => {
      hideTimeout = null;
      isVisible = false;
      setTimeout(() => {
        if (!isVisible) void invoke('hide_timeline').catch(console.error);
      }, HIDE_ANIM_MS);
    }, HIDE_DELAY_MS);
  }

  function cancelHideTimer() {
    if (hideTimeout) {
      clearTimeout(hideTimeout);
      hideTimeout = null;
    }
  }

  function clearTooltipLeaveTimer() {
    if (tooltipLeaveTimer) {
      clearTimeout(tooltipLeaveTimer);
      tooltipLeaveTimer = null;
    }
  }

  function scheduleTooltipHide() {
    clearTooltipLeaveTimer();
    tooltipLeaveTimer = setTimeout(() => {
      tooltipLeaveTimer = null;
      hoveredSessionId = null;
      tooltipOpen = false;
    }, HOVER_BRIDGE_MS);
  }

  function positionTooltipFromNodeEl(nodeEl: HTMLElement) {
    const dot = nodeEl.querySelector('.dot');
    if (!dot) return;
    const r = dot.getBoundingClientRect();
    let left = r.left - TOOLTIP_WIDTH - TOOLTIP_GAP;
    if (left < 8) left = 8;
    const top = r.top + r.height / 2;
    tooltipLeft = left;
    tooltipTop = top;
  }

  function onNodePointerEnter(sessionId: string, el: HTMLElement) {
    clearTooltipLeaveTimer();
    hoveredSessionId = sessionId;
    positionTooltipFromNodeEl(el);
    tooltipOpen = true;
  }

  function onNodePointerLeave() {
    scheduleTooltipHide();
  }

  function onTooltipPointerEnter() {
    clearTooltipLeaveTimer();
  }

  function onTooltipPointerLeave() {
    scheduleTooltipHide();
  }

  function onScrollReposition() {
    if (!hoveredSessionId || !scrollEl) return;
    const el = scrollEl.querySelector<HTMLElement>(
      `[data-session-id="${CSS.escape(hoveredSessionId)}"]`
    );
    if (el) positionTooltipFromNodeEl(el);
  }

  function formatTimestamp(ts: string): string {
    const d = new Date(ts);
    if (Number.isNaN(d.getTime())) return ts;
    return d.toLocaleString();
  }

  function getTimestampMs(ts: string): number {
    const ms = Date.parse(ts);
    return Number.isFinite(ms) ? ms : 0;
  }

  function getTemporalGapPx(deltaMs: number): number {
    if (deltaMs <= 0) return 14;
    const normalized = deltaMs / 60_000;
    const logarithmic = Math.log1p(normalized) * 9;
    return Math.max(10, Math.min(88, Math.round(logarithmic)));
  }

  function buildTimelineNodes(summaries: SessionSummary[]): TimelineNode[] {
    if (!summaries.length) return [];
    const ordered = [...summaries].sort(
      (a, b) => getTimestampMs(a.timestamp) - getTimestampMs(b.timestamp)
    );
    return ordered.map((session, i) => {
      if (i === 0) return { session, gap: 0 };
      const delta = Math.abs(
        getTimestampMs(session.timestamp) - getTimestampMs(ordered[i - 1].timestamp)
      );
      return { session, gap: getTemporalGapPx(delta) };
    });
  }

  async function refreshSessions() {
    try {
      sessionSummaries = await invoke<SessionSummary[]>('get_session_history');
    } catch (e) {
      console.error('get_session_history failed', e);
    }
  }

  async function onNodeClick(sessionId: string) {
    try {
      await invoke('load_session_into_main', { sessionId });
    } catch (e) {
      console.error('load_session_into_main failed', e);
    }
  }

  async function onDeleteSession(sessionId: string, e: MouseEvent) {
    e.stopPropagation();
    e.preventDefault();
    try {
      await invoke('delete_session_history', { sessionId });
      sessionSummaries = sessionSummaries.filter((s) => s.id !== sessionId);
      if (hoveredSessionId === sessionId) {
        hoveredSessionId = null;
        tooltipOpen = false;
      }
      await emit('session-deleted', sessionId);
    } catch (e) {
      console.error('delete_session_history failed', e);
    }
  }

  async function onNewSignal() {
    try {
      await invoke('new_signal_in_main');
    } catch (e) {
      console.error('new_signal_in_main failed', e);
    }
  }

  onMount(async () => {
    document.documentElement.classList.add('timeline-route-host');
    window.addEventListener('focus', onWindowFocus);
    window.addEventListener('resize', onScrollReposition);
    void refreshSessions();
    unlistenSessionListChanged = await listen('session-list-changed', () => {
      void refreshSessions();
    });
  });

  onDestroy(() => {
    cancelHideTimer();
    clearTooltipLeaveTimer();
    document.documentElement.classList.remove('timeline-route-host');
    window.removeEventListener('focus', onWindowFocus);
    window.removeEventListener('resize', onScrollReposition);
    if (unlistenSessionListChanged) {
      unlistenSessionListChanged();
      unlistenSessionListChanged = null;
    }
  });
</script>

<div
  class="fade-container"
  class:visible={isVisible}
  role="presentation"
>
  {#if tooltipSession}
    <div
      class="tooltip-float"
      class:open={tooltipOpen}
      style:left="{tooltipLeft}px"
      style:top="{tooltipTop}px"
      role="tooltip"
      onpointerenter={onTooltipPointerEnter}
      onpointerleave={onTooltipPointerLeave}
    >
      <span class="time">{formatTimestamp(tooltipSession.timestamp)}</span>
      <span class="snippet">{tooltipSession.first_user_message_snippet || '[NO_USER_MESSAGE]'}</span>
      <button
        class="tooltip-delete"
        type="button"
        title="Delete session"
        onclick={(e) => {
          if (tooltipSession) onDeleteSession(tooltipSession.id, e);
        }}
      >X</button>
    </div>
  {/if}

  <div
    class="timeline-container"
    class:show={isVisible}
    onmouseenter={() => {
      mouseInside = true;
      cancelHideTimer();
    }}
    onmouseleave={() => {
      mouseInside = false;
      startHideTimer();
    }}
    role="presentation"
  >
  <div
    class="timeline-rail"
    role="presentation"
  >
    <main class="timeline-shell">
      <div class="timeline-drag" data-tauri-drag-region aria-hidden="true"></div>
      <div class="timeline-laser" aria-hidden="true"></div>

      <div
        class="timeline-scroll"
        bind:this={scrollEl}
        onscroll={onScrollReposition}
      >
        {#each timelineNodes as node (node.session.id)}
          <div
            class="timeline-node"
            data-session-id={node.session.id}
            style="margin-top: {node.gap}px;"
            onclick={() => onNodeClick(node.session.id)}
            onkeydown={(e) => e.key === 'Enter' && onNodeClick(node.session.id)}
            onpointerenter={(e) => onNodePointerEnter(node.session.id, e.currentTarget as HTMLElement)}
            onpointerleave={onNodePointerLeave}
            role="button"
            tabindex="0"
          >
            <div class="dot"></div>
          </div>
        {/each}
      </div>

      <button class="timeline-btn" type="button" onclick={onNewSignal}>
        <span class="icon">[+]</span>
        <span class="label" aria-hidden="true">N</span>
      </button>
    </main>
  </div>
  </div>
</div>

<style>
  /*
   * Global annihilation — scoped to timeline Webview via html.timeline-route-host
   * (same intent as bare :global(html/body), without nuking the main app in dev).
   */
  :global(html.timeline-route-host),
  :global(html.timeline-route-host body) {
    background-color: transparent !important;
    background: transparent !important;
    backdrop-filter: none !important;
    -webkit-backdrop-filter: none !important;
    box-shadow: none !important;
    margin: 0;
    padding: 0;
    overflow: hidden !important;
    width: 100%;
    height: 100%;
    pointer-events: none;
    color-scheme: dark;
  }

  .timeline-shell {
    --rail-w: 38px;
    position: relative;
    width: var(--rail-w);
    height: 100vh;
    margin: 0;
    display: flex;
    flex-direction: column;
    background: transparent !important;
    color: var(--astrocyte-neural-purple);
    overflow: visible;
    box-sizing: border-box;
    pointer-events: none;
  }

  .fade-container {
    position: fixed;
    inset: 0;
    background: transparent !important;
    background-color: transparent !important;
    backdrop-filter: none !important;
    -webkit-backdrop-filter: none !important;
    box-shadow: none !important;
    pointer-events: none;
    overflow: hidden;
    z-index: 10;
  }

  /* 与 Sidebar 同构：显隐由 isVisible 驱动，物理唤醒仅靠 window focus */
  .timeline-container {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: 38px;
    pointer-events: auto;
    transform: translateX(-100px);
    opacity: 0;
    transition:
      transform 0.3s cubic-bezier(0.16, 1, 0.3, 1),
      opacity 0.3s cubic-bezier(0.16, 1, 0.3, 1);
    will-change: transform, opacity;
  }

  .timeline-container.show {
    transform: translateX(0);
    opacity: 1;
  }

  .timeline-rail {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: 38px;
    overflow: visible;
    background: transparent !important;
    pointer-events: auto;
  }

  .timeline-drag {
    flex-shrink: 0;
    height: 24px;
    width: 100%;
    -webkit-app-region: drag;
    background: transparent;
    pointer-events: auto;
  }

  .timeline-laser {
    position: absolute;
    right: calc((38px - 2px) / 2);
    top: 24px;
    bottom: 48px;
    width: 2px;
    z-index: 1;
    pointer-events: none;
    background: linear-gradient(
      180deg,
      var(--astrocyte-purple-a-10) 0%,
      rgba(211, 184, 255, 0.9) 50%,
      var(--astrocyte-purple-a-10) 100%
    );
    background-size: 100% 200%;
    box-shadow:
      0 0 6px var(--astrocyte-purple-a-50),
      0 0 12px rgba(150, 100, 255, 0.3),
      0 0 25px rgba(120, 80, 200, 0.15);
    animation: energyFlow 4s linear infinite;
  }

  @keyframes energyFlow {
    0% {
      background-position: 0% 0%;
    }
    100% {
      background-position: 0% 200%;
    }
  }

  .timeline-scroll {
    flex: 1;
    min-height: 0;
    width: 100%;
    overflow-y: auto;
    overflow-x: hidden;
    padding-top: 28px;
    padding-bottom: 20px;
    padding-right: 0;
    z-index: 2;
    scrollbar-width: none;
    pointer-events: auto;
  }

  .timeline-scroll::-webkit-scrollbar {
    display: none;
  }

  .timeline-node {
    position: relative;
    width: 100%;
    min-height: var(--space-4);
    cursor: pointer;
    margin-bottom: 8px;
    pointer-events: auto;
  }

  .dot {
    position: absolute;
    right: calc((38px - var(--space-3)) / 2);
    top: 50%;
    transform: translateY(-50%);
    width: var(--space-3);
    height: var(--space-3);
    background: var(--surface-bright);
    border-radius: 50%;
    box-shadow: 0 0 8px #fff, 0 0 16px var(--astrocyte-neural-purple);
    transition:
      transform 0.2s,
      background 0.2s;
    pointer-events: auto;
  }

  .timeline-node:hover .dot,
  .timeline-node:focus-visible .dot {
    background: var(--astrocyte-neural-purple);
    transform: translateY(-50%) scale(1.6);
    box-shadow:
      0 0 15px var(--astrocyte-neural-purple),
      0 0 30px var(--astrocyte-neural-purple);
  }

  .tooltip-float {
    position: fixed;
    width: 200px;
    max-height: 120px;
    padding: var(--space-2);
    border-radius: 6px;
    background: var(--surface-floating);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid var(--astrocyte-purple-a-30);
    color: #a9b1d6;
    opacity: 0;
    pointer-events: none;
    transform: translateY(-50%) translateX(var(--space-3));
    transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
    z-index: 200;
    overflow-y: auto;
    box-sizing: border-box;
  }

  .tooltip-float.open {
    opacity: 1;
    pointer-events: auto;
    transform: translateY(-50%) translateX(-8px);
  }

  .tooltip-float .time {
    display: block;
    font-family: var(--font-mono);
    font-size: 9px;
    margin-bottom: 4px;
    color: #d7c1ff;
    font-weight: bold;
    letter-spacing: 0.04em;
  }

  .tooltip-float .snippet {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: var(--font-body);
    font-size: 11px;
    line-height: 1.4;
    color: #9aa2c7;
  }

  .tooltip-delete {
    position: absolute;
    top: 3px;
    right: 3px;
    padding: 1px var(--space-1);
    font-size: 9px;
    font-weight: bold;
    color: rgba(255, 255, 255, 0.35);
    background: transparent;
    border: none;
    border-radius: 3px;
    cursor: pointer;
    transition:
      color 0.2s,
      background 0.2s;
  }

  .tooltip-delete:hover {
    color: #ff6b6b;
    background: var(--surface-danger-surface);
  }

  .timeline-btn {
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    color: var(--astrocyte-neural-purple);
    cursor: pointer;
    overflow: hidden;
    padding: 0;
    z-index: 5;
    margin-top: auto;
    transition: background 0.2s;
    pointer-events: auto;
  }

  .timeline-btn:hover {
    background: transparent;
    color: #e0d3ff;
  }

  .timeline-btn .icon {
    width: 100%;
    flex-shrink: 0;
    text-align: center;
    font-weight: bold;
    font-size: 13px;
  }

  .timeline-btn .label {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
</style>
