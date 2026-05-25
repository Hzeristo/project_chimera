# Final Contract: Structured Tool Output & Artifact Carriage

**Phase:** III.C (FC.1–FC.6) | **Status:** Active  
**Updated:** 2026-05-25 (against commit `9a4e542`)

## Purpose

Phase III.C closes two frictions: E3 (tool results unlinkable — vault tool outputs had no structured identity) and E4 (message delete pipeline confirmation). This document records the contracts established across the six sprints so future phases can extend without re-deriving invariants.

---

## 1. `ToolOutput` / `Artifact` Shape (FC.1)

### Schema

```python
# crucible_core/src/crucible/core/schemas.py:437
class Artifact(BaseModel):
    kind: str          # e.g. "vault_note"
    path: str          # vault-relative or absolute path
    metadata: dict[str, Any] | None = None

# crucible_core/src/crucible/core/schemas.py:460
class ToolOutput(BaseModel):
    text: str
    artifacts: list[Artifact] | None = None
```

### Contract

- `ToolOutput` is an **opt-in** return shape for tools that produce structured side-channel data. All other tools continue returning `str`.
- `_execute_tool` branches on `isinstance(result, ToolOutput)`: sets `raw_result = result.text` and captures `result.artifacts`. The `str(result)` coercion path at `agent.py:660` is the only edit site.
- `ToolOutput.text` participates in wash exactly as a `str` return does — `bypass_wash_tools` / `force_wash_tools` semantics are unchanged.
- **Artifacts never enter `result_body`** (the LLM-facing render). `_render_tool_results_for_llm` / `_format_one_tool_result_xml` are artifact-unaware by design.

### Current adopters

Three vault tools in `crucible_core/src/oligo/tools/vault_tools.py` return `ToolOutput`:
`search_vault`, `search_vault_attribute`, `obsidian_graph_query`.

`search_vault` and `search_vault_attribute` return `artifacts=None` (no structured tier in the adapter; parsing back from display string would brittle-couple the tool to formatter changes — accepted partial FC.1).

---

## 2. SSE `bb-message-artifacts` Event (FC.2a)

### Emit contract

```
crucible_core/src/oligo/core/agent.py:1398–1406
```

- `OligoAgent._session_artifacts: list[Artifact]` accumulates across all turns of `_run_theater_stream`. Initialized at `agent.py:431`; appended after `_wash_tool_results` at `agent.py:1022`. Deduped by `(kind, path)` tuple.
- Emitted **once**, as a single `bb-message-artifacts` SSE frame, immediately before the success-path `return` at `agent.py:1398`. Empty list → no emit.
- Payload shape: `{"artifacts": [{"kind": ..., "path": ..., "metadata": ...}, ...]}`
- **Invariant:** artifacts are emitted after all chunks, before `return`. The `bb-stream-done` success emit lives in Rust (`lib.rs`), not Python — this ordering is preserved.

### What this event is NOT

- Not part of `bb-tool-done` payload (tool-strip semantics are independent).
- Not a `__SYS_TOOL_CALL__` subkind.
- Not emitted on error paths — `bb-stream-done` error ownership (Python) is unchanged.

---

## 3. Tauri Forward + Persistence Boundary (FC.2b)

### Rust forward

```
astrocyte/src-tauri/src/llm_client.rs:328–348
```

`stream_oligo_agent`'s event match has an explicit `"bb-message-artifacts"` branch: parses JSON payload, calls `app.emit("bb-message-artifacts", payload)`. Placed before the success-path `Ok(Some(...))` return so Rust's `"DONE"` emit follows it.

### Rust schema

```rust
// astrocyte/src-tauri/src/memory.rs:67
pub struct Artifact {
    pub kind: String,
    pub path: String,
    pub metadata: Option<serde_json::Value>,
}

// astrocyte/src-tauri/src/memory.rs:91
pub artifacts: Option<Vec<Artifact>>,  // on ChatEntry, bb rows only
```

`#[serde(default)]` on `artifacts` ensures legacy JSONL entries (no `artifacts` key) deserialize without error.

### Persistence boundary

```
astrocyte/src-tauri/src/lib.rs:692
/// JSONL / timeline persistence is strictly **user + bb (assistant)** turns only.
```

- `artifacts` is carried on `bb` `ChatEntry` rows only. `user` rows never carry artifacts.
- `state::Message` (runtime LLM history) does **not** carry `artifacts` — the field would risk leakage into outbound `evaluate_payload` (HSC #2 violation). `ChatEntry.artifacts` persists; `Message.artifacts` does not exist.

### Svelte state

```
astrocyte/src/routes/+page.svelte:1579
```

`bb-message-artifacts` listener locates the in-flight BB entry (last `sender === 'bb'` with `isLoading`) and attaches `artifacts`. On session reload, `load_session_archive` returns `ChatEntry.artifacts` and populates `HistoryEntry.artifacts` directly.

---

## 4. Persona Stage Rule (FC.4)

### Rule

`final_persona_override` is registered for `PromptStage.FINAL` only (`prompt_composer.py:403`). The Router-stage compose path has a secondary guard at `prompt_composer.py:271–282` (HOTFIX.3): any component whose template contains `{persona}` is dropped with a `WARNING` log if it somehow reaches the Router stage.

`_compute_active_router_components` (`agent.py:441`) never includes `final_persona_override`. `_compute_active_final_components` (`agent.py:456`) adds it only when `self._persona` differs from `self._system_core`.

### Out of scope

Direct-mode (Rust) persona injection at `lib.rs:706–735` bypasses `PromptComposer` entirely. That path is not covered by this contract.

### Test coverage

`crucible_core/tests/oligo/test_prompt_composer.py::test_router_persona_invariance` — Router compose output is byte-identical across persona variants.  
`crucible_core/tests/oligo/test_prompt_composer.py::test_router_drops_persona_component_with_warning` — `{persona}`-bearing ROUTER component is dropped with a named warning.

---

## 5. Path-Containment Rule for `open_vault_note` (FC.3a)

### Command

```
astrocyte/src-tauri/src/lib.rs:1151
async fn open_vault_note(path: String, state, app) -> Result<(), String>
```

Registered in `invoke_handler!`. Reads `vault_root` from `state.chimera.read().system.vault_root`.

### `vault_contains_path` helper

```
astrocyte/src-tauri/src/lib.rs:1120
fn vault_contains_path(vault_root: &Path, raw: &str) -> Result<PathBuf, String>
```

Two-layer guard:
1. **Pre-filesystem:** rejects any path containing a `..` component before any I/O.
2. **Post-canonicalize:** `fs::canonicalize` on both root and candidate; asserts candidate starts with root. Rejects symlink escapes.

Returns the canonical `PathBuf` on success; bracket-prefixed error string on failure.

URI construction uses manual percent-encoding (byte-by-byte) to avoid a new dependency. Launches via `tauri-plugin-opener` `open_url`.

---

## 6. Chip Rendering Contract (FC.3b)

```
astrocyte/src/routes/+page.svelte:2026–2039  (template)
astrocyte/src/routes/+page.svelte:1411       (openVaultNote handler)
astrocyte/src/routes/+page.svelte:2821       (CSS)
```

- Chips render only on `msg.sender === 'bb'` entries with `msg.artifacts?.length > 0`.
- Each chip calls `invoke('open_vault_note', { path: art.path })` on click.
- Errors surface via `notifySystem('[OPEN_VAULT_NOTE_ERROR] ...')` — no new toast library.
- CSS uses only `--astrocyte-*` / `--surface-*` design tokens. No inline preview (Phase V territory).
- Label shows `art.path.split('/').pop()` (filename only); full path in `title` attribute.

---

## 7. Message Delete Pipeline Boundary (FC.5)

Full checklist verification: `docs/audits/FC.5-verify.md`.

| Component | Location | Role |
|-----------|----------|------|
| `delete_chat_message` Tauri command | `lib.rs:1067` | Entry point; calls `delete_entry` via `spawn_blocking` |
| `delete_entry` | `memory.rs:285` | Rewrites JSONL file, skipping the matched `id` |
| State reload | `lib.rs:1094–1111` | `get_entries_for_session` + `set_history_for_session` after delete |
| `deleteMessage` | `+page.svelte:1383` | Invokes command, filters local `history` array |
| `onAiAction` | `+page.svelte:1517` | Wires Delete button → `deleteMessage` |
| `stripStageCards` | `+page.svelte:690` | Removes unsettled stage cards before each dispatch |
| Persistence boundary | `lib.rs:692` | `user` + `bb` rows only; stage cards / system_log are webview-only |

Stage cards with `feedback !== undefined` (settled) survive `stripStageCards` and remain in the in-memory `history`, but are never written to JSONL — they are not deletable via `delete_chat_message`.

---

## Hard Sealing Conditions

| # | Condition | Verification | Status |
|---|-----------|-------------|--------|
| HSC #1 | Three vault tools return `ToolOutput` | `grep -c "ToolOutput" crucible_core/src/oligo/tools/vault_tools.py` ≥ 3 | **PASS** (18 hits) |
| HSC #2 | Artifacts never in LLM payload | `grep "artifacts" agent.py` shows zero hits in `messages.append` / `_render_tool_results_for_llm` | **PASS** |
| HSC #3 | Router persona-invariant | `pytest tests/oligo/test_prompt_composer.py -k persona` | **PASS** (2/2) |
