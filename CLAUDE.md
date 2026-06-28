# Project Chimera

Personal research OS for a single PhD-student user. Not a framework. Not SaaS.

## Start here
- Current phase state: `docs/ROADMAP.md`
- Active frictions: `docs/FRICTION_LOG.md`
- Accepted trade-offs: `docs/ACCEPTED_PARTIALS.md`
- Open debt: `docs/TECHNICAL_DEBT.md`

## Current state
Pointer only — `docs/ROADMAP.md` is authoritative, do not treat this as a second source.
- Last sealed: Phase V.A — Exocortex node ontology (K/T/I/D nodes, staging protocol,
  `vault_query`, structured daily-pipeline output) — 2026-06-16.
- Before it: Phase IV.A — Async agent core (AWAITING_TASK suspension, coroutine-native
  theater loop) — 2026-06-14.
- Next: Phase V.A E2E smoke + a Use Week, then Phase V (Exocortex & Memory) — queued.

## Skills
1. `chimera-core-philosophy` — always active
2. `chimera-sprint-discipline` — planning / reviewing
3. `chimera-code-taste` — batch sprint execution (code/UI taste)
4. `chimera-dependency-veto` — adding dependencies
5. `chimera-commit-style` — drafting commits

## Hard rules
- This repo has ONE user. Do not generalize.
- Never modify files in the "Start here" list without explicit in-conversation user approval.
- Skill rules override generic best practices.

## Do not auto-modify
- Obsidian vault `templates/` — the user manually syncs these from
  `crucible_core/prompts/obsidian_tpl/`. Edit the repo copies, not the vault copies.
- `docs/staging/` candidates — user-reviewed; never auto-promoted to the vault.

## Architecture
See `docs/ARCHITECTURE/`.

## Repository layout
Beyond the crates, a few directories that aren't self-evident:
- `docs/audits/` — per-phase and debt-week audit trail.
- `docs/staging/` — staging-protocol candidates (`StagingService` output, Phase V.A.3).
- `crucible_core/prompts/obsidian_tpl/` — K/T/I/D node templates (Knowledge / Thought /
  Insight / Decision) as Jinja2; source of truth the vault `templates/` is synced from.

### Source of truth: phases / sprints / audits
A phase's record = **sprints + audits**. The folders answer different questions —
do not treat one as a substitute for another:
- `docs/phases/phase-{X.Y}.md` — **intent** ("what to do"): sparse manifest of
  sprint names + one-line goals. User-authored; skills never auto-write it.
- `docs/sprints/phase-{X.Y}/{sprint-id}.md` — **execution** ("what was done"):
  per-sprint summary written by `chimera-code-taste` at each commit. The
  batch-history source of truth for phase review.
- `docs/audits/{prereq-sprint-id}.md` — **evidence** ("right or wrong"): usually
  the pre-sprint prerequisite audit that drives planning; sometimes review/debt
  evidence (exceptions exist). Written by `chimera-sprint-discipline`.
- `docs/plans/Phase-{X.Y}-batch.md` — the batch plan derived from the audit
  (capital-P; written by `chimera-sprint-discipline` batch_planning).

If the execution record (`docs/sprints/phase-{X.Y}/`) is missing or empty, fall
back to phase **intent** to reconstruct batch history and flag the review as
"reconstructed". Operational R/W spec for skills: `.claude/skills/_shared/doc_folders.md`.

## Development environment

### Python (crucible_core)
- Path: `crucible_core/.venv/Scripts/python.exe`
- Version: 3.11+
- Package manager: uv
- Manifest: `crucible_core/pyproject.toml`
- Lockfiles: `crucible_core/requirements.txt`, `crucible_core/requirements-dev.txt`
- Activation prefix for tool calls: `Bash("./crucible_core/.venv/Scripts/python -m {tool}")`

### Testing
- Run from `crucible_core/`: `Bash("./crucible_core/.venv/Scripts/python -m pytest")`
- `asyncio_mode = "auto"` (`pyproject.toml`) — async tests are collected and run without
  per-test `@pytest.mark.asyncio` markers.

### Configuration
- Single source: `~/.chimera/config.toml` → `SystemConfig` (`crucible_core/src/crucible/core/config.py`).
- Recent fields: `staging_dir` (defaults to `docs/staging`), `vault_router_url`
  (vault-router worker endpoint; `None` falls back to the default URL).

### Rust (astrocyte)
- Standard cargo workspace, no special config
- Lockfile: `astrocyte/Cargo.lock` is authoritative

### Obsidian Vault
- Path: `D:\MAS\project_chimera_vault` — a SIBLING directory, not inside the repo.
- Set via `system.vault_root` in `~/.chimera/config.toml` (required; no hardcoded default).
- Claude Code has read access for auditing; write access only for staging-area
  operations (Phase V.A.3+).
