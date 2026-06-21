# crucible_core/tests/services/test_batch_filter_workflow.py
"""Parity test for the sync/async batch-filter paths.

run_batch_filter (sync) and filter_queue_worker (async) share the
verdict-recording logic via _record_verdict_stats. This drives both over the
same papers and asserts they produce identical verdict-driven stats, and that
stats.total == input_count for each path.
"""
from __future__ import annotations

import asyncio
from pathlib import Path

import pytest

import src.crucible.services.batch_filter_workflow as bfw
from src.crucible.core.schemas import (
    BatchFilterStats,
    Paper,
    PaperAnalysisResult,
    VerdictDecision,
)

_VERDICTS = [
    VerdictDecision.MUST_READ,
    VerdictDecision.SKIM,
    VerdictDecision.REJECT,
]


def _verdict_for(stem: str) -> VerdictDecision:
    return _VERDICTS[int(stem.removeprefix("paper")) % 3]


class _FakeLoader:
    def load_paper(self, md_path: Path) -> Paper:
        stem = md_path.stem
        return Paper(
            id=stem,
            title=f"Title {stem}",
            content_path=md_path,
            raw_text=f"body {stem}",
        )


class _FakeEngine:
    def __init__(self, **_kw: object) -> None:
        pass

    def evaluate_paper(self, paper: Paper) -> PaperAnalysisResult:
        return PaperAnalysisResult(
            verdict=_verdict_for(paper.id),
            short_moniker=f"Mon{paper.id[-1]}",
            score=7,
            novelty_delta="delta",
            mechanism_summary="mech",
        )


class _FakeWriter:
    def __init__(self, **_kw: object) -> None:
        pass

    def write_knowledge_node(self, paper: Paper, result: PaperAnalysisResult) -> Path:
        return Path(f"{paper.id}.md")


class _FakeRouter:
    def __init__(self, **_kw: object) -> None:
        pass

    def route_and_cleanup(self, paper: Paper, result: PaperAnalysisResult) -> None:
        pass

    def route_failed_cleanup(self, *, paper_id: str, md_path: Path) -> None:
        pass


class _FakeSettings:
    def ensure_directories(self) -> None:
        pass


@pytest.fixture(autouse=True)
def _patch_deps(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(bfw, "PaperLoader", _FakeLoader)
    monkeypatch.setattr(bfw, "FilterService", _FakeEngine)
    monkeypatch.setattr(bfw, "VaultNoteWriter", _FakeWriter)
    monkeypatch.setattr(bfw, "PaperArchiveAdapter", _FakeRouter)
    monkeypatch.setattr(bfw, "PromptManager", lambda *a, **k: object())
    monkeypatch.setattr(bfw, "build_openai_client", lambda *a, **k: object())


def _make_papers(tmp_path: Path, n: int) -> list[Path]:
    for i in range(n):
        (tmp_path / f"paper{i}.md").write_text(f"# paper {i}\n", encoding="utf-8")
    return sorted(tmp_path.glob("*.md"))


def test_sync_and_async_paths_produce_identical_verdict_stats(tmp_path: Path) -> None:
    n = 6  # 2 each of MUST_READ / SKIM / REJECT
    md_files = _make_papers(tmp_path, n)

    sync_stats = bfw.run_batch_filter(tmp_path, settings=_FakeSettings())

    async def _run_async() -> BatchFilterStats:
        stats = BatchFilterStats(source_dir=tmp_path)
        lock = asyncio.Lock()
        queue: asyncio.Queue[Path | None] = asyncio.Queue()
        for p in md_files:  # same (sorted) order as the sync glob
            queue.put_nowait(p)
        queue.put_nowait(None)  # poison pill
        await bfw.filter_queue_worker(queue, stats, lock, settings=_FakeSettings())
        return stats

    async_stats = asyncio.run(_run_async())

    # total == input_count on both paths (sync sets once, async increments)
    assert sync_stats.total == n
    assert async_stats.total == n
    assert sync_stats.errors == 0
    assert async_stats.errors == 0

    # verdict-driven fields identical across the shared _record_verdict_stats
    assert sync_stats.must_read == async_stats.must_read == 2
    assert sync_stats.skim == async_stats.skim == 2
    assert sync_stats.reject == async_stats.reject == 2
    assert sync_stats.must_read_titles == async_stats.must_read_titles
    assert sync_stats.must_read_items == async_stats.must_read_items
    assert sync_stats.skim_items == async_stats.skim_items
    assert sync_stats.reject_items == async_stats.reject_items
    assert sorted(sync_stats.processed_ids) == sorted(async_stats.processed_ids)
