# Audit — V.A.2b ToolOutput.text degrades when must_read=0

**Scope:** read-only. Locate exact lines to change for the fix the user
already decided (incident `2026-06-20-ToolOutput.text-degrades-when-must_read=0.md`).
**Verdict on root cause:** incident hypothesis **(a)** confirmed, **(b) ruled out**.
ToolOutput is always constructed; the defect is that `summary` itself is
terse when `must_read=0`. **But the fix is not containable to
`daily_chimera_service.py`** — see Q5 / Fix points.

---

## Q1 — Where is ToolOutput constructed?

| What | file:line | Value |
|---|---|---|
| Construction | `daily_chimera_service.py:153` | `ToolOutput(text=summary, artifacts=artifacts if artifacts else None).model_dump_json()` |
| `.text` source | built at `daily_chimera_service.py:279-288` | The V.A.2a "enriched" summary — but enrichment (`must_read_lines`) is appended **only** `if must_read_lines:` (line 285). When `must_read=0` it is the **terse counts** string (lines 279-283) with nothing appended. |
| `.artifacts` source | `daily_chimera_service.py:152` → `_collect_pipeline_artifacts(stats, inbox_folder)` | Iterates **`stats.must_read_items` only** (line 88). Empty when `must_read=0` → coerced to `None` by the `if artifacts else None` guard. |

Exact construction block:

```python
151  inbox_folder = settings.require_path("inbox_folder")
152  artifacts = _collect_pipeline_artifacts(stats, inbox_folder)
153  return ToolOutput(text=summary, artifacts=artifacts if artifacts else None).model_dump_json()
```

The `summary` passed in is whatever `_run_pipelined_async` returned (lines 279-288).

---

## Q2 — `_collect_pipeline_artifacts` loop body

`daily_chimera_service.py:84-99`. Loop body:

```python
88  for item in stats.must_read_items:
89      moniker = sanitize_filename(item.short_moniker) if item.short_moniker else ""
90      basename = f"{item.id}-{moniker}" if moniker else sanitize_filename(item.id)
91      path = str(inbox_folder / "Must_Read" / f"{basename}.md")
92      artifacts.append(
93          Artifact(
94              kind="vault_note",
95              path=path,
96              metadata={"arxiv_id": item.id, "verdict": "must_read", "score": item.score},
97          )
98      )
```

- Iterates **`stats.must_read_items` only** — not "all filtered items."
- Produces an Artifact for **must_read items only**.
- **No** Artifact for skim items. **No** Artifact for reject items.

Note: the design decision says artifacts should include **skim** too (vault-resident),
excluding only reject. This loop currently omits skim — see Fix points.

---

## Q3 — Where is the V.A.2a enrichment applied?

There is no dedicated "summary builder" function. The summary is assembled
inline in `_run_pipelined_async`:

| Part | file:line |
|---|---|
| Base terse counts string | `daily_chimera_service.py:279-283` |
| Enrichment append (guarded) | `daily_chimera_service.py:284-286` |
| Line producer | `_collect_must_read_lines` `daily_chimera_service.py:68-81` |

```python
279  summary = (
280      f"Daily pipeline completed. new_pdfs={new_pdfs_count} ingested={ingested_count} "
281      f"batch_total={stats.total} must_read={stats.must_read} skim={stats.skim} "
282      f"reject={stats.reject} errors={stats.errors} telegram={'no' if skip_telegram else 'yes'}"
283  )
284  must_read_lines = _collect_must_read_lines(stats)
285  if must_read_lines:
286      summary += "\nMust Read:\n" + "\n".join(must_read_lines)
```

- Per-paper titles are included **only when `must_read_items` is non-empty**
  (guard at line 285; `_collect_must_read_lines` iterates `stats.must_read_items`
  at line 70). With all papers rejected → `must_read_lines == []` → no titles.
- Skim items: **not enumerated.** Reject items: **not enumerated.**
- This is the exact HSC-2 failure: `must_read=0` ⇒ terse-counts-only text.

---

## Q4 — Wiring path (return value → `Task.result`)

| Step | file:line | Behavior |
|---|---|---|
| Pipeline returns | `daily_chimera_service.py:153` | **Always** `ToolOutput(...).model_dump_json()`. **No** plain-str branch. |
| Tool wrapper returns it | `miner_tools.py:61-68` (`_run_daily_with_progress`) | Returns the string verbatim. |
| Scheduled as `work` | `miner_tools.py:110-116` | `task_service.run_task(task_id, work)` |
| Result captured | `task_service.py:363` | `result = await work` |
| Emitted as summary | `task_service.py:367` | `emit_completed(task_id, summary=result)` |
| Persisted | `task_service.py:197` | `task.result = summary if summary is not None else task.result` |

**Confirmed:** `Task.result` is always the `ToolOutput.model_dump_json()` string.
There is **no** branch returning a plain `str` when `artifacts=[]` — the
artifacts list is coerced to `None` but ToolOutput is still built and dumped.
**Incident hypothesis (b) is false; (a) is the actual cause:** the JSON's
`text` field carries the terse string because `summary` was never enriched
for the non-must_read verdicts.

---

## Q5 — Stats data shape (the blocker)

`BatchFilterStats` (`schemas.py:343-352`):

| Field | Type | Notes |
|---|---|---|
| `total` | `int` | counter |
| `must_read` | `int` | counter |
| `skim` | `int` | counter |
| `reject` | `int` | counter |
| `errors` | `int` | counter |
| `processed_ids` | `list[str]` | ids only, no verdict/title |
| `must_read_titles` | `list[str]` | must_read only |
| `must_read_items` | `list[BatchMustReadItem]` | must_read only |
| `source_dir` | `Path \| None` | — |

`BatchMustReadItem` (`schemas.py:333-340`): `score, id, paper_id, short_moniker, filename, title, novelty`.

- `must_read_items: list[BatchMustReadItem]` — fields as above.
- **`skim_items` does NOT exist.**
- **`reject_items` does NOT exist.**
- `errors` is an `int` counter (no per-error detail).

Producers confirm skim/reject capture **nothing but a counter**:
- live path `filter_queue_worker`: `stats.skim += 1` (`batch_filter_workflow.py:189`),
  `stats.reject += 1` (`:191`) — no item, no title appended.
- sync path `run_batch_filter`: `stats.skim += 1` (`:105`), `stats.reject += 1` (`:108`) — same.

**Consequence:** the enrichment **cannot** list skim or reject papers' titles
today — the titles are never recorded. The user's design decision ("text MUST
list all filtered papers' titles regardless of verdict") is **not achievable by
editing `daily_chimera_service.py` alone.** Upstream schema + producer changes
are required first.

---

## Fix points

Ordered; later edits depend on earlier ones. **The fix spans 3 files, not the
1–2 named in the incident's "Files likely touched."** Flagging because it
contradicts the stated scope.

### 1. `schemas.py` — capture skim & reject titles (prerequisite)
- **`schemas.py:333-340`** — add a lightweight item model (or reuse a trimmed
  one) for non-must_read verdicts, e.g. `BatchFilteredItem { id, short_moniker,
  title, score, verdict }`. (must_read can keep `BatchMustReadItem`.)
- **`schemas.py:350-351`** — add to `BatchFilterStats`:
  `skim_items: list[BatchFilteredItem] = Field(default_factory=list)` and
  `reject_items: list[BatchFilteredItem] = Field(default_factory=list)`.
  (Alternative: one `filtered_items` list with a `verdict` field — fewer fields,
  one loop downstream. Either satisfies the contract.)

### 2. `batch_filter_workflow.py` — populate the new lists (prerequisite)
- **`filter_queue_worker` (live path):** at the SKIM branch
  **`batch_filter_workflow.py:188-189`** and the reject `else` branch
  **`:190-191`**, append an item (title built from `paper.id` + `result.short_moniker`,
  same recipe as lines 171-174) instead of only bumping the counter. Mutation is
  already under `stats_lock` (`:167`) — keep the appends inside it.
- **`run_batch_filter` (sync parity path):** mirror the same appends at the SKIM
  branch **`batch_filter_workflow.py:104-106`** and reject `else` **`:107-108`**.

### 3. `daily_chimera_service.py` — emit enriched text + skim artifacts
- **`daily_chimera_service.py:284-286`** — replace the must_read-only append with
  an always-on enumeration of **all** processed papers (must_read + skim + reject),
  each as `title [score/10] — verdict`. Drop the `if must_read_lines:` guard so
  text is enriched even when `must_read=0`. Generalize/replace
  `_collect_must_read_lines` (**`:68-81`**) into a `_collect_all_verdict_lines`
  that walks `must_read_items` + `skim_items` + `reject_items`.
- **`_collect_pipeline_artifacts` `daily_chimera_service.py:88-98`** — add a second
  loop over `stats.skim_items`, emitting `Artifact(kind="vault_note", verdict="skim",
  path=inbox_folder/"Skim"/…)`. Per the design decision, **do not** emit artifacts
  for reject items (no vault path — they go to `papers/filtered/Reject/`).
  Verify the skim vault subfolder name against `VaultNoteWriter` before hardcoding
  `"Skim"`.
- **`daily_chimera_service.py:153`** — no change needed; once `summary` is enriched
  the `text` field is correct. The `artifacts if artifacts else None` coercion is
  fine to keep.

### Unchanged (confirmed)
- `miner_tools.py` wiring (`:61`, `:110-116`) — no change; it forwards the string.
- `task_service.py` (`:363`, `:367`, `:197`) — no change; persists result verbatim.
- arxiv_miner / legacy plain-string path — untouched by all edits above.

### Open question for the user
The incident's artifact rule says skim papers **are** vault-resident and should
get chips, but `_collect_pipeline_artifacts` currently needs their on-disk
filename to build the path. `BatchMustReadItem` carries `filename`; the proposed
`BatchFilteredItem` should carry it too if skim artifacts must resolve to the
exact written note (skim path = `inbox_folder/"Skim"/<filename>`). Confirm whether
skim notes land under a `Skim/` subfolder or elsewhere before fixing the path
literal at the new artifact loop.
