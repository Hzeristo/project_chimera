# Batch Plan: Phase III.F — Path Canonicalization

**Output location:** `docs/plans/Phase-III.F-batch.md`
**Audit reference:** F.0 audit delivered in-session (2026-06-12); no separate audit file
**Phase doc:** `docs/phases/phase-III.F.md`
**Driving frictions:** friction-260611 Entry 2 (papers downloaded to `crucible_core/` instead of repo root)
**Related debt:** DEBT-010 (env entropy — fold in only if F.0 finds connection; F.0 found no connection, out of scope)

This document is a single unit. User approves the whole sequence or rejects
the whole sequence. After approval, hand off to `chimera-code-taste`
batch_execution mode.

---

## Sprint Sequence

```
F.0 (audit — done) → F.1 → F.2 → F.3 → F.4
```

Linear. No parallelism. F.3 (data migration) cannot run before F.1+F.2 anchor and
bypass fixes are in place — migrating before fixing would send data to the still-wrong
canonical location.

---

## Sprint F.0: Audit

**Status:** ✅ Complete — delivered in-session 2026-06-12

**Key findings:**
- `config.py:_repo_root()` returns `parents[3]` = `crucible_core/`, not repo root (`parents[4]`)
- `platform.py` has `get_chimera_root()` but no `get_project_root()` — the gap to close in F.1
- `arxiv_fetch.py:244` bypasses `papers_root`, hardcodes `project_root / "papers" / "audit_log.csv"`
- `daily_chimera_service.py:89,103–107,185,199–202` has dead-code fallbacks to `project_root / "papers" / ...`
- `jinja_prompt_manager.py:26` `parents[4]` and `prompt_composer.py:21` `__file__-relative` are local-resource references, confirmed correct, do not touch
- `scripts/*.py` `parents[1]` — sys.path only, not data anchoring, out of scope

---

## Sprint F.1: Canonical Root

**Predecessor assumptions:**
- F.0 complete ✅

**Risk level:** 🟡 MED — `config.py` is imported at startup by everything; wrong depth breaks all path resolution.

### Goal
Add `get_project_root()` to `platform.py` and route `config.py`'s root anchor through it.
All downstream consumers of `ChimeraConfig.project_root` fix themselves.

### Design (audit-derived)
- `platform.py` is at `crucible_core/src/crucible/core/platform.py`; `parents[4]` = `project_chimera/` (repo root) — audit-verified
- `PROJECT_ROOT` is a module-level constant used in `SettingsConfigDict.env_file` (class-level attr, computed at import time); `get_project_root()` is also import-time pure path computation — no ordering issue
- `_repo_root()` in `config.py` can be eliminated; its only caller is `PROJECT_ROOT = _repo_root()` and `Field(default_factory=_repo_root)`; both replaced by a single `get_project_root()` call

### Scope
1. `platform.py`: add `get_project_root() -> Path` — returns `Path(__file__).resolve().parents[4]` (~4 lines)
2. `config.py`: import `get_project_root`; replace `_repo_root()` function and `PROJECT_ROOT = _repo_root()` with `PROJECT_ROOT = get_project_root()`; replace `Field(default_factory=_repo_root)` with `Field(default=get_project_root())`; delete `_repo_root()`

### Acceptance
- `python -c "from src.crucible.core.config import PROJECT_ROOT; print(PROJECT_ROOT)"` prints `project_chimera/` not `crucible_core/`
- `python -c "from src.crucible.core.platform import get_project_root; print(get_project_root())"` prints same
- No import errors; existing tests pass (DEBT-001 async tests remain skipped — not regressed)

### Red lines
- ❌ Do not touch `jinja_prompt_manager.py` or `prompt_composer.py` path logic
- ❌ Do not touch scripts' `parents[1]` sys.path blocks
- ❌ Do not opportunistically refactor `config.py`

### Output locations
- `crucible_core/src/crucible/core/platform.py`
- `crucible_core/src/crucible/core/config.py`

---

## Sprint F.2: papers_root Bypasses

**Predecessor assumptions:**
- F.1 complete — `project_root` now resolves to repo root; `papers_root` default is now correct via `_ensure_paper_miner_dirs`

**Risk level:** 🟢 LOW — local fixes in two files; all paths already flow through config after F.1.

### Goal
Eliminate the two remaining bypasses that hardcode `project_root / "papers" / ...` instead of using
the configured `papers_root`. Document the override path in `config.example.toml`.

### Design (audit-derived)
- `arxiv_fetch.py:244` reads audit log via `self.settings.project_root / "papers" / "audit_log.csv"` — must use `self.settings.paper_miner_or_default.papers_root / "audit_log.csv"` (audit log lives under `papers_root` by convention)
- `daily_chimera_service.py` fallbacks at lines 89, 103–107, 185, 199–202: `pm.arxivpdf_dir or (settings.project_root / "papers" / "arxivpdf")` — `_ensure_paper_miner_dirs` always fills these after F.1, making the `or` branch dead code; remove fallbacks, use `pm.arxivpdf_dir` / `pm.md_papers_raw_dir` / `pm.md_papers_dir` directly
- `papers_root` override already supported: `PaperMinerSettings.papers_root: Path | None = None`; if set in `config.toml` it takes priority over default; just needs a documented example

### Scope
1. `arxiv_fetch.py:244`: `self.settings.project_root / "papers" / "audit_log.csv"` → `self.settings.paper_miner_or_default.papers_root / "audit_log.csv"`
2. `daily_chimera_service.py`: remove three `or (settings.project_root / "papers" / ...)` fallback expressions (lines 89, 103–104, 106–107 in first pass; 185, 199–200, 202–203 in second pass)
3. `config.example.toml`: add commented `papers_root = "/absolute/path/to/data"` under `[paper_miner]` with one-line rationale

### Acceptance
- `grep -n "project_root.*papers" crucible_core/src/` — zero matches (all bypasses eliminated)
- `config.example.toml` has `[paper_miner]` section with commented `papers_root` entry
- Import + instantiation smoke: `python -c "from src.crucible.core.config import get_config; c = get_config(); print(c.paper_miner.papers_root)"` prints `project_chimera/papers`

### Red lines
- ❌ Do not change `_ensure_paper_miner_dirs` logic — only remove the now-dead fallback branches in callers
- ❌ Do not introduce a new root-computation pattern

### Output locations
- `crucible_core/src/crucible/ports/arxiv/arxiv_fetch.py`
- `crucible_core/src/crucible/services/daily_chimera_service.py`
- `~/.chimera/config.example.toml` (or repo's `config.example.toml` — verify location first)

---

## Sprint F.3: Data Migration

**Predecessor assumptions:**
- F.1 complete — anchor fixed; new canonical root is `project_chimera/`
- F.2 complete — all consumers now read from `papers_root`, which will resolve to `project_chimera/papers`

**Risk level:** 🔴 HIGH — data movement; dedup state (audit_log.csv) must survive intact.

### Goal
Move existing paper data from `crucible_core/papers/` to `<repo_root>/papers/`
and verify dedup is preserved.

### Design (audit-derived)
- Migration must happen BEFORE any pipeline run against the new config, otherwise the pipeline
  sees an empty `papers_root` and re-downloads everything
- `audit_log.csv` is the dedup source of truth; it must arrive at the new location intact and be
  the first thing confirmed before any pipeline run
- Do NOT delete source until post-move verification passes (sealing condition 3)

### Scope
1. Verify current state: list `crucible_core/papers/` — confirm `audit_log.csv` and subdirs present
2. Create `project_chimera/papers/` if it doesn't exist
3. Move contents of `crucible_core/papers/` to `project_chimera/papers/` (robocopy or Move-Item; preserve timestamps)
4. Verify `project_chimera/papers/audit_log.csv` readable and row count matches pre-move count
5. Run `daily_paper_pipeline` smoke (or `arxiv_fetch` alone with `max_results=1`) — confirm no re-downloads of already-seen IDs
6. After smoke passes: remove the now-empty `crucible_core/papers/` directory

### Acceptance
- `project_chimera/papers/audit_log.csv` exists and row count == pre-move count
- Pipeline smoke run produces zero new downloads for IDs already in audit log
- `crucible_core/papers/` is gone (post-verification cleanup)
- Satisfies Hard Sealing Condition 3

### Red lines
- ❌ Do NOT delete source data before post-move verification passes
- ❌ Do NOT run the pipeline before migration is complete — it would write to the new location but read dedup from the old

### Output locations
- `project_chimera/papers/` (new location)
- `crucible_core/papers/` deleted post-verification

---

## Sprint F.4: Enforcement

**Predecessor assumptions:**
- F.3 complete — canonical location in use, dedup verified

**Risk level:** 🟢 LOW — documentation + grep verification only.

### Goal
Prevent root-anchor bypass from regrowing. Codify the constraint so future code-taste
review catches violations without needing to re-audit.

### Design
- Enforcement is grep-verifiable: `grep -rn "Path(__file__).resolve().parents" crucible_core/src/` should
  match exactly two known-OK local-resource sites (jinja_prompt_manager, prompt_composer) and zero root-anchor sites
- Code-taste rule goes into the `chimera-code-taste` skill — one rule, one line

### Scope
1. Add rule to `chimera-code-taste` skill (wherever rules are stored):
   > **Root anchor rule:** All repo-root anchoring must call `platform.get_project_root()`. `Path(__file__).parents[N]` and `os.getcwd()` are forbidden for root derivation outside `platform.py`. Exception: `__file__`-relative paths for local resources (templates, prompt dirs co-located with the module) are allowed but must be commented as such.
2. Verify sealing condition 1: run the grep above; confirm only `jinja_prompt_manager.py` and `prompt_composer.py` remain, both with local-resource comments
3. Cross-device sim (sealing condition 4): move or symlink the repo to a temp absolute path, run `get_project_root()`, confirm it resolves to the new location's root

### Acceptance
- `chimera-code-taste` skill contains the root anchor rule
- Grep shows zero unaccounted `parents[N]` root-anchor sites in `crucible_core/src/`
- Cross-device sim passes (sealing condition 4)
- All four Hard Sealing Conditions verified and checked off in phase doc

### Red lines
- ❌ Do not add a pre-commit hook or CI gate — single-user OS, skill rule is sufficient

### Output locations
- `chimera-code-taste` skill file (location TBD at execution time)

---

## Phase-wide Red Lines

- ❌ Do NOT introduce a new root-computation pattern — everything routes through `platform.get_project_root()`
- ❌ Do NOT leave any `Path(__file__).parents[N]` or `os.getcwd()` for ROOT anchoring (local-resource `__file__`-relative refs are OK with comment)
- ❌ Do NOT touch `jinja_prompt_manager.py` or `prompt_composer.py` prompt-path logic
- ❌ Do NOT delete historical data — move it, verify, then clean
- ❌ Do NOT change `project_root` anchor before migrating dependent data (F.3 is gated after F.1+F.2)
- ❌ Do NOT opportunistically refactor

---

## Hard Sealing Conditions (carried from phase doc)

1. Single root definition: grep across `crucible_core/` shows ALL project-root anchoring goes through `platform.get_project_root()`; zero `parents[N]`/`getcwd()` root bypasses remain
2. papers_root behavior: with no config override, papers download to `<repo_root>/papers`; with a `config.toml` absolute override, they download there instead — verified by two live runs
3. Dedup preserved: after migration, re-running `daily_paper_pipeline` does NOT re-download papers already in `audit_log.csv`
4. Cross-device sim: deleting the config override + moving the repo to a different absolute path still resolves `papers_root` correctly to `new_repo_root/papers`

---

## Approval

User approves whole sequence or rejects whole sequence.

Upon approval, hand off to `chimera-code-taste` with:
> "Execute batch for Phase III.F per `docs/plans/Phase-III.F-batch.md`."

---

*Generated by chimera-sprint-discipline batch_planning mode — 2026-06-12.*
