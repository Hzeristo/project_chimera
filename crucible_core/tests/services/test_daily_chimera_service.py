# crucible_core/tests/services/test_daily_chimera_service.py
"""Unit tests for daily_chimera_service helpers."""

from __future__ import annotations

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
