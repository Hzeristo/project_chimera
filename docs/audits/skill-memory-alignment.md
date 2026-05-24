# Skill ↔ Memory Layer Alignment Audit

**Date:** 2026-05-24
**Auditor:** read-only pass, no modifications made

---

## Scope

### Skills audited
- `chimera-core-philosophy` (SKILL.md only — no references/ or assets/)
- `chimera-sprint-discipline` (SKILL.md + 5 references/ + 4 assets/)
- `chimera-code-taste` (SKILL.md + 4 references/ + 1 asset/)
- `chimera-dependency-veto` (SKILL.md only — no references/ or assets/)
- `chimera-commit-style` (SKILL.md only — no references/ or assets/)

### Memory files surveyed
- `docs/ROADMAP.md`
- `docs/ACCEPTED_PARTIALS.md`
- `docs/TECHNICAL_DEBT.md`
- `docs/logs/friction_260426.md`
- `docs/logs/friction-260506.md`
- `docs/logs/friction-260518.md`
- `docs/logs/friction_260523.md`
- `docs/phases/Phase-III.C.md`
- `docs/phases/phase-III.C/_progress.md`
- `docs/plans/Phase-III.C-batch.md`
- `docs/audits/FC.0.md`
- `docs/audits/python-deps.md`
- `docs/sprints/phase-III.C/FC.1.md`
- `docs/sprints/phase-III.C/FC.2a.md`
- `docs/sprints/phase-III.C/FC.2b.md`

### What this audit does NOT cover
- Runtime behavior of skills (whether the LLM actually follows the instructions)
- Code correctness of sprint outputs
- Whether skill policies are sensible as policies
- Architecture docs under `docs/architecture/` (no skill references them by path in a way that requires memory-layer alignment)

---

## Per-skill findings

### chimera-core-philosophy

No explicit references to docs/ paths, file name patterns, structural fields, or entry statuses. The skill references `docs/plunder_list.md` as a filing destination for SOTA ideas.

| Reference | Actual | Category | Severity | Evidence |
|---|---|---|---|---|
| `docs/plunder_list.md` (filing destination for SOTA ideas) | File does not exist; no equivalent found in docs/ tree | A (Path drift) | DEGRADING | `chimera-core-philosophy/SKILL.md:63` — "file the idea in `docs/plunder_list.md` for future reference" |

---

### chimera-sprint-discipline

#### bootstrap_protocol references

| Reference | Actual | Category | Severity | Evidence |
|---|---|---|---|---|
| `docs/logs/friction_xxxxxx.md` (latest) | Four files exist; two use underscore separator (`friction_260426.md`, `friction_260523.md`), two use hyphen (`friction-260506.md`, `friction-260518.md`). No single canonical pattern. | B (Pattern drift) | DEGRADING | `SKILL.md:22`; `docs/logs/` directory listing |
| `docs/FRICTION_LOG.md` (implied by bootstrap step 3 label "friction logs") | File does not exist at this path. All friction content lives under `docs/logs/friction-*.md` / `friction_*.md`. | A (Path drift) | BLOCKING | `SKILL.md:22` ("docs/logs/friction_xxxxxx.md (latest)") — the label is correct but the bootstrap step description says "friction logs" without a canonical path; `phase-review-process.md:99` explicitly reads `docs/FRICTION_LOG.md` as a fallback |

#### phase-audit-process.md references

| Reference | Actual | Category | Severity | Evidence |
|---|---|---|---|---|
| Phase doc at `docs/phases/phase-{X}.md` (precondition 1) | Active phase doc is at `docs/phases/Phase-III.C.md` (capital P). Pattern is inconsistent: `Phase-III.C.md` vs the lowercase `phase-{X}` template. | B (Pattern drift) | DEGRADING | `phase-audit-process.md:10`; actual file `docs/phases/Phase-III.C.md` |
| Output written to `docs/audits/phase-{X}.md` (step 6) | FC.0 audit was written to `docs/audits/FC.0.md`, not `docs/audits/phase-III.C.md`. The naming convention diverged from the template at first use. | B (Pattern drift) | DEGRADING | `phase-audit-process.md:39`; actual file `docs/audits/FC.0.md` |

#### batch-planning-process.md references

| Reference | Actual | Category | Severity | Evidence |
|---|---|---|---|---|
| Phase audit at `docs/audits/phase-{X}.md` (precondition 1) | Actual audit is `docs/audits/FC.0.md`. If a future batch_planning invocation uses the template path literally, it will not find the file. | B (Pattern drift) | BLOCKING | `batch-planning-process.md:11`; `docs/plans/Phase-III.C-batch.md:3` correctly cites `docs/audits/FC.0.md` — the plan itself adapted, but the process doc still says `phase-{X}` |
| Batch plan output implied to go to `docs/phases/phase-{X}.md` (step 5 template reference) | Actual batch plan is at `docs/plans/Phase-III.C-batch.md`, not `docs/phases/Phase-III.C.md`. The phase doc and the batch plan are separate files. | A (Path drift) | BLOCKING | `batch-planning-process.md:44` ("Output sprint sequence using assets/batch-plan-template.md"); `batch-plan-template.md:4` header says `docs/phases/phase-{X.Y}.md`; actual output is `docs/plans/Phase-III.C-batch.md` |

#### phase-review-process.md references

| Reference | Actual | Category | Severity | Evidence |
|---|---|---|---|---|
| `docs/phases/phase-{X.Y}/_progress.md` (step 0) | Actual file is `docs/phases/phase-III.C/_progress.md`. Path pattern matches if `{X.Y}` = `III.C`. No drift here — this one is consistent. | — | — | `phase-review-process.md:24`; `docs/phases/phase-III.C/_progress.md` exists |
| `docs/audits/phase-{X}.md` (step 2, precondition 1) | Actual audit is `docs/audits/FC.0.md`. Same pattern drift as above. | B (Pattern drift) | BLOCKING | `phase-review-process.md:14`, `phase-review-process.md:53` |
| `docs/phases/phase-{X}.md` (step 2, precondition 2 — "batch plan") | Actual batch plan is `docs/plans/Phase-III.C-batch.md`. The review process reads the batch plan from `docs/phases/`, but it lives in `docs/plans/`. | A (Path drift) | BLOCKING | `phase-review-process.md:54`; actual file `docs/plans/Phase-III.C-batch.md` |
| `docs/sprints/phase-{X.Y}/{sprint-id}.md` (step 1+) | Actual sprint records are at `docs/sprints/phase-III.C/FC.1.md` etc. Pattern matches. No drift. | — | — | `phase-review-process.md:45`; `docs/sprints/phase-III.C/FC.1.md` exists |
| `docs/FRICTION_LOG.md` OR `docs/logs/friction-*.md` (step 6) | `docs/FRICTION_LOG.md` does not exist. The `OR` fallback to `docs/logs/friction-*.md` is present but the primary path is dead. | A (Path drift) | DEGRADING | `phase-review-process.md:99` |
| `docs/ACCEPTED_PARTIALS.md` (step 4, step 8) | File exists at this exact path. No drift. | — | — | `phase-review-process.md:77`, `phase-review-process.md:119` |
| `docs/TECHNICAL_DEBT.md` (step 4, step 8) | File exists at this exact path. No drift. | — | — | `phase-review-process.md:78`, `phase-review-process.md:121` |
| `docs/ROADMAP.md` (step 8) | File exists at this exact path. No drift. | — | — | `phase-review-process.md:118` |
| Delete `docs/phases/phase-{X.Y}/_progress.md` at N+1 (step N+1) | File exists and is correctly identified as transient. Deletion instruction is correct. | — | — | `phase-review-process.md:128` |

#### batch-plan-template.md references

| Reference | Actual | Category | Severity | Evidence |
|---|---|---|---|---|
| Header declares `docs/phases/phase-{X.Y}.md` as the phase doc path | Actual phase doc is `docs/phases/Phase-III.C.md` (capital P). Actual batch plan was written to `docs/plans/Phase-III.C-batch.md`, not `docs/phases/`. The template conflates phase doc and batch plan into one file. | A (Path drift) + C (Schema drift) | DEGRADING | `batch-plan-template.md:4`; actual separation into `docs/phases/` (phase doc) and `docs/plans/` (batch plan) |

#### phase-audit-template.md references

| Reference | Actual | Category | Severity | Evidence |
|---|---|---|---|---|
| Template footer says output goes to `docs/audits/phase-{X}.md` | Actual audit output was `docs/audits/FC.0.md`. | B (Pattern drift) | DEGRADING | `phase-audit-template.md:65`; `docs/audits/FC.0.md` |

#### friction_entry_template.md schema

| Reference | Actual | Category | Severity | Evidence |
|---|---|---|---|---|
| Status values: `OPEN \| SCHEDULED \| RESOLVED \| WONTFIX` | Actual friction files use: `OPEN` (`friction_260523.md:1`), `CLOSED` (`friction-260518.md:1`), `SCHEDULED` (`friction-260518.md:12`). `CLOSED` is used in the wild but absent from the template's status vocabulary. `WONTFIX` appears in template but not in any actual entry. | D (Lifecycle drift) | DEGRADING | `friction_entry_template.md:1`; `docs/logs/friction-260518.md:1` ("Status: CLOSED"); `docs/logs/friction_260523.md:1` ("Status: OPEN") |
| Entry heading format: `### Entry {N} [Status: ...]` | `friction_260426.md` uses `### entry {N}` (lowercase, no status tag). `friction-260506.md` uses `### Entry {N}` (no status tag). `friction-260518.md` uses `### Entry {N} [Status: CLOSED]` / `[Status: SCHEDULED]`. `friction_260523.md` uses `### Entry 1 [Status: OPEN]`. | C (Schema drift) | DEGRADING | `friction_entry_template.md:1`; `docs/logs/friction_260426.md:3`; `docs/logs/friction-260506.md:1`; `docs/logs/friction-260518.md:1` |
| Field label `想做:` | `friction_260426.md` uses `想做的事：`, `friction-260506.md` uses `我想做的事情：`. Template uses `想做:`. | C (Schema drift) | COSMETIC | `friction_entry_template.md:3`; `docs/logs/friction_260426.md:4`; `docs/logs/friction-260506.md:2` |
| Field label `实际:` | `friction_260426.md` uses `实际怎么做的：`, `friction-260506.md` uses `实际怎么做的：` / `我实际怎么做的：`. Template uses `实际:`. | C (Schema drift) | COSMETIC | `friction_entry_template.md:4`; `docs/logs/friction_260426.md:5` |
| Field label `根因:` | Present in template and in `friction-260518.md:4`. Absent from `friction_260426.md` and `friction-260506.md`. | C (Schema drift) | DEGRADING | `friction_entry_template.md:5`; `docs/logs/friction_260426.md` (no 根因 field) |
| Field label `成本:` | Template uses `成本:`. `friction_260426.md` uses `摩擦成本：`. `friction-260506.md` uses `摩擦成本：`. `friction-260518.md` uses `成本:`. | C (Schema drift) | COSMETIC | `friction_entry_template.md:6`; `docs/logs/friction_260426.md:7` |
| Resolution block: `**Resolution:** Phase {X.Y} / Sprint {N} — {short_summary}` | `phase-review-process.md:step 6` checks for status transitions (OPEN/SCHEDULED → RESOLVED) but does not mandate the `**Resolution:**` block format. No actual friction entry has been marked RESOLVED yet, so the format is untested. | F (Silent contract) | COSMETIC | `friction_entry_template.md:9-10`; `phase-review-process.md:100-104` |

#### rules_and_antipatterns.md references

No direct docs/ path references. No findings.

---

### chimera-code-taste

#### bootstrap_protocol references

| Reference | Actual | Category | Severity | Evidence |
|---|---|---|---|---|
| `docs/ARCHITECTURE/PROMPT_MIDDLEWARE.md` | File exists at `docs/architecture/PROMPT_MIDDLEWARE.md` (lowercase `architecture`). | A (Path drift) | DEGRADING | `SKILL.md:21`; actual path `docs/architecture/PROMPT_MIDDLEWARE.md` |
| `docs/ARCHITECTURE/TOOL_PROTOCOL.md` | File exists at `docs/architecture/TOOL_PROTOCOL.md` (lowercase). | A (Path drift) | DEGRADING | `SKILL.md:22`; actual path `docs/architecture/TOOL_PROTOCOL.md` |
| `docs/ARCHITECTURE/INTENT_AND_DEGRADATION.md` | File exists at `docs/architecture/INTENT_AND_DEGRADATION.md` (lowercase). | A (Path drift) | DEGRADING | `SKILL.md:23`; actual path `docs/architecture/INTENT_AND_DEGRADATION.md` |
| `docs/ARCHITECTURE/TASK_PROGRESS_SYSTEM.md` | File exists at `docs/architecture/TASK_PROGRESS_SYSTEM.md` (lowercase). | A (Path drift) | DEGRADING | `SKILL.md:24`; actual path `docs/architecture/TASK_PROGRESS_SYSTEM.md` |

Note: On a case-insensitive filesystem (Windows NTFS, macOS HFS+) these paths resolve correctly. On a case-sensitive filesystem (Linux ext4, most CI environments) they would fail. Severity is DEGRADING rather than BLOCKING because the current dev environment is Windows, but the mismatch is a latent portability hazard.

#### batch_execution_process.md references

| Reference | Actual | Category | Severity | Evidence |
|---|---|---|---|---|
| Batch plan at `docs/phases/phase-{X}.md` (precondition 1) | Actual batch plan is `docs/plans/Phase-III.C-batch.md`. Same path drift as sprint-discipline. | A (Path drift) | BLOCKING | `batch_execution_process.md:11` |
| `docs/phases/phase-{X.Y}/_progress.md` (step 0, step 6, step N) | Actual file is `docs/phases/phase-III.C/_progress.md`. Pattern matches. No drift. | — | — | `batch_execution_process.md:19`, `57`, `67` |
| Sprint summary written to `docs/sprints/phase-{X.Y}/{sprint-id}.md` (step 6) | Actual sprint records are at `docs/sprints/phase-III.C/FC.1.md` etc. Pattern matches. No drift. | — | — | `batch_execution_process.md:56`; `docs/sprints/phase-III.C/FC.1.md` exists |

#### modification-summary-template.md

No docs/ path references. Template is self-contained. No findings.

---

### chimera-dependency-veto

| Reference | Actual | Category | Severity | Evidence |
|---|---|---|---|---|
| `docs/plunder_list.md` (filing destination for SOTA ideas) | File does not exist. Same missing file as referenced by chimera-core-philosophy. | A (Path drift) | DEGRADING | `chimera-dependency-veto/SKILL.md:53` — "file the idea in `docs/plunder_list.md` for future reference" |

---

### chimera-commit-style

No references to docs/ paths, file name patterns, structural fields, or entry statuses. The skill references friction IDs (`Refs: friction E1, E3`) and audit reports (`Refs: audit report 2026-04-25`) as free-text citation conventions, but does not specify file paths. No findings.

---

## Cross-skill findings

### CS-1: Batch plan path inconsistency (chimera-sprint-discipline + chimera-code-taste)

Both `chimera-sprint-discipline/references/batch-planning-process.md:44` and `chimera-code-taste/references/batch_execution_process.md:11` treat the batch plan as living at `docs/phases/phase-{X}.md`. The actual batch plan for Phase III.C is at `docs/plans/Phase-III.C-batch.md`. The phase doc (`docs/phases/Phase-III.C.md`) and the batch plan are separate files. Both skills will fail to locate the batch plan if they follow their own process docs literally.

**Severity: BLOCKING** — affects both the planning handoff (sprint-discipline → code-taste) and the review precondition check (sprint-discipline phase_review step 2).

### CS-2: Audit output path inconsistency (chimera-sprint-discipline)

`phase-audit-process.md`, `batch-planning-process.md`, and `phase-review-process.md` all reference the audit artifact as `docs/audits/phase-{X}.md`. The actual audit for Phase III.C is `docs/audits/FC.0.md`. The naming convention used in practice (sprint-ID-based) diverges from the template convention (phase-ID-based). Any skill that reads the audit by constructing the path from the template will not find it.

**Severity: BLOCKING** — phase_review precondition check and batch_planning precondition check both depend on locating this file.

### CS-3: `docs/FRICTION_LOG.md` ghost reference (chimera-sprint-discipline)

`phase-review-process.md:99` reads `docs/FRICTION_LOG.md` as the primary friction log path, with `docs/logs/friction-*.md` as a fallback. The primary path does not exist. The bootstrap protocol in `SKILL.md:22` correctly points to `docs/logs/friction_xxxxxx.md`. The two references within the same skill are inconsistent with each other and with the actual file layout.

**Severity: DEGRADING** — the fallback glob pattern is present, so the skill can recover, but the primary read will silently fail or require the skill to handle a missing-file case it may not anticipate.

### CS-4: `docs/plunder_list.md` ghost reference (chimera-core-philosophy + chimera-dependency-veto)

Both skills instruct filing SOTA ideas to `docs/plunder_list.md`. The file does not exist. Ideas filed per these instructions have nowhere to land, and any future skill or session that tries to read the plunder list will find nothing.

**Severity: DEGRADING** — no skill currently reads this file, so no downstream consumer breaks today. But the write instruction is a dead end.

### CS-5: Friction entry status vocabulary mismatch (chimera-sprint-discipline)

`phase-review-process.md:step 6` checks whether friction entries have moved from `OPEN/SCHEDULED` to `RESOLVED`. The template defines `OPEN | SCHEDULED | RESOLVED | WONTFIX`. Actual entries use `CLOSED` (not `RESOLVED`) in `friction-260518.md:1`. If the review process checks for `RESOLVED` status and the entry says `CLOSED`, the check will produce a false negative (friction appears unresolved when it is in fact closed).

**Severity: DEGRADING** — the review process will misread the friction resolution state for any entry marked `CLOSED`.

---

## Memory-layer observations

These are facts about `docs/` that no skill currently references, noted for completeness.

1. **`docs/plans/` directory exists but no skill references it.** The batch plan for Phase III.C lives at `docs/plans/Phase-III.C-batch.md`. No skill's process doc or template acknowledges `docs/plans/` as a distinct directory. Skills that construct the batch plan path from templates will look in `docs/phases/` instead.

2. **`docs/audits/python-deps.md` exists but no skill references it.** It was produced as part of DEBT-010 resolution work. It is not referenced by any skill's bootstrap, process, or template. This is not a misalignment (absence of reference is not a finding per audit scope), but it is noted as an unreferenced artifact.

3. **Friction log filename separator is inconsistent across the four existing files.** Two use underscore (`friction_260426.md`, `friction_260523.md`), two use hyphen (`friction-260506.md`, `friction-260518.md`). The bootstrap protocol pattern `friction_xxxxxx.md` (underscore) would miss the hyphen-named files if used as a glob.

4. **`docs/phases/Phase-III.C.md` uses a capital `P` in `Phase`.** The process docs and templates consistently use lowercase `phase-{X}`. On case-sensitive filesystems this is a path resolution failure.

5. **`docs/ACCEPTED_PARTIALS.md` update protocol says "Append-only at sprint seal."** The `phase-review-process.md` correctly outputs proposed diffs rather than writing directly. The append-only contract is honored by the process. No misalignment, but the contract is only documented in `ACCEPTED_PARTIALS.md:77` — no skill's process doc cites this constraint explicitly, making it a silent contract (category F) that could be violated if a skill writes directly.

6. **`docs/TECHNICAL_DEBT.md` update protocol says "New entries via `chimera-sprint-discipline` review process."** Same observation as above — the constraint is in the file footer but not cited in any skill process doc.

---

## Suggested next

The findings cluster into three work areas:

1. **Path canonicalization sprint** — Align process docs and templates to the actual `docs/plans/` vs `docs/phases/` split, the `docs/audits/FC.0.md` naming convention, the lowercase `docs/architecture/` path, and the `docs/FRICTION_LOG.md` ghost. This is a docs-only change with no code impact.

2. **Friction log standardization** — Decide on a canonical filename separator (underscore or hyphen), a canonical status vocabulary (`RESOLVED` vs `CLOSED`), and a canonical field label set. Apply retroactively or document the variance as accepted. This affects the lifecycle-drift findings and the cross-skill CS-3/CS-5 items.

3. **`docs/plunder_list.md` creation or reference removal** — Either create the file (even as an empty stub) so the write instruction has a valid target, or remove the reference from both skills and document where SOTA ideas should go instead.

---

## Finding summary

| Category | Count |
|---|---|
| A — Path drift | 10 |
| B — Pattern drift | 5 |
| C — Schema drift | 6 |
| D — Lifecycle drift | 1 |
| E — Capability gap | 0 |
| F — Silent contract | 1 |
| **Total** | **23** |

| Severity | Count |
|---|---|
| BLOCKING | 5 |
| DEGRADING | 14 |
| COSMETIC | 4 |

### Top BLOCKING items

1. **CS-1 / batch_execution_process.md:11 + batch-planning-process.md:44** — Both skills look for the batch plan at `docs/phases/phase-{X}.md`; actual location is `docs/plans/Phase-III.C-batch.md`.
2. **CS-2 / phase-audit-process.md:39 + batch-planning-process.md:11 + phase-review-process.md:14,53** — All three process docs look for the audit at `docs/audits/phase-{X}.md`; actual location is `docs/audits/FC.0.md`.
3. **batch-planning-process.md:11** — Precondition 1 reads `docs/audits/phase-{X}.md`; file not found under that name.
4. **phase-review-process.md:54** — Precondition 2 reads batch plan from `docs/phases/phase-{X}.md`; file not found there.
5. **phase-review-process.md:14** — Precondition 1 reads audit from `docs/audits/phase-{X}.md`; file not found there.

---

*Generated by read-only audit pass. No skill or doc modifications made.*
