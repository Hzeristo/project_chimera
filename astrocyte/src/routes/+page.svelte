<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import ActiveTaskPanel from '$lib/ActiveTaskPanel.svelte';
  import ActiveToolTelemetry from '$lib/ActiveToolTelemetry.svelte';
  import { marked } from 'marked';
  import markedKatex from 'marked-katex-extension';
  import DOMPurify from 'dompurify';
  import hljs from 'highlight.js';
  import 'highlight.js/styles/atom-one-dark.css';
  import 'katex/dist/katex.min.css';

  type StageKind = 'router' | 'tool' | 'wash' | 'final';
  type Sender = 'system' | 'user' | 'bb' | 'system_log' | 'error' | 'stage_card';
  type HistoryEntry = {
    id: string;
    sender: Sender;
    text: string;
    timestamp: string;
    persona?: string;
    isLoading?: boolean;
    /** Set when user aborted mid-stream; assistant text stays in UI but was not persisted. */
    streamAborted?: boolean;
    /** 本条 BB 回复生成时所选 Skill（用于反馈统计）；无 Skill 时为 null。 */
    skill_id?: string | null;
    /** 粗略 token 估计（约 chars/4），供 skill_stats 写入。 */
    tokens?: number;
    /** 对 BB：用户对本条回复的满意/不满意。对 stage_card：流在 bb-stream-done 中落盘后置为 true，供 stripStageCards 只剔除「未完成」卡片。 */
    feedback?: boolean;
    /** Oligo 分段遥测（sender === stage_card） */
    stage?: StageKind;
    tool_name?: string;
    decision?: string;
    stage_feedback?: 'good' | 'bad';
  };
  type ProviderConfig = {
    id: string;
    name: string;
    api_key: string;
    base_url: string;
    model_name: string;
    /** Oligo 每请求温度；`null` 表示不发送，由服务端使用 Chimera working 默认温度 */
    temperature?: number | null;
  };
  type AstrocyteConfig = {
    active_provider_id: string | null;
    is_oligo_mode: boolean;
    active_skill_id?: string | null;
  };

  /** 与 `get_available_providers` / `~/.chimera/config.toml` `[llm.providers]` 一致 */
  type TomlProviderRow = {
    id: string;
    name: string;
    apiKey: string;
    baseUrl: string;
    model: string;
    temperature: number;
    timeoutSeconds: number;
  };
  type PersonaConfig = {
    id: string;
    name: string;
    system_prompt: string;
    authors_note?: string | null;
  };
  type PersonaSnapshot = {
    active_persona_id: string;
    personas: PersonaConfig[];
  };
  type SkillAggregatedStats = {
    usage_count: number;
    success_rate: number;
    avg_tokens: number;
  };
  /** 与 Rust `SkillWithStats`（flatten 定义 + stats）JSON 一致 */
  type SkillWithStats = {
    id: string;
    name: string;
    description: string;
    system_override: string;
    allowed_tools?: string[] | null;
    category: string;
    target_paper_type: string[];
    expected_output_format?: string | null;
    version: string;
    last_updated?: string | null;
    stats: SkillAggregatedStats;
  };
  type TelemetryLine = { id: string; html: string; settled: boolean };
  /** 与 Rust `get_background_task_status` / Oligo Task JSON 对齐 */
  type BackgroundTaskFile = {
    id: string;
    type: string;
    status: string;
    progress: number;
    result?: string | null;
    error?: string | null;
  };
  type MinerTaskUiRow = { progress: number; line: string };
  type BackendMessage = {
    role: string;
    content: string;
  };
  type SessionSummary = {
    id: string;
    timestamp: string;
    first_user_message_snippet: string;
  };
  type SessionArchiveEntry = {
    id: string;
    timestamp: string;
    role: string;
    content: string;
    persona?: string | null;
    session_id: string;
  };
  type TimelineNode = {
    session: SessionSummary;
    gap: number;
  };
  /** 与 Tauri `get_system_metrics` / Oligo `MetricsService.get_summary` 对齐 */
  type SystemMetricsTopSkill = { id: string; count: number; tokens: number };
  type SystemToolStat = {
    name: string;
    count: number;
    success_rate: number;
    avg_latency_ms: number;
  };
  type WashByToolEntry = { count: number; original: number; washed: number };
  type WashStatsSummary = {
    total_washes: number;
    avg_compression_rate: number;
    by_tool: Record<string, WashByToolEntry>;
  };
  type SystemMetricsSummary = {
    total_requests: number;
    success_rate: number;
    avg_latency_ms: number;
    total_tokens: number;
    top_skills: SystemMetricsTopSkill[];
    tool_stats: SystemToolStat[];
    wash_stats: WashStatsSummary;
  };

  const DEFAULT_PERSONA = 'bb';

  /** C3：仅暴露 Working 模型三槽；与 `config.toml` `[llm.providers]` 键名对齐。 */
  const WORKING_PROVIDER_SLOTS = [
    { id: 'openai', name: 'OpenAI', default_base_url: 'https://api.openai.com/v1' },
    { id: 'deepseek', name: 'DeepSeek', default_base_url: 'https://api.deepseek.com' },
    { id: 'anthropic', name: 'Anthropic', default_base_url: 'https://api.anthropic.com/v1' },
  ] as const;

  const makeId = () =>
    globalThis.crypto?.randomUUID?.() ??
    `${Date.now()}-${Math.random().toString(16).slice(2)}`;
  const nowIso = () => new Date().toISOString();
  const roughTokenEstimate = (text: string) => Math.max(0, Math.ceil(text.length / 4));
  const makeSessionId = () => `session-${Date.now()}-${Math.random().toString(16).slice(2, 8)}`;
  const makeBootEntry = (): HistoryEntry => ({
    id: makeId(),
    sender: 'system',
    text: 'System online...',
    timestamp: nowIso(),
    persona: DEFAULT_PERSONA,
  });

  let history: HistoryEntry[] = [makeBootEntry()];
  let activeSessionId = makeSessionId();
  let sessionSummaries: SessionSummary[] = [];
  let history_sessions: TimelineNode[] = [];
  let viewingArchive: SessionSummary | null = null;
  let phantomHoverTimeout: ReturnType<typeof setTimeout> | null = null;
  let timelineHoverTimeout: ReturnType<typeof setTimeout> | null = null;
  let inputSignal = '';
  let inputEl: HTMLTextAreaElement | null = null;
  /** 顶部拉条拖曳时注册的 pointerup 清理，onDestroy 需调用 */
  let inputSizeDragRelease: (() => void) | null = null;
  let outputEl: HTMLElement | null = null;
  let currentBBMessageId: string | null = null;
  let pendingAssistantMessageId: string | null = null;
  /** 与当前轮次 `evaluate_payload` 对齐的 Skill，用于流结束后写入消息的 skill_id。 */
  let pendingSkillFeedbackSkillId: string | null = null;
  let isGenerating = false;
  let pendingChunkBuffer = '';
  let pendingFlushFrame: number | null = null;
  /** First non-empty BB stream chunk: purge ephemeral `system_log` rows from the canvas. */
  let pendingStripSystemLogsOnFirstChunk = false;
  let unlistenClipboardHijack: (() => void) | null = null;
  let unlistenBBChunk: (() => void) | null = null;
  let unlistenBBSysEvent: (() => void) | null = null;
  let unlistenBBDone: (() => void) | null = null;
  let unlistenSublimateRequest: (() => void) | null = null;
  let unlistenLoadSession: (() => void) | null = null;
  let unlistenNewSignal: (() => void) | null = null;
  let editTextareaEl: HTMLTextAreaElement | null = null;
  const markdownCache = new Map<string, { text: string; html: string }>();

  let showSettingsPanel = false;
  let settingsSaving = false;
  let settingsError = '';
  let settingsStatus = '';
  let settingsTab: 'provider' | 'persona' | 'health' = 'provider';
  const defaultWashStats = (): WashStatsSummary => ({
    total_washes: 0,
    avg_compression_rate: 0,
    by_tool: {},
  });
  function normalizeWashStats(raw: unknown): WashStatsSummary {
    const d = raw && typeof raw === 'object' ? (raw as Record<string, unknown>) : {};
    const by_tool: Record<string, WashByToolEntry> = {};
    const bt = d.by_tool;
    if (bt && typeof bt === 'object' && !Array.isArray(bt)) {
      for (const [k, v] of Object.entries(bt as Record<string, unknown>)) {
        if (v && typeof v === 'object') {
          const o = v as Record<string, unknown>;
          by_tool[k] = {
            count: Number(o.count) || 0,
            original: Number(o.original) || 0,
            washed: Number(o.washed) || 0,
          };
        }
      }
    }
    return {
      total_washes: Number(d.total_washes) || 0,
      avg_compression_rate: Number(d.avg_compression_rate) || 0,
      by_tool,
    };
  }
  function washToolBreakdown(by_tool: Record<string, WashByToolEntry>) {
    return Object.entries(by_tool)
      .map(([id, st]) => ({
        id,
        count: st.count,
        compression: st.original > 0 ? 1 - st.washed / st.original : 0,
      }))
      .sort((a, b) => b.count - a.count);
  }
  const defaultSystemMetrics = (): SystemMetricsSummary => ({
    total_requests: 0,
    success_rate: 0,
    avg_latency_ms: 0,
    total_tokens: 0,
    top_skills: [],
    tool_stats: [],
    wash_stats: defaultWashStats(),
  });
  let systemMetrics: SystemMetricsSummary = defaultSystemMetrics();
  let systemMetricsRefresh: ReturnType<typeof setInterval> | null = null;
  let config: AstrocyteConfig = {
    active_provider_id: null,
    is_oligo_mode: false,
    active_skill_id: null,
  };
  let tomlProviders: TomlProviderRow[] = [];
  let selectedProviderId: string | null = 'openai';
  let providerDraft: ProviderConfig = {
    id: 'openai',
    name: 'OpenAI',
    api_key: '',
    base_url: 'https://api.openai.com/v1',
    model_name: '',
    temperature: null,
  };
  type ProviderPingState = 'idle' | 'probing' | 'up' | 'down';
  let pingByProviderId: Record<string, ProviderPingState> = {};
  let pingRequestSeq = 0;
  let selectedProviderPingState: ProviderPingState = 'idle';
  let personaSnapshot: PersonaSnapshot = { active_persona_id: '', personas: [] };
  let selectedPersonaId: string | null = null;
  let personaDraft: PersonaConfig = {
    id: makeId(),
    name: '',
    system_prompt: '',
    authors_note: '',
  };
  let personaStatus = '';
  let personaError = '';
  let activePersonaName = 'BB (Default)';

  $: selectedPersona =
    personaSnapshot.personas.find((p) => p.id === personaSnapshot.active_persona_id) ?? null;

  let availableSkills: SkillWithStats[] = [];
  let activeSkillId: string | null = null;
  $: activeSkillId = activeSkillId === '' ? null : activeSkillId;
  /** Oligo tool traces: sticky telemetry strip, not chat bubbles. */
  let telemetryLines: TelemetryLine[] = [];
  /** Miner 异步任务轮询 UI（`[Task Started]` 后由 `~/.chimera/tasks/*.json` 驱动） */
  let minerTaskById: Record<string, MinerTaskUiRow> = {};
  const minerPollTimers = new Map<string, ReturnType<typeof setInterval>>();
  const MINER_TASK_STARTED_RE = /\[Task Started\][\s\S]*?created:\s*([0-9a-f]{8})/i;
  $: selectedSkill = activeSkillId
    ? availableSkills.find((s) => s.id === activeSkillId) ?? null
    : null;
  $: activeSkillDisplayName = selectedSkill?.name ?? null;

  /** HUD 快速选择：Persona 下拉 + Skill 卡片网格 */
  let personaDropdownOpen = false;
  let skillPanelOpen = false;
  /** 距窗口底不足 300px 时向上展开，避免 Tauri 窗口裁切 */
  let personaDropdownUp = false;
  let skillPanelUp = false;
  const DROPDOWN_BOTTOM_CLEARANCE = 300;

  function closeQuickSwitchDropdowns() {
    personaDropdownOpen = false;
    skillPanelOpen = false;
  }

  function togglePersonaDropdown(e: MouseEvent) {
    skillPanelOpen = false;
    if (personaDropdownOpen) {
      personaDropdownOpen = false;
      return;
    }
    const el = e.currentTarget as HTMLButtonElement;
    const rect = el.getBoundingClientRect();
    personaDropdownUp = window.innerHeight - rect.bottom < DROPDOWN_BOTTOM_CLEARANCE;
    personaDropdownOpen = true;
  }

  function toggleSkillPanel(e: MouseEvent) {
    personaDropdownOpen = false;
    if (skillPanelOpen) {
      skillPanelOpen = false;
      return;
    }
    const el = e.currentTarget as HTMLButtonElement;
    const rect = el.getBoundingClientRect();
    skillPanelUp = window.innerHeight - rect.bottom < DROPDOWN_BOTTOM_CLEARANCE;
    skillPanelOpen = true;
  }

  async function refreshAvailableSkills() {
    try {
      availableSkills = await invoke<SkillWithStats[]>('get_available_skills');
    } catch {
      /* keep previous list */
    }
  }

  let editingMessageId: string | null = null;
  let editBuffer = '';

  $: archiveBannerText = viewingArchive
    ? `[VIEWING HISTORICAL ARCHIVE: ${viewingArchive.id} @ ${formatSessionTimestamp(viewingArchive.timestamp)}]`
    : '';
  $: history_sessions = buildTimelineNodes(sessionSummaries);

  $: selectedProviderPingState = selectedProviderId
    ? (pingByProviderId[selectedProviderId] ?? 'idle')
    : 'idle';
  $: activePersonaName =
    personaSnapshot.personas.find((persona) => persona.id === personaSnapshot.active_persona_id)?.name ??
    'BB (Default)';

  const markdownRenderer = new marked.Renderer();
  markdownRenderer.code = (token: any) => {
    const source = typeof token?.text === 'string' ? token.text : '';
    const rawLang = typeof token?.lang === 'string' ? token.lang : '';
    const lang = rawLang && hljs.getLanguage(rawLang) ? rawLang : 'plaintext';
    const highlighted = hljs.highlight(source, { language: lang }).value;
    const safeLang = lang.replace(/[^\w-]/g, '');
    return `<pre><code class="hljs language-${safeLang}">${highlighted}</code></pre>`;
  };

  marked.use(
    {
      gfm: true,
      breaks: true,
      renderer: markdownRenderer,
    },
    markedKatex({
      throwOnError: false,
      nonStandard: true,
      output: 'html',
    }),
  );

  function notifySystem(text: string) {
    history = [
      ...history,
      {
        id: makeId(),
        sender: 'system',
        text,
        timestamp: nowIso(),
        persona: personaSnapshot.active_persona_id || DEFAULT_PERSONA,
      },
    ];
  }

  async function scrollToBottom() {
    await tick();
    if (outputEl) {
      outputEl.scrollTop = outputEl.scrollHeight;
    }
  }

  function extractMinerTaskIdFromAssistantText(text: string): string | null {
    const m = text.match(MINER_TASK_STARTED_RE);
    return m?.[1] ?? null;
  }

  function formatMinerStatusLine(row: BackgroundTaskFile): string {
    const st = (row.status || '').toLowerCase();
    if (st === 'completed') {
      return `[Task Completed] ${row.result ?? ''}`;
    }
    if (st === 'failed') {
      return `[Task Failed] ${row.error ?? ''}`;
    }
    const pct = Math.round((row.progress ?? 0) * 100);
    return `[Task ${st.toUpperCase() || 'UNKNOWN'}] Progress: ${pct}%`;
  }

  function stopMinerTaskPoll(taskId: string) {
    const t = minerPollTimers.get(taskId);
    if (t !== undefined) {
      clearInterval(t);
      minerPollTimers.delete(taskId);
    }
  }

  function clearMinerTaskPolls() {
    for (const id of [...minerPollTimers.keys()]) {
      stopMinerTaskPoll(id);
    }
    minerTaskById = {};
  }

  /** 轮询 `~/.chimera/tasks/{id}.json`（与 Oligo TaskService 一致）。返回 true 表示已终态并应停止 interval。 */
  async function pollMinerTaskOnce(taskId: string): Promise<boolean> {
    try {
      const row = await invoke<BackgroundTaskFile>('get_background_task_status', { taskId });
      const st = (row.status || '').toLowerCase();
      const line = formatMinerStatusLine(row);
      if (st === 'completed' || st === 'failed') {
        const { [taskId]: _, ...rest } = minerTaskById;
        minerTaskById = rest;
        const fullText = line;
        history = [
          ...history,
          {
            id: makeId(),
            sender: 'bb',
            text: fullText,
            timestamp: nowIso(),
            persona: personaSnapshot.active_persona_id || DEFAULT_PERSONA,
            skill_id: activeSkillId,
            tokens: roughTokenEstimate(fullText),
          },
        ];
        await scrollToBottom();
        await syncTimelineToBackend();
        await refreshSessionHistory();
        return true;
      }
      minerTaskById = { ...minerTaskById, [taskId]: { progress: row.progress ?? 0, line } };
      await scrollToBottom();
      return false;
    } catch {
      minerTaskById = {
        ...minerTaskById,
        [taskId]: { progress: 0, line: `[Task poll] no local status for ${taskId} (is Oligo running?)` },
      };
      return false;
    }
  }

  /** Oligo 回复含 `[Task Started]` 时启动；每 5s 读本地 task 文件，避免与主对话 SSE 交错。 */
  function startMinerTaskPolling(taskId: string) {
    if (!config.is_oligo_mode) return;
    if (minerPollTimers.has(taskId)) return;

    const run = async () => {
      const done = await pollMinerTaskOnce(taskId);
      if (done) {
        stopMinerTaskPoll(taskId);
      }
    };

    void run();
    const handle = setInterval(() => {
      void run();
    }, 5000);
    minerPollTimers.set(taskId, handle);
  }

  function parseMarkdown(rawText: string) {
    const parsed = marked.parse(rawText || '', { async: false }) as string;
    return DOMPurify.sanitize(parsed, {
      USE_PROFILES: { html: true },
      FORBID_TAGS: ['script', 'style'],
      FORBID_ATTR: ['onerror', 'onload', 'onclick'],
    });
  }

  function renderMarkdown(msg: HistoryEntry) {
    const cached = markdownCache.get(msg.id);
    if (cached && cached.text === msg.text) {
      return cached.html;
    }
    const html = parseMarkdown(msg.text);
    markdownCache.set(msg.id, { text: msg.text, html });
    return html;
  }

  /** Escape for inline HTML inside controlled spans (system_log beautifier). */
  function escapeHtmlPlain(s: string): string {
    return s
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  /**
   * Oligo `bb-sys-event` payload (already stripped of `__SYS_TOOL_CALL__` on the wire in some paths).
   * Renders as a single cyber line: `[SYSTEM] Accessing vault: search_vault("…")…`
   */
  function formatTelemetryHtml(payload: string): string {
    const t = payload.trim();
    if (!t) {
      return DOMPurify.sanitize(
        `<span class="system-telemetry-prefix">[SYSTEM]</span> <span class="system-telemetry-cmd">(empty)</span>`,
        { ALLOWED_TAGS: ['span'], ALLOWED_ATTR: ['class'] }
      );
    }
    let inner: string;
    if (t.startsWith('parallel::')) {
      const rest = t.slice('parallel::'.length).trim();
      inner =
        `<span class="system-telemetry-prefix">[SYSTEM]</span> ` +
        `<span class="system-telemetry-cmd">${escapeHtmlPlain(`Parallel dispatch · ${rest}`)}</span>`;
    } else if (t.startsWith('wash::')) {
      const rest = t.slice('wash::'.length).trim();
      inner =
        `<span class="system-telemetry-prefix">[SYSTEM]</span> ` +
        `<span class="system-telemetry-cmd">${escapeHtmlPlain(rest)}</span>`;
    } else {
      const sep = '::';
      const idx = t.indexOf(sep);
      if (idx === -1) {
        inner =
          `<span class="system-telemetry-prefix">[SYSTEM]</span> ` +
          `<span class="system-telemetry-cmd">${escapeHtmlPlain(t)}</span>`;
      } else {
        const tool = t.slice(0, idx);
        const rawArgs = t.slice(idx + sep.length).trim();
        let display: string;
        try {
          const j = JSON.parse(rawArgs) as Record<string, unknown>;
          if (tool === 'search_vault' && typeof j.query === 'string') {
            display = `Accessing vault: ${tool}(${JSON.stringify(j.query)})`;
          } else if (tool === 'search_vault_attribute' && typeof j.key === 'string') {
            const v = typeof j.value === 'string' ? j.value : '';
            display = `Accessing vault: ${tool}(key: ${JSON.stringify(j.key)}, value: ${JSON.stringify(v)})`;
          } else {
            display = `${tool}(${JSON.stringify(j)})`;
          }
        } catch {
          const short = rawArgs.length > 140 ? `${rawArgs.slice(0, 140)}…` : rawArgs;
          display = `${tool}(${short})`;
        }
        inner =
          `<span class="system-telemetry-prefix">[SYSTEM]</span> ` +
          `<span class="system-telemetry-cmd">${escapeHtmlPlain(`${display}…`)}</span>`;
      }
    }
    return DOMPurify.sanitize(inner, { ALLOWED_TAGS: ['span'], ALLOWED_ATTR: ['class'] });
  }

  const SYSTEM_LOG_TOOL_LABELS: Record<string, string> = {
    search_vault: '[ACCESSING VAULT]',
  };

  /**
   * Turn harsh `System accessing: tool::{json}` lines into scan-line style HTML.
   * Output is DOMPurify-sanitized (spans + class only).
   */
  function beautifySystemLogForDisplay(stored: string): string {
    const stripped = stored.replace(/^\s*System accessing:\s*/i, '').trim();
    const toolSep = /^([a-zA-Z0-9_]+)::([\s\S]*)$/;
    const m = stripped.match(toolSep);

    let inner: string;
    if (m) {
      const tool = m[1];
      const args = m[2].trim();
      const mapped = SYSTEM_LOG_TOOL_LABELS[tool];
      const labelHtml = mapped ?? `[NEURAL_INSTR::${escapeHtmlPlain(tool)}]`;
      inner = `<span class="system-log-glyph">&gt;&gt;</span><span class="system-log-label">${labelHtml}</span> <span class="system-log-payload-dim">${escapeHtmlPlain(args)}</span>`;
    } else {
      inner = `<span class="system-log-glyph">&gt;&gt;</span><span class="system-log-fallback">${escapeHtmlPlain(stripped)}</span>`;
    }

    return DOMPurify.sanitize(inner, {
      ALLOWED_TAGS: ['span'],
      ALLOWED_ATTR: ['class'],
    });
  }

  function toBackendHistory(entries: HistoryEntry[]): BackendMessage[] {
    return entries
      .filter(
        (entry) =>
          (entry.sender === 'user' || entry.sender === 'bb') && !entry.isLoading && !entry.streamAborted
      )
      .map((entry) => ({
        role: entry.sender === 'user' ? 'user' : 'assistant',
        content: entry.text,
      }));
  }

  function toSyncEntries(entries: HistoryEntry[]): Array<{ id: string; role: string; content: string; timestamp: string; persona?: string }> {
    return entries
      .filter(
        (entry) =>
          (entry.sender === 'user' || entry.sender === 'bb') && !entry.isLoading && !entry.streamAborted
      )
      .map((entry) => ({
        id: entry.id,
        role: entry.sender === 'user' ? 'user' : 'assistant',
        content: entry.text,
        timestamp: entry.timestamp,
        persona: entry.persona,
      }));
  }

  function formatSessionTimestamp(timestamp: string): string {
    const parsed = new Date(timestamp);
    if (Number.isNaN(parsed.getTime())) return timestamp;
    return parsed.toLocaleString();
  }

  function getTimestampMs(timestamp: string): number {
    const parsedMs = Date.parse(timestamp);
    return Number.isFinite(parsedMs) ? parsedMs : 0;
  }

  function getTemporalGapPx(deltaMs: number): number {
    if (deltaMs <= 0) return 14;
    const minute = 60_000;
    const normalized = deltaMs / minute;
    const logarithmic = Math.log1p(normalized) * 9;
    return Math.max(10, Math.min(88, Math.round(logarithmic)));
  }

  function buildTimelineNodes(summaries: SessionSummary[]): TimelineNode[] {
    if (!summaries.length) return [];
    const ordered = [...summaries].sort(
      (a, b) => getTimestampMs(a.timestamp) - getTimestampMs(b.timestamp)
    );
    return ordered.map((session, index) => {
      if (index === 0) {
        return { session, gap: 0 };
      }
      const deltaMs = Math.abs(
        getTimestampMs(session.timestamp) - getTimestampMs(ordered[index - 1].timestamp)
      );
      return { session, gap: getTemporalGapPx(deltaMs) };
    });
  }

  function mapRoleToSender(role: string): Sender {
    if (role === 'user') return 'user';
    if (role === 'assistant' || role === 'bb') return 'bb';
    if (role === 'error') return 'error';
    return 'system';
  }

  function stripEphemeralSystemLogs(entries: HistoryEntry[]): HistoryEntry[] {
    return entries.filter((m) => m.sender !== 'system_log');
  }

  function stripStageCards(entries: HistoryEntry[]): HistoryEntry[] {
    return entries.filter((m) => {
      if (m.sender !== 'stage_card') return true;
      // 只保留本回合流已结束、已打标保留的 Stage Card
      return m.feedback !== undefined;
    });
  }

  /** 在 bb-stream-done 调用：给本段流中的 stage_card 打标，供下一轮 stripStageCards 保留。 */
  function markStageCardsSettled(entries: HistoryEntry[]): HistoryEntry[] {
    return entries.map((h) =>
      h.sender === 'stage_card' ? { ...h, feedback: true } : h
    );
  }

  type StageCardPayload = {
    stage: StageKind;
    content: string;
    tool_name?: string;
    decision?: string;
  };

  function objectToStageEntry(o: Record<string, unknown>): StageCardPayload | null {
    const st = o.stage;
    if (st !== 'router' && st !== 'tool' && st !== 'wash' && st !== 'final') return null;
    const content = typeof o.content === 'string' ? o.content : '';
    return {
      stage: st,
      content,
      tool_name: typeof o.tool_name === 'string' ? o.tool_name : undefined,
      decision: typeof o.decision === 'string' ? o.decision : undefined,
    };
  }

  /** 旧版 Oligo 纯文本 `__SYS_TOOL_CALL__` 后缀（无 JSON） */
  function legacyStringToStageEntry(detail: string): StageCardPayload | null {
    const t = detail.trim();
    if (!t) return null;
    if (t.startsWith('parallel::')) {
      return { stage: 'router', content: t, decision: 'parallel' };
    }
    if (t.startsWith('wash::')) {
      const rest = t.slice('wash::'.length).trim();
      const tool_name = rest.split('::')[0] ?? '';
      return { stage: 'wash', content: t, tool_name: tool_name || undefined };
    }
    if (t.startsWith('denied::')) {
      const tool_name = t.slice('denied::'.length).trim();
      return { stage: 'tool', content: t, tool_name: tool_name || undefined };
    }
    if (t.startsWith('completed::')) {
      const parts = t.split('::');
      const tool_name = parts[1] ?? '';
      return { stage: 'tool', content: t, tool_name: tool_name || undefined };
    }
    const idx = t.indexOf('::');
    if (idx !== -1) {
      const tool = t.slice(0, idx);
      return {
        stage: 'router',
        content: t,
        tool_name: tool,
        decision: tool,
      };
    }
    return { stage: 'router', content: t };
  }

  function appendStageCard(ev: StageCardPayload) {
    history = [
      ...history,
      {
        id: makeId(),
        sender: 'stage_card',
        text: ev.content,
        timestamp: nowIso(),
        stage: ev.stage,
        tool_name: ev.tool_name,
        decision: ev.decision,
        persona: DEFAULT_PERSONA,
      },
    ];
  }

  function stageLabel(stage: StageKind): string {
    const m: Record<StageKind, string> = {
      router: '[Router]',
      tool: '[Tool Execution]',
      wash: '[Wash]',
      final: '[Final]',
    };
    return m[stage];
  }

  async function feedbackStage(rowId: string, rating: 'good' | 'bad', reason?: string | null) {
    const row = history.find((h) => h.id === rowId);
    if (!row || row.sender !== 'stage_card' || !row.stage) return;
    if (row.stage_feedback) return;
    try {
      await invoke('submit_segment_feedback', {
        conversationId: activeSessionId,
        stage: row.stage,
        toolName: row.tool_name ?? null,
        decision: row.decision ?? null,
        rating,
        reason: reason ?? null,
      });
    } catch (error) {
      const errText = typeof error === 'string' ? error : 'submit_segment_feedback failed';
      notifySystem(`[SEGMENT_FEEDBACK_ERROR] ${errText}`);
      return;
    }
    history = history.map((h) => (h.id === rowId ? { ...h, stage_feedback: rating } : h));
  }

  function clearTransientState() {
    editingMessageId = null;
    editBuffer = '';
    currentBBMessageId = null;
    pendingAssistantMessageId = null;
    isGenerating = false;
    pendingChunkBuffer = '';
    pendingStripSystemLogsOnFirstChunk = false;
    if (pendingFlushFrame !== null) {
      cancelAnimationFrame(pendingFlushFrame);
      pendingFlushFrame = null;
    }
  }

  async function refreshSessionHistory() {
    try {
      sessionSummaries = await invoke<SessionSummary[]>('get_session_history');
    } catch (error) {
      const errText = typeof error === 'string' ? error : 'failed to load session history';
      notifySystem(`[SESSION_HISTORY_ERROR] ${errText}`);
    }
  }

  async function loadSessionArchive(summary: SessionSummary) {
    clearTransientState();
    clearMinerTaskPolls();
    telemetryLines = [];
    try {
      const archive = await invoke<SessionArchiveEntry[]>('load_session_archive', { sessionId: summary.id });
      history = archive.map((entry) => ({
        id: entry.id || makeId(),
        sender: mapRoleToSender(entry.role),
        text: entry.content,
        timestamp: entry.timestamp || nowIso(),
        persona: entry.persona ?? DEFAULT_PERSONA,
      }));
      activeSessionId = summary.id;
      viewingArchive = summary;
      markdownCache.clear();
      await scrollToBottom();
    } catch (error) {
      const errText = typeof error === 'string' ? error : 'failed to load session archive';
      notifySystem(`[ARCHIVE_LOAD_ERROR] ${errText}`);
    }
  }

  async function resetToNewSignal() {
    clearTransientState();
    clearMinerTaskPolls();
    telemetryLines = [];
    history = [];
    activeSessionId = makeSessionId();
    viewingArchive = null;
    markdownCache.clear();
    await refreshSessionHistory();
  }

  function calculateGap(session: TimelineNode): number {
    return session.gap;
  }

  async function loadSession(sessionId: string) {
    const summary = sessionSummaries.find((item) => item.id === sessionId);
    if (!summary) return;
    await loadSessionArchive(summary);
  }

  async function deleteSession(sessionId: string, e: MouseEvent) {
    e.stopPropagation();
    e.preventDefault();
    try {
      await invoke('delete_session_history', { sessionId });
      sessionSummaries = sessionSummaries.filter((s) => s.id !== sessionId);
      if (activeSessionId === sessionId) {
        clearTransientState();
        history = [];
        activeSessionId = makeSessionId();
        viewingArchive = null;
        markdownCache.clear();
      }
    } catch (err) {
      const msg = typeof err === 'string' ? err : 'failed to delete session';
      notifySystem(`[SESSION_DELETE_ERROR] ${msg}`);
    }
  }

  async function startNewSignal() {
    await resetToNewSignal();
  }

  async function syncTimelineToBackend() {
    const entries = toSyncEntries(history);
    await invoke('sync_session_history', { sessionId: activeSessionId, entries });
  }

  function buildEmptyWorkingSlot(slot: (typeof WORKING_PROVIDER_SLOTS)[number]): ProviderConfig {
    return {
      id: slot.id,
      name: slot.name,
      api_key: '',
      base_url: slot.default_base_url,
      model_name: '',
      temperature: null,
    };
  }

  function resolveSlotFromToml(slot: (typeof WORKING_PROVIDER_SLOTS)[number]): ProviderConfig {
    const p = tomlProviders.find((x) => x.id === slot.id);
    if (p) {
      return {
        id: slot.id,
        name: (p.name || '').trim() || slot.name,
        api_key: p.apiKey ?? '',
        base_url: (p.baseUrl || '').trim() || slot.default_base_url,
        model_name: p.model ?? '',
        temperature: Number.isFinite(p.temperature) ? p.temperature : null,
      };
    }
    return buildEmptyWorkingSlot(slot);
  }

  const workingSlotIdSet = new Set(WORKING_PROVIDER_SLOTS.map((s) => s.id as string));

  function updatePingState(providerId: string, nextState: ProviderPingState) {
    pingByProviderId = {
      ...pingByProviderId,
      [providerId]: nextState,
    };
  }

  async function triggerProviderPing(provider: ProviderConfig | null | undefined) {
    if (!provider) return;
    const providerId = provider.id.trim();
    if (!providerId) return;

    const baseUrl = provider.base_url.trim();
    const apiKey = provider.api_key.trim();
    if (!baseUrl || !apiKey) {
      updatePingState(providerId, 'down');
      return;
    }

    const seq = ++pingRequestSeq;
    updatePingState(providerId, 'probing');
    try {
      const ok = await invoke<boolean>('ping_provider', {
        baseUrl: baseUrl,
        apiKey: apiKey,
      });
      if (seq !== pingRequestSeq) return;
      updatePingState(providerId, ok ? 'up' : 'down');
    } catch (e) {
      console.error("[CHIMERA FATAL] IPC Ping Failed! Reason:", e);
      if (seq !== pingRequestSeq) return;
      updatePingState(providerId, 'down');
    }
  }

  function applyLoadedConfig(nextConfig: AstrocyteConfig) {
    config = {
      active_provider_id: nextConfig?.active_provider_id ?? null,
      is_oligo_mode: nextConfig?.is_oligo_mode ?? false,
      active_skill_id: nextConfig?.active_skill_id ?? null,
    };
    const sid = (nextConfig?.active_skill_id ?? '').trim();
    activeSkillId = sid === '' ? null : sid;

    const pickFromId = (id: string) => {
      const slot = WORKING_PROVIDER_SLOTS.find((s) => s.id === id);
      if (!slot) return null;
      return resolveSlotFromToml(slot);
    };

    if (selectedProviderId && workingSlotIdSet.has(selectedProviderId)) {
      const got = pickFromId(selectedProviderId);
      if (got) {
        providerDraft = { ...got };
        return;
      }
    }
    if (config.active_provider_id && workingSlotIdSet.has(config.active_provider_id)) {
      const got = pickFromId(config.active_provider_id);
      if (got) {
        selectedProviderId = got.id;
        providerDraft = { ...got };
        return;
      }
    }
    for (const slot of WORKING_PROVIDER_SLOTS) {
      if (tomlProviders.some((p) => p.id === slot.id)) {
        selectedProviderId = slot.id;
        providerDraft = { ...resolveSlotFromToml(slot) };
        return;
      }
    }
    const first = WORKING_PROVIDER_SLOTS[0]!;
    selectedProviderId = first.id;
    providerDraft = { ...resolveSlotFromToml(first) };
  }

  function resetPersonaDraft() {
    personaDraft = {
      id: makeId(),
      name: '',
      system_prompt: '',
      authors_note: '',
    };
    selectedPersonaId = null;
  }

  function applyLoadedPersonas(snapshot: PersonaSnapshot) {
    personaSnapshot = {
      active_persona_id: snapshot?.active_persona_id ?? '',
      personas: snapshot?.personas ?? [],
    };

    if (selectedPersonaId) {
      const selected = personaSnapshot.personas.find((persona) => persona.id === selectedPersonaId);
      if (selected) {
        personaDraft = { ...selected };
        return;
      }
    }
    const active = personaSnapshot.personas.find(
      (persona) => persona.id === personaSnapshot.active_persona_id
    );
    if (active) {
      selectedPersonaId = active.id;
      personaDraft = { ...active };
      return;
    }
    resetPersonaDraft();
  }

  function logPersonaShift(personaName: string) {
    console.log(`%c[SYSTEM] Neural link shifted to: ${personaName}`, 'color: #bb9af7; font-weight: 700;');
  }

  async function loadPersonas() {
    personaError = '';
    personaStatus = '';
    try {
      const loaded = await invoke<PersonaSnapshot>('get_personas');
      applyLoadedPersonas(loaded);
      personaStatus = '[PERSONA_MATRIX_SYNCED]';
    } catch (error) {
      const errText = typeof error === 'string' ? error : 'Failed to load persona matrix';
      personaError = errText;
      notifySystem(`[PERSONA_LOAD_ERROR] ${errText}`);
    }
  }

  function selectPersona(persona: PersonaConfig) {
    selectedPersonaId = persona.id;
    personaDraft = { ...persona };
  }

  async function activatePersona(id: string) {
    personaError = '';
    personaStatus = '';
    try {
      await invoke('set_active_persona', { id });
      await loadPersonas();
      const active = personaSnapshot.personas.find((persona) => persona.id === personaSnapshot.active_persona_id);
      if (active) {
        logPersonaShift(active.name);
      }
      personaStatus = '[ACTIVE_PERSONA_SHIFTED]';
    } catch (error) {
      const errText = typeof error === 'string' ? error : 'Failed to set active persona';
      personaError = errText;
      notifySystem(`[PERSONA_ACTIVE_ERROR] ${errText}`);
    }
  }

  async function savePersonaToMatrix() {
    settingsSaving = true;
    personaError = '';
    personaStatus = '';
    try {
      const payload: PersonaConfig = {
        id: personaDraft.id.trim() || makeId(),
        name: personaDraft.name.trim(),
        system_prompt: personaDraft.system_prompt.trim(),
        authors_note: personaDraft.authors_note?.trim() || null,
      };
      const snapshot = await invoke<PersonaSnapshot>('save_persona', { persona: payload });
      applyLoadedPersonas(snapshot);
      selectedPersonaId = payload.id;
      personaStatus = '[PERSONA_SLOT_SAVED]';
    } catch (error) {
      const errText = typeof error === 'string' ? error : 'Failed to save persona';
      personaError = errText;
      notifySystem(`[PERSONA_SAVE_ERROR] ${errText}`);
    } finally {
      settingsSaving = false;
    }
  }

  async function removePersona(id: string) {
    settingsSaving = true;
    personaError = '';
    personaStatus = '';
    try {
      const snapshot = await invoke<PersonaSnapshot>('delete_persona', { id });
      applyLoadedPersonas(snapshot);
      const active = personaSnapshot.personas.find((persona) => persona.id === personaSnapshot.active_persona_id);
      if (active) {
        logPersonaShift(active.name);
      }
      personaStatus = '[PERSONA_SLOT_PURGED]';
    } catch (error) {
      const errText = typeof error === 'string' ? error : 'Failed to delete persona';
      personaError = errText;
      notifySystem(`[PERSONA_DELETE_ERROR] ${errText}`);
    } finally {
      settingsSaving = false;
    }
  }

  async function loadTomlProviders() {
    try {
      tomlProviders = await invoke<TomlProviderRow[]>('get_available_providers');
    } catch (error) {
      tomlProviders = [];
      const errText = typeof error === 'string' ? error : 'Failed to load TOML providers';
      notifySystem(`[TOML_PROVIDERS_ERROR] ${errText}`);
    }
  }

  async function loadConfig() {
    settingsError = '';
    settingsStatus = '';
    try {
      await loadTomlProviders();
      const loaded = await invoke<AstrocyteConfig>('get_config');
      applyLoadedConfig(loaded);
      settingsStatus = '[ARSENAL_SYNCED]';
    } catch (error) {
      const errText = typeof error === 'string' ? error : 'Failed to load arsenal config';
      settingsError = errText;
      notifySystem(`[CONFIG_LOAD_ERROR] ${errText}`);
    }
  }

  async function reloadProvidersFromToml() {
    settingsSaving = true;
    settingsError = '';
    try {
      await invoke('reload_chimera_config');
      await loadConfig();
      const sid = (selectedProviderId || '').trim();
      const slot =
        WORKING_PROVIDER_SLOTS.find((s) => s.id === sid) ?? WORKING_PROVIDER_SLOTS[0]!;
      providerDraft = { ...resolveSlotFromToml(slot) };
      void triggerProviderPing(providerDraft);
    } catch (error) {
      const errText = typeof error === 'string' ? error : 'Failed to reload config.toml';
      settingsError = errText;
      notifySystem(`[TOML_RELOAD_ERROR] ${errText}`);
    } finally {
      settingsSaving = false;
    }
  }

  async function activateProvider(id: string) {
    settingsError = '';
    settingsStatus = '';
    try {
      await invoke('set_active_provider', { id });
      await loadConfig();
      settingsStatus = '[ACTIVE_PROVIDER_SWITCHED]';
    } catch (error) {
      const errText = typeof error === 'string' ? error : 'Failed to set active provider';
      settingsError = errText;
      notifySystem(`[CONFIG_ACTIVE_ERROR] ${errText}`);
    }
  }

  function selectProvider(provider: ProviderConfig) {
    selectedProviderId = provider.id;
    providerDraft = { ...provider };
    void triggerProviderPing(provider);
    void activateProvider(provider.id);
  }

  async function loadSystemMetrics() {
    try {
      const m = await invoke<SystemMetricsSummary>('get_system_metrics');
      systemMetrics = {
        total_requests: m.total_requests ?? 0,
        success_rate: m.success_rate ?? 0,
        avg_latency_ms: m.avg_latency_ms ?? 0,
        total_tokens: m.total_tokens ?? 0,
        top_skills: Array.isArray(m.top_skills) ? m.top_skills : [],
        tool_stats: Array.isArray(m.tool_stats) ? m.tool_stats : [],
        wash_stats: normalizeWashStats(m.wash_stats),
      };
    } catch (error) {
      const errText = typeof error === 'string' ? error : 'Failed to load metrics';
      notifySystem(`[METRICS_ERROR] ${errText}`);
      systemMetrics = defaultSystemMetrics();
    }
  }

  function openSettingsPanel() {
    showSettingsPanel = true;
    settingsError = '';
    personaError = '';
    void loadConfig();
    void loadPersonas();
    void loadSystemMetrics();
  }

  function closeSettingsPanel() {
    if (settingsSaving) return;
    showSettingsPanel = false;
  }

  async function toggleFireSelector() {
    const next = !config.is_oligo_mode;
    try {
      await invoke('set_is_oligo_mode', { enabled: next });
      await loadConfig();
    } catch (error) {
      const errText = typeof error === 'string' ? error : 'Failed to toggle fire mode';
      notifySystem(`[FIRE_SELECTOR_ERROR] ${errText}`);
    }
  }

  function onOverlayKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      closeSettingsPanel();
    }
  }

  function onPhantomTriggerEnter() {
    if (phantomHoverTimeout !== null) {
      clearTimeout(phantomHoverTimeout);
      phantomHoverTimeout = null;
    }
    phantomHoverTimeout = setTimeout(() => {
      phantomHoverTimeout = null;
      void invoke('set_phantom_sidebar_visible', { visible: true }).catch((error) => {
        console.error(error);
      });
    }, 300);
  }

  function onPhantomTriggerLeave() {
    if (phantomHoverTimeout !== null) {
      clearTimeout(phantomHoverTimeout);
      phantomHoverTimeout = null;
    }
  }

  function onTimelineTriggerEnter() {
    if (timelineHoverTimeout !== null) {
      clearTimeout(timelineHoverTimeout);
      timelineHoverTimeout = null;
    }
    timelineHoverTimeout = setTimeout(() => {
      timelineHoverTimeout = null;
      void invoke('set_timeline_visible', { visible: true }).catch(console.error);
    }, 300);
  }

  function onTimelineTriggerLeave() {
    if (timelineHoverTimeout !== null) {
      clearTimeout(timelineHoverTimeout);
      timelineHoverTimeout = null;
    }
  }

  async function flushPendingChunk() {
    if (!pendingChunkBuffer || !currentBBMessageId) return;
    const mergedChunk = pendingChunkBuffer;
    pendingChunkBuffer = '';
    history = history.map((msg) =>
      msg.id === currentBBMessageId
        ? {
            ...msg,
            text: `${msg.text}${mergedChunk}`,
          }
        : msg
    );
    await scrollToBottom();
  }

  function queueBBChunk(fragment: string) {
    pendingChunkBuffer += fragment;
    if (pendingFlushFrame !== null) return;
    pendingFlushFrame = requestAnimationFrame(async () => {
      pendingFlushFrame = null;
      await flushPendingChunk();
    });
  }

  function ensureBBStreamEntry(): void {
    const last = history[history.length - 1];
    const currentExists = currentBBMessageId
      ? history.some((msg) => msg.id === currentBBMessageId)
      : false;
    if (currentExists && last?.id === currentBBMessageId) {
      return;
    }
    const loadingId = pendingAssistantMessageId ?? makeId();
    pendingAssistantMessageId = null;
    history = [
      ...history,
      {
        id: loadingId,
        sender: 'bb',
        text: '',
        timestamp: nowIso(),
        persona: personaSnapshot.active_persona_id || DEFAULT_PERSONA,
        isLoading: true,
      },
    ];
    currentBBMessageId = loadingId;
  }

  function beginEditingMessage(msg: HistoryEntry) {
    if (msg.isLoading) return;
    editingMessageId = msg.id;
    editBuffer = msg.text;
    void tick().then(() => {
      editTextareaEl?.focus();
      const end = editBuffer.length;
      editTextareaEl?.setSelectionRange(end, end);
    });
  }

  async function commitEditingMessage() {
    if (!editingMessageId) return;
    const targetId = editingMessageId;
    const nextText = editBuffer;
    history = history.map((msg) =>
      msg.id === targetId
        ? {
            ...msg,
            text: nextText,
            timestamp: nowIso(),
          }
        : msg
    );
    editingMessageId = null;
    editBuffer = '';
    try {
      await syncTimelineToBackend();
    } catch (error) {
      const errText = typeof error === 'string' ? error : 'failed to sync edited message';
      notifySystem(`[TIMELINE_SYNC_ERROR] ${errText}`);
    }
  }

  async function submitFeedback(messageId: string, success: boolean) {
    const message = history.find((m) => m.id === messageId);
    if (!message || message.sender !== 'bb' || message.isLoading) return;

    const skillId = (message.skill_id && message.skill_id.trim()) || 'unknown';
    const tokens = message.tokens ?? 0;

    try {
      await invoke('submit_skill_feedback', { skillId, success, tokens });
    } catch (error) {
      const errText = typeof error === 'string' ? error : 'submit_skill_feedback failed';
      notifySystem(`[FEEDBACK_ERROR] ${errText}`);
      return;
    }

    history = history.map((msg) =>
      msg.id === messageId ? { ...msg, feedback: success } : msg
    );
    void refreshAvailableSkills();
  }

  async function deleteMessage(msg: HistoryEntry) {
    if (msg.isLoading) return;
    try {
      await invoke('delete_chat_message', { sessionId: activeSessionId, msgId: msg.id });
      history = history.filter((entry) => entry.id !== msg.id);
      if (currentBBMessageId === msg.id) {
        currentBBMessageId = null;
        isGenerating = false;
      }
      await syncTimelineToBackend();
    } catch (error) {
      const errText = typeof error === 'string' ? error : 'failed to delete message';
      notifySystem(`[DELETE_ERROR] ${errText}`);
    }
  }

  async function onAbortGeneration() {
    if (!isGenerating) return;
    try {
      await invoke('abort_generation');
    } catch (error) {
      const errText = typeof error === 'string' ? error : 'abort_generation failed';
      notifySystem(`[ABORT_ERROR] ${errText}`);
      return;
    }
    isGenerating = false;
  }

  async function dispatchEvaluate(message: string, appendUser: boolean) {
    if (isGenerating) return;
    const normalized = message.trim();
    if (!normalized) return;

    let userMsgId: string;
    if (appendUser) {
      const userEntry: HistoryEntry = {
        id: makeId(),
        sender: 'user',
        text: normalized,
        timestamp: nowIso(),
        persona: personaSnapshot.active_persona_id || DEFAULT_PERSONA,
      };
      history = [...history, userEntry];
      userMsgId = userEntry.id;
    } else {
      const lastUser = history.filter((m) => m.sender === 'user').pop();
      userMsgId = lastUser?.id ?? makeId();
    }

    currentBBMessageId = null;
    pendingAssistantMessageId = makeId();
    pendingSkillFeedbackSkillId = activeSkillId;
    isGenerating = true;
    pendingStripSystemLogsOnFirstChunk = true;
    history = stripStageCards(history);
    telemetryLines = [];
    await scrollToBottom();

    try {
      // L2 `persona` 仍从当前选中 Persona 下发；L3 Author's Note 仅来自 Persona 设置中的持久化 `authors_note`（Rust 侧从 active_persona 读取）。
      await invoke('evaluate_payload', {
        payload: normalized,
        sessionId: activeSessionId,
        userMessageId: userMsgId,
        assistantMessageId: pendingAssistantMessageId,
        skillId: activeSkillId,
        persona: selectedPersona?.system_prompt || null,
      });
    } catch (error) {
      const errText = typeof error === 'string' ? error : 'Unknown gateway failure';
      ensureBBStreamEntry();
      if (currentBBMessageId) {
        history = history.map((msg) =>
          msg.id === currentBBMessageId
            ? {
                ...msg,
                text: `[ERROR] ${errText}`,
                timestamp: nowIso(),
                isLoading: false,
                skill_id: pendingSkillFeedbackSkillId,
                tokens: roughTokenEstimate(`[ERROR] ${errText}`),
              }
            : msg
        );
      }
      currentBBMessageId = null;
      pendingAssistantMessageId = null;
      pendingSkillFeedbackSkillId = null;
      isGenerating = false;
      pendingStripSystemLogsOnFirstChunk = false;
      telemetryLines = telemetryLines.map((line) => ({ ...line, settled: true }));
    }
  }

  async function retryFromMessage(msg: HistoryEntry) {
    if (msg.sender !== 'bb' || msg.isLoading || isGenerating) return;
    const index = history.findIndex((entry) => entry.id === msg.id);
    if (index <= 0) return;

    let prevUser: HistoryEntry | null = null;
    let prevUserIndex = -1;
    for (let i = index - 1; i >= 0; i -= 1) {
      if (history[i].sender === 'user') {
        prevUser = history[i];
        prevUserIndex = i;
        break;
      }
    }
    if (!prevUser || prevUserIndex < 0) {
      notifySystem('[RETRY_ABORTED] Cannot find previous user message.');
      return;
    }

    // Cut through the last real user turn; drop this BB and any transient system_log in between.
    history = history.slice(0, prevUserIndex + 1);
    try {
      await syncTimelineToBackend();
    } catch (error) {
      const errText = typeof error === 'string' ? error : 'failed to sync retry timeline';
      notifySystem(`[TIMELINE_SYNC_ERROR] ${errText}`);
      return;
    }
    await dispatchEvaluate(prevUser.text, false);
  }

  async function onAiAction(action: 'edit' | 'delete' | 'retry', msg: HistoryEntry) {
    if (action === 'edit') {
      beginEditingMessage(msg);
      return;
    }
    if (action === 'delete') {
      await deleteMessage(msg);
      return;
    }
    await retryFromMessage(msg);
  }

  function onEditKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && event.shiftKey) {
      event.preventDefault();
      void commitEditingMessage();
    }
    if (event.key === 'Escape') {
      editingMessageId = null;
      editBuffer = '';
    }
  }

  onMount(async () => {
    await loadConfig();
    await loadPersonas();
    try {
      availableSkills = await invoke<SkillWithStats[]>('get_available_skills');
    } catch (error) {
      availableSkills = [];
      const errText = typeof error === 'string' ? error : 'failed to load available skills';
      notifySystem(`[SKILLS_LOAD_ERROR] ${errText}`);
    }
    await refreshSessionHistory();
    await tick();
    if (inputEl) resetInputHeight();

    unlistenClipboardHijack = await listen<string>('clipboard-hijack', async (event) => {
      const payload = typeof event.payload === 'string' ? event.payload : '';
      if (!payload.trim()) return;

      inputSignal = inputSignal ? `${payload} ${inputSignal}` : payload;
      await tick();
      if (inputEl) {
        inputEl.focus();
        const cursor = inputEl.value.length;
        inputEl.setSelectionRange(cursor, cursor);
        adjustInputHeight();
      }
    });

    unlistenBBChunk = await listen<string>('bb-stream-chunk', async (event) => {
      const fragment = typeof event.payload === 'string' ? event.payload : '';
      if (!fragment) return;
      if (fragment.trim() && pendingStripSystemLogsOnFirstChunk) {
        history = stripEphemeralSystemLogs(history);
        pendingStripSystemLogsOnFirstChunk = false;
      }
      ensureBBStreamEntry();
      queueBBChunk(fragment);
    });

    unlistenBBSysEvent = await listen<unknown>('bb-sys-event', async (event) => {
      const raw = event.payload;
      if (typeof raw === 'string') {
        if (!raw.trim() || raw === '[Generation Aborted by User]') return;
        const entry = legacyStringToStageEntry(raw);
        if (entry) {
          appendStageCard(entry);
        } else {
          telemetryLines = [
            ...telemetryLines,
            {
              id: makeId(),
              html: formatTelemetryHtml(raw),
              settled: false,
            },
          ];
        }
        await scrollToBottom();
        return;
      }
      if (raw && typeof raw === 'object') {
        const entry = objectToStageEntry(raw as Record<string, unknown>);
        if (entry) {
          appendStageCard(entry);
          await scrollToBottom();
        }
      }
    });

    unlistenBBDone = await listen<
      { error?: boolean; aborted?: boolean; message?: string; reason?: string } | string
    >('bb-stream-done', async (event) => {
        const raw = event.payload;
        const isError = typeof raw === 'object' && raw !== null && raw.error === true;
        const isAborted = typeof raw === 'object' && raw !== null && raw.aborted === true;
        if (
          isError &&
          typeof raw === 'object' &&
          raw !== null &&
          typeof (raw as { message?: string }).message === 'string' &&
          (raw as { message: string }).message.trim()
        ) {
          history = [
            ...history,
            {
              id: makeId(),
              sender: 'error',
              text: (raw as { message: string }).message.trim(),
              timestamp: nowIso(),
              persona: personaSnapshot.active_persona_id || DEFAULT_PERSONA,
            },
          ];
        }
        if (pendingFlushFrame !== null) {
          cancelAnimationFrame(pendingFlushFrame);
          pendingFlushFrame = null;
        }
        await flushPendingChunk();
        if (isAborted) {
          if (currentBBMessageId) {
            history = history.map((msg) =>
              msg.id === currentBBMessageId
                ? {
                    ...msg,
                    isLoading: false,
                    streamAborted: true,
                    timestamp: nowIso(),
                    skill_id: pendingSkillFeedbackSkillId,
                    tokens: roughTokenEstimate(msg.text),
                  }
                : msg
            );
          }
          history = markStageCardsSettled(stripEphemeralSystemLogs(history));
          telemetryLines = telemetryLines.map((line) => ({ ...line, settled: true }));
          isGenerating = false;
          pendingStripSystemLogsOnFirstChunk = false;
          currentBBMessageId = null;
          pendingAssistantMessageId = null;
          pendingSkillFeedbackSkillId = null;
          await scrollToBottom();
          await refreshSessionHistory();
          return;
        }
        let minerProbeText: string | null = null;
        if (currentBBMessageId) {
          history = history.map((msg) => {
            if (msg.id !== currentBBMessageId) return msg;
            const fallback = isError
              ? (msg.text.trim() ? msg.text : '[STREAM_ABORTED]')
              : msg.text;
            if (!isError) {
              minerProbeText = fallback;
            }
            return {
              ...msg,
              text: fallback,
              isLoading: false,
              timestamp: nowIso(),
              skill_id: pendingSkillFeedbackSkillId,
              tokens: roughTokenEstimate(fallback),
            };
          });
        }
        if (!isError && !isAborted && minerProbeText) {
          const tid = extractMinerTaskIdFromAssistantText(minerProbeText);
          if (tid) {
            startMinerTaskPolling(tid);
          }
        }
        history = markStageCardsSettled(stripEphemeralSystemLogs(history));
        telemetryLines = telemetryLines.map((line) => ({ ...line, settled: true }));
        isGenerating = false;
        pendingStripSystemLogsOnFirstChunk = false;
        currentBBMessageId = null;
        pendingAssistantMessageId = null;
        pendingSkillFeedbackSkillId = null;
        await scrollToBottom();
        await refreshSessionHistory();
      }
    );

    unlistenSublimateRequest = await listen<string>('sublimate-request', async (event) => {
      const payload = typeof event.payload === 'string' ? event.payload : '';
      if (!payload.trim()) return;
      inputSignal = '[System Direct: Please sublimate these notes]\n\n' + payload;
      await tick();
      void submitInput();
    });

    unlistenLoadSession = await listen<string>('load-session', async (event) => {
      const sessionId = typeof event.payload === 'string' ? event.payload : '';
      if (!sessionId.trim()) return;
      await loadSession(sessionId);
    });

    unlistenNewSignal = await listen('new-signal', async () => {
      await startNewSignal();
    });

    await loadSystemMetrics();
    systemMetricsRefresh = setInterval(() => {
      void loadSystemMetrics();
    }, 30000);
  });

  onDestroy(() => {
    if (unlistenClipboardHijack) {
      unlistenClipboardHijack();
      unlistenClipboardHijack = null;
    }
    if (unlistenBBChunk) {
      unlistenBBChunk();
      unlistenBBChunk = null;
    }
    if (unlistenBBSysEvent) {
      unlistenBBSysEvent();
      unlistenBBSysEvent = null;
    }
    if (unlistenBBDone) {
      unlistenBBDone();
      unlistenBBDone = null;
    }
    if (unlistenSublimateRequest) {
      unlistenSublimateRequest();
      unlistenSublimateRequest = null;
    }
    if (unlistenLoadSession) {
      unlistenLoadSession();
      unlistenLoadSession = null;
    }
    if (unlistenNewSignal) {
      unlistenNewSignal();
      unlistenNewSignal = null;
    }
    if (systemMetricsRefresh !== null) {
      clearInterval(systemMetricsRefresh);
      systemMetricsRefresh = null;
    }
    if (pendingFlushFrame !== null) {
      cancelAnimationFrame(pendingFlushFrame);
      pendingFlushFrame = null;
    }
    if (phantomHoverTimeout !== null) {
      clearTimeout(phantomHoverTimeout);
      phantomHoverTimeout = null;
    }
    if (timelineHoverTimeout !== null) {
      clearTimeout(timelineHoverTimeout);
      timelineHoverTimeout = null;
    }
    clearMinerTaskPolls();
    inputSizeDragRelease?.();
    inputSizeDragRelease = null;
  });

  const INPUT_MIN_HEIGHT = 60;
  const INPUT_MAX_HEIGHT_RATIO = 0.4;

  function adjustInputHeight() {
    if (!inputEl) return;
    const maxH = window.innerHeight * INPUT_MAX_HEIGHT_RATIO;
    const prevH = inputEl.getBoundingClientRect().height;
    inputEl.style.height = 'auto';
    const natural = Math.min(
      Math.max(inputEl.scrollHeight, INPUT_MIN_HEIGHT),
      maxH
    );
    const h = Math.min(Math.max(natural, prevH), maxH);
    inputEl.style.height = `${h}px`;
    inputEl.style.overflowY = h >= maxH ? 'auto' : 'hidden';
  }

  function resetInputHeight() {
    if (!inputEl) return;
    inputEl.style.height = `${INPUT_MIN_HEIGHT}px`;
    inputEl.style.overflowY = 'hidden';
  }

  /** 顶部拖条：向上拖增大输入区（原生 resize 在 flex/WebView 中常不显示，且只支持右下角拉） */
  function onInputSizeGripDown(e: PointerEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (!inputEl) return;
    inputSizeDragRelease?.();
    const track = e.currentTarget as HTMLElement;
    const startY = e.clientY;
    const startH = inputEl.getBoundingClientRect().height;
    const maxH = () => window.innerHeight * INPUT_MAX_HEIGHT_RATIO;

    function move(ev: PointerEvent) {
      if (!inputEl) return;
      const next = Math.min(
        Math.max(startH + (startY - ev.clientY), INPUT_MIN_HEIGHT),
        maxH()
      );
      inputEl.style.height = `${next}px`;
      inputEl.style.overflowY = 'auto';
    }
    function up() {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      try {
        track.releasePointerCapture(e.pointerId);
      } catch {
        /* no-op */
      }
      inputSizeDragRelease = null;
    }
    try {
      track.setPointerCapture(e.pointerId);
    } catch {
      /* no-op */
    }
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
    inputSizeDragRelease = up;
  }

  function onInputKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      void submitInput();
    }
  }

  async function onPersonaMenuPick(persona: PersonaConfig) {
    personaDropdownOpen = false;
    if (persona.id === personaSnapshot.active_persona_id) return;
    await activatePersona(persona.id);
  }

  async function onSkillMenuPick(id: string | null) {
    skillPanelOpen = false;
    const next = id && id.trim() !== '' ? id : null;
    if (next === activeSkillId) return;
    activeSkillId = next;
    await onSkillSelect();
  }

  function selectSkill(skill: SkillWithStats | null) {
    void onSkillMenuPick(skill?.id ?? null);
  }

  async function onSkillSelect() {
    const sid = activeSkillId && activeSkillId !== '' ? activeSkillId : null;
    try {
      await invoke('set_active_skill_id', { skillId: sid });
    } catch (error) {
      const errText = typeof error === 'string' ? error : 'failed to persist active skill';
      notifySystem(`[SKILL_PERSIST_ERROR] ${errText}`);
    }
  }

  async function submitInput(event?: SubmitEvent) {
    event?.preventDefault();
    const message = inputSignal.trim();
    if (!message || isGenerating) return;
    inputSignal = '';
    resetInputHeight();
    await dispatchEvaluate(message, true);
  }
</script>

<svelte:window on:click={closeQuickSwitchDropdowns} />

<main class="hud-shell">
  <ActiveTaskPanel />
  <ActiveToolTelemetry />
  <div class="hud-main">
    <header class="hud-header" data-tauri-drag-region>
      <span class="hud-header-brand">[ BB_CHANNEL :: ASTROCYTE GATEWAY ]</span>
      <div class="header-right">
        <button
          class="fire-selector"
          class:active={config.is_oligo_mode}
          type="button"
          on:click={toggleFireSelector}
          title={config.is_oligo_mode ? 'Oligo Core engaged – click to switch to Direct Link' : 'Direct Link – click to engage Oligo Core'}
        >
          {config.is_oligo_mode ? '[OLIGO CORE]' : '[DIRECT LINK]'}
        </button>
        <button class="settings-trigger" type="button" on:click={openSettingsPanel} title="Arsenal Settings">
          ⚙
        </button>
      </div>
    </header>

    <div class="middle-arena">
      <div
        class="timeline-trigger-zone"
        on:mouseenter={onTimelineTriggerEnter}
        on:mouseleave={onTimelineTriggerLeave}
        aria-hidden="true"
      ></div>

      <section class="hud-output" aria-label="output" bind:this={outputEl}>
        {#if viewingArchive}
          <div class="archive-banner">{archiveBannerText}</div>
        {/if}
        {#if telemetryLines.length > 0}
          <div class="oligo-telemetry-stack" aria-live="polite">
            {#each telemetryLines as line (line.id)}
              <div
                class="system-log-raw oligo-telemetry-line"
                class:oligo-telemetry-line--settled={line.settled}
              >
                {@html line.html}
              </div>
            {/each}
          </div>
        {/if}
        {#if Object.keys(minerTaskById).length > 0}
          <div class="miner-task-stack" aria-live="polite">
            {#each Object.entries(minerTaskById) as [tid, row] (tid)}
              <div class="miner-task-row">
                <span class="miner-task-meta">
                  Miner task <code class="miner-task-id">{tid}</code>
                  <span class="miner-task-typeHint">· polling ~/.chimera/tasks</span>
                </span>
                <div
                  class="miner-task-bar"
                  role="progressbar"
                  aria-valuemin="0"
                  aria-valuemax="100"
                  aria-valuenow={Math.round(row.progress * 100)}
                >
                  <div class="miner-task-bar-fill" style="width: {Math.min(100, Math.max(0, row.progress * 100))}%"></div>
                </div>
                <div class="miner-task-line">{row.line}</div>
              </div>
            {/each}
          </div>
        {/if}
        {#each history as msg (msg.id)}
        <article
          class="msg-row"
          data-sender={msg.sender}
          data-stream-aborted={msg.streamAborted ? 'true' : undefined}
        >
          {#if editingMessageId === msg.id}
            <textarea
              class="msg-editor"
              bind:this={editTextareaEl}
              bind:value={editBuffer}
              on:keydown={onEditKeydown}
              on:blur={commitEditingMessage}
            ></textarea>
          {:else if msg.sender === 'stage_card' && msg.stage}
            <div class="stage-card" data-stage={msg.stage}>
              <div class="stage-header">
                <span class="stage-label">{stageLabel(msg.stage)}</span>
                {#if msg.tool_name}
                  <span class="stage-name">{msg.tool_name}</span>
                {/if}
              </div>
              <div class="stage-content">{msg.text}</div>
              {#if msg.stage_feedback === undefined}
                <div class="stage-feedback">
                  <button
                    type="button"
                    class="btn btn--hud stage-fb stage-fb-good"
                    title="Good"
                    on:click={() => feedbackStage(msg.id, 'good')}
                  >
                    +
                  </button>
                  <button
                    type="button"
                    class="btn btn--hud stage-fb stage-fb-bad"
                    title="Bad"
                    on:click={() => feedbackStage(msg.id, 'bad')}
                  >
                    −
                  </button>
                </div>
              {:else}
                <div class="stage-feedback-done" data-rating={msg.stage_feedback}>
                  {msg.stage_feedback === 'good' ? 'Rated: good' : 'Rated: bad'}
                </div>
              {/if}
            </div>
          {:else if msg.sender === 'system_log'}
            <div class="system-log-raw">{@html beautifySystemLogForDisplay(msg.text)}</div>
          {:else if msg.sender === 'error'}
            <div class="error-card" role="alert">
              <span class="error-icon" aria-hidden="true">⚠️</span>
              <span class="error-text">{msg.text}</span>
            </div>
          {:else if msg.sender === 'bb' || msg.sender === 'user'}
            <div class="msg-content" data-sender={msg.sender} data-loading={msg.isLoading ? 'true' : undefined}>
              {@html renderMarkdown(msg)}
            </div>
            {#if msg.sender === 'bb' && !msg.isLoading && msg.feedback === undefined}
              <div class="feedback-buttons">
                <button
                  type="button"
                  class="btn btn--hud feedback-btn feedback-good"
                  on:click={() => submitFeedback(msg.id, true)}
                  title="满意"
                >
                  满意
                </button>
                <button
                  type="button"
                  class="btn btn--hud feedback-btn feedback-bad"
                  on:click={() => submitFeedback(msg.id, false)}
                  title="不满意"
                >
                  不满意
                </button>
              </div>
            {:else if msg.sender === 'bb' && msg.feedback !== undefined}
              <div class="feedback-indicator">
                {msg.feedback ? '已标记为满意' : '已标记为不满意'}
              </div>
            {/if}
          {:else}
            <div class="msg-content" data-sender={msg.sender} data-loading={msg.isLoading ? 'true' : undefined}>
              {msg.text}
            </div>
          {/if}
          {#if msg.sender === 'user' || msg.sender === 'bb'}
            <div class="msg-actions" aria-label="assistant message actions">
              <button class="btn btn--icon msg-action" type="button" title="Edit" on:click={() => onAiAction('edit', msg)}>E</button>
              <button class="btn btn--icon msg-action" type="button" title="Delete" on:click={() => onAiAction('delete', msg)}>D</button>
              <button class="btn btn--icon msg-action" type="button" title="Retry" disabled={msg.sender !== 'bb' || msg.isLoading || isGenerating} on:click={() => onAiAction('retry', msg)}>R</button>
            </div>
          {/if}
        </article>
      {/each}
      </section>
    </div>

    <div
      class="phantom-trigger-zone"
      on:mouseenter={onPhantomTriggerEnter}
      on:mouseleave={onPhantomTriggerLeave}
      aria-hidden="true"
    ></div>

    <form class="hud-input-wrap" on:submit={submitInput}>
      <div
        class="hud-input-size-grip"
        role="separator"
        aria-orientation="horizontal"
        aria-label="拖动以调整输入区高度，向上为增大"
        on:pointerdown|stopPropagation={onInputSizeGripDown}
      ></div>
      <div class="persona-quick-switch">
        <span class="quick-switch-legend" id="persona-quick-legend">persona</span>
        <div class="persona-selector">
          <button
            type="button"
            class="persona-trigger"
            id="persona-quick-select"
            aria-haspopup="listbox"
            aria-expanded={personaDropdownOpen}
            aria-labelledby="persona-quick-legend"
            on:click|stopPropagation={togglePersonaDropdown}
          >
            {#if selectedPersona}
              <span class="persona-label">[PERSONA: {selectedPersona.name}]</span>
            {:else}
              <span class="persona-label-empty">[PERSONA: None]</span>
            {/if}
          </button>

          {#if personaDropdownOpen}
            <div
              class="persona-dropdown{personaDropdownUp ? ' drop-up' : ''}"
              aria-label="Select persona"
            >
              {#each personaSnapshot.personas as persona (persona.id)}
                <button
                  type="button"
                  class="persona-item"
                  class:persona-item--active={persona.id === personaSnapshot.active_persona_id}
                  on:click|stopPropagation={() => onPersonaMenuPick(persona)}
                >
                  {persona.name}
                </button>
              {/each}
            </div>
          {/if}
        </div>

        <span class="quick-switch-legend" id="skill-quick-legend">skill</span>
        <div class="skill-selector skill-selector-container">
          <button
            type="button"
            class="skill-trigger"
            id="skill-quick-select"
            aria-haspopup="dialog"
            aria-expanded={skillPanelOpen}
            aria-labelledby="skill-quick-legend"
            on:click|stopPropagation={toggleSkillPanel}
          >
            {#if selectedSkill}
              <span class="skill-label">[SKILL: {selectedSkill.name}]</span>
              <div class="skill-trigger-stats">
                <div class="stat-item stat-item--inline">
                  <span class="stat-value">{selectedSkill.stats.usage_count}</span>
                  <span class="stat-label">次</span>
                </div>
                <div class="stat-item stat-item--inline">
                  <span class="stat-value">{(selectedSkill.stats.success_rate * 100).toFixed(0)}%</span>
                  <span class="stat-label">成功率</span>
                </div>
                <div class="stat-item stat-item--inline">
                  <span class="stat-value">{selectedSkill.stats.avg_tokens}</span>
                  <span class="stat-label">tokens</span>
                </div>
              </div>
            {:else}
              <span class="skill-label-empty">[SKILL: None]</span>
            {/if}
          </button>

          {#if skillPanelOpen}
            <div
              class="skill-panel{skillPanelUp ? ' skill-panel--drop-up' : ''}"
              aria-label="Select skill"
              on:click|stopPropagation
              role="presentation"
            >
              <div
                class="skill-card skill-card--none"
                class:active={!activeSkillId}
                role="button"
                tabindex="0"
                on:click={() => selectSkill(null)}
                on:keydown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    selectSkill(null);
                  }
                }}
              >
                <h3 class="skill-name">[No Skill]</h3>
                <div class="skill-meta-row">
                  <span class="skill-category">—</span>
                  <div class="skill-stats">
                    <div class="stat-item">
                      <span class="stat-value">0</span>
                      <span class="stat-label">次</span>
                    </div>
                    <div class="stat-item">
                      <span class="stat-value">—</span>
                      <span class="stat-label">成功率</span>
                    </div>
                    <div class="stat-item">
                      <span class="stat-value">0</span>
                      <span class="stat-label">tokens</span>
                    </div>
                  </div>
                </div>
                <p class="skill-desc">不使用预设 Skill，仅 Persona / 直连。</p>
              </div>
              {#each availableSkills as skill (skill.id)}
                <div
                  class="skill-card"
                  class:active={selectedSkill?.id === skill.id}
                  role="button"
                  tabindex="0"
                  on:click={() => selectSkill(skill)}
                  on:keydown={(e) => {
                    if (e.key === 'Enter' || e.key === ' ') {
                      e.preventDefault();
                      selectSkill(skill);
                    }
                  }}
                >
                  <h3 class="skill-name">{skill.name}</h3>
                  <div class="skill-meta-row">
                    <span class="skill-category">{skill.category}</span>
                    <div class="skill-stats">
                      <div class="stat-item">
                        <span class="stat-value">{skill.stats.usage_count}</span>
                        <span class="stat-label">次</span>
                      </div>
                      <div class="stat-item">
                        <span class="stat-value">{(skill.stats.success_rate * 100).toFixed(0)}%</span>
                        <span class="stat-label">成功率</span>
                      </div>
                      <div class="stat-item">
                        <span class="stat-value">{skill.stats.avg_tokens}</span>
                        <span class="stat-label">tokens</span>
                      </div>
                    </div>
                  </div>
                  <p class="skill-desc">
                    {skill.description?.trim() ? skill.description : '—'}
                  </p>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </div>
      <div class="hud-input-row">
        {#if activeSkillDisplayName}
          <span class="skill-active-chip" title="Active skill preset">[SKILL: {activeSkillDisplayName}]</span>
        {/if}
        <div class="hud-input-field">
          <textarea
            class="hud-input"
            bind:this={inputEl}
            bind:value={inputSignal}
            on:input={adjustInputHeight}
            on:keydown={onInputKeydown}
            placeholder="> Awaiting signal..."
            autocomplete="off"
            spellcheck="false"
            rows="3"
          ></textarea>
        </div>
        {#if isGenerating}
          <button
            type="button"
            class="override-guillotine"
            title="Hard-stop generation (partial reply is not saved)"
            aria-label="Sever connection — critical override"
            on:click|stopPropagation={() => onAbortGeneration()}
          >
            <span class="override-guillotine__idle" aria-hidden="true">[ // ]</span>
            <span class="override-guillotine__hot" aria-hidden="true">[ SEVER_CONNECTION ]</span>
          </button>
        {/if}
      </div>
    </form>
  </div>

  <!-- Settings overlay -->
  {#if showSettingsPanel}
    <div
      class="settings-overlay"
      role="button"
      tabindex="0"
      aria-label="Close settings panel"
      on:click|self={closeSettingsPanel}
      on:keydown={onOverlayKeydown}
    >
      <div class="settings-modal" role="dialog" aria-modal="true" aria-label="Settings">
        <div class="arsenal-title-row">
          <h2>
            {settingsTab === 'provider'
              ? 'Working model'
              : settingsTab === 'persona'
                ? '[PERSONA MATRIX]'
                : 'System health'}
          </h2>
          {#if settingsTab === 'provider' && selectedProviderId}
            <div class="provider-ping" data-state={selectedProviderPingState} aria-live="polite">
              <span class="provider-ping-dot"></span>
              {#if selectedProviderPingState === 'up'}
                <span class="provider-ping-label">[LINK_ESTABLISHED]</span>
              {:else if selectedProviderPingState === 'down'}
                <span class="provider-ping-label">[LINK_DEAD]</span>
              {/if}
            </div>
          {/if}
        </div>
        <div class="settings-tabs">
          <button
            class="settings-tab"
            data-active={settingsTab === 'provider' ? 'true' : 'false'}
            type="button"
            on:click={() => (settingsTab = 'provider')}
          >
            Working model
          </button>
          <button
            class="settings-tab"
            data-active={settingsTab === 'persona' ? 'true' : 'false'}
            type="button"
            on:click={() => (settingsTab = 'persona')}
          >
            [PERSONA MATRIX]
          </button>
          <button
            class="settings-tab"
            data-active={settingsTab === 'health' ? 'true' : 'false'}
            type="button"
            on:click={() => {
              settingsTab = 'health';
              void loadSystemMetrics();
            }}
          >
            System health
          </button>
        </div>

        {#if settingsTab === 'provider'}
          <div class="arsenal-layout">
            <section class="arsenal-list">
              <div class="arsenal-list-head">Provider</div>
              {#each WORKING_PROVIDER_SLOTS as slot (slot.id)}
                {@const row = resolveSlotFromToml(slot)}
                {@const rowActive = config.active_provider_id === slot.id}
                <div class="arsenal-provider-row" data-active={rowActive ? 'true' : 'false'}>
                  <button
                    class="arsenal-provider-row__main"
                    type="button"
                    disabled={settingsSaving}
                    on:click={() => selectProvider(row)}
                  >
                    <span class="arsenal-name">{slot.name}</span>
                  </button>
                </div>
              {/each}
            </section>

            <section class="arsenal-editor">
              <p class="working-model-help">
                Active slot: <strong>{providerDraft.name}</strong> (read-only; from <code>~/.chimera/config.toml</code>
                <code>[llm.providers.{providerDraft.id}]</code>)
              </p>

              <label for="base-url-input">Base URL</label>
              <input
                id="base-url-input"
                class="settings-input"
                type="text"
                readonly
                value={providerDraft.base_url}
                placeholder="'/chat/completions' is automatically appended"
                autocomplete="off"
                spellcheck="false"
              />

              <label for="api-key-input">API Key</label>
              <input
                id="api-key-input"
                class="settings-input"
                type="password"
                readonly
                value={providerDraft.api_key}
                placeholder="Set in config.toml or environment variables"
                autocomplete="off"
                spellcheck="false"
              />

              <label for="model-name-input">Model Name</label>
              <input
                id="model-name-input"
                class="settings-input"
                type="text"
                readonly
                value={providerDraft.model_name}
                placeholder="provider-model"
                autocomplete="off"
                spellcheck="false"
              />

              <label for="temp-input"
                >Temperature (Oligo, from <code>[llm.providers.*]</code>; if unset, the working-model default applies)</label
              >
              <div class="settings-temp-row">
                <input
                  id="temp-input"
                  class="settings-input"
                  type="text"
                  readonly
                  value={providerDraft.temperature ?? ''}
                  placeholder="—"
                  autocomplete="off"
                />
              </div>
            </section>
          </div>

          {#if settingsError}
            <p class="settings-hint settings-error">[ERROR] {settingsError}</p>
          {:else if settingsStatus}
            <p class="settings-hint settings-status">{settingsStatus}</p>
          {/if}

          <div class="settings-actions">
            <button
              class="btn settings-btn ghost"
              type="button"
              on:click={() => void reloadProvidersFromToml()}
              disabled={settingsSaving}
            >
              Reload from TOML
            </button>
            <button
              class="btn settings-btn ghost"
              type="button"
              on:click={() => triggerProviderPing(providerDraft)}
              disabled={settingsSaving}
            >
              Test connection
            </button>
            <button class="btn settings-btn ghost" type="button" on:click={closeSettingsPanel} disabled={settingsSaving}>
              Close
            </button>
          </div>
          <div class="advanced-config-hint">
            ⚙️ Provider keys and models are in <code>~/.chimera/config.toml</code> under
            <code>[llm.providers.*]</code>. Wash/Router still use <code>[llm.wash]</code> /
            <code>[llm.router]</code>. After editing the TOML, click &quot;Reload from TOML&quot; or restart the app.
          </div>
        {:else if settingsTab === 'health'}
          <p class="health-panel-hint">
            Data is written by Oligo to <code>~/.chimera/metrics.json</code>. Refreshes about every 30s; opening this
            tab fetches the latest immediately.
          </p>
          <div class="health-panel">
            <div class="metric-row">
              <span class="metric-label">Total requests</span>
              <span class="metric-value">{systemMetrics.total_requests}</span>
            </div>
            <div class="metric-row">
              <span class="metric-label">Success rate</span>
              <span class="metric-value">{(systemMetrics.success_rate * 100).toFixed(1)}%</span>
            </div>
            <div class="metric-row">
              <span class="metric-label">Avg latency</span>
              <span class="metric-value">{systemMetrics.avg_latency_ms.toFixed(0)} ms</span>
            </div>
            <div class="metric-row">
              <span class="metric-label">Tokens (cumulative)</span>
              <span class="metric-value">{systemMetrics.total_tokens.toLocaleString()}</span>
            </div>
            {#if systemMetrics.top_skills.length > 0}
              <div class="health-top-skills">
                <div class="health-top-skills__title">Top skills (Top 5)</div>
                <ul class="health-top-skills__list">
                  {#each systemMetrics.top_skills as row (row.id)}
                    <li>
                      <span class="skill-id">{row.id}</span>
                      <span class="skill-stat">{row.count} calls · {row.tokens.toLocaleString()} tokens</span>
                    </li>
                  {/each}
                </ul>
              </div>
            {/if}
            {#if systemMetrics.tool_stats.length > 0}
              <div class="tool-stats">
                <h4 class="tool-stats__title">Tool usage</h4>
                {#each systemMetrics.tool_stats as tool (tool.name)}
                  <div class="tool-stat-row">
                    <span class="tool-name">{tool.name}</span>
                    <span class="tool-count">{tool.count} calls</span>
                    <span class="tool-success">{(tool.success_rate * 100).toFixed(0)}%</span>
                    <span class="tool-latency">{tool.avg_latency_ms.toFixed(0)} ms</span>
                  </div>
                {/each}
              </div>
            {/if}
            <div class="wash-stats">
              <h4 class="wash-stats__title">Wash statistics</h4>
              <div class="metric-row wash-stats__row">
                <span class="metric-label">Total washes</span>
                <span class="metric-value">{systemMetrics.wash_stats.total_washes}</span>
              </div>
              <div class="metric-row wash-stats__row">
                <span class="metric-label">Avg compression rate</span>
                <span class="metric-value"
                  >{(systemMetrics.wash_stats.avg_compression_rate * 100).toFixed(1)}%</span
                >
              </div>
              {#if washToolBreakdown(systemMetrics.wash_stats.by_tool).length > 0}
                <ul class="wash-by-tool">
                  {#each washToolBreakdown(systemMetrics.wash_stats.by_tool) as row (row.id)}
                    <li>
                      <span class="wash-by-tool__name">{row.id}</span>
                      <span class="wash-by-tool__meta"
                        >{row.count} calls · {(row.compression * 100).toFixed(0)}% compression</span
                      >
                    </li>
                  {/each}
                </ul>
              {/if}
            </div>
          </div>
          <div class="settings-actions">
            <button class="btn settings-btn ghost" type="button" on:click={() => void loadSystemMetrics()}>
              Refresh now
            </button>
            <button class="btn settings-btn ghost" type="button" on:click={closeSettingsPanel}>
              Close
            </button>
          </div>
        {:else}
          <div class="arsenal-layout">
            <section class="arsenal-list">
              <div class="arsenal-list-head">Personas</div>
              {#if personaSnapshot.personas.length === 0}
                <div class="arsenal-empty">[NO_PERSONA]</div>
              {:else}
                {#each personaSnapshot.personas as persona (persona.id)}
                  <button
                    class="arsenal-item"
                    data-active={personaSnapshot.active_persona_id === persona.id ? 'true' : 'false'}
                    type="button"
                    on:click={() => selectPersona(persona)}
                  >
                    <span class="arsenal-name">{persona.name}</span>
                    <span
                      class="arsenal-delete"
                      role="button"
                      tabindex="0"
                      on:click|stopPropagation={() => removePersona(persona.id)}
                      on:keydown={(event) => event.key === 'Enter' && removePersona(persona.id)}
                    >
                      x
                    </span>
                  </button>
                {/each}
              {/if}
            </section>

            <section class="arsenal-editor">
              <label for="persona-name-input">Persona Name</label>
              <input
                id="persona-name-input"
                class="settings-input"
                type="text"
                bind:value={personaDraft.name}
                placeholder="e.g. Reviewer Zero"
                autocomplete="off"
                spellcheck="false"
              />

              <label for="persona-note-input">Author's note (persisted with this persona)</label>
              <textarea
                id="persona-note-input"
                class="settings-input persona-note-input"
                bind:value={personaDraft.authors_note}
                placeholder="L3 format and tone hints; saved with this persona and injected into the prompt"
                autocomplete="off"
                spellcheck="false"
                rows="3"
              ></textarea>

              <label for="persona-prompt-input">System Prompt</label>
              <textarea
                id="persona-prompt-input"
                class="settings-input persona-prompt-input"
                bind:value={personaDraft.system_prompt}
                placeholder="Paste long-form system prompt here..."
                spellcheck="false"
              ></textarea>
            </section>
          </div>

          {#if personaError}
            <p class="settings-hint settings-error">[ERROR] {personaError}</p>
          {:else if personaStatus}
            <p class="settings-hint settings-status">{personaStatus}</p>
          {/if}

          <div class="settings-actions">
            <button class="btn settings-btn ghost" type="button" on:click={resetPersonaDraft} disabled={settingsSaving}>
              New Persona
            </button>
            <button class="btn settings-btn ghost" type="button" on:click={closeSettingsPanel} disabled={settingsSaving}>
              Close
            </button>
            <button class="btn settings-btn ghost" type="button" on:click={() => selectedPersonaId && activatePersona(selectedPersonaId)} disabled={settingsSaving || !selectedPersonaId}>
              Activate
            </button>
            <button class="btn btn--primary settings-btn primary" type="button" on:click={savePersonaToMatrix} disabled={settingsSaving}>
              {settingsSaving ? 'Saving...' : 'Save Persona'}
            </button>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</main>


<style>
  .hud-shell {
    width: 100vw;
    height: 100vh;
    position: relative;
    display: flex;
    flex-direction: column;
    background: var(--surface-0);
    color: var(--astrocyte-neural-purple);
  }

  .hud-main {
    position: relative;
    flex: 1;
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .middle-arena {
    position: relative;
    z-index: 20;
    flex: 1;
    width: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .phantom-trigger-zone {
    width: 20px;
    position: absolute;
    right: 0;
    top: 28px;
    bottom: 0;
    z-index: 50;
    background: transparent;
    pointer-events: auto;
  }

  .timeline-trigger-zone {
    width: 20px;
    position: absolute;
    left: 0;
    top: 28px;
    bottom: 0;
    z-index: 50;
    background: transparent;
    pointer-events: auto;
  }

  .hud-header {
    position: relative;
    z-index: 10;
    height: 28px;
    display: flex;
    align-items: center;
    padding: 0 var(--space-3);
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--astrocyte-neural-purple);
    border-bottom: 1px solid var(--astrocyte-purple-a-30);
    background: var(--surface-chrome-92);
    -webkit-app-region: drag;
    user-select: none;
    justify-content: space-between;
  }

  .archive-banner {
    padding: var(--space-1) var(--space-3);
    border-bottom: 1px solid var(--astrocyte-purple-a-22);
    background: var(--surface-archive);
    color: rgba(216, 193, 255, 0.92);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .settings-trigger {
    width: 20px;
    height: 20px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--astrocyte-purple-a-35);
    background: var(--surface-chrome-88);
    color: var(--astrocyte-neural-purple);
    font-size: 11px;
    line-height: 1;
    cursor: pointer;
    -webkit-app-region: no-drag;
    transition: border-color 120ms ease, box-shadow 120ms ease, color 120ms ease;
  }

  .settings-trigger:hover {
    border-color: var(--astrocyte-purple-a-85);
    box-shadow: 0 0 10px var(--astrocyte-purple-a-25);
    color: #d9c3ff;
  }

  .hud-output {
    flex: 1;
    width: 100%;
    padding: var(--space-4) var(--space-4) var(--space-4) 40px;
    box-sizing: border-box;
    overflow-y: auto;
    background: var(--surface-absolute-black);
    color: rgba(226, 232, 240, 0.72);
    -webkit-app-region: no-drag;
  }

  .msg-row {
    position: relative;
    margin: 0 0 0.5em;
    /* Magic number: specific layout requirement — 为消息行右侧 HUD 控件/操作区预留，避免与气泡重叠 */
    padding-right: 52px;
  }

  .hud-output .msg-content {
    margin: 0;
    white-space: normal;
  }

  .msg-editor {
    width: 100%;
    min-height: 88px;
    resize: vertical;
    box-sizing: border-box;
    border: none;
    background: var(--surface-code);
    color: var(--astrocyte-read-fg);
    font-family: var(--font-body);
    font-size: 14px;
    line-height: 1.65;
    letter-spacing: 0.02em;
    padding: var(--space-3) var(--space-3);
    outline: none;
  }

  .hud-output .msg-content[data-loading='true'] {
    opacity: 0.7;
  }

  .msg-actions {
    position: absolute;
    top: 0;
    right: 0;
    display: inline-flex;
    gap: var(--space-1);
    opacity: 0;
    pointer-events: none;
    transition: opacity 120ms ease;
  }

  .msg-row:hover .msg-actions {
    opacity: 1;
    pointer-events: auto;
  }

  /* 消息行 E/D/R：基类见 app.css .btn--icon；此处仅作可选微调 */
  .msg-action:disabled {
    opacity: 0.4;
  }

  .feedback-buttons {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-2);
    opacity: 0.6;
    transition: opacity 0.2s;
  }

  .feedback-buttons:hover {
    opacity: 1;
  }

  /* 反馈：HUD 上的小字文案（与 .btn--hud 组合） */
  .feedback-btn {
    font-size: 10px;
    letter-spacing: 0.04em;
    padding-inline: var(--space-2);
  }

  .feedback-btn:hover:not(:disabled) {
    border-color: var(--astrocyte-neural-purple);
    background: var(--astrocyte-purple-a-10);
  }

  .feedback-indicator {
    margin-top: var(--space-2);
    font-size: 11px;
    letter-spacing: 0.04em;
    color: var(--astrocyte-purple-a-75);
  }

  .hud-input-wrap {
    display: flex;
    flex-direction: column;
    width: 100%;
    margin: 0;
    border-top: 1px solid var(--astrocyte-purple-a-30);
    background: var(--surface-0);
    -webkit-app-region: no-drag;
  }

  .hud-input-size-grip {
    flex-shrink: 0;
    height: 8px;
    width: 100%;
    box-sizing: border-box;
    cursor: ns-resize;
    touch-action: none;
    user-select: none;
    background: transparent;
    border: none;
  }

  .hud-input-size-grip:hover {
    background: var(--astrocyte-purple-a-04);
  }

  .hud-input-row {
    display: flex;
    flex-direction: row;
    align-items: flex-end;
    gap: var(--space-3);
    padding: 0 var(--space-2) var(--space-3) var(--space-3);
    box-sizing: border-box;
  }

  .hud-input-row .hud-input-field {
    flex: 1;
    min-width: 0;
    min-height: 60px;
    display: block;
  }

  .hud-input-row .hud-input {
    display: block;
    padding: var(--space-3) var(--space-2) var(--space-3) var(--space-3);
  }

  .persona-quick-switch {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4) 0;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--astrocyte-purple-a-66);
  }

  .persona-quick-switch .quick-switch-legend {
    flex-shrink: 0;
  }

  .persona-quick-switch .persona-selector,
  .persona-quick-switch .skill-selector {
    min-width: 200px;
  }

  .persona-quick-switch .skill-selector-container {
    min-width: min(320px, 42vw);
  }

  .hud-input {
    width: 100%;
    min-height: 60px;
    box-sizing: border-box;
    border: none;
    background: transparent;
    color: var(--astrocyte-neural-purple);
    font-size: 14px;
    line-height: 1.2;
    padding: var(--space-4);
    caret-color: var(--astrocyte-neural-purple);
    resize: none;
    overflow-x: hidden;
    overflow-y: auto;
  }

  .hud-input::placeholder {
    color: var(--astrocyte-purple-a-45);
  }

  .hud-input:focus {
    outline: none;
    box-shadow: none;
  }

  .settings-overlay {
    position: fixed;
    inset: 0;
    z-index: 30;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--surface-scrim);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
  }

  .settings-modal {
    width: min(780px, calc(100vw - 36px));
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-6);
    box-sizing: border-box;
    background: var(--surface-1);
    border: 1px solid var(--border-hud);
    border-radius: var(--radius-md);
    color: var(--astrocyte-neural-purple);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5);
  }

  .settings-modal h2 {
    margin: 0 0 var(--radius-xs);
    font-size: 12px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--astrocyte-bb-fg);
  }

  .settings-tabs {
    display: inline-flex;
    gap: var(--space-2);
    margin: var(--radius-xs) 0 var(--space-1);
  }

  .settings-tab {
    border: 1px solid var(--border-neutral);
    background: var(--surface-modal-inner);
    color: var(--astrocyte-purple-a-72);
    font: inherit;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    padding: var(--space-2) var(--space-2);
    cursor: pointer;
  }

  .settings-tab[data-active='true'] {
    color: #e5d3ff;
    border-color: var(--astrocyte-purple-a-80);
    box-shadow: 0 0 12px var(--astrocyte-purple-a-20);
  }

  .arsenal-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
  }

  .provider-ping {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 10px;
  }

  /* Datacenter LED: 6×6, state-driven (see keyframes in block below) */
  .provider-ping-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
    background: var(--status-led-mute);
    box-shadow: none;
  }

  .provider-ping[data-state='idle'] .provider-ping-dot {
    background: var(--status-led-mute-dim);
    opacity: 0.85;
  }

  /* Checking: gray micro-glow, 0.5s staccato blink */
  .provider-ping[data-state='probing'] .provider-ping-dot {
    background: var(--status-led-probe);
    animation: neural-led-checking 0.5s ease-in-out infinite;
  }

  /* Online: cold white + halo, 3s breathe (halo driven by keyframes) */
  .provider-ping[data-state='up'] .provider-ping-dot {
    background: var(--surface-bright);
    animation: neural-led-breathe 3s ease-in-out infinite;
  }

  /* Offline: dead purple-black, no bloom */
  .provider-ping[data-state='down'] .provider-ping-dot {
    background: var(--status-led-off);
    box-shadow: none;
    animation: none;
    opacity: 1;
  }

  .provider-ping-label {
    font-size: 8px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: rgba(211, 184, 255, 0.62);
  }

  @keyframes neural-led-checking {
    0%,
    100% {
      opacity: 0.28;
      box-shadow: 0 0 2px rgba(140, 140, 150, 0.25);
    }
    50% {
      opacity: 1;
      box-shadow:
        0 0 4px rgba(200, 200, 210, 0.55),
        0 0 8px rgba(180, 185, 200, 0.35);
    }
  }

  @keyframes neural-led-breathe {
    0%,
    100% {
      opacity: 0.88;
      box-shadow:
        0 0 8px rgba(255, 255, 255, 0.6),
        0 0 12px rgba(210, 230, 255, 0.3);
      transform: scale(1);
    }
    50% {
      opacity: 1;
      box-shadow:
        0 0 10px rgba(255, 255, 255, 0.78),
        0 0 18px rgba(220, 240, 255, 0.45);
      transform: scale(1.12);
    }
  }

  .settings-modal label {
    margin-top: 4px;
    font-size: 11px;
    color: rgba(211, 184, 255, 0.88);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .arsenal-layout {
    display: grid;
    grid-template-columns: minmax(180px, 220px) 1fr;
    gap: var(--space-3);
    min-height: 280px;
  }

  .arsenal-list {
    border: 1px solid var(--border-neutral);
    background: var(--surface-embed);
    padding: var(--space-2);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .arsenal-list-head {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: rgba(211, 184, 255, 0.72);
    margin-bottom: 2px;
  }

  .arsenal-empty {
    font-size: 11px;
    color: var(--astrocyte-purple-a-56);
    border: 1px dashed var(--border-neutral);
    padding: var(--space-2);
  }

  .arsenal-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    border: 1px solid var(--border-neutral);
    background: var(--surface-tile-90);
    color: var(--astrocyte-neural-purple);
    font: inherit;
    font-size: 11px;
    padding: var(--space-2) var(--space-2);
    cursor: pointer;
    text-align: left;
    transition: border-color 120ms ease, box-shadow 120ms ease;
  }

  .arsenal-provider-row {
    display: flex;
    align-items: stretch;
    gap: var(--space-1);
    width: 100%;
    border: 1px solid var(--border-neutral);
    background: var(--surface-tile-90);
    transition: border-color 120ms ease, box-shadow 120ms ease;
  }

  .arsenal-provider-row[data-active='true'] {
    border-color: var(--astrocyte-purple-a-82);
    box-shadow: 0 0 14px var(--astrocyte-purple-a-22);
  }

  .arsenal-provider-row__main {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    border: none;
    background: transparent;
    color: var(--astrocyte-neural-purple);
    font: inherit;
    font-size: 11px;
    padding: var(--space-2);
    cursor: pointer;
    text-align: left;
  }

  .arsenal-provider-row__main:hover:not(:disabled) {
    background: var(--astrocyte-purple-a-08);
  }

  .arsenal-provider-row__main:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .arsenal-item[data-active='true'] {
    border-color: var(--astrocyte-purple-a-82);
    box-shadow: 0 0 14px var(--astrocyte-purple-a-22);
  }

  .arsenal-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 150px;
  }

  .arsenal-delete {
    margin-left: 8px;
    width: 12px;
    height: 12px;
    border: 1px solid var(--astrocyte-purple-a-45);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 9px;
    color: rgba(255, 150, 170, 0.9);
  }

  .arsenal-editor {
    border: 1px solid var(--border-neutral);
    background: var(--surface-embed-deep);
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .settings-temp-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }

  .settings-temp-row .settings-input {
    flex: 1;
    min-width: 6rem;
  }

  .settings-input {
    width: 100%;
    box-sizing: border-box;
    border: 1px solid var(--border-neutral);
    background: var(--surface-input-dim);
    color: var(--astrocyte-neural-purple);
    font: inherit;
    font-size: 13px;
    line-height: 1.2;
    padding: var(--space-3);
    outline: none;
    transition: border-color 120ms ease, box-shadow 120ms ease;
  }

  .settings-input:focus {
    border-color: var(--astrocyte-purple-a-72);
    box-shadow: 0 0 0 1px var(--astrocyte-purple-a-16);
  }

  .persona-prompt-input {
    min-height: 100px;
    resize: vertical;
    font-family: "Fira Code", "JetBrains Mono", "Cascadia Mono", "Consolas", monospace;
    line-height: 1.4;
  }

  .persona-note-input {
    min-height: 60px;
    resize: vertical;
    border: none;
    border-bottom: 1px solid var(--astrocyte-purple-a-20);
  }

  .persona-note-input:focus {
    border-bottom-color: var(--astrocyte-purple-a-72);
  }

  .settings-hint {
    margin: var(--space-2) 0 0;
    font-size: 11px;
    line-height: 1.4;
  }

  .settings-error {
    color: var(--feedback-bad);
  }

  .settings-status {
    color: var(--feedback-good);
  }

  .health-panel-hint {
    margin: 0 0 var(--space-3);
    font-size: 12px;
    line-height: 1.45;
    color: var(--astrocyte-neural-purple);
    opacity: 0.85;
  }

  .health-panel {
    border: 1px solid var(--border-neutral);
    background: var(--surface-embed-deep);
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    border-radius: 4px;
  }

  .health-panel .metric-row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: var(--space-3);
    font-size: 13px;
    padding: var(--space-2) 0;
    border-bottom: 1px solid var(--border-neutral);
  }

  .health-panel .metric-row:last-of-type {
    border-bottom: none;
  }

  .health-panel .metric-label {
    color: var(--astrocyte-neural-purple);
    opacity: 0.9;
  }

  .health-panel .metric-value {
    font-variant-numeric: tabular-nums;
    color: var(--astrocyte-neural-purple);
    font-weight: 600;
  }

  .health-top-skills {
    margin-top: var(--space-2);
    padding-top: var(--space-2);
    border-top: 1px dashed var(--border-neutral);
  }

  .health-top-skills__title {
    font-size: 11px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    opacity: 0.75;
    margin-bottom: var(--space-2);
  }

  .health-top-skills__list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    font-size: 12px;
  }

  .health-top-skills__list li {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: var(--space-2);
  }

  .health-top-skills__list .skill-id {
    font-family: ui-monospace, monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 55%;
  }

  .health-top-skills__list .skill-stat {
    font-variant-numeric: tabular-nums;
    opacity: 0.85;
    flex-shrink: 0;
  }

  .tool-stats {
    margin-top: var(--space-2);
    padding-top: var(--space-2);
    border-top: 1px dashed var(--border-neutral);
  }

  .tool-stats__title {
    margin: 0 0 var(--space-2);
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--astrocyte-neural-purple);
  }

  .tool-stat-row {
    display: grid;
    grid-template-columns: minmax(0, 1.2fr) auto auto auto;
    gap: var(--space-2);
    align-items: baseline;
    font-size: 12px;
    padding: var(--space-2) 0;
    border-bottom: 1px solid var(--border-neutral);
  }

  .tool-stat-row:last-child {
    border-bottom: none;
  }

  .tool-stat-row .tool-name {
    font-family: ui-monospace, monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tool-stat-row .tool-count,
  .tool-stat-row .tool-success,
  .tool-stat-row .tool-latency {
    font-variant-numeric: tabular-nums;
    opacity: 0.9;
    white-space: nowrap;
  }

  .wash-stats {
    margin-top: var(--space-2);
    padding-top: var(--space-2);
    border-top: 1px dashed var(--border-neutral);
  }

  .wash-stats__title {
    margin: 0 0 var(--space-2);
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--astrocyte-neural-purple);
  }

  .wash-stats__row {
    padding: var(--space-2) 0;
  }

  .wash-by-tool {
    list-style: none;
    margin: var(--space-2) 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: 11px;
  }

  .wash-by-tool li {
    display: flex;
    justify-content: space-between;
    gap: var(--space-2);
    opacity: 0.9;
  }

  .wash-by-tool__name {
    font-family: ui-monospace, monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 55%;
  }

  .wash-by-tool__meta {
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }

  .settings-actions {
    margin-top: var(--space-2);
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  .btn.settings-btn {
    min-width: 132px;
    font: inherit;
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    padding: var(--space-2) var(--space-3);
  }

  .btn.settings-btn.ghost {
    border-color: var(--border-neutral);
    background: var(--surface-row-muted);
    color: var(--astrocyte-neural-purple);
  }

  .btn.settings-btn.ghost:hover:not(:disabled) {
    border-color: var(--astrocyte-purple-a-92);
    box-shadow: 0 0 12px var(--astrocyte-purple-a-28);
    color: #ddc8ff;
    background: var(--surface-row-muted);
  }

  .working-model-help {
    margin: 0 0 var(--space-3);
    font-size: 11px;
    letter-spacing: 0.04em;
    color: var(--astrocyte-purple-a-75);
  }

  .working-model-help strong {
    color: rgba(226, 232, 240, 0.92);
    font-weight: 600;
  }

  .advanced-config-hint {
    margin-top: var(--space-4);
    padding: var(--space-2);
    background: var(--astrocyte-purple-a-05);
    border-left: 2px solid var(--astrocyte-purple-a-30);
    font-size: 0.85rem;
    color: #888;
  }

  .advanced-config-hint code {
    font-family: var(--font-mono, 'JetBrains Mono', monospace);
    font-size: 0.88em;
    color: var(--astrocyte-purple-a-90);
  }

  /* Oligo tool telemetry: sticky strip above transcript (uses global .system-log-raw from app.css) */
  .oligo-telemetry-stack {
    position: sticky;
    top: 0;
    z-index: 3;
    margin: 0 0 var(--space-3);
    padding: var(--space-2) 0 var(--space-3);
    background: linear-gradient(
      180deg,
      rgba(0, 0, 0, 0.98) 0%,
      rgba(0, 0, 0, 0.82) 78%,
      transparent 100%
    );
    border-bottom: 1px solid rgba(122, 162, 247, 0.12);
  }

  .oligo-telemetry-line {
    margin: 0.25rem 0 0.45rem;
  }

  .oligo-telemetry-line--settled {
    opacity: 0.3;
    animation: none !important;
    transition: opacity 0.75s ease;
  }

  .oligo-telemetry-line--settled::after {
    animation: none !important;
    opacity: 0 !important;
  }

  :global(.oligo-telemetry-line .system-telemetry-prefix) {
    color: rgba(137, 220, 235, 0.55);
    font-weight: 600;
  }

  :global(.oligo-telemetry-line .system-telemetry-cmd) {
    color: #7aa2f7;
    font-weight: 500;
  }

  .miner-task-stack {
    position: sticky;
    top: 0;
    z-index: 2;
    margin: 0 0 var(--space-3);
    padding: var(--space-2) var(--space-3) var(--space-3);
    background: linear-gradient(
      180deg,
      rgba(12, 18, 28, 0.96) 0%,
      rgba(8, 12, 20, 0.88) 90%,
      transparent 100%
    );
    border-bottom: 1px solid rgba(167, 139, 250, 0.2);
    font-size: 11px;
    letter-spacing: 0.03em;
  }

  .miner-task-row + .miner-task-row {
    margin-top: var(--space-3);
    padding-top: var(--space-2);
    border-top: 1px solid rgba(167, 139, 250, 0.08);
  }

  .miner-task-meta {
    display: block;
    color: rgba(226, 232, 240, 0.85);
    margin-bottom: var(--space-2);
  }

  .miner-task-id {
    font-family: var(--font-mono, 'JetBrains Mono', monospace);
    font-size: 10px;
    color: #c4b5fd;
    padding: 0 var(--radius-xs);
  }

  .miner-task-typeHint {
    color: rgba(148, 163, 184, 0.75);
    font-size: 10px;
  }

  .miner-task-bar {
    height: 4px;
    border-radius: 2px;
    background: var(--surface-progress-track);
    overflow: hidden;
    margin-bottom: var(--space-2);
  }

  .miner-task-bar-fill {
    height: 100%;
    border-radius: 2px;
    background: linear-gradient(90deg, var(--astrocyte-accent-violet), var(--astrocyte-accent-cyan));
    transition: width 0.35s ease;
  }

  .miner-task-line {
    color: rgba(186, 230, 253, 0.92);
    font-family: var(--font-mono, 'JetBrains Mono', monospace);
    font-size: 10px;
    line-height: 1.45;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .skill-active-chip {
    flex-shrink: 0;
    align-self: center;
    font-family: var(--font-mono, 'JetBrains Mono', monospace);
    font-size: 9px;
    letter-spacing: 0.05em;
    color: var(--astrocyte-neural-purple);
    text-shadow: 0 0 10px var(--astrocyte-purple-a-20);
    padding: var(--radius-3) var(--space-2);
    border: 1px solid var(--astrocyte-purple-border);
    background: var(--astrocyte-purple-subtle);
    max-width: 12rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
