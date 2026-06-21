# crucible_core/tests/services/test_daily_chimera_service.py
"""Unit tests for daily_chimera_service helpers."""

from __future__ import annotations

import asyncio
from pathlib import Path

from src.crucible.core.schemas import BatchFilterStats, BatchMustReadItem
from src.crucible.services.daily_chimera_service import (
    _collect_all_filtered_lines,
    _collect_must_read_lines,
    _collect_pipeline_artifacts,
)


def _item(**kwargs) -> BatchMustReadItem:
    defaults = dict(score=7, id="", paper_id="", short_moniker="", filename="", title="", novelty="")
    return BatchMustReadItem(**(defaults | kwargs))


def test_collect_uses_short_moniker():
    item = _item(score=8, id="2501.12345", paper_id="2501.12345", short_moniker="FooNet")
    lines = _collect_must_read_lines(BatchFilterStats(must_read=1, must_read_items=[item]))
    assert len(lines) == 1
    assert "FooNet" in lines[0]
    assert "[8/10]" in lines[0]


def test_collect_falls_back_to_title():
    item = _item(score=5, id="2501.99999", paper_id="2501.99999", title="Some Paper Title")
    lines = _collect_must_read_lines(BatchFilterStats(must_read=1, must_read_items=[item]))
    assert "Some Paper Title" in lines[0]
    assert "[5/10]" in lines[0]


def test_collect_empty_stats():
    assert _collect_must_read_lines(BatchFilterStats()) == []


# --- HSC-2 scenario: 0 must_read / 1 skim / 2 reject ---

def _make_stats_0mr_1skim_2reject() -> BatchFilterStats:
    skim = _item(score=6, id="2501.00001", paper_id="2501.00001", short_moniker="SkimNet", filename="2501.00001-SkimNet.md")
    r1 = _item(score=2, id="2501.00002", paper_id="2501.00002", short_moniker="TrashA", filename="")
    r2 = _item(score=1, id="2501.00003", paper_id="2501.00003", short_moniker="TrashB", filename="")
    return BatchFilterStats(
        total=3, must_read=0, skim=1, reject=2,
        skim_items=[skim],
        reject_items=[r1, r2],
    )


def test_all_filtered_lines_lists_all_verdicts():
    stats = _make_stats_0mr_1skim_2reject()
    section = _collect_all_filtered_lines(stats)
    assert "SkimNet" in section
    assert "TrashA" in section
    assert "TrashB" in section
    assert "Skim:" in section
    assert "Reject:" in section
    assert "Must Read:" not in section  # no must_read items


def test_all_filtered_lines_empty_when_no_papers():
    assert _collect_all_filtered_lines(BatchFilterStats()) == ""


def test_collect_pipeline_artifacts_skim_only():
    stats = _make_stats_0mr_1skim_2reject()
    inbox = Path("/vault/inbox")
    artifacts = _collect_pipeline_artifacts(stats, inbox)
    assert len(artifacts) == 1
    assert artifacts[0].metadata["verdict"] == "skim"
    assert "Skim" in artifacts[0].path
    assert "2501.00001" in artifacts[0].path
    assert artifacts[0].path.endswith(".md")


def test_collect_pipeline_artifacts_no_reject_artifacts():
    stats = _make_stats_0mr_1skim_2reject()
    artifacts = _collect_pipeline_artifacts(stats, Path("/vault/inbox"))
    assert all(a.metadata["verdict"] != "reject" for a in artifacts)


async def test_convert_worker_processes_serially() -> None:
    """convert_queue_worker must never have two MinerU subprocesses concurrent.

    Verifies the single-worker contract: items are processed one-at-a-time.
    Concurrency level must never exceed 1.
    """
    from unittest.mock import MagicMock, patch

    import src.crucible.ports.ingest.mineru_pipeline as mp

    call_log: list[str] = []
    active: list[int] = [0]

    async def fake_convert(pdf_path: Path) -> Path:
        active[0] += 1
        call_log.append(f"start:{active[0]}")
        assert active[0] == 1, f"Two converts ran concurrently! active={active[0]}"
        await asyncio.sleep(0)
        active[0] -= 1
        call_log.append(f"end:{pdf_path.stem}")
        return Path(f"/fake/{pdf_path.stem}.md")

    pdf_queue: asyncio.Queue[Path | None] = asyncio.Queue()
    md_queue: asyncio.Queue[Path | None] = asyncio.Queue()
    raw_dir = Path("/fake/raw")
    clean_dir = Path("/fake/clean")

    pdfs = [Path(f"/fake/{i}.pdf") for i in range(4)]
    for p in pdfs:
        pdf_queue.put_nowait(p)
    pdf_queue.put_nowait(None)

    with (
        patch.object(mp, "MineruClient") as MockClient,
        patch.object(mp, "PaperLoader") as MockLoader,
        patch("shutil.rmtree"),
    ):
        mock_client = MagicMock()
        MockClient.return_value = mock_client

        mock_loader = MagicMock()
        MockLoader.return_value = mock_loader

        original_to_thread = asyncio.to_thread

        async def patched_to_thread(fn, *args, **kwargs):
            if fn is mock_client.convert:
                return await fake_convert(args[0])
            if fn is mock_loader.extract_and_clean:
                stem = kwargs.get("paper_stem") or args[0]
                return clean_dir / f"{stem}.md"
            return await original_to_thread(fn, *args, **kwargs)

        with patch("asyncio.to_thread", side_effect=patched_to_thread):
            count = await mp.convert_queue_worker(pdf_queue, md_queue, raw_dir, clean_dir)

    assert count == 4
    # Drain all md_paths then confirm sentinel is last
    items: list[Path | None] = []
    while True:
        item = await md_queue.get()
        items.append(item)
        if item is None:
            break
    assert items[-1] is None
    assert len(items) == 5  # 4 md_paths + 1 sentinel
    start_events = [e for e in call_log if e.startswith("start:")]
    assert all(e == "start:1" for e in start_events), f"Saw concurrent converts: {call_log}"
