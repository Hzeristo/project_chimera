# Phase III.F — Path Canonicalization

**Status:** Sealed — 2026-06-12
**Sealed predecessor:** Phase III.E (2026-06-11)
**Driving frictions:**
- friction-260611 Entry 2 partially (papers downloaded to wrong dir — crucible_core/
  instead of repo root)
**Related debt:** DEBT-010 (conda env / pyproject vs requirements alignment —
  same launch-script territory, fold in if F.0 finds it connected)

## Mission

Establish a single, canonical project-root definition routed through
platform.py, eliminate the four conflicting root computations, make
papers_root default to repo-root and configurable, and migrate existing
data without losing audit-log dedup state. This closes the Phase I.M2 gap:
config was unified but bypass was never enforced.

## Product Decision (from ST 2026-06-12, not re-derivable)

- papers_root DEFAULT = <repo_root>/papers (open-box: clone & run on a
  new device)
- papers_root OVERRIDE = absolute path in config.toml (point at existing
  data when relocating across devices)
- Rationale: cross-device portability — code follows git, data follows
  config. User anticipates a hardware change; data location must be
  decoupled from repo location.

## Sprint Sequence

| Sprint | One-line goal | Status |
|---|---|---|
| F.0 | Audit: ALL project_root / root-anchor consumers + the 4 conflicting definitions + every platform.py bypass | ✅ Done |
| F.1 | Canonical root: fix config.py anchor, route all root computation through platform.py, eliminate bypasses | ✅ Done |
| F.2 | papers_root: repo-root default + config.toml override, absolute-anchored | ✅ Done |
| F.3 | Data migration: move existing crucible_core/papers + audit_log.csv to canonical location, preserve dedup | ✅ Done |
| F.4 | Enforcement: code-taste rule forbidding root-anchor bypass + regression smoke | ✅ Done |

Dependencies: F.0 precedes all. F.1 precedes F.2 (root must be canonical
before papers_root derives from it). F.3 AFTER F.1/F.2 (migrate to the
NEW canonical location). F.4 last (enforcement + verification).

## Cross-Sprint Red Lines

- ❌ Do NOT change project_root anchor without first migrating dependent data
  (historical papers + audit_log.csv must move BEFORE or ATOMIC-WITH the
  anchor change, else dedup breaks → duplicate downloads)
- ❌ Do NOT introduce a NEW root-computation pattern — everything routes
  through platform.py
- ❌ Do NOT leave any Path(__file__).parents[N] or os.getcwd() used for
  ROOT anchoring (path-relative-to-a-known-file for non-root purposes is OK)
- ❌ Do NOT touch prompt_composer's prompts/ path unless F.0 proves it's a
  root-anchor bug (it's __file__-relative for a local resource, likely fine)
- ❌ Do NOT delete historical data — move it, verify, then optionally clean

## Hard Sealing Conditions

1. Single root definition: grep across crucible_core/ shows ALL project-root
   anchoring goes through one platform.py function; zero parents[N]/getcwd
   root bypasses remain (verified by grep + the F.0 consumer list).
2. papers_root behavior: with no config override, papers download to
   <repo_root>/papers; with a config.toml absolute override, they download
   there instead (verified by two live runs).
3. Dedup preserved: after migration, re-running daily_paper_pipeline does
   NOT re-download papers already in audit_log.csv (verified — the migrated
   audit log is read from the new location).
4. Cross-device sim: deleting the config override + moving the repo to a
   different absolute path still resolves papers correctly to the new
   repo_root/papers (verified by relocating the repo or simulating it).

## Design Decisions

- **Two-layer papers_root (default + override)**: default derives from
  canonical repo_root via platform.py; override is an absolute path in
  config.toml. Decouples data location from repo location for portability.

- **platform.py as the ONLY root authority (closes M2 gap)**: Phase I.M2
  unified config but never enforced platform.py usage, so four conflicting
  root definitions regrew (config.py parents[3], jinja parents[4],
  prompt_composer __file__-relative, start_oligo parents[1]). F.1 makes
  platform.py the single authority; F.4 enforces it so bypass cannot regrow.

- **Migration before anchor change (data safety)**: the anchor fix and data
  migration must be ordered so audit_log.csv is readable at the new location
  before dedup runs against it. Otherwise the pipeline re-downloads
  everything. F.3 handles this explicitly, gated after F.1/F.2.

## Out of Scope

- Vault root relocation (only papers_root in scope; vault already works)
- Full config schema redesign (only papers_root + root anchor)
- prompt_composer / jinja prompt paths IF F.0 confirms they're correct
  local-resource resolution (not root anchoring)
- The proxy/network jitter from friction-260611 Entry 2b (local network config)
