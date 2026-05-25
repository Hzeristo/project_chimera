# FC.5 — Message Delete Pipeline Verification

**Date:** 2026-05-25
**Sprint:** FC.5 (verify-only; no source changes)
**Audit basis:** batch plan FC.5 checklist + audit Q7 / cross-finding 1

---

## Checklist

### Item 1 — `delete_chat_message` command exists and calls `delete_entry`

- **Location:** `astrocyte/src-tauri/src/lib.rs:1067–1114`
- **Verdict:** PASS
- **Evidence:** `async fn delete_chat_message(session_id, msg_id, state)` defined at line 1067. Calls `delete_entry(&s, &m)` via `spawn_blocking` at line 1089.

### Item 2 — `delete_entry` removes the JSONL line by `id`

- **Location:** `astrocyte/src-tauri/src/memory.rs:285–330`
- **Verdict:** PASS
- **Evidence:** `pub fn delete_entry(session_id, entry_id)` at line 285. Reads all lines, skips the line whose `id` field matches `entry_id` (`if id == entry_id { continue; }`), rewrites the file.

### Item 3 — After delete, `set_history_for_session` reloads state

- **Location:** `astrocyte/src-tauri/src/lib.rs:1094–1112`
- **Verdict:** PASS
- **Evidence:** After `delete_entry` returns, `get_entries_for_session` reloads the file, maps entries to `Message` structs, calls `state.create_session` + `state.set_history_for_session` at line 1111.

### Item 4 — Frontend `deleteMessage` invokes the command and updates local state

- **Location:** `astrocyte/src/routes/+page.svelte:1383–1397`
- **Verdict:** PASS
- **Evidence:** `async function deleteMessage(msg)` calls `invoke('delete_chat_message', { sessionId, msgId })` then filters `history` to remove the entry. Errors routed to `notifySystem('[DELETE_ERROR] ...')`.

### Item 5 — `onAiAction` wires UI Delete button → `deleteMessage`

- **Location:** `astrocyte/src/routes/+page.svelte:1517–1527`
- **Verdict:** PASS
- **Evidence:** `onAiAction('delete', msg)` branch calls `await deleteMessage(msg)`. Delete button at line 2073 calls `onAiAction('delete', msg)`.

### Item 6 — `stripStageCards` invocation keeps stage cards out of persistence

- **Location:** `astrocyte/src/routes/+page.svelte:690–696, 1446`
- **Verdict:** PASS
- **Evidence:** `stripStageCards` filters out `stage_card` entries whose `feedback` is `undefined` (i.e., unsettled cards). Called at line 1446 before each new dispatch. Settled stage cards (marked via `markStageCardsSettled` on `bb-stream-done`) are retained in history but are not `user`/`bb` rows and are never written to JSONL (persistence boundary at `lib.rs:692`).

### Item 7 — `lib.rs:692` comment still reflects current persistence boundary

- **Location:** `astrocyte/src-tauri/src/lib.rs:692`
- **Verdict:** PASS
- **Evidence:** Comment reads: `"JSONL / timeline persistence is strictly **user + bb (assistant)** turns only."` Unchanged. FC.2b did not add `artifacts` to `user` rows; only `bb` `ChatEntry` carries the optional `artifacts` field — consistent with this boundary.

---

## E2E Manual Smoke

**Status:** Deferred to FC.6 E2E sprint.

The automated checklist above confirms the pipeline is structurally complete. The FC.6 smoke will exercise the full delete path (delete BB message → restart → confirm gone) as part of the phase seal.

---

## Summary

All 7 checklist items: **PASS**. No gaps found. No source changes required. Hard Sealing Condition for E4 (message delete) is satisfied structurally; E2E confirmation deferred to FC.6.
