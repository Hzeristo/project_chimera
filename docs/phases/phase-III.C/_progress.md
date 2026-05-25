# Phase III.C — Batch Progress Log

**Transient artifact.** Deleted at phase_review N+1 after seal.
**Purpose:** Cross-session resumption + audit trail for chimera-sprint-discipline phase_review.

Format: one entry per sprint completion, plus session-boundary entries.
Newest entries appended at the bottom.

---

## Sprint Entries

### FC.1 — Vault tools opt-in to `ToolOutput(text, artifacts)`

- **Status:** Pass
- **Commit:** `4a4cf0c`
- **Sprint record:** `docs/sprints/phase-III.C/FC.1.md`
- **Accepted partials proposed at FC.6 review:**
  - `search_vault` / `search_vault_attribute` return `artifacts=None` (adapter exposes no structured tier; parsing back from display string would brittle-couple the tool to formatter changes)
- **Verification status:** ruff/mypy/pytest deferred (env `paper` lacks tooling); schema + vault tool smoke via `conda run -n paper python` PASS
- **Planning deviation:** `Artifact`/`ToolOutput` placed in `crucible/core/schemas.py`, not `registry.py` as plan said — preserves one-way `oligo` → `crucible/core` import direction

---

### FC.2a — Backend artifact aggregation + `bb-message-artifacts` SSE emit

- **Status:** Pass
- **Commit:** `094f28d`
- **Sprint record:** `docs/sprints/phase-III.C/FC.2a.md`
- **Accepted partials proposed at FC.6 review:**
  - Smoke uses `_SmartMockClient` local to test file (conftest's `MockLLMClient` lacks PASS-switch on `[SYSTEM TOOL RESULTS]`)
- **Verification status:** ruff/mypy/pytest deferred; FC.2a end-to-end smoke confirmed event ordering (chunks → artifacts → return), no Python `bb-stream-done` on success, `messages` purity
- **Planning deviation:** Emit placed AFTER chunking loop (semantically right — UI receives artifacts when message is complete), still before `return` — cross-finding 4 invariant honored
- **DEBT filed:** DEBT-009 (test fixture migration) — committed in `fb25b47` per user pre-emptive stage

---

### FC.2b — Rust forward + Svelte state for `bb-message-artifacts`

- **Status:** Pass
- **Commit:** `dcb9807`
- **Sprint record:** `docs/sprints/phase-III.C/FC.2b.md`
- **Accepted partials proposed at FC.6 review:**
  - `state::Message` (runtime LLM history) NOT widened with `artifacts`; audit Q4 suggested it but inspection showed the field would risk leakage into outbound `evaluate_payload` (HSC #2 violation). ChatEntry.artifacts persisted; UI reads via SSE event + `load_session_archive` return.
- **Verification status:** `cargo test --lib` 4/4 pass on memory module; svelte-check deferred (no `node_modules`); manual UI smoke deferred to FC.6 E2E

---

## Session Boundary Entries

### Session 1 — 2026-05-23 (FC.0 audit done; batch plan + FC.1 + FC.2a + FC.2b)

- **Sprints completed this session:** FC.1, FC.2a, FC.2b (3 of 8)
- **Cumulative accepted partials this session:** 3 (one per sprint, listed above)
- **DEBT filed this session:** DEBT-009 (test fixture migration)
- **Process drift observations:**
  - 3 commits picked up files I didn't explicitly stage. Root cause: user concurrent edits to skill references / TECHNICAL_DEBT.md / friction logs were already in the index when I ran `git add` of explicit paths. `.claude/settings.local.json` correctly stayed unstaged each time.
  - Specific hitches:
    - `7a9a7fc` (batch plan commit): clean
    - `094f28d` (FC.2a commit): hitched 3 user-authored files (skill reference updates + friction_260523.md)
    - `fb25b47` (FC.1/FC.2a sprint records commit): hitched DEBT-009 entry
    - `dcb9807` (FC.2b commit): clean (4 files exactly)
    - `ad3d9ff` (FC.2b sprint record commit): clean
  - **Going-forward rule:** Always run `git diff --cached --name-only` immediately before `git commit` and either narrow with `git restore --staged` (for unauthorized files) or expand the message scope. The two clean commits (FC.2b + FC.2b record) prove the discipline works once applied.
- **Next session resumption point:** FC.3a (Rust `open_vault_note` + path-containment helper)
- **Predecessor assumptions for FC.3a (verify on resume):**
  - `astrocyte/src-tauri/src/memory.rs::Artifact` struct exists with `kind: String, path: String, metadata: Option<serde_json::Value>` — verify with `Grep "pub struct Artifact" astrocyte/src-tauri/src/memory.rs`
  - `HistoryEntry.artifacts` is populated end-to-end on a vault tool turn — assumed from FC.2b acceptance, would need running app to fully verify (FC.6 E2E)
  - No central vault-root containment helper exists yet — re-grep at FC.3a start to confirm none has appeared
  - `vault_root` source-of-truth in Rust state is unchanged — locate at FC.3a execute time
- **Tasks remaining:** FC.3a, FC.3b, FC.4, FC.5, FC.6
- **HSC tracking (interim):**
  - HSC #1 (3 vault tools return ToolOutput): satisfied by FC.1 — `Grep "ToolOutput" vault_tools.py` returns 12 hits across 3 tool sites
  - HSC #2 (artifacts NEVER in messages sent to LLM): held by FC.2a + FC.2b — Python `self.messages.append` sites artifact-free; Rust `state::Message` artifact-free
  - HSC #3 (Router persona-invariance): pending FC.4

---

## Hygiene Commits (out-of-batch)

### 2026-05-24 — Skill-memory alignment fixes

- **Trigger:** `docs/audits/skill-memory-alignment.md` (5 BLOCKING + 14 DEGRADING findings)
- **Reason for out-of-batch:** BLOCKING findings would break batch_execution preconditions on next session bootstrap (FC.3a). Must complete before resumption.
- **Files renamed (3):**
  - `docs/phases/Phase-III.C.md` → `docs/phases/phase-III.C.md`
  - `docs/logs/friction_260426.md` → `docs/logs/friction-260426.md`
  - `docs/logs/friction_260523.md` → `docs/logs/friction-260523.md`
- **Skill files edited (9):** SKILL.md ×2 (sprint-discipline, code-taste), references/ ×4 (phase-audit, batch-planning, phase-review, batch_execution), assets/ ×3 (phase-audit-template, batch-plan-template, phase-review-verdict-template), assets friction_entry_template (legacy variance note)
- **Content edits (2):** `friction-260518.md` status `CLOSED` → `RESOLVED`; created `docs/plunder_list.md` stub
- **DEBT filed:** DEBT-011 (cosmetic friction label alignment), DEBT-012 (silent append-only contract documentation)
- **Source code untouched:** verified — only `.claude/skills/`, `docs/`, no `crucible_core/src/` or `astrocyte/`
- **Phase III.C batch state:** untouched. FC.3a–FC.6 sprints still pending. HSC tracking unchanged.
- **Next action:** Resume Phase III.C at FC.3a in a fresh Sonnet session.

---

### FC.3a — Rust `open_vault_note` + path-containment guard

- **Status:** Pass
- **Commit:** `4261a84`
- **Sprint record:** `docs/sprints/phase-III.C/FC.3a.md`
- **Accepted partials:** None
- **Verification status:** `cargo test path_containment` 5/5 PASS; svelte-check deferred (no `node_modules`)
- **Planning deviation:** None

---

### FC.3b — Svelte artifact chip rendering

- **Status:** Pass
- **Commit:** `b7582bf`
- **Sprint record:** `docs/sprints/phase-III.C/FC.3b.md`
- **Accepted partials proposed at FC.6 review:**
  - `svelte-check` deferred — `node_modules` absent on dev host (same condition as FC.2b)
- **Verification status:** `cargo test path_containment` 5/5 PASS (no regression); svelte-check deferred; manual UI smoke deferred to FC.6 E2E
- **Planning deviation:** None

---

### FC.4 — Persona/Router separation — test-only

- **Status:** Pass
- **Commit:** `9a4e542`
- **Sprint record:** `docs/sprints/phase-III.C/FC.4.md`
- **Accepted partials:** None
- **Verification status:** `pytest tests/oligo/test_prompt_composer.py -k persona` 2/2 PASS
- **Planning deviation:** None
- **HSC #3 satisfied:** Router persona-invariance locked by these tests

---

### FC.5 — Message delete — verify-only

- **Status:** Pass
- **Commit:** (this record commit)
- **Sprint record:** `docs/sprints/phase-III.C/FC.5.md`
- **Accepted partials proposed at FC.6 review:**
  - E2E manual smoke deferred to FC.6
- **Verification status:** All 7 checklist items PASS; see `docs/audits/FC.5-verify.md`
- **Planning deviation:** None. No gaps found; no source changes needed.

---

*Generated by chimera-code-taste batch_execution mode, session-boundary protocol per `44cba3a`.*
