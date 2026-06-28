# Skills — Final Audit vs. skill-creator Common-Sense Specs

**Date:** 2026-06-27
**Scope:** All 5 chimera skills + `.claude/skills/_shared/`, scored against the
skill-creator anatomy & rules (frontmatter, progressive disclosure, writing
style, bundled-resource organization, lack of surprise, self-containment).
**Method:** Verification pass on current disk state. No skill edits in this pass.
**Predecessors:** skill-self-assessment.md → skill-reaudit.md → docs-skill-alignment.md.

---

## Scorecard

| # | skill-creator rule | Verdict | Evidence |
|---|---|---|---|
| 1 | Frontmatter has `name` + `description` | ✅ PASS | All 5 present. |
| 2 | Description states *what* + *when* | ✅ PASS | Every description has an "Invoke/Activate when …" clause. |
| 3 | Sibling skills disambiguate (no trigger collision) | ✅ PASS | sprint-discipline: "Code modifications are delegated to chimera-code-taste"; code-taste scoped to "executing approved sprints batch-style". |
| 4 | SKILL.md ≤ ~500 lines | ✅ PASS | 66 / 66 / 121 / 128 / 141. Comfortable headroom. |
| 5 | Reference files >300 lines carry a TOC | ✅ PASS (vacuous) | Largest reference is 178 (phase-review-process.md); largest asset 157 (verdict-template). None crosses 300. |
| 6 | References pointed to with when-to-read guidance | ✅ PASS | Both process skills' `<invocation_modes>` map user phrase → process doc → template; `rules_summary` points to references/. |
| 7 | Bundled resources organized (scripts/references/assets) | ✅ PASS | code-taste: scripts/ (check_taste.ps1) + references/ + assets/. sprint-discipline: references/ + assets/. Advisory 3 are single-file (resources optional). |
| 8 | Imperative voice; explain *why*, not musty MUSTs | ✅ PASS | Only 5 all-caps imperatives repo-wide, each load-bearing (see below). Rationale-first style throughout (`<key_insight>`, "Why the separation matters"). |
| 9 | Examples included where useful | ✅ PASS | taste_rules Bad/Good pairs; commit-style 3-tier examples; verdict-template worked example; rules_and_antipatterns Bad/Good. |
| 10 | Principle of lack of surprise (no malware/misleading) | ✅ PASS | All content matches stated intent; check_taste.ps1 is a transparent ruff+mypy+pytest gate. |
| 11 | Self-contained / packageable | ⚠️ ACCEPTED DEVIATION | 12 cross-skill `../_shared/` references reach outside the skill dir — see D1. |

**10 of 11 pass; 1 accepted deviation. No blocking issues.**

---

## Detail on the non-perfect items

### D1 (⚠️ accepted) — self-containment vs. `_shared/`
`scripts/package_skill.py` bundles only a single skill directory, so the 12
`../_shared/` and `../../_shared/` pointers (execution_environment, incident_protocol,
subagent_routing, expected_model, doc_folders) would dangle in an exported
`.skill`. This is the deliberate trade made to kill ~100 lines of hand-synced
duplication across the two process skills. **Verdict: keep.** These skills are
repo-local instruments for one repo, never distributed. The dedup + single-source
maintenance win outweighs export portability the project will never use.
*If export is ever needed:* add a pre-package step that inlines `_shared/` into
each skill, or relocate the shared blocks into each skill and re-accept the
duplication. Not worth doing speculatively.

### N-1 (note) — all-caps imperatives are all load-bearing
The 5 occurrences are not the "musty MUST" anti-pattern the rule warns about:
- code-taste `SKILL.md:70` — "subagent MUST return … exit code": the verification
  contract; the whole gate depends on it.
- sprint-discipline `SKILL.md:71` — "NEVER WRITE": the write-authority boundary header.
- phase-review-process.md:83 — "main session MUST re-Grep": the seal-time
  no-trust-the-subagent safeguard (B2).
- batch-plan-template.md:88 — "These MUST Pass": sealing conditions.
- verdict-template.md:144 — "artifacts NEVER in messages": inside a worked
  example (a red-line label), not an instruction.
Each is a hard gate where ambiguity would cause real harm. No action.

### N-2 (note) — description pushiness
Descriptions are accurate with clear when-clauses but not aggressively "pushy."
For these skills that is **correct**: the two process skills are mode-driven
(the user types "audit phase X" / "execute batch"), so precise triggering beats
broad triggering, and over-pushiness would cross-fire sprint-discipline ↔
code-taste. The one skill that could benefit from firing on *indirect* cues is
**chimera-dependency-veto** (e.g. "let's just pull in langchain for the agent
loop" without the word "dependency"). That remains the single best candidate for
the skill-creator description-optimization loop (`run_loop.py`) if you ever want
it — offered, not required.

### N-3 (note) — advisory-skill least privilege
core-philosophy & dependency-veto declare no `allowed-tools` (default broad
access); they are read-only advisory, so this is harmless but not least-privilege.
commit-style grants `Edit`. Cosmetic; left as-is.

---

## Cross-cutting health (post-fix verification)

- **Pointer integrity:** every `references/`, `assets/`, and `_shared/` target
  resolves (re-confirmed this session). No dangling references.
- **Path conventions ↔ disk:** aligned (M1/M2 applied) — `Phase-{X.Y}-batch.md`
  capital-P, phases lowercase, examples now use IV.A (a phase whose artifacts all
  exist).
- **Folder R/W authority:** explicit and non-contradictory — each mode
  autonomously writes its own output artifact (audit→audits/, plan→plans/,
  exe→sprints/); phases/ is human intent only; `_shared/doc_folders.md` is the
  single source, mirrored in CLAUDE.md.
- **Resilience:** phase_review has a reconstruct-from-intent fallback when the
  execution record is missing (closes the V.A-no-sprints-dir gap).
- **Activation behavior:** both skills carry inline model-recommendation triggers
  (N1 regression fixed); execution_environment keeps its inline "use PowerShell"
  reminder so the model doesn't emit Bash before reading the shared file.

---

## Bottom line

The skill set is in good shape against skill-creator common-sense specs: **10/11
rules pass cleanly, with one deliberate, documented deviation** (`_shared/`
non-portability, accepted for a repo-local skill set). No structural or
correctness issues remain. The only open *optional* lever is description-trigger
optimization for chimera-dependency-veto — a tuning opportunity, not a defect.

Recommended next action: **none required.** Treat this as the closeout of the
skill-hardening arc (self-assessment → re-audit → alignment → SOT/authority →
final). Revisit only if a skill is exported (D1) or triggering misses are
observed in real use (N-2).

*Audit only — no skill files modified in this pass.*
