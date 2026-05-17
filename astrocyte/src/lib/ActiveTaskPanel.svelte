<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';

  type TaskStatus = 'running' | 'completed' | 'failed' | 'cancelled';

  type TaskRow = {
    task_id: string;
    task_type: string;
    stage_id: string | null;
    stage_label: string;
    overall_progress: number;
    stage_started_at_ms: number;
    elapsed_in_stage_s: number;
    status: TaskStatus;
    error: string | null;
    finished_at_ms: number | null;
  };

  type TaskEventPayload = {
    task_id?: string;
    task_type?: string;
    stage_id?: string | null;
    stage_label?: string | null;
    overall_progress?: number;
    message?: string | null;
    error?: string | null;
  };

  type TaskEventEnvelope = {
    event_type?: string;
    payload?: TaskEventPayload;
  };

  const TICK_MS = 100;
  const REMOVE_DELAY_MS = 5000;

  let tasksById = $state<Record<string, TaskRow>>({});
  let unlistenTaskEvent: (() => void) | null = null;
  let ticker: ReturnType<typeof setInterval> | null = null;
  const removeTimers: Record<string, ReturnType<typeof setTimeout> | undefined> = {};

  const rows = $derived(Object.values(tasksById).sort((a, b) => b.stage_started_at_ms - a.stage_started_at_ms));

  function toTaskString(value: unknown): string | null {
    return typeof value === 'string' && value.trim().length > 0 ? value : null;
  }

  function toTaskProgress(value: unknown, fallback: number): number {
    const raw = typeof value === 'number' ? value : Number(value);
    if (!Number.isFinite(raw)) return fallback;
    return Math.min(1, Math.max(0, raw));
  }

  function effectiveStageId(payload: TaskEventPayload, current: TaskRow | null): string | null {
    if (payload.stage_id === undefined) return current?.stage_id ?? null;
    return typeof payload.stage_id === 'string' ? payload.stage_id : null;
  }

  function clearRemovalTimer(taskId: string): void {
    const timer = removeTimers[taskId];
    if (timer === undefined) return;
    clearTimeout(timer);
    delete removeTimers[taskId];
  }

  function scheduleRemoval(taskId: string): void {
    clearRemovalTimer(taskId);
    const timer = setTimeout(() => {
      const next = { ...tasksById };
      if (next[taskId]) {
        delete next[taskId];
        tasksById = next;
      }
      delete removeTimers[taskId];
    }, REMOVE_DELAY_MS);
    removeTimers[taskId] = timer;
  }

  function updateTask(taskId: string, updater: (current: TaskRow | null, nowMs: number) => TaskRow): void {
    const nowMs = Date.now();
    const current = tasksById[taskId] ?? null;
    const nextTask = updater(current, nowMs);
    tasksById = {
      ...tasksById,
      [taskId]: nextTask,
    };
  }

  function refreshElapsedInStage(): void {
    const nowMs = Date.now();
    let hasRunning = false;
    const next: Record<string, TaskRow> = {};
    for (const [taskId, task] of Object.entries(tasksById)) {
      if (task.status === 'running') {
        hasRunning = true;
        next[taskId] = {
          ...task,
          elapsed_in_stage_s: Math.max(0, (nowMs - task.stage_started_at_ms) / 1000),
        };
      } else {
        next[taskId] = task;
      }
    }
    if (hasRunning) {
      tasksById = next;
    }
  }

  function handleTaskEvent(envelope: TaskEventEnvelope): void {
    const eventType = toTaskString(envelope.event_type);
    const payload = envelope.payload ?? {};
    const taskId = toTaskString(payload.task_id);
    if (!eventType || !taskId) return;

    if (eventType === 'task-created') {
      clearRemovalTimer(taskId);
      updateTask(taskId, (current, nowMs) => {
        const stageStarted = current?.stage_started_at_ms ?? nowMs;
        return {
          task_id: taskId,
          task_type: toTaskString(payload.task_type) ?? current?.task_type ?? 'task',
          stage_id: effectiveStageId(payload, current),
          stage_label: toTaskString(payload.stage_label) ?? toTaskString(payload.message) ?? current?.stage_label ?? 'Created',
          overall_progress: toTaskProgress(payload.overall_progress, current?.overall_progress ?? 0),
          stage_started_at_ms: stageStarted,
          elapsed_in_stage_s: Math.max(0, (nowMs - stageStarted) / 1000),
          status: 'running',
          error: null,
          finished_at_ms: null,
        };
      });
      return;
    }

    if (eventType === 'task-stage_start') {
      clearRemovalTimer(taskId);
      updateTask(taskId, (current, nowMs) => {
        const nextStageId = effectiveStageId(payload, current);
        const previousStageId = current?.stage_id ?? null;
        const stageChanged = current === null || nextStageId !== previousStageId;
        const stageStarted = stageChanged ? nowMs : (current?.stage_started_at_ms ?? nowMs);
        return {
          task_id: taskId,
          task_type: toTaskString(payload.task_type) ?? current?.task_type ?? 'task',
          stage_id: nextStageId,
          stage_label: toTaskString(payload.stage_label) ?? current?.stage_label ?? 'Running',
          overall_progress: toTaskProgress(payload.overall_progress, current?.overall_progress ?? 0),
          stage_started_at_ms: stageStarted,
          elapsed_in_stage_s: Math.max(0, (nowMs - stageStarted) / 1000),
          status: 'running',
          error: null,
          finished_at_ms: null,
        };
      });
      return;
    }

    if (eventType === 'task-stage_progress') {
      clearRemovalTimer(taskId);
      updateTask(taskId, (current, nowMs) => {
        const stageStarted = current?.stage_started_at_ms ?? nowMs;
        return {
          task_id: taskId,
          task_type: toTaskString(payload.task_type) ?? current?.task_type ?? 'task',
          stage_id: effectiveStageId(payload, current),
          stage_label: toTaskString(payload.stage_label) ?? current?.stage_label ?? 'Running',
          overall_progress: toTaskProgress(payload.overall_progress, current?.overall_progress ?? 0),
          stage_started_at_ms: stageStarted,
          elapsed_in_stage_s: Math.max(0, (nowMs - stageStarted) / 1000),
          status: 'running',
          error: null,
          finished_at_ms: null,
        };
      });
      return;
    }

    if (eventType === 'task-completed') {
      updateTask(taskId, (current, nowMs) => {
        const stageStarted = current?.stage_started_at_ms ?? nowMs;
        return {
          task_id: taskId,
          task_type: toTaskString(payload.task_type) ?? current?.task_type ?? 'task',
          stage_id: effectiveStageId(payload, current),
          stage_label: toTaskString(payload.message) ?? toTaskString(payload.stage_label) ?? current?.stage_label ?? 'Completed',
          overall_progress: toTaskProgress(payload.overall_progress, 1),
          stage_started_at_ms: stageStarted,
          elapsed_in_stage_s: Math.max(0, (nowMs - stageStarted) / 1000),
          status: 'completed',
          error: null,
          finished_at_ms: nowMs,
        };
      });
      scheduleRemoval(taskId);
      return;
    }

    if (eventType === 'task-failed' || eventType === 'task-cancelled') {
      updateTask(taskId, (current, nowMs) => {
        const stageStarted = current?.stage_started_at_ms ?? nowMs;
        return {
          task_id: taskId,
          task_type: toTaskString(payload.task_type) ?? current?.task_type ?? 'task',
          stage_id: effectiveStageId(payload, current),
          stage_label: toTaskString(payload.stage_label) ?? current?.stage_label ?? 'Failed',
          overall_progress: toTaskProgress(payload.overall_progress, current?.overall_progress ?? 0),
          stage_started_at_ms: stageStarted,
          elapsed_in_stage_s: Math.max(0, (nowMs - stageStarted) / 1000),
          status: eventType === 'task-cancelled' ? 'cancelled' : 'failed',
          error: toTaskString(payload.error) ?? toTaskString(payload.message) ?? (eventType === 'task-cancelled' ? 'Task cancelled.' : 'Task failed.'),
          finished_at_ms: nowMs,
        };
      });
      scheduleRemoval(taskId);
    }
  }

  onMount(async () => {
    unlistenTaskEvent = await listen<TaskEventEnvelope>('bb-task-event', (event) => {
      handleTaskEvent(event.payload ?? {});
    });

    ticker = setInterval(() => {
      refreshElapsedInStage();
    }, TICK_MS);
  });

  onDestroy(() => {
    if (unlistenTaskEvent) {
      unlistenTaskEvent();
      unlistenTaskEvent = null;
    }
    if (ticker !== null) {
      clearInterval(ticker);
      ticker = null;
    }
    for (const timer of Object.values(removeTimers)) {
      if (timer !== undefined) {
        clearTimeout(timer);
      }
    }
    for (const taskId of Object.keys(removeTimers)) {
      delete removeTimers[taskId];
    }
  });
</script>

{#if rows.length > 0}
  <section class="active-task-panel" aria-live="polite">
    {#each rows as task (task.task_id)}
      <div
        class="task-row"
        class:status-completed={task.status === 'completed'}
        class:status-failed={task.status === 'failed' || task.status === 'cancelled'}
      >
        <div class="task-header">
          <span class="task-type">{task.task_type}</span>
          <span class="task-stage">{task.stage_label}</span>
          <span class="task-elapsed">{task.elapsed_in_stage_s.toFixed(1)}s</span>
        </div>

        <div
          class="task-progress-bar"
          role="progressbar"
          aria-valuemin="0"
          aria-valuemax="100"
          aria-valuenow={Math.round(task.overall_progress * 100)}
        >
          <div class="task-progress-fill" style="width: {Math.min(100, Math.max(0, task.overall_progress * 100))}%"></div>
        </div>

        {#if task.error}
          <div class="task-error">{task.error}</div>
        {/if}
      </div>
    {/each}
  </section>
{/if}
