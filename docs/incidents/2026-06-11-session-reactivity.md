# Incident 2026-06-11 — Session Reactivity Gaps

**Status:** Fixed  
**Friction ref:** friction-260611 Entry 4

## Gaps

**A** — New session: main HUD updated its own `sessionSummaries` via `refreshSessionHistory()` but never notified the timeline webview. Timeline only re-fetched on `window focus`.

**B** — Delete current session (from timeline): timeline webview called `invoke('delete_session_history')` and filtered its own local list, but emitted no event to the main HUD. Main HUD `deleteSession()` (which clears chat) was never called. Chat stayed stale.

## Root cause

Two separate webview JS processes. No shared store. The only cross-webview channel is Tauri events. Neither webview emitted an event to the other after create/delete.

Mutation patterns were fine (reassignment / filter — no `push`). Svelte reactivity within each webview worked correctly.

## Fix

**A** — `resetToNewSignal()` in `+page.svelte` now emits `session-list-changed` after `refreshSessionHistory()`. Timeline webview listens in `onMount` and calls `refreshSessions()` (re-fetches from disk).

**B** — `onDeleteSession()` in `timeline/+page.svelte` now emits `session-deleted` with the sessionId. Main HUD listens for `session-deleted`; if the deleted id matches `activeSessionId`, calls `resetToNewSignal()` (clears chat + re-fetches list).

## Changed files

| File | Change |
|---|---|
| `astrocyte/src/routes/+page.svelte` | add `emit` import; emit `session-list-changed` in `resetToNewSignal`; add `session-deleted` listener + cleanup |
| `astrocyte/src/routes/timeline/+page.svelte` | add `emit`/`listen`/`UnlistenFn` imports; emit `session-deleted` in `onDeleteSession`; listen for `session-list-changed` in `onMount`; unlisten in `onDestroy` |

No Rust changes. No CSS changes.
