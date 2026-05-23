# Batch Plan: Phase III.C — Structured Final Contract

**Audit reference:** `docs/audits/FC.0.md` (date: 2026-05-18)
**Phase doc:** `docs/phases/Phase-III.C.md`
**Driving frictions:** E3 (tool results unlinkable, `friction_260426.md` entry 3), E4 (cannot delete messages, `friction_260426.md` entry 2 UI residue)

This document is a single unit. User approves the whole sequence or rejects
the whole sequence. After approval, hand off to `chimera-code-taste`
batch_execution mode.

---

## Sprint Sequence

```
FC.0 (audit, done)
  └→ FC.1 ──→ FC.2a ──→ FC.2b ──→ FC.3a ──→ FC.3b ──┐
                                                      ├→ FC.6 (seal)
  └→ FC.4 (test-only, parallelizable) ───────────────┤
  └→ FC.5 (verify-only, parallelizable) ─────────────┘
```

Audit-driven changes vs. L1 manifest:
- **FC.2 split** into FC.2a (Python emit hook) + FC.2b (Rust forward + Svelte state) — audit Q3/Q4 show artifact emission requires *two* hooks (Python before `return`, Rust before success-path `"DONE"`). One sprint cannot honor the ≤3-file / ≤50-line guideline.
- **FC.3 split** into FC.3a (Rust `open_vault_note` + path-containment) + FC.3b (Svelte chip component + click wiring) — original FC.3 spanned both languages and ~70 lines, exceeding the ≤3-file / ≤50-line guideline. The split also isolates the security-adjacent path-validation work in its own sprint with its own seal.
- **FC.4 reduced to test-only** — audit Q6 + cross-finding 2: persona is already FINAL-only on Oligo path; only a unit test is missing.
- **FC.5 reduced to verify-only** — audit Q7 + cross-finding 1: delete pipeline is materially complete; sprint becomes a checklist walkthrough, not a build.

Dependencies stated in each sprint's "Predecessor assumptions" block.

---

## Sprint FC.1: Vault tools opt-in to `ToolOutput(text, artifacts)`

**Friction reference:** E3 (tool results unlinkable) — driving.

**Predecessor assumptions:**
- Audit Q1/Q1.sub findings hold: zero tools currently return non-`str`. `ToolOutput` is a *new* opt-in shape, not alignment with existing convention. Re-plan trigger if any tool already returns a structured shape.
- `_VaultToolPort.query_graph` continues to return `list[dict[str, Any]]` (audit Q1.sub, `vault_tools.py:15-20`). If the adapter shape changes, FC.1 must re-derive the artifact-extraction path.

**Risk level:** 🟡 MED
- New dataclass + 3 tool signature widenings + `_execute_tool` branch on return type. Touches a coercion choke point (`agent.py:646-672`) so blast radius across all 8 tools needs careful guarding.

### 目标
Define `ToolOutput(text, artifacts)` and let the three vault tools return it while keeping the `str`-returning contract intact for all other tools.

### 设计要点 (audit-derived)
- `ToolOutput` is a dataclass with `text: str` and `artifacts: list[Artifact] | None = None`. Keep `Artifact` minimal: at least `kind: str`, `path: str`, plus a free-form `metadata: dict[str, Any] | None`. — audit Q1.sub (`vault_tools.py:15-20`) + cross-ref bullet 1 (`agent.py:706-716`)
- `ExecutedToolResult` gains an optional `artifacts` field constructed from `ToolOutput.artifacts` when present, else `None`. The LLM-facing render path keeps using `washed_result or raw_result or error_message` as today — *artifacts never enter `result_body`*. — audit Q2 (`agent.py:1040-1085`, `agent.py:1053-1057`); cross-ref bullet 1
- `_execute_tool` widens its result handling: if `result` is a `ToolOutput`, set `raw = result.text` and capture `result.artifacts`; else keep `raw = str(result)`. The `str(result)` choke point at `agent.py:660` is the only edit site. — audit Q2 (`agent.py:646-672`)
- Wash bypass behavior unchanged: `ToolOutput.text` participates in wash exactly as today's `str` returns do (subject to `bypass_wash_tools` / `force_wash_tools`). Document this explicitly in the `ToolOutput` docstring per cross-ref bullet 4.

### 任务范围
1. Add `ToolOutput` + `Artifact` dataclasses (`crucible_core/src/oligo/tools/registry.py` near `ToolFn`/`ToolSpec`, ~25 lines) — audit Q1 (`registry.py:22`).
2. Widen `ExecutedToolResult` with `artifacts: list[Artifact] | None = None` (`crucible_core/src/oligo/core/agent.py` near the existing dataclass, ~3 lines) — audit Q2 + cross-ref bullet 1.
3. Branch `_execute_tool` on `isinstance(result, ToolOutput)` to populate `raw_result` and `artifacts` (`crucible_core/src/oligo/core/agent.py:646-672`, ~8 lines) — audit Q2.
4. Convert three vault tools (`search_vault`, `search_vault_attribute`, `obsidian_graph_query`) to return `ToolOutput`; for `obsidian_graph_query`, derive artifacts from the adapter's `list[dict]` *before* string-flattening (`crucible_core/src/oligo/tools/vault_tools.py:41,68,91,114,140-168`, ~20 lines total) — audit Q1 + cross-finding 5.
5. Tests: `tests/oligo/test_tool_execution.py` add a parametrized case asserting (a) `ToolOutput`-returning tool yields `ExecutedToolResult.artifacts is not None`, (b) `str`-returning tool yields `artifacts is None`, (c) LLM-facing render contains `text` only, never artifact paths. ~30 lines.

### 验收
- `Grep "ToolOutput" crucible_core/src/oligo/tools/vault_tools.py` returns ≥3 hits (one per converted tool).
- `Grep "result_body" crucible_core/src/oligo/core/agent.py` shows `payload` source unchanged from `washed_result or raw_result or error_message` — verifiable by reading `agent.py:1053-1057` post-change.
- `pytest tests/oligo/test_tool_execution.py -k tooloutput` passes (3 new cases).
- `obsidian_graph_query`'s string output is byte-identical to current behavior on a fixture covering the `vault_tools.py:154-167` flatten path (regression guard).

### 红线
- ❌ Artifacts may not enter the LLM-facing render path (`agent.py:1053-1057`) — phase-wide.
- ❌ Do NOT change other 5 tools' return shape in this sprint (web_search, miner_tools×3) — sprint-specific.
- ❌ Do NOT modify `_render_tool_results_for_llm` or `_format_one_tool_result_xml` — sprint-specific.
- ❌ Do NOT change wash bypass policy or `bypass_wash_tools` semantics.
- ❌ 不进行机会主义重构 — DEBT-002/DEBT-004 stay open even if you brush against the long-name functions.

### 输出位置
- 代码: `crucible_core/src/oligo/tools/registry.py`, `crucible_core/src/oligo/tools/vault_tools.py`, `crucible_core/src/oligo/core/agent.py`
- 测试: `tests/oligo/test_tool_execution.py`
- 文档: 推迟至 FC.6 统一更新

---

## Sprint FC.2a: Backend artifact aggregation + SSE emit

**Friction reference:** E3 (tool results unlinkable) — derivative.

**Predecessor assumptions:**
- FC.1 sealed: `ExecutedToolResult.artifacts` exists and is populated when a tool returns `ToolOutput`. Re-plan trigger: artifact field shape differs from FC.1 design.
- Audit Q3/Q8 hold: `_run_theater_stream`'s last yield site is `agent.py:1355-1356`, and `executed_results` is per-turn-local at `agent.py:1199`. Re-plan trigger: theater loop refactored.

**Risk level:** 🟡 MED
- New instance attribute, new SSE event, dedup logic. Single Python file; tight blast radius but the emission must precede the success-path `return` exactly.

### 目标
Accumulate artifacts across all turns of `_run_theater_stream` and emit them as a single `bb-message-artifacts` SSE frame at the last yield before the success-path return.

### 设计要点 (audit-derived)
- New instance attribute `self._session_artifacts: list[Artifact]` initialized in `OligoAgent.__init__` near other per-run state. — audit Q8 (`agent.py:382-431`)
- Append from each turn's `executed_results` *after* `_wash_tool_results` at `agent.py:1213-1215`, since wash may have post-processed metadata. Dedup by `(kind, path)` tuple. — audit Q8 (`agent.py:1213-1215`)
- Emit once at the **last yield before `return` at `agent.py:1356`** — payload shape `{"artifacts": [...]}` mirroring `bb-tool-start`/`bb-tool-done` precedent (cross-ref bullet 2). Event name: `bb-message-artifacts`. Empty list → emit nothing (don't pollute frame stream). — audit Q3 (`agent.py:1355-1356`) + cross-finding 4
- `server.py` requires no change: the safe-wrapper's error-only `bb-stream-done` semantics are preserved (audit Q3, `server.py:140-153`).

### 任务范围
1. Add `_session_artifacts` init + reset hook in `OligoAgent.__init__` and any per-run reset path (`crucible_core/src/oligo/core/agent.py:382-431`, ~5 lines).
2. Append + dedup after `_wash_tool_results` (`agent.py:1213-1215`, ~8 lines).
3. Emit `bb-message-artifacts` SSE frame just before `return` in `_run_theater_stream` (`agent.py:1355-1356`, ~6 lines, gated on non-empty list).
4. Tests: `tests/oligo/test_theater_stream.py` (new or extend) — assert (a) two-turn run with vault tool emits exactly one `bb-message-artifacts` frame containing both turns' artifacts, (b) zero-artifact run emits no such frame, (c) duplicate `(kind, path)` across turns deduped. ~40 lines.

### 验收
- `pytest tests/oligo/test_theater_stream.py -k artifacts` passes.
- Manual: `Grep "bb-message-artifacts" crucible_core/src/oligo/core/agent.py` returns exactly the emit site (1 hit, plus test references).
- Replay an existing two-turn fixture and confirm by snapshot that the new event sits *before* the success-path return (Python-side; Rust-side covered in FC.2b).

### 红线
- ❌ Artifacts may not enter `self.messages` / LLM payload (Hard Sealing Condition #2) — phase-wide.
- ❌ Do NOT emit artifacts as part of `bb-tool-done` payload — sprint-specific (keeps tool-strip semantics intact, audit cross-ref bullet 2).
- ❌ Do NOT change `bb-stream-done` ownership split between Python (error) and Rust (success) — audit cross-finding 4.
- ❌ Do NOT add a new `__SYS_TOOL_CALL__` subkind for artifacts — keep them on a dedicated event.
- ❌ 不进行机会主义重构

### 输出位置
- 代码: `crucible_core/src/oligo/core/agent.py`
- 测试: `tests/oligo/test_theater_stream.py`
- 文档: 推迟至 FC.6

---

## Sprint FC.2b: Rust forward + Svelte state attachment

**Friction reference:** E3 — derivative.

**Predecessor assumptions:**
- FC.2a sealed: backend emits `bb-message-artifacts` before success-path return.
- Audit Q4/Q5 hold: `state::Message` lacks an artifacts field; `HistoryEntry` lacks an `artifacts` field; `stream_oligo_agent`'s event match has unknown-event fall-through. Re-plan trigger: any of these schemas changed.

**Risk level:** 🟡 MED
- Touches both Rust (`llm_client.rs`, `state.rs`) and Svelte (`+page.svelte`). Two files exceed 30 lines is plausible; held under MED by sticking to additive fields.

### 目标
Forward `bb-message-artifacts` from the backend SSE through Tauri to the Svelte `HistoryEntry` so the data is available for FC.3's chip rendering — without rendering anything yet.

### 设计要点 (audit-derived)
- Add explicit branch in `stream_oligo_agent`'s event match for `"bb-message-artifacts"`: parse JSON, `app.emit("bb-message-artifacts", payload)`. Place it *before* the success-path `Ok(Some(...))` return so Rust's `lib.rs:979` `"DONE"` emit follows it. — audit Q4 (`llm_client.rs:294-324`) + cross-finding 4
- Widen `state::Message` with an additive `artifacts: Option<Vec<Artifact>>` field (`#[serde(default)]`), and define `Artifact { kind: String, path: String, metadata: Option<serde_json::Value> }` mirroring the Python dataclass. JSONL persistence at `memory.rs:420` continues to write `user`/`bb` only (audit Q7); artifacts ride on the `bb` entry as a sibling field. — audit Q4 (`state.rs:14-18`); audit Q7 (`memory.rs:66-76`)
- Widen `HistoryEntry` with `artifacts?: Artifact[]` (`+page.svelte:14-36`). On `bb-message-artifacts` event, locate the in-flight assistant entry (last `sender === 'bb'` with `isLoading`) and attach. No rendering this sprint. — audit Q5 (`+page.svelte:14-36`)
- Persistence: append the `artifacts` field on the BB `ChatEntry` so reload preserves them. The persistence-boundary comment at `lib.rs:691` ("user + bb only") is unchanged in spirit. — audit Q7 (`memory.rs:66-76, 420`)

### 任务范围
1. Rust: new event branch + struct (`astrocyte/src-tauri/src/llm_client.rs:294-324`, ~12 lines).
2. Rust: widen `state::Message` + add `Artifact` (`astrocyte/src-tauri/src/state.rs:14-18`, ~10 lines).
3. Rust: widen `ChatEntry` to optionally serialize `artifacts` (`astrocyte/src-tauri/src/memory.rs:66-76`, ~5 lines). Confirm `append_session_entries` (`memory.rs:420`) round-trips when field is present.
4. Svelte: widen `HistoryEntry` type + `bb-message-artifacts` listener attaching to in-flight BB entry (`astrocyte/src/routes/+page.svelte:14-36` and event-listener block near other `bb-*` listeners — locate at execute time, audit Q4/Q5 give the surrounding range, ~25 lines).
5. Tests: Rust unit test for `Message`/`ChatEntry` (de)serialization with and without `artifacts` (round-trip + missing-field default). ~20 lines. Svelte: integration smoke deferred to FC.3 (no rendering yet to assert).

### 验收
- `cargo test -p astrocyte_tauri --lib` passes including new round-trip test.
- `Grep "bb-message-artifacts" astrocyte/src-tauri/src/llm_client.rs` returns exactly 1 match (the new branch).
- `Grep "artifacts" astrocyte/src/routes/+page.svelte` returns ≥2 matches (type + listener); no rendering yet.
- Manual: a fixture run that produced artifacts in FC.2a results in the corresponding `HistoryEntry` carrying a non-empty `artifacts` array (verified via Svelte devtools or a temporary `console.log`, removed before commit).
- Reload round-trip: kill+restart astrocyte, reopen session, BB message still has `artifacts`.

### 红线
- ❌ Do NOT render chips in this sprint (FC.3 territory; phase-wide red line about no rich preview still applies).
- ❌ Do NOT add `artifacts` to `user` `ChatEntry` rows — only `bb` (matches `lib.rs:691` boundary).
- ❌ Do NOT change `stage_card` / `system_log` / `error` sender semantics in `+page.svelte:14-36`.
- ❌ Do NOT add new design tokens (FC.3 will).
- ❌ 不进行机会主义重构

### 输出位置
- 代码: `astrocyte/src-tauri/src/llm_client.rs`, `astrocyte/src-tauri/src/state.rs`, `astrocyte/src-tauri/src/memory.rs`, `astrocyte/src/routes/+page.svelte`
- 测试: `astrocyte/src-tauri/src/state.rs` (inline `#[cfg(test)]`) or new test file under existing test layout (locate at execute time)
- 文档: 推迟至 FC.6

**Accepted overrun**: 4 files / 72 lines. Reason: artifact carriage requires coherent edits across SSE forward, struct widening, persistence, and frontend type. Further splitting fragments single semantic change.

---

## Sprint FC.3a: Rust `open_vault_note` command + path-containment

**Friction reference:** E3 (tool results unlinkable) — derivative; serves the completion signal ("点紫色 chip,Obsidian 自动打开").

**Predecessor assumptions:**
- FC.2b sealed: `HistoryEntry.artifacts` populated end-to-end; Rust `Artifact` struct exists in `state.rs`. Re-plan trigger: artifact path field renamed.
- Audit cross-ref bullet 5 holds: no central vault-root containment helper exists; FC.3a must add one. Re-plan trigger: such a helper has appeared since 2026-05-18.

**Risk level:** 🟡 MED
- Security-adjacent path validation. Single-language scope (Rust) but the helper is the security boundary; the test matrix must be exhaustive.

### 目标
Add a Tauri command `open_vault_note(path)` that validates the path is canonically inside the vault root before launching the file in Obsidian.

### 设计要点 (audit-derived)
- New Tauri command `open_vault_note(path: String) -> Result<(), String>` placed beside other vault commands in `lib.rs`. Validates `path` is canonical-inside `vault_root` *before* spawning Obsidian via the `obsidian://open?path=...` URI scheme (or platform-specific equivalent already in use — locate at execute time). — audit cross-ref bullet 5
- Path-containment helper: a Rust util that canonicalizes both the candidate path and `vault_root` then asserts containment. Reject symlinks that escape the root and traversal segments (`..`). Reuse `vault_root` source from existing Rust state if available; otherwise read from config. — audit cross-ref bullet 5
- Command registration in the Tauri `invoke_handler!` happens in this sprint; the Svelte side that calls it lands in FC.3b. Until FC.3b ships, the command is reachable only via test/devtools — that is acceptable.

### 任务范围
1. Path-containment helper + Rust unit tests (`astrocyte/src-tauri/src/lib.rs` or a small util module per existing layout — locate at execute time, ~30 lines including `#[cfg(test)]`).
2. `open_vault_note` Tauri command implementation calling the helper then launching the URI (`astrocyte/src-tauri/src/lib.rs`, ~15 lines).
3. Register the command in `invoke_handler!` (`astrocyte/src-tauri/src/lib.rs`, ~1 line).

### 验收
- `cargo test -p astrocyte_tauri --lib path_containment` passes 4 cases: inside-root ✓, outside-root ✗, symlink-escape ✗, traversal `../` ✗.
- `Grep "open_vault_note" astrocyte/src-tauri/src/lib.rs` returns command def + handler registration (≥2 hits).
- Manual: invoke command via devtools with a known good path → Obsidian opens; with a path outside vault → command returns `Err`.

### 红线
- ❌ Do NOT bypass path-containment check in `open_vault_note` — security boundary.
- ❌ Do NOT touch Svelte in this sprint — FC.3b territory.
- ❌ Do NOT add new dependencies for URI launching beyond what is already used elsewhere in `lib.rs` (`chimera-dependency-veto`).
- ❌ Do NOT change `vault_root` source-of-truth or read it from a new place; reuse the existing channel.
- ❌ 不进行机会主义重构

### 输出位置
- 代码: `astrocyte/src-tauri/src/lib.rs` (+ small util module if pattern fits)
- 测试: Rust inline `#[cfg(test)]` for path containment
- 文档: 推迟至 FC.6

---

## Sprint FC.3b: Svelte chip rendering + click wiring

**Friction reference:** E3 + completion signal ("点紫色 chip,Obsidian 自动打开") — closes the user-visible loop.

**Predecessor assumptions:**
- FC.3a sealed: `open_vault_note` command registered and path-validated. Re-plan trigger: command signature differs from `(path: String) -> Result<(), String>`.
- FC.2b sealed: `HistoryEntry.artifacts` populated end-to-end. Re-plan trigger: artifact field renamed.
- Audit Q5 holds: design tokens (`--astrocyte-neural-purple`, `--border-hud-emphasis`, `--space-*`, `--radius-*`) usable as cited; full palette read deferred from FC.0 to *this* sprint's planning. Re-plan trigger: tokens removed/renamed.

**Risk level:** 🟡 MED
- Two Svelte files (chip component + history block wiring), 35–45 lines. Visual surface so a manual smoke is part of acceptance.

### 目标
Render artifacts as purple chips on the BB `HistoryEntry`, and on click invoke `open_vault_note` with toast-on-error.

### 设计要点 (audit-derived)
- Svelte chip component: new file under existing component dir (locate at execute time; audit did not enumerate). Uses tokens cited in audit Q5: background `var(--astrocyte-neural-purple)` at low opacity, border `var(--border-hud-emphasis)`, padding `var(--space-1) var(--space-2)`, radius `var(--radius-sm)`. Renders `kind` icon + truncated filename. Click → `invoke('open_vault_note', { path })`. — audit Q5 (`app.css:97-125`)
- Chip strip rendered inside the BB `HistoryEntry` block in `+page.svelte` near where other BB metadata renders (locate at execute time). Only when `entry.artifacts?.length > 0`.
- Error handling: surface command `Err` via the existing toast mechanism (locate at execute time; do NOT introduce a new toast lib).

### 任务范围
1. Chip component (~25 lines including scoped styles).
2. Chip strip render block in BB `HistoryEntry` + click→`invoke` wiring with toast on failure (`astrocyte/src/routes/+page.svelte`, ~15 lines).

### 验收
- Manual: end-to-end click on chip in a fixture session opens the corresponding vault file in Obsidian (verifies E3 friction resolution and the completion signal).
- Manual: click on a chip whose path was tampered to be outside vault → user-visible error toast; no crash.
- `Grep -n "rich" astrocyte/src/routes/+page.svelte` shows no new inline-preview code (phase-wide red line guard).
- `Grep "open_vault_note" astrocyte/src/routes/+page.svelte` returns the invoke site (≥1 hit).

### 红线
- ❌ Do NOT do rich inline preview (phase-wide red line).
- ❌ Do NOT introduce a new design-token color outside the existing `--astrocyte-*` / `--border-*` / `--space-*` / `--radius-*` set (audit Q5).
- ❌ Do NOT let chip-click mutate `HistoryEntry.artifacts` or trigger a new BB turn.
- ❌ Do NOT add a new toast / notification library — reuse existing pattern.
- ❌ 不进行机会主义重构

### 输出位置
- 代码: `astrocyte/src/routes/+page.svelte`, new chip component file
- 测试: 手动 smoke (FC.6 will harden into E2E)
- 文档: 推迟至 FC.6

---

## Sprint FC.4: Persona/Router separation — test-only

**Friction reference:** Anticipatory (no friction logged) — justification: Hard Sealing Condition #3 demands a unit test asserting persona-invariance; audit Q6 + cross-finding 2 confirm the *behavior* is in place. This sprint is the test that locks the invariant.

**Predecessor assumptions:**
- Audit Q6/cross-finding 2 hold: `final_persona_override` is `stage=PromptStage.FINAL`, Router stage filters out `{persona}`-bearing components with a warning. Re-plan trigger: Router prompt assembly refactored.

**Risk level:** 🟢 LOW
- Test-only. No source code changes outside `tests/`.

### 目标
Add a unit test asserting that the Router probe prompt is byte-identical when `persona` is varied while all other inputs are held constant.

### 设计要点 (audit-derived)
- Build two `PromptComposer` instances differing only in active persona registration; call the Router-stage compose; assert byte equality of the resulting prompt. — audit Q6 (`prompt_composer.py:271-282`, `agent.py:433-446`)
- Test must cover the warning-log path: assert that registering a `{persona}`-bearing component for the Router stage produces the documented warning *and* the component is dropped. — audit Q6 (`prompt_composer.py:271-282`)
- Out of scope: direct-mode persona injection in `lib.rs:706-735` (audit Q6 caveat). Document this as out-of-scope in the test docstring with a one-line pointer.

### 任务范围
1. New test `tests/oligo/test_prompt_composer.py::test_router_persona_invariance` (~30 lines).
2. New test `tests/oligo/test_prompt_composer.py::test_router_drops_persona_component_with_warning` (~20 lines, uses `caplog`).

### 验收
- `pytest tests/oligo/test_prompt_composer.py -k persona` passes both new cases.
- Hard Sealing Condition #3 verifiable by these tests at FC.6.

### 红线
- ❌ Do NOT modify `prompt_composer.py` or `agent.py` source — sprint-specific (any change implies the test is masking a defect; escalate instead).
- ❌ Do NOT extend scope to direct-mode (Rust) persona injection — out of FC.4 per audit Q6 caveat.
- ❌ Do NOT introduce new persona fixtures into shared conftest if a local-scope fixture suffices.
- ❌ 不进行机会主义重构

### 输出位置
- 测试: `tests/oligo/test_prompt_composer.py`
- 文档: 推迟至 FC.6

---

## Sprint FC.5: Message delete — verify-only

**Friction reference:** E4 (cannot delete messages, friction_260426 entry 2 UI residue) — driving, but materially already implemented per audit cross-finding 1.

**Predecessor assumptions:**
- Audit Q7/cross-finding 1 hold: `delete_chat_message` Tauri command, frontend `deleteMessage`, JSONL persistence, and stage-card-non-deletability invariant all exist as cited. Re-plan trigger: any of those code sites moved or behavior changed since 2026-05-18.

**Risk level:** 🟢 LOW
- Verify-only checklist walk; no code changes expected. If a gap surfaces, halt the batch and re-plan rather than expanding this sprint.

### 目标
Walk the existing delete pipeline against an explicit checklist and confirm Hard Sealing Conditions are satisfied; record gaps (if any) as new tickets without expanding this sprint.

### 设计要点 (audit-derived)
- Checklist items from audit Q7 + cross-finding 1:
  1. `delete_chat_message(session_id, msg_id)` exists at `lib.rs:1060-1109` and calls `delete_entry`.
  2. `delete_entry` at `memory.rs:267` removes the JSONL line by `id`.
  3. After delete, `state.set_history_for_session` reloads (`lib.rs` near the command).
  4. Frontend `deleteMessage` at `+page.svelte:1373-1387` invokes the command and updates local state.
  5. `onAiAction` at `+page.svelte:1498-1508` wires UI button → `deleteMessage`.
  6. `stripStageCards(history)` invocation at `+page.svelte:1427` keeps stage cards out of persistence.
  7. `lib.rs:691` comment ("user + bb only") still reflects current persistence boundary.
- A short E2E manual smoke (delete a BB message, restart astrocyte, confirm gone).

### 任务范围
1. Read each cited line range; record a checklist with PASS/FAIL per item to `docs/audits/FC.5-verify.md` (committed alongside the sprint, sibling to FC.0).
2. Manual E2E smoke per design-points bullet (no automation here; FC.6 handles E2E).
3. If any FAIL surfaces: HALT batch, file a new sprint, do not expand FC.5 to fix it.

### 验收
- All 7 checklist items PASS in `docs/audits/FC.5-verify.md`.
- E2E smoke: deleted message stays deleted across restart.

### 红线
- ❌ Do NOT modify source under FC.5 — verify-only by definition.
- ❌ Do NOT silently fix small drifts; halt and escalate.
- ❌ Do NOT couple FC.5 to FC.2b's `artifacts` field — delete pipeline is independent of artifact carriage.
- ❌ 不进行机会主义重构

### 输出位置
- 检查产物: `docs/audits/FC.5-verify.md` (committed; FC.0 audit's natural sibling).

---

## Sprint FC.6: Documentation + E2E smoke (seal sprint)

**Friction reference:** E3 (tool results unlinkable) + E4 (cannot delete messages) — both close at this sprint via the FINAL_CONTRACT.md surface and the E2E smoke that exercises the full chip→Obsidian path and the delete pipeline.

**Predecessor assumptions:**
- FC.1, FC.2a, FC.2b, FC.3a, FC.3b, FC.4, FC.5 all sealed. Re-plan trigger: any predecessor unsealed.

**Risk level:** 🟡 MED
- Doc-heavy plus an E2E smoke script. Doc is low-risk; smoke pulls together backend + Rust + frontend so failure modes are integration-shaped.

### 目标
Author `docs/ARCHITECTURE/FINAL_CONTRACT.md` and add an end-to-end smoke that exercises a vault-tool turn → artifact emit → chip render → Obsidian open path; satisfy all three Hard Sealing Conditions.

### 设计要点 (audit-derived)
- `FINAL_CONTRACT.md` documents:
  - `ToolOutput` / `Artifact` shape (FC.1) and contract that artifacts never enter LLM payload.
  - SSE `bb-message-artifacts` event shape and emit-site invariant: last yield before success-path return; one event per message; deduped (FC.2a).
  - Tauri forward + persistence boundary: `bb` entries only carry artifacts; `lib.rs:691` boundary remains canonical (FC.2b).
  - Persona stage rule: persona is FINAL-only on Oligo path; direct-mode is out-of-scope (FC.4).
  - Path-containment rule for `open_vault_note` (FC.3a) and chip rendering contract (FC.3b).
  - Message delete pipeline boundary: stage cards / system_log are webview-only; only `user` + `bb` rows persist and are deletable (FC.5, audit Q7).
- E2E smoke: scripted run (or documented manual procedure) that:
  1. Triggers a vault tool via Astrocyte (E3 friction's exact path).
  2. Confirms `bb-message-artifacts` lands at the assistant entry.
  3. Confirms chip renders.
  4. Confirms click opens Obsidian to the right note.
  5. Restarts astrocyte and confirms the artifacts persist on the message.
  6. Deletes the BB message via UI; restarts; confirms it stays deleted (E4 friction's path).
- Hard sealing condition verifications wired into smoke or referenced from this sprint:
  - HSC #1: `Grep "ToolOutput" crucible_core/src/oligo/tools/vault_tools.py` ≥3 hits.
  - HSC #2: `Grep -r "artifacts" crucible_core/src/oligo/core/agent.py` shows artifacts only in emit/aggregation paths, never in `self.messages.append` or `_render_tool_results_for_llm`.
  - HSC #3: `pytest tests/oligo/test_prompt_composer.py -k persona` (from FC.4) green.

### 任务范围
1. Author `docs/ARCHITECTURE/FINAL_CONTRACT.md` (~120 lines, follow style of existing `PROMPT_MIDDLEWARE.md` / `TOOL_PROTOCOL.md` / `INTENT_AND_DEGRADATION.md`).
2. E2E smoke script (locate testing convention at execute time; if no E2E harness exists, deliver a documented manual procedure as a checklist — do NOT introduce a new E2E framework here, audit `chimera-dependency-veto`).
3. Append to FRICTION_LOG.md the resolution of E3 (chip→Obsidian path) and E4 (delete pipeline confirmed by FC.5 verify) — propose the diff, user applies.
4. Update ROADMAP.md "Last sealed" to III.C — propose the diff, user applies.
5. Update ACCEPTED_PARTIALS.md with any FC.* trade-offs that surfaced — propose the diff, user applies.

### 验收
- `docs/ARCHITECTURE/FINAL_CONTRACT.md` exists and links from ROADMAP.md / Phase-III.C.md.
- All three Hard Sealing Conditions PASS via the cited Greps + tests.
- E2E smoke (or manual procedure) runs green at least once on user's machine.
- `chimera-sprint-discipline` `phase_review` mode produces a SEAL verdict against this batch.

### 红线
- ❌ Do NOT introduce a new E2E framework dependency — phase-wide (`chimera-dependency-veto`).
- ❌ Do NOT modify any "Start here" file (CLAUDE.md, ROADMAP.md, FRICTION_LOG.md, ACCEPTED_PARTIALS.md, TECHNICAL_DEBT.md) without proposing a diff for user approval — repo hard rule.
- ❌ Do NOT seal the phase if any HSC fails — escalate to user, file new sprint.
- ❌ 不进行机会主义重构

### 输出位置
- 文档: `docs/ARCHITECTURE/FINAL_CONTRACT.md`
- 测试: E2E smoke script location TBD at execute time (existing convention or manual procedure document under `docs/`)
- 元文档 diffs: ROADMAP.md, FRICTION_LOG.md, ACCEPTED_PARTIALS.md (proposed, user-applied)

---

## Phase-wide Red Lines

These apply across ALL sprints in this batch. Violation in any sprint halts the batch:

- ❌ Do NOT introduce dependency on OpenAI structured-output API (phase doc).
- ❌ Do NOT let artifacts influence LLM tool decisions (phase doc).
- ❌ Do NOT let artifacts enter messages payload to LLM (phase doc + HSC #2).
- ❌ Do NOT do rich inline preview in frontend (phase doc; Phase V territory).
- ❌ Do NOT modify "Start here" files (CLAUDE.md, ROADMAP.md, FRICTION_LOG.md, ACCEPTED_PARTIALS.md, TECHNICAL_DEBT.md) without explicit in-conversation user approval (repo hard rule).
- ❌ Do NOT add new dependencies without `chimera-dependency-veto` review.
- ❌ Do NOT touch DEBT-001…DEBT-008 opportunistically; debt week is its own thing.
- ❌ Do NOT extend scope of FC.4 / FC.5 from test-only / verify-only into source modification — halt and re-plan instead.

---

## Hard Sealing Conditions (carried from phase doc)

These MUST Pass at phase_review for sealing:

1. Three vault tools actually return `ToolOutput` — verified by `Grep "ToolOutput" crucible_core/src/oligo/tools/vault_tools.py` returning ≥3 hits at seal time.
2. Artifacts NEVER appear in messages sent to LLM — verified by Grep on `_render_tool_results_for_llm` / `_format_one_tool_result_xml` / `self.messages.append` paths in `agent.py` showing no `artifacts` reference.
3. Router behavior identical across persona changes — verified by `pytest tests/oligo/test_prompt_composer.py -k persona` (FC.4) green.

---

## Approval

User approves whole sequence or rejects whole sequence.

Upon approval, hand off to `chimera-code-taste` with:
> "Execute batch for Phase III.C per `docs/phases/Phase-III.C.md` and `docs/plans/Phase-III.C-batch.md`."

---

*Generated by chimera-sprint-discipline batch_planning mode against `docs/audits/FC.0.md` (2026-05-18).*
