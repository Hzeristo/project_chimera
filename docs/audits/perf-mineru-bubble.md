# Perf Audit: MinerU GPU Bubble in Daily Pipeline

**Date:** 2026-06-18  
**Scope:** Read-only incident investigation. No code changes proposed here.  
**Symptom:** GPU utilization sawtooth — 80%→0%→80%→0% during `run_daily`.

---

## Q1 — Pipeline Stage Structure

**Top-level call chain** (`daily_chimera_service.py:run_daily_pipeline`):

```
run_arxiv_fetch()        lines 131–133   ← ALL downloads first
run_pdf_ingestion()      lines 145–151   ← ALL converts second
run_batch_filter()       lines 162–163   ← ALL LLM triage third
```

**Structure: batched at the top level, strictly serial within each batch.**

No paper starts convert until the last paper finishes download. No paper starts
filter until the last paper finishes convert.

### Inner loops

`run_pdf_ingestion` (`mineru_pipeline.py:189–222`):
```python
for idx, pdf_path in enumerate(progress, start=1):   # line 189
    raw_md = client.convert(pdf_path)                 # line 192 — blocks on MinerU subprocess
    paper_loader.extract_and_clean(...)               # line 198 — cleans after each convert
```
One PDF fully converted and cleaned before the next starts. Strictly serial.

`run_batch_filter` (`batch_filter_workflow.py:75–122`):
```python
for idx, md_file in enumerate(progress, start=1):    # line 75
    paper = loader.load_paper(md_file)                # line 79
    result = engine.evaluate_paper(paper)             # line 82 — LLM call
    writer.write_knowledge_node(paper, result)        # line 90/105 — vault write
    router.route_and_cleanup(paper, result)           # line 108
```
One paper fully triaged and written before the next starts. Strictly serial.

---

## Q2 — Where the GPU Goes Idle

| Stage | File:line | Binding | GPU state |
|---|---|---|---|
| `run_arxiv_fetch` → `download_pdfs` | `arxiv_fetch.py:204–240` | Network (HTTP streaming, sequential per-PDF) | **Idle** |
| `run_pdf_ingestion` → `client.convert` | `paper2md.py:63–84` | GPU (`-d cuda` subprocess) | **Active** |
| `extract_and_clean` (post-convert) | `mineru_pipeline.py:198` | IO (file read/write) | Idle |
| `run_batch_filter` → `engine.evaluate_paper` | `batch_filter_workflow.py:82` | Network (DeepSeek HTTP) | **Idle** |
| `writer.write_knowledge_node` | `batch_filter_workflow.py:90,105` | IO + possibly another LLM call | Idle |
| `router.route_and_cleanup` | `batch_filter_workflow.py:108` | IO (file move/delete) | Idle |

**Download phase** (`arxiv_fetch.py:221–230`): `requests.get(stream=True)` in a plain
`for` loop — one HTTP stream at a time, sequential, no GPU.

**MinerU convert** (`paper2md.py:63–73`): subprocess CLI with `-d cuda`. The GPU is
active only for the duration of `subprocess.run()` (line 76), which blocks until the
process exits. When the loop moves to `extract_and_clean` the GPU is already released
(process exited).

**batch_filter LLM** (`batch_filter_workflow.py:82`): `FilterService.evaluate_paper`
calls `build_openai_client` → OpenAI-compatible HTTP client → DeepSeek network call.
Pure network-bound. GPU idle throughout. Each paper takes one round-trip.

The sawtooth is `convert(paper 1) → extract_and_clean(1) → convert(paper 2) → ...`
with GPU idling during every `extract_and_clean` plus the full download and full
filter phases flanking the ingest batch.

---

## Q3 — MinerU Concurrency Safety

**Invocation mode: subprocess CLI** (`paper2md.py:63–84`).

```python
cmd = [self.cmd, "-p", str(pdf_path), "-o", str(self.output_root), "-m", "auto", "-d", "cuda"]
proc = subprocess.run(cmd, ...)   # paper2md.py:76
```

Each call spawns a fresh `mineru` process. The parent Python process blocks at line 76
until the child exits.

**Collision analysis for concurrent subprocesses:**

- **Output directory**: `-o {self.output_root}` is the same for all calls
  (`pm.md_papers_raw_dir`). Each paper writes to `{output_root}/{pdf_stem}/`
  (`paper2md.py:55–57`). arxiv IDs are unique → no directory collision.
- **Model cache** (`~/.cache/huggingface` or equivalent): read-only after first
  download. Concurrent reads are safe; no write-lock needed during inference.
- **GPU model weights**: each subprocess loads the full MinerU model suite into VRAM
  independently. **Two concurrent subprocesses = two full model loads in VRAM.**
  This is the primary concurrency risk (see Q4).
- **Temp files**: MinerU writes intermediate files inside `{output_root}/{stem}/`.
  Per-paper isolation means no temp-file collision.

**Verdict**: Filesystem-safe to run ≥2 concurrent converts. GPU-memory unsafe unless
VRAM headroom is confirmed (see Q4). No Python-level thread-safety concern because
the work is in subprocesses, not threads sharing state.

---

## Q4 — Memory Headroom

**RTX 5060 VRAM**: 8 GB (non-Ti). The Ti variant has 16 GB — but the incident report
says "RTX 5060", not Ti.

**MinerU VRAM footprint per subprocess** (not measurable from code — must be profiled):
- MinerU's default `auto` pipeline loads: layout detection model (DocLayout-YOLO or
  LayoutLMv3), formula detection (MFD), formula recognition (MFR), and optionally OCR.
- Reported community measurements for the full stack: **4–8 GB** depending on model
  variant and page complexity.
- At 8 GB VRAM with one convert at ~4–6 GB loaded: a second concurrent convert is
  likely OOM on an 8 GB card.

**Must be measured**: run `nvidia-smi dmon -s mu` while a single MinerU convert is
active to get actual peak VRAM. Until then, assume a single concurrent convert is the
safe ceiling on RTX 5060 non-Ti.

**Implication for Q6**: `convert(N) ∥ convert(N+1)` is almost certainly unsafe on this
hardware. Only one GPU-bound stage at a time is the safe operating assumption.

---

## Q5 — batch_filter LLM Stage

`run_batch_filter` is called at `daily_chimera_service.py:162` — **after
`run_pdf_ingestion` returns completely** (line 151). There is no overlap between the
two phases at all in the current design.

Within `run_batch_filter` (`batch_filter_workflow.py:75–122`), per-paper sequence:
1. `loader.load_paper(md_file)` — IO read, ~ms
2. `engine.evaluate_paper(paper)` — DeepSeek HTTP, **network-bound**, GPU idle, likely
   the longest single step per paper (LLM round-trip latency)
3. `writer.write_knowledge_node(paper, result)` — IO write (+possibly a second LLM
   call for the Obsidian node template, also network-bound if so)
4. `router.route_and_cleanup(paper, result)` — IO file move

All steps are strictly sequential per paper; no per-paper parallelism. The entire
`batch_filter` phase is GPU-dark: DeepSeek is a remote API, not a local GPU model.

**This phase is the best overlap candidate** because it is pure network+IO and cannot
contend with a local GPU no matter what.

---

## Q6 — Safe Overlap Opportunities

Ranked by safety × expected bubble-reduction:

### Tier 1 — Safe, high impact

**`batch_filter(paper N)` ∥ `convert(paper N+1)`** — network vs GPU

- No resource contention: DeepSeek HTTP has zero interaction with local VRAM.
- Expected bubble reduction: eliminates most of the convert idle time that currently
  sits between the ingest phase end and batch_filter completion. For a batch of K
  papers: the effective wall-clock becomes `max(convert_time, filter_time)` per paper
  rather than `convert_time + filter_time`.
- Requires: per-paper pipeline instead of two separate batch loops. One paper's
  convert result feeds immediately into filter while the next paper's convert starts.
- Safety caveat: none from a resource perspective. The only design change is the loop
  structure.

**`download(paper N+1)` ∥ `convert(paper N)`** — network vs GPU

- No resource contention: HTTP downloads touch only disk and NIC.
- Expected bubble reduction: hides download latency behind convert time. Downloads
  are already complete before converts start, so the bubble here is the full download
  phase (currently a GPU-dark prefix to the run).
- For typical arxiv PDFs (~2–5 MB each, home connection), download per paper is
  short (~2–10 s); MinerU convert per paper is longer (~30–120 s). Download latency
  may already be negligible relative to convert — **measure before prioritizing this**.
- Requires: same per-paper producer-consumer structure as above.

### Tier 2 — Conditionally safe, moderate impact

**`convert(N)` ∥ `convert(N+1)`** — GPU vs GPU

- Safe **only if** VRAM headroom permits two full MinerU model loads.
- On RTX 5060 8 GB: almost certainly unsafe (OOM risk). Do not attempt until VRAM
  per-convert is measured (Q4).
- On RTX 5060 Ti 16 GB: possibly safe if one convert peaks ≤ 7 GB. Must measure.
- If VRAM permits, this doubles GPU throughput but adds scheduling complexity and
  makes VRAM a hard runtime constraint.
- **Block on Q4 measurement before considering.**

### Not applicable

`batch_filter(N)` ∥ `batch_filter(N+1)` — two concurrent DeepSeek calls. Technically
safe locally (network-only), but multiplies API cost and rate-limit pressure. Out of
scope for GPU bubble.

---

## Summary

The bubble structure is:

```
[download phase: network, GPU=0%]
  for each PDF:
    [MinerU convert: GPU=80%]
    [extract_and_clean: IO, GPU=0%]
[batch_filter phase: network, GPU=0%]
```

The two largest idle periods flanking the ingest batch (download prefix and
batch_filter suffix) plus the intra-ingest `extract_and_clean` micro-idles are all
caused by the same root: **three strictly-serial batch phases with no cross-paper
pipelining**.

The single highest-value, zero-risk intervention is overlapping `batch_filter(N)` with
`convert(N+1)` — pure network/GPU split, no VRAM risk, no subprocess collision risk.
That alone would collapse the effective per-paper latency to `max(convert, filter)`
from `convert + filter`.

`convert ∥ convert` is the only option that could eliminate the intra-ingest GPU dips
entirely, but it is gated on the Q4 VRAM measurement.
