<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  /**
   * 与 `scratchpad.rs` 中 `ScratchpadNote` 的 **JSON 键名**一致（serde `rename` → camelCase）。
   * `add_note` 须传 `{ payload: { content, contextId?, focusDuration? } }`（Tauri 按形参名取键）。
   */
  interface ScratchpadNote {
    id: string;
    content: string;
    createdAt: string;
    updatedAt: string;
    contextId: string | null;
    focusDurationMins: number | null;
  }

  let notes = $state<ScratchpadNote[]>([]);
  let captureDraft = $state('');

  const NOTE_UPDATE_DEBOUNCE_MS = 500;
  const noteUpdateTimers = new Map<string, ReturnType<typeof setTimeout>>();

  let timerMinutes = $state(25);
  let timerSeconds = $state(0);
  let isTimerRunning = $state(false);
  let pomodoroInterval: ReturnType<typeof setInterval> | null = null;
  let isPomodoroFinished = $state(false);
  let timerPresetMinutes = $state(25);
  const timerPresetOptions = [15, 25, 45];

  /** 番茄运行中的专注会话 id；Pause / 倒计时结束 / Reset 时清空 */
  let currentSessionId = $state<string | null>(null);

  let isVisible = $state(false);
  let mouseInside = $state(false);
  let hideTimeout: ReturnType<typeof setTimeout> | null = null;

  /** 焦点已在侧栏内时不应启动自动隐藏，避免 window focus 早于 mouseenter 或纯键盘聚焦时误关 */
  function isFocusInsideDrawer(): boolean {
    const root = document.getElementById('drawer-container');
    if (!root) return false;
    const ae = document.activeElement;
    return ae instanceof Node && root.contains(ae);
  }

  function onWindowFocus() {
    clearHide();
    isVisible = true;
    if (!mouseInside && !isFocusInsideDrawer()) {
      startHideTimer();
    }
  }

  const HIDE_DELAY_MS = 1000;
  const HIDE_ANIM_MS = 300;

  function startHideTimer() {
    if (hideTimeout) clearTimeout(hideTimeout);
    hideTimeout = setTimeout(() => {
      hideTimeout = null;
      isVisible = false;
      setTimeout(() => {
        if (!isVisible) void invoke('hide_phantom_sidebar').catch(console.error);
      }, HIDE_ANIM_MS);
    }, HIDE_DELAY_MS);
  }

  function clearHide() {
    if (hideTimeout) {
      clearTimeout(hideTimeout);
      hideTimeout = null;
    }
  }

  function formatPomodoroTime(): string {
    return `${timerMinutes.toString().padStart(2, '0')}:${timerSeconds.toString().padStart(2, '0')}`;
  }

  function clearFocusContext() {
    currentSessionId = null;
  }

  function pausePomodoro() {
    clearFocusContext();
    isTimerRunning = false;
    if (pomodoroInterval) {
      clearInterval(pomodoroInterval);
      pomodoroInterval = null;
    }
  }

  function startPomodoro() {
    if (isTimerRunning) return;
    currentSessionId = 'focus_' + Date.now();
    isTimerRunning = true;
    isPomodoroFinished = false;
    pomodoroInterval = setInterval(() => {
      if (timerSeconds === 0) {
        if (timerMinutes === 0) {
          pausePomodoro();
          isPomodoroFinished = true;
          return;
        }
        timerMinutes -= 1;
        timerSeconds = 59;
      } else {
        timerSeconds -= 1;
      }
    }, 1000);
  }

  function resetPomodoro() {
    pausePomodoro();
    timerMinutes = timerPresetMinutes;
    timerSeconds = 0;
    isPomodoroFinished = false;
  }

  function applyPomodoroPreset(minutes: number) {
    if (isTimerRunning) return;
    timerPresetMinutes = minutes;
    timerMinutes = minutes;
    timerSeconds = 0;
    isPomodoroFinished = false;
  }

  function fitCardTextarea(el: HTMLTextAreaElement) {
    el.style.height = 'auto';
    el.style.height = `${Math.max(36, el.scrollHeight)}px`;
  }

  function scheduleNoteUpdate(note: ScratchpadNote) {
    const prev = noteUpdateTimers.get(note.id);
    if (prev) clearTimeout(prev);
    noteUpdateTimers.set(
      note.id,
      setTimeout(() => {
        noteUpdateTimers.delete(note.id);
        void invoke('update_note', { id: note.id, content: note.content }).catch((error) => {
          console.error('update_note failed', error);
        });
      }, NOTE_UPDATE_DEBOUNCE_MS)
    );
  }

  function onCardInput(note: ScratchpadNote, el: HTMLTextAreaElement) {
    scheduleNoteUpdate(note);
    fitCardTextarea(el);
  }

  /** 统一 IPC/磁盘可能混用的键名，保证 `contextId` / `focusDurationMins` 存在并驱动 UI */
  function coerceScratchpadNote(raw: unknown): ScratchpadNote {
    const r = raw as Record<string, unknown>;
    const cid = r.contextId ?? r.context_id;
    const fid = r.focusDurationMins ?? r.focus_duration_mins;
    return {
      id: String(r.id ?? ''),
      content: String(r.content ?? ''),
      createdAt: String(r.createdAt ?? r.created_at ?? ''),
      updatedAt: String(r.updatedAt ?? r.updated_at ?? ''),
      contextId: typeof cid === 'string' && cid.trim() !== '' ? cid.trim() : null,
      focusDurationMins:
        typeof fid === 'number' && Number.isFinite(fid) ? Math.floor(fid) : null
    };
  }

  async function onCaptureEnter(e: KeyboardEvent) {
    if (e.key !== 'Enter') return;
    e.preventDefault();
    const trimmed = captureDraft.trim();
    if (!trimmed) return;
    /* Tauri 2：形参名 `payload` 对应 IPC JSON 顶层键 `payload`（见 tauri ipc/command.rs CommandItem::deserialize_json） */
    const notePayload = {
      content: trimmed,
      contextId: currentSessionId ? currentSessionId : null,
      focusDuration: currentSessionId ? timerMinutes : null
    };
    const invokeArgs = { payload: notePayload };
    try {
      const raw = await invoke<unknown>('add_note', invokeArgs);
      const newNote = coerceScratchpadNote(raw);
      captureDraft = '';
      notes = [newNote, ...notes];
      await tick();
      document.querySelectorAll<HTMLTextAreaElement>('.mm-card-textarea').forEach(fitCardTextarea);
    } catch (error) {
      console.error('add_note failed', error);
    }
  }

  async function deleteNote(id: string) {
    const t = noteUpdateTimers.get(id);
    if (t) clearTimeout(t);
    noteUpdateTimers.delete(id);
    try {
      await invoke('delete_note', { id });
      notes = notes.filter((n) => n.id !== id);
    } catch (error) {
      console.error('delete_note failed', error);
    }
  }

  function onSublimateCard(note: ScratchpadNote) {
    console.log('Sublimating', note.id);
  }

  async function loadNotes() {
    try {
      const loaded = await invoke<unknown[]>('get_notes');
      notes = [...loaded].reverse().map(coerceScratchpadNote);
      await tick();
      document.querySelectorAll<HTMLTextAreaElement>('.mm-card-textarea').forEach(fitCardTextarea);
    } catch (error) {
      console.error('get_notes failed', error);
      notes = [];
    }
  }

  onMount(async () => {
    window.addEventListener('focus', onWindowFocus);
    requestAnimationFrame(() => {
      isVisible = true;
    });
    await loadNotes();
  });

  onDestroy(() => {
    pausePomodoro();
    clearHide();
    window.removeEventListener('focus', onWindowFocus);
    for (const t of noteUpdateTimers.values()) clearTimeout(t);
    noteUpdateTimers.clear();
  });
</script>

<div
  id="drawer-container"
  class="fade-container"
  class:visible={isVisible}
  onmouseenter={() => {
    mouseInside = true;
    clearHide();
  }}
  onmouseleave={() => {
    mouseInside = false;
    if (!isFocusInsideDrawer()) {
      startHideTimer();
    }
  }}
  onfocusin={() => {
    clearHide();
  }}
  role="presentation"
>
  <main class="sidebar-shell">
    <div class="sidebar-drag" data-tauri-drag-region aria-hidden="true"></div>

    <section class="sidebar-pomodoro" aria-label="Pomodoro timer">
      <div
        class="sidebar-timer"
        class:running={isTimerRunning}
        class:finished={isPomodoroFinished}
      >
        {formatPomodoroTime()}
      </div>
      <div class="sidebar-timer-presets" aria-label="Pomodoro presets">
        {#each timerPresetOptions as minutes (minutes)}
          <button
            type="button"
            class:active={timerPresetMinutes === minutes}
            onclick={() => applyPomodoroPreset(minutes)}
            disabled={isTimerRunning}
          >
            [{minutes}]
          </button>
        {/each}
      </div>
      <div class="sidebar-timer-controls">
        <button type="button" onclick={startPomodoro} disabled={isTimerRunning}>Start</button>
        <button type="button" onclick={pausePomodoro} disabled={!isTimerRunning}>Pause</button>
        <button type="button" onclick={resetPomodoro}>Reset</button>
      </div>
    </section>

    <section class="sidebar-notes memory-matrix" aria-label="Memory matrix">
      <header class="mm-header">
        <span class="mm-title">Memory Matrix</span>
      </header>
      <input
        type="text"
        class="mm-capture"
        bind:value={captureDraft}
        onkeydown={onCaptureEnter}
        onfocus={clearHide}
        placeholder="Capture a spark..."
        autocomplete="off"
        spellcheck="false"
        aria-label="Capture a new note"
      />
      <div class="sidebar-memory-matrix-scroll mm-list">
        {#each notes as note (note.id)}
          <article class="mm-card">
            <div class="mm-card-chrome">
              <textarea
                class="mm-card-textarea"
                bind:value={note.content}
                oninput={(e) => onCardInput(note, e.currentTarget)}
                onfocus={clearHide}
                rows="1"
                spellcheck="false"
                aria-label="Note content"
              ></textarea>
              {#if note.contextId}
                <span class="focus-context-label" aria-hidden="true">
                  [⧖ Focus Context: {note.focusDurationMins != null ? `${note.focusDurationMins}m` : '—'}]
                </span>
              {/if}
              <div class="mm-card-actions" aria-label="Note actions">
                <button
                  type="button"
                  class="mm-action"
                  title="Sublimate"
                  onclick={() => onSublimateCard(note)}
                >
                  S
                </button>
                <button
                  type="button"
                  class="mm-action"
                  title="Delete"
                  onclick={() => deleteNote(note.id)}
                  aria-label="Delete note"
                >
                  D
                </button>
              </div>
            </div>
          </article>
        {/each}
      </div>
    </section>
  </main>
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    width: 100%;
    height: 100%;
    background: transparent;
    overflow: hidden;
  }

  .sidebar-shell {
    width: 100vw;
    height: 100vh;
    margin: 0;
    display: flex;
    flex-direction: column;
    background: rgba(15, 12, 22, 0.65);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border: 1px solid rgba(187, 154, 247, 0.2);
    border-radius: 12px;
    color: #bb9af7;
    overflow: hidden;
    box-sizing: border-box;
  }

  .sidebar-drag {
    flex-shrink: 0;
    height: 24px;
    width: 100%;
    -webkit-app-region: drag;
    background: rgba(10, 10, 15, 0.92);
    border-bottom: 1px solid rgba(187, 154, 247, 0.2);
  }

  .sidebar-pomodoro {
    flex-shrink: 0;
    padding: 16px 14px 14px;
    border-bottom: 1px solid rgba(187, 154, 247, 0.16);
  }

  .sidebar-timer {
    text-align: center;
    font-family: var(--font-mono);
    font-size: 4rem;
    line-height: 1;
    letter-spacing: 0.04em;
    color: #6b5b8a;
    margin-bottom: 14px;
    transition:
      color 140ms ease,
      text-shadow 140ms ease;
  }

  .sidebar-timer-presets {
    display: flex;
    justify-content: center;
    gap: 6px;
    margin-bottom: 10px;
  }

  .sidebar-timer-presets button {
    background: transparent;
    border: 1px solid rgba(187, 154, 247, 0.22);
    color: rgba(187, 154, 247, 0.78);
    font: inherit;
    font-size: 10px;
    padding: 2px 6px;
    cursor: pointer;
    -webkit-app-region: no-drag;
    transition:
      border-color 120ms ease,
      box-shadow 120ms ease,
      color 120ms ease;
  }

  .sidebar-timer-presets button:hover:not(:disabled) {
    border-color: rgba(187, 154, 247, 0.78);
    color: #e0d3ff;
    box-shadow: 0 0 8px rgba(187, 154, 247, 0.2);
  }

  .sidebar-timer-presets button.active {
    border-color: rgba(187, 154, 247, 0.95);
    color: #f1e7ff;
    box-shadow: 0 0 10px rgba(187, 154, 247, 0.32);
  }

  .sidebar-timer-presets button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .sidebar-timer.running {
    color: #e8ebf4;
  }

  .sidebar-timer.finished {
    color: #bb9af7;
    animation: sidebar-timer-pulse 1s ease-in-out infinite;
  }

  @keyframes sidebar-timer-pulse {
    0%,
    100% {
      text-shadow: 0 0 8px rgba(187, 154, 247, 0.35);
    }
    50% {
      text-shadow: 0 0 22px rgba(187, 154, 247, 0.85);
    }
  }

  .sidebar-timer-controls {
    display: flex;
    justify-content: center;
    gap: 8px;
  }

  .sidebar-timer-controls button {
    background: transparent;
    border: 1px solid rgba(187, 154, 247, 0.25);
    color: rgba(187, 154, 247, 0.86);
    font: inherit;
    font-size: 11px;
    padding: 4px 10px;
    cursor: pointer;
    -webkit-app-region: no-drag;
    transition:
      border-color 120ms ease,
      box-shadow 120ms ease,
      color 120ms ease;
  }

  .sidebar-timer-controls button:hover:not(:disabled) {
    border-color: rgba(187, 154, 247, 0.82);
    box-shadow: 0 0 10px rgba(187, 154, 247, 0.2);
    color: #e0d3ff;
  }

  .sidebar-timer-controls button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  /* ─── Memory Matrix ─── */
  .sidebar-notes.memory-matrix {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 10px 12px 12px;
    gap: 10px;
  }

  .mm-header {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .mm-title {
    font-size: 9px;
    letter-spacing: 0.22em;
    text-transform: uppercase;
    color: rgba(122, 106, 154, 0.55);
    font-family: var(--font-mono);
  }

  .mm-capture {
    flex-shrink: 0;
    width: 100%;
    box-sizing: border-box;
    margin: 0;
    padding: 8px 2px 10px;
    background: transparent;
    border: none;
    border-bottom: 1px solid rgba(187, 154, 247, 0.12);
    outline: none;
    color: #d3b8ff;
    font: inherit;
    font-size: 13px;
    line-height: 1.35;
    caret-color: #bb9af7;
    -webkit-app-region: no-drag;
    transition: border-color 160ms ease;
  }

  .mm-capture:focus {
    border-bottom-color: rgba(187, 154, 247, 0.35);
  }

  .mm-capture::placeholder {
    color: rgba(122, 106, 154, 0.38);
  }

  .mm-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 4px 2px 6px;
    -webkit-app-region: no-drag;
    scroll-behavior: smooth;
  }

  .mm-card {
    flex-shrink: 0;
    border-radius: 8px;
    border: 1px solid rgba(187, 154, 247, 0.1);
    border-left: 2px solid rgba(187, 154, 247, 0.42);
    background: rgba(20, 20, 25, 0.4);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    box-shadow:
      0 0 0 1px rgba(0, 0, 0, 0.25) inset,
      0 4px 20px rgba(0, 0, 0, 0.35);
    transition:
      border-color 160ms ease,
      box-shadow 160ms ease;
  }

  .mm-card:hover {
    border-color: rgba(187, 154, 247, 0.2);
    border-left-color: rgba(187, 154, 247, 0.65);
    box-shadow:
      0 0 0 1px rgba(187, 154, 247, 0.06) inset,
      0 6px 24px rgba(0, 0, 0, 0.45);
  }

  .mm-card-chrome {
    position: relative;
    padding: 28px 10px 10px 12px;
  }

  .mm-card-textarea {
    display: block;
    width: 100%;
    box-sizing: border-box;
    margin: 0;
    padding: 0;
    min-height: 36px;
    background: transparent;
    border: none;
    outline: none;
    resize: none;
    color: #c4b5e0;
    font: inherit;
    font-size: 12px;
    line-height: 1.5;
    caret-color: #bb9af7;
  }

  .mm-card-textarea::placeholder {
    color: rgba(122, 106, 154, 0.4);
  }

  .focus-context-label {
    display: block;
    margin-top: 6px;
    padding: 0 2px 0 0;
    text-align: right;
    font-family: var(--font-mono);
    font-size: 0.7em;
    line-height: 1.35;
    color: rgba(118, 108, 152, 0.82);
    letter-spacing: 0.04em;
    user-select: none;
  }

  /* 与主界面聊天消息 .msg-actions / .msg-action (E/D/R) 对齐；略内缩避免贴边 */
  .mm-card-actions {
    position: absolute;
    top: 7px;
    right: 8px;
    display: inline-flex;
    gap: 4px;
    opacity: 0;
    pointer-events: none;
    transition: opacity 120ms ease;
  }

  .mm-card:hover .mm-card-actions,
  .mm-card:focus-within .mm-card-actions {
    opacity: 1;
    pointer-events: auto;
  }

  .mm-action {
    box-sizing: border-box;
    width: 14px;
    height: 14px;
    margin: 0;
    padding: 0;
    border: 1px solid rgba(187, 154, 247, 0.45);
    border-radius: 0;
    background: rgba(10, 10, 15, 0.88);
    color: #bb9af7;
    font-family: var(--font-mono);
    font-size: 9px;
    line-height: 1;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .mm-action:hover {
    border-color: rgba(187, 154, 247, 0.8);
  }

  .mm-action:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
