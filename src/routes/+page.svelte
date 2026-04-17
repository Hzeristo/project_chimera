<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { marked } from 'marked';
  import markedKatex from 'marked-katex-extension';
  import DOMPurify from 'dompurify';
  import hljs from 'highlight.js';
  import 'highlight.js/styles/atom-one-dark.css';
  import 'katex/dist/katex.min.css';

  type Sender = 'system' | 'user' | 'bb' | 'system_log';
  type HistoryEntry = {
    id: string;
    sender: Sender;
    text: string;
    timestamp: string;
    persona?: string;
    isLoading?: boolean;
    /** Set when user aborted mid-stream; assistant text stays in UI but was not persisted. */
    streamAborted?: boolean;
  };
  type ProviderConfig = {
    id: string;
    name: string;
    api_key: string;
    base_url: string;
    model_name: string;
  };
  type AstrocyteConfig = {
    active_provider_id: string | null;
    providers: ProviderConfig[];
    is_oligo_mode: boolean;
    active_skill_id?: string | null;
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
  type SkillDefinition = {
    id: string;
    name: string;
    system_override: string;
    allowed_tools?: string[] | null;
  };
  type TelemetryLine = { id: string; html: string; settled: boolean };
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

  const DEFAULT_PERSONA = 'bb';
  const makeId = () =>
    globalThis.crypto?.randomUUID?.() ??
    `${Date.now()}-${Math.random().toString(16).slice(2)}`;
  const nowIso = () => new Date().toISOString();
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
  let outputEl: HTMLElement | null = null;
  let currentBBMessageId: string | null = null;
  let pendingAssistantMessageId: string | null = null;
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
  let settingsTab: 'provider' | 'persona' = 'provider';
  let config: AstrocyteConfig = {
    active_provider_id: null,
    providers: [],
    is_oligo_mode: false,
    active_skill_id: null,
  };
  let selectedProviderId: string | null = null;
  let providerDraft: ProviderConfig = {
    id: makeId(),
    name: '',
    api_key: '',
    base_url: '',
    model_name: '',
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

  let availableSkills: SkillDefinition[] = [];
  let activeSkillId: string | null = null;
  $: activeSkillId = activeSkillId === '' ? null : activeSkillId;
  /** Oligo tool traces: sticky telemetry strip, not chat bubbles. */
  let telemetryLines: TelemetryLine[] = [];
  $: activeSkillDisplayName = activeSkillId
    ? availableSkills.find((s) => s.id === activeSkillId)?.name ?? null
    : null;

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
    return 'system';
  }

  function stripEphemeralSystemLogs(entries: HistoryEntry[]): HistoryEntry[] {
    return entries.filter((m) => m.sender !== 'system_log');
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

  function resetDraft() {
    providerDraft = {
      id: makeId(),
      name: '',
      api_key: '',
      base_url: '',
      model_name: '',
    };
    selectedProviderId = null;
  }

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
      providers: nextConfig?.providers ?? [],
      is_oligo_mode: nextConfig?.is_oligo_mode ?? false,
      active_skill_id: nextConfig?.active_skill_id ?? null,
    };
    const sid = (nextConfig?.active_skill_id ?? '').trim();
    activeSkillId = sid === '' ? null : sid;
    if (selectedProviderId) {
      const selected = config.providers.find((provider) => provider.id === selectedProviderId);
      if (selected) {
        providerDraft = { ...selected };
        return;
      }
    }
    const active = config.providers.find((provider) => provider.id === config.active_provider_id);
    if (active) {
      selectedProviderId = active.id;
      providerDraft = { ...active };
      return;
    }
    resetDraft();
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

  async function loadConfig() {
    settingsError = '';
    settingsStatus = '';
    try {
      const loaded = await invoke<AstrocyteConfig>('get_config');
      applyLoadedConfig(loaded);
      settingsStatus = '[ARSENAL_SYNCED]';
    } catch (error) {
      const errText = typeof error === 'string' ? error : 'Failed to load arsenal config';
      settingsError = errText;
      notifySystem(`[CONFIG_LOAD_ERROR] ${errText}`);
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

  async function saveProviderToArsenal() {
    settingsSaving = true;
    settingsError = '';
    settingsStatus = '';
    try {
      const payload: ProviderConfig = {
        id: providerDraft.id.trim() || makeId(),
        name: providerDraft.name.trim(),
        api_key: providerDraft.api_key.trim(),
        base_url: providerDraft.base_url.trim(),
        model_name: providerDraft.model_name.trim(),
      };
      await invoke('save_provider', { provider: payload });
      selectedProviderId = payload.id;
      await loadConfig();
      const savedProvider = config.providers.find((provider) => provider.id === payload.id);
      void triggerProviderPing(savedProvider ?? payload);
      settingsStatus = '[ARSENAL_SLOT_SAVED]';
    } catch (error) {
      const errText = typeof error === 'string' ? error : 'Failed to save provider preset';
      settingsError = errText;
      notifySystem(`[CONFIG_SAVE_ERROR] ${errText}`);
    } finally {
      settingsSaving = false;
    }
  }

  async function removeProvider(id: string) {
    settingsSaving = true;
    settingsError = '';
    settingsStatus = '';
    try {
      await invoke('delete_provider', { id });
      await loadConfig();
      settingsStatus = '[ARSENAL_SLOT_PURGED]';
    } catch (error) {
      const errText = typeof error === 'string' ? error : 'Failed to delete provider preset';
      settingsError = errText;
      notifySystem(`[CONFIG_DELETE_ERROR] ${errText}`);
    } finally {
      settingsSaving = false;
    }
  }

  function openSettingsPanel() {
    showSettingsPanel = true;
    settingsError = '';
    personaError = '';
    void loadConfig();
    void loadPersonas();
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
    isGenerating = true;
    pendingStripSystemLogsOnFirstChunk = true;
    telemetryLines = [];
    await scrollToBottom();

    try {
      await invoke('evaluate_payload', {
        payload: normalized,
        sessionId: activeSessionId,
        userMessageId: userMsgId,
        assistantMessageId: pendingAssistantMessageId,
        skillId: activeSkillId,
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
              }
            : msg
        );
      }
      currentBBMessageId = null;
      pendingAssistantMessageId = null;
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
      availableSkills = await invoke<SkillDefinition[]>('get_available_skills');
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

    unlistenBBSysEvent = await listen<string>('bb-sys-event', async (event) => {
      const payload = typeof event.payload === 'string' ? event.payload : '';
      if (!payload.trim()) return;
      if (payload === '[Generation Aborted by User]') return;
      telemetryLines = [
        ...telemetryLines,
        {
          id: makeId(),
          html: formatTelemetryHtml(payload),
          settled: false,
        },
      ];
      await scrollToBottom();
    });

    unlistenBBDone = await listen<{ error?: boolean; aborted?: boolean } | string>(
      'bb-stream-done',
      async (event) => {
        const raw = event.payload;
        const isError = typeof raw === 'object' && raw !== null && raw.error === true;
        const isAborted = typeof raw === 'object' && raw !== null && raw.aborted === true;
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
                  }
                : msg
            );
          }
          history = stripEphemeralSystemLogs(history);
          telemetryLines = telemetryLines.map((line) => ({ ...line, settled: true }));
          isGenerating = false;
          pendingStripSystemLogsOnFirstChunk = false;
          currentBBMessageId = null;
          pendingAssistantMessageId = null;
          await scrollToBottom();
          await refreshSessionHistory();
          return;
        }
        if (currentBBMessageId) {
          history = history.map((msg) => {
            if (msg.id !== currentBBMessageId) return msg;
            const fallback =
              isError && !msg.text.trim() ? '[STREAM_ABORTED]' : isError ? `${msg.text} [STREAM_ABORTED]` : msg.text;
            return {
              ...msg,
              text: fallback,
              isLoading: false,
              timestamp: nowIso(),
            };
          });
        }
        history = stripEphemeralSystemLogs(history);
        telemetryLines = telemetryLines.map((line) => ({ ...line, settled: true }));
        isGenerating = false;
        pendingStripSystemLogsOnFirstChunk = false;
        currentBBMessageId = null;
        pendingAssistantMessageId = null;
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
  });

  const INPUT_MIN_HEIGHT = 52;
  const INPUT_MAX_HEIGHT_RATIO = 0.3;

  function adjustInputHeight() {
    if (!inputEl) return;
    inputEl.style.height = 'auto';
    const maxH = window.innerHeight * INPUT_MAX_HEIGHT_RATIO;
    const h = Math.min(Math.max(inputEl.scrollHeight, INPUT_MIN_HEIGHT), maxH);
    inputEl.style.height = `${h}px`;
    inputEl.style.overflowY = h >= maxH ? 'auto' : 'hidden';
  }

  function resetInputHeight() {
    if (!inputEl) return;
    inputEl.style.height = `${INPUT_MIN_HEIGHT}px`;
    inputEl.style.overflowY = 'hidden';
  }

  function onInputKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      void submitInput();
    }
  }

  async function onPersonaQuickSelect(event: Event) {
    const target = event.currentTarget as HTMLSelectElement | null;
    const nextId = target?.value?.trim() ?? '';
    if (!nextId || nextId === personaSnapshot.active_persona_id) return;
    await activatePersona(nextId);
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

<main class="hud-shell">
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
          {:else if msg.sender === 'system_log'}
            <div class="system-log-raw">{@html beautifySystemLogForDisplay(msg.text)}</div>
          {:else if msg.sender === 'bb' || msg.sender === 'user'}
            <div class="msg-content" data-sender={msg.sender} data-loading={msg.isLoading ? 'true' : undefined}>
              {@html renderMarkdown(msg)}
            </div>
          {:else}
            <div class="msg-content" data-sender={msg.sender} data-loading={msg.isLoading ? 'true' : undefined}>
              {msg.text}
            </div>
          {/if}
          {#if msg.sender === 'user' || msg.sender === 'bb'}
            <div class="msg-actions" aria-label="assistant message actions">
              <button class="msg-action" type="button" title="Edit" on:click={() => onAiAction('edit', msg)}>E</button>
              <button class="msg-action" type="button" title="Delete" on:click={() => onAiAction('delete', msg)}>D</button>
              <button class="msg-action" type="button" title="Retry" disabled={msg.sender !== 'bb' || msg.isLoading || isGenerating} on:click={() => onAiAction('retry', msg)}>R</button>
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
      <div class="persona-quick-switch">
        <label for="persona-quick-select">persona</label>
        <select
          id="persona-quick-select"
          class="persona-quick-select"
          value={personaSnapshot.active_persona_id}
          on:change={onPersonaQuickSelect}
        >
          {#each personaSnapshot.personas as persona (persona.id)}
            <option value={persona.id}>{persona.name}</option>
          {/each}
        </select>
        <label for="skill-quick-select">skill</label>
        <select
          id="skill-quick-select"
          class="persona-quick-select"
          bind:value={activeSkillId}
          on:change={onSkillSelect}
        >
          <option value="">[No Skill]</option>
          {#each availableSkills as skill (skill.id)}
            <option value={skill.id}>{skill.name}</option>
          {/each}
        </select>
      </div>
      <div class="hud-input-row">
        {#if activeSkillDisplayName}
          <span class="skill-active-chip" title="Active skill preset">[SKILL: {activeSkillDisplayName}]</span>
        {/if}
        <textarea
          class="hud-input"
          bind:this={inputEl}
          bind:value={inputSignal}
          on:input={adjustInputHeight}
          on:keydown={onInputKeydown}
          placeholder="> Awaiting signal..."
          autocomplete="off"
          spellcheck="false"
          rows="1"
        ></textarea>
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

  <!-- 设置面板遮罩... 保持原有代码不变 -->
  {#if showSettingsPanel}
    <div
      class="settings-overlay"
      role="button"
      tabindex="0"
      aria-label="Close settings panel"
      on:click|self={closeSettingsPanel}
      on:keydown={onOverlayKeydown}
    >
      <div class="settings-modal" role="dialog" aria-modal="true" aria-label="Provider Arsenal Panel">
        <div class="arsenal-title-row">
          <h2>{settingsTab === 'provider' ? 'Provider Arsenal' : '[PERSONA MATRIX]'}</h2>
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
            Provider Arsenal
          </button>
          <button
            class="settings-tab"
            data-active={settingsTab === 'persona' ? 'true' : 'false'}
            type="button"
            on:click={() => (settingsTab = 'persona')}
          >
            [PERSONA MATRIX]
          </button>
        </div>

        {#if settingsTab === 'provider'}
          <div class="arsenal-layout">
            <section class="arsenal-list">
              <div class="arsenal-list-head">Slots</div>
              {#if config.providers.length === 0}
                <div class="arsenal-empty">[NO_PRESET]</div>
              {:else}
                {#each config.providers as provider (provider.id)}
                  <button
                    class="arsenal-item"
                    data-active={config.active_provider_id === provider.id ? 'true' : 'false'}
                    type="button"
                    on:click={() => selectProvider(provider)}
                  >
                    <span class="arsenal-name">{provider.name}</span>
                    <span
                      class="arsenal-delete"
                      role="button"
                      tabindex="0"
                      on:click|stopPropagation={() => removeProvider(provider.id)}
                      on:keydown={(event) => event.key === 'Enter' && removeProvider(provider.id)}
                    >
                      x
                    </span>
                  </button>
                {/each}
              {/if}
            </section>

            <section class="arsenal-editor">
              <label for="preset-name-input">Preset Name</label>
              <input
                id="preset-name-input"
                class="settings-input"
                type="text"
                bind:value={providerDraft.name}
                placeholder="e.g. DeepSeek Core / Local Llama"
                autocomplete="off"
                spellcheck="false"
              />

              <label for="base-url-input">Base URL</label>
              <input
                id="base-url-input"
                class="settings-input"
                type="text"
                bind:value={providerDraft.base_url}
                placeholder="'/chat/completions' is automatically appended"
                autocomplete="off"
                spellcheck="false"
              />

              <label for="api-key-input">API Key</label>
              <input
                id="api-key-input"
                class="settings-input"
                type="password"
                bind:value={providerDraft.api_key}
                placeholder="sk-..."
                autocomplete="off"
                spellcheck="false"
              />

              <label for="model-name-input">Model Name</label>
              <input
                id="model-name-input"
                class="settings-input"
                type="text"
                bind:value={providerDraft.model_name}
                placeholder="provider-model"
                autocomplete="off"
                spellcheck="false"
              />
            </section>
          </div>

          {#if settingsError}
            <p class="settings-hint settings-error">[ERROR] {settingsError}</p>
          {:else if settingsStatus}
            <p class="settings-hint settings-status">{settingsStatus}</p>
          {/if}

          <div class="settings-actions">
            <button class="settings-btn ghost" type="button" on:click={resetDraft} disabled={settingsSaving}>
              New Slot
            </button>
            <button class="settings-btn ghost" type="button" on:click={closeSettingsPanel} disabled={settingsSaving}>
              Close
            </button>
            <button class="settings-btn primary" type="button" on:click={saveProviderToArsenal} disabled={settingsSaving}>
              {settingsSaving ? 'Saving...' : 'Save to Arsenal'}
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

              <label for="persona-note-input">Author's Note (Optional)</label>
              <textarea
                id="persona-note-input"
                class="settings-input persona-note-input"
                bind:value={personaDraft.authors_note}
                placeholder="Optional tactical steering note"
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
            <button class="settings-btn ghost" type="button" on:click={resetPersonaDraft} disabled={settingsSaving}>
              New Persona
            </button>
            <button class="settings-btn ghost" type="button" on:click={closeSettingsPanel} disabled={settingsSaving}>
              Close
            </button>
            <button class="settings-btn ghost" type="button" on:click={() => selectedPersonaId && activatePersona(selectedPersonaId)} disabled={settingsSaving || !selectedPersonaId}>
              Activate
            </button>
            <button class="settings-btn primary" type="button" on:click={savePersonaToMatrix} disabled={settingsSaving}>
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
    background: #0a0a0f;
    color: #bb9af7;
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
    padding: 0 12px;
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #bb9af7;
    border-bottom: 1px solid rgba(187, 154, 247, 0.3);
    background: rgba(10, 10, 15, 0.92);
    -webkit-app-region: drag;
    user-select: none;
    justify-content: space-between;
  }

  .archive-banner {
    padding: 5px 12px;
    border-bottom: 1px solid rgba(187, 154, 247, 0.22);
    background: rgba(6, 6, 10, 0.95);
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
    border: 1px solid rgba(187, 154, 247, 0.35);
    background: rgba(10, 10, 15, 0.88);
    color: #bb9af7;
    font-size: 11px;
    line-height: 1;
    cursor: pointer;
    -webkit-app-region: no-drag;
    transition: border-color 120ms ease, box-shadow 120ms ease, color 120ms ease;
  }

  .settings-trigger:hover {
    border-color: rgba(187, 154, 247, 0.85);
    box-shadow: 0 0 10px rgba(187, 154, 247, 0.25);
    color: #d9c3ff;
  }

  .hud-output {
    flex: 1;
    width: 100%;
    padding: 14px 16px 14px 40px;
    box-sizing: border-box;
    overflow-y: auto;
    background: #000000;
    color: rgba(226, 232, 240, 0.72);
    -webkit-app-region: no-drag;
  }

  .msg-row {
    position: relative;
    margin: 0 0 0.5em;
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
    background: #050505;
    color: #e2e8f0;
    font-family: var(--font-body);
    font-size: 14px;
    line-height: 1.65;
    letter-spacing: 0.02em;
    padding: 10px 12px;
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
    gap: 4px;
    opacity: 0;
    pointer-events: none;
    transition: opacity 120ms ease;
  }

  .msg-row:hover .msg-actions {
    opacity: 1;
    pointer-events: auto;
  }

  .msg-action {
    width: 14px;
    height: 14px;
    border: 1px solid rgba(187, 154, 247, 0.45);
    background: rgba(10, 10, 15, 0.88);
    color: #bb9af7;
    font-size: 9px;
    line-height: 1;
    padding: 0;
    cursor: pointer;
  }

  .msg-action:hover {
    border-color: rgba(187, 154, 247, 0.8);
  }

  .msg-action:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .hud-input-wrap {
    width: 100%;
    margin: 0;
    border-top: 1px solid rgba(187, 154, 247, 0.3);
    background: #0a0a0f;
    -webkit-app-region: no-drag;
  }

  .hud-input-row {
    display: flex;
    flex-direction: row;
    align-items: flex-end;
    gap: 10px;
    padding: 0 8px 10px 12px;
    box-sizing: border-box;
  }

  .hud-input-row .hud-input {
    flex: 1;
    min-width: 0;
    padding: 12px 8px 12px 12px;
  }

  .persona-quick-switch {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 7px 16px 0;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: rgba(187, 154, 247, 0.66);
  }

  .persona-quick-select {
    min-width: 180px;
    border: 1px solid rgba(187, 154, 247, 0.25);
    background: rgba(10, 10, 15, 0.88);
    color: rgba(187, 154, 247, 0.92);
    font: inherit;
    font-size: 10px;
    padding: 4px 6px;
    outline: none;
  }

  .persona-quick-select:focus {
    border-color: rgba(187, 154, 247, 0.75);
  }

  .hud-input {
    width: 100%;
    min-height: 52px;
    box-sizing: border-box;
    border: none;
    background: transparent;
    color: #bb9af7;
    font-size: 14px;
    line-height: 1.2;
    padding: 16px;
    caret-color: #bb9af7;
    resize: none;
    overflow-x: hidden;
    overflow-y: hidden;
  }

  .hud-input::placeholder {
    color: rgba(187, 154, 247, 0.45);
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
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
  }

  .settings-modal {
    width: min(780px, calc(100vw - 36px));
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px;
    box-sizing: border-box;
    background: rgba(8, 8, 12, 0.94);
    border: 1px solid #333;
    color: #bb9af7;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5);
  }

  .settings-modal h2 {
    margin: 0 0 2px;
    font-size: 12px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: #d3b8ff;
  }

  .settings-tabs {
    display: inline-flex;
    gap: 8px;
    margin: 2px 0 4px;
  }

  .settings-tab {
    border: 1px solid #333;
    background: rgba(8, 8, 12, 0.92);
    color: rgba(187, 154, 247, 0.72);
    font: inherit;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    padding: 6px 8px;
    cursor: pointer;
  }

  .settings-tab[data-active='true'] {
    color: #e5d3ff;
    border-color: rgba(187, 154, 247, 0.8);
    box-shadow: 0 0 12px rgba(187, 154, 247, 0.2);
  }

  .arsenal-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .provider-ping {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-height: 10px;
  }

  /* Datacenter LED: 6×6, state-driven (see keyframes in block below) */
  .provider-ping-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
    background: rgba(100, 100, 108, 0.55);
    box-shadow: none;
  }

  .provider-ping[data-state='idle'] .provider-ping-dot {
    background: rgba(88, 88, 96, 0.5);
    opacity: 0.85;
  }

  /* Checking: gray micro-glow, 0.5s staccato blink */
  .provider-ping[data-state='probing'] .provider-ping-dot {
    background: rgba(160, 162, 170, 0.95);
    animation: neural-led-checking 0.5s ease-in-out infinite;
  }

  /* Online: cold white + halo, 3s breathe (halo driven by keyframes) */
  .provider-ping[data-state='up'] .provider-ping-dot {
    background: #ffffff;
    animation: neural-led-breathe 3s ease-in-out infinite;
  }

  /* Offline: dead purple-black, no bloom */
  .provider-ping[data-state='down'] .provider-ping-dot {
    background: #4a2060;
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
    gap: 12px;
    min-height: 280px;
  }

  .arsenal-list {
    border: 1px solid #333;
    background: rgba(0, 0, 0, 0.38);
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
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
    color: rgba(187, 154, 247, 0.56);
    border: 1px dashed #333;
    padding: 8px;
  }

  .arsenal-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    border: 1px solid #333;
    background: rgba(8, 8, 12, 0.9);
    color: #bb9af7;
    font: inherit;
    font-size: 11px;
    padding: 7px 8px;
    cursor: pointer;
    text-align: left;
    transition: border-color 120ms ease, box-shadow 120ms ease;
  }

  .arsenal-item[data-active='true'] {
    border-color: rgba(187, 154, 247, 0.82);
    box-shadow: 0 0 14px rgba(187, 154, 247, 0.22);
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
    border: 1px solid rgba(187, 154, 247, 0.45);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 9px;
    color: rgba(255, 150, 170, 0.9);
  }

  .arsenal-editor {
    border: 1px solid #333;
    background: rgba(0, 0, 0, 0.34);
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .settings-input {
    width: 100%;
    box-sizing: border-box;
    border: 1px solid #333;
    background: rgba(0, 0, 0, 0.5);
    color: #bb9af7;
    font: inherit;
    font-size: 13px;
    line-height: 1.2;
    padding: 10px;
    outline: none;
    transition: border-color 120ms ease, box-shadow 120ms ease;
  }

  .settings-input:focus {
    border-color: rgba(187, 154, 247, 0.72);
    box-shadow: 0 0 0 1px rgba(187, 154, 247, 0.16);
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
    border-bottom: 1px solid rgba(187, 154, 247, 0.2);
  }

  .persona-note-input:focus {
    border-bottom-color: rgba(187, 154, 247, 0.72);
  }

  .settings-hint {
    margin: 6px 0 0;
    font-size: 11px;
    line-height: 1.4;
  }

  .settings-error {
    color: #ff8fa3;
  }

  .settings-status {
    color: #8ef1b6;
  }

  .settings-actions {
    margin-top: 8px;
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .settings-btn {
    min-width: 132px;
    border: 1px solid #333;
    background: rgba(12, 12, 20, 0.92);
    color: #bb9af7;
    font: inherit;
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    padding: 8px 10px;
    cursor: pointer;
    transition: border-color 120ms ease, box-shadow 120ms ease, color 120ms ease;
  }

  .settings-btn:hover:not(:disabled) {
    border-color: rgba(187, 154, 247, 0.92);
    box-shadow: 0 0 12px rgba(187, 154, 247, 0.28);
    color: #ddc8ff;
  }

  .settings-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .settings-btn.primary {
    border-color: rgba(187, 154, 247, 0.52);
  }

  /* Oligo tool telemetry: sticky strip above transcript (uses global .system-log-raw from app.css) */
  .oligo-telemetry-stack {
    position: sticky;
    top: 0;
    z-index: 3;
    margin: 0 0 12px;
    padding: 6px 0 10px;
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

  .skill-active-chip {
    flex-shrink: 0;
    align-self: center;
    font-family: var(--font-mono, 'JetBrains Mono', monospace);
    font-size: 9px;
    letter-spacing: 0.05em;
    color: rgba(192, 132, 252, 0.98);
    text-shadow:
      0 0 8px rgba(139, 92, 246, 0.45),
      0 0 14px rgba(91, 33, 182, 0.25);
    padding: 3px 7px;
    border: 1px solid rgba(139, 92, 246, 0.4);
    background: rgba(45, 20, 70, 0.42);
    max-width: 12rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
