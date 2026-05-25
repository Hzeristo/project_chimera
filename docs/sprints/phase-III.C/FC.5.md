# FC.5 — Message delete — verify-only

- **Commit:** (this record commit)
- **Status:** Pass
- **Files changed:** `docs/audits/FC.5-verify.md` (new)

## What was done

Walked the existing delete pipeline against the 7-item checklist from batch plan FC.5. No source changes.

**Checklist results (all PASS):**
1. `delete_chat_message` at `lib.rs:1067` calls `delete_entry` via `spawn_blocking`
2. `delete_entry` at `memory.rs:285` removes JSONL line by `id` field match
3. After delete, `get_entries_for_session` + `set_history_for_session` reloads state at `lib.rs:1111`
4. Frontend `deleteMessage` at `+page.svelte:1383` invokes command + filters local history
5. `onAiAction` at `+page.svelte:1517` wires Delete button → `deleteMessage`
6. `stripStageCards` at `+page.svelte:690` + call at line 1446 keeps unsettled stage cards out of persistence
7. `lib.rs:692` comment "user + bb only" still reflects current persistence boundary; FC.2b `artifacts` field on `bb` entries is consistent with this boundary

Full checklist with evidence: `docs/audits/FC.5-verify.md`

## Verification

All 7 items PASS. E2E manual smoke deferred to FC.6.

## Accepted partials

- E2E manual smoke deferred to FC.6 — same pattern as other manual smokes in this batch.

## Planning deviation

None. No gaps found; batch continues to FC.6 as planned.
