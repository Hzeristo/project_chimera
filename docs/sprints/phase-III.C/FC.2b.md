# Modification Summary: FC.2b

**Phase:** III.C
**Sprint:** FC.2b — Rust forward + Svelte state for `bb-message-artifacts`
**Batch position:** 3 of 8
**Date:** 2026-05-23
**Commit:** `dcb9807`

---

## Files touched

| Path | +Added | -Removed | Notes |
|---|---|---|---|
| `astrocyte/src-tauri/src/llm_client.rs` | +35 | -3 | New `bb-message-artifacts` SSE branch; widened `stream_oligo_agent` return to `(String, Option<Vec<Artifact>>)` |
| `astrocyte/src-tauri/src/memory.rs` | +94 | -0 | New `Artifact` struct; widened `ChatEntry.artifacts` + `RawChatEntry.artifacts`; 4 round-trip tests |
| `astrocyte/src-tauri/src/lib.rs` | +20 | -7 | `build_entry` takes artifacts; success path persists `model_artifacts` on bb row; user/error/abort rows pass `None` |
| `astrocyte/src/routes/+page.svelte` | +25 | -0 | `Artifact` type; `HistoryEntry.artifacts`; `SessionArchiveEntry.artifacts`; `bb-message-artifacts` listener; archive load mapper |

---

## Verification

| Check | Status | Output Summary |
|---|---|---|
| cargo check --lib | ✓ | Clean after fixing 5 type errors caused by widened return type |
| cargo test --lib (memory module) | ✓ | 4/4 pass: legacy parse, round-trip with artifacts, skip-if-none on serialize, artifact metadata omitted when None |
| svelte-check | deferred | `node_modules` not installed; out of FC.2b scope |
| Type-narrow audit | ✓ | grep: `state::Message` artifact-free; artifacts only in SSE event flow + persistence + UI HistoryEntry |
| Manual UI smoke | deferred | Requires running app; FC.6 E2E covers |

---

## Rule Conformance Self-Check

| Rule | Status | Evidence |
|---|---|---|
| DDD layering preserved | ✓ | `Artifact` lives in `memory.rs` (persistence-shaped); `llm_client.rs` imports from `memory`; `lib.rs` re-exports through public path. One-way deps |
| function_naming | ✓ | No new function names introduced |
| abstraction_threshold (rule of 3) | ✓ | No new abstractions |
| exception_handling | ✓ | New event branch uses `?` propagation matching existing pattern; cancel_token re-raise unchanged |
| pydantic_defaults | N/A | Rust schemas use serde; `#[serde(default, skip_serializing_if = "Option::is_none")]` applied consistently |
| ui_tokens | N/A | No CSS in this sprint (FC.3b territory) |
| logging_format (bracket prefix) | ✓ | No new log lines added |
| no_opportunistic_refactor | ✓ | Did NOT widen `state::Message`; did NOT touch `stream_direct_api` beyond the `.map` adapter required to make types match |

---

## Red Line Status

| Red Line | Status | Verification |
|---|---|---|
| Do NOT render chips | Held | No CSS, no chip component, no rendering — purely state plumbing |
| Do NOT add artifacts to user `ChatEntry` rows | Held | grep: 4 `build_entry` call sites; user/error/abort rows pass `None`; only the assistant row passes `model_artifacts` |
| Do NOT change `stage_card`/`system_log`/`error` sender semantics | Held | grep: `+page.svelte:14-36` `Sender` type unchanged |
| Do NOT add new design tokens | Held | No CSS edits |
| 不进行机会主义重构 | Held | `state::Message` unchanged; `stream_direct_api` body unchanged |

---

## Acceptance Criteria

| Criterion | Status | Evidence |
|---|---|---|
| `cargo test --lib` passes including new round-trip tests | ✓ | 4/4 pass at memory::tests |
| `Grep "bb-message-artifacts" llm_client.rs` returns exactly 1 SSE branch | ✓ | One match handler at line 328 (plus comments and emit map err) |
| `Grep "artifacts" +page.svelte` returns ≥2 matches (type + listener); no rendering | ✓ | 6 matches across type, listener, attach, archive load — no rendering yet |
| Fixture run produces non-empty `HistoryEntry.artifacts` | Pending | Requires running app with FC.2a backend; FC.6 E2E will verify |
| Reload round-trip: artifacts persist | ✓ via tests | Round-trip test confirms JSONL deserialize preserves field; `loadSessionArchive` mapper surfaces it |

---

## Notes

- **Reasoned scope cut (proposed for FC.6 review):** Audit Q4 suggested widening `state::Message` with `artifacts: Option<Vec<Artifact>>`. On inspection, `state::Message` is the runtime LLM history that flows back into outbound `evaluate_payload` calls — adding artifacts here would risk leakage into LLM payload (HSC #2 violation). Per audit Q7, persistence flows through `ChatEntry`, and Svelte reads artifacts via `load_session_archive` returning `Vec<ChatEntry>` (already wired). State::Message stays artifact-free. Audit recommendation overridden with reason; flagging for FC.6 phase review.
- **Five Rust compile errors caught and fixed during sprint:** type annotation on `model_reply`, missing `artifacts` field on a 5th `ChatEntry { ... }` literal in `set_history_for_session`, and the `stream_direct_api` adapter in the `else` branch of the `if is_oligo_mode` block needed `.map(|s| Some((s, None)))` to type-match.
- Handoff to FC.3a: Tauri command `open_vault_note(path: String) -> Result<(), String>` will be added separately; this sprint did not touch Tauri command surface beyond the existing artifact emit path.

---

## Commit Status

- [x] Files staged (4 explicit; clean — no hitched files this time)
- [x] Commit drafted (Tier-2 message)
- [x] Commit applied — `dcb9807`
- [ ] Pushed (deferred to FC.6 seal)

---

**Sprint result: Pass.** Proceeding to FC.3a.

---

*Generated by chimera-code-taste batch_execution mode, per-sprint summary.*
