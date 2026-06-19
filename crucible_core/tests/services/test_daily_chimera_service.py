# crucible_core/tests/services/test_daily_chimera_service.py
"""Unit tests for daily_chimera_service helpers."""

from __future__ import annotations

import asyncio
from pathlib import Path

from src.crucible.core.schemas import BatchFilterStats, BatchMustReadItem
from src.crucible.services.daily_chimera_service import _collect_must_read_lines


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
