# crucible_core/tests/oligo/test_vault_query.py
"""Tests for vault_query: frontmatter parse + filter logic + latency smoke."""

from __future__ import annotations

import asyncio
import time
from pathlib import Path
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from src.oligo.tools.vault_query import _parse_frontmatter, vault_query


# --- Unit: _parse_frontmatter ---

def test_parse_frontmatter_valid():
    text = "---\ntype: knowledge\nstatus: unverified\ntitle: FooNet\n---\n\n# Body"
    fm = _parse_frontmatter(text)
    assert fm["type"] == "knowledge"
    assert fm["title"] == "FooNet"


def test_parse_frontmatter_no_fm():
    assert _parse_frontmatter("# No frontmatter") == {}


def test_parse_frontmatter_malformed():
    assert _parse_frontmatter("---\n: bad: yaml: [\n---\n") == {}


# --- Integration: filter + rg subprocess (mocked) ---

def _make_note(tmp_path: Path, name: str, fm: dict, body: str = "") -> Path:
    import yaml
    p = tmp_path / name
    p.write_text(f"---\n{yaml.dump(fm)}---\n\n{body}", encoding="utf-8")
    return p


@pytest.fixture
def vault_dir(tmp_path):
    _make_note(tmp_path, "k1.md", {"type": "knowledge", "status": "unverified", "title": "PaperA", "graph_edges": {"derives_from": ["2501.00001"]}})
    _make_note(tmp_path, "t1.md", {"type": "thought", "status": "active", "title": "HypB", "graph_edges": {}})
    _make_note(tmp_path, "t2.md", {"type": "thought", "status": "dead_end", "title": "HypC", "graph_edges": {}})
    return tmp_path


def _mock_rg(paths: list[Path]):
    stdout = "\n".join(str(p) for p in paths).encode()
    mock_proc = MagicMock()
    mock_proc.communicate = AsyncMock(return_value=(stdout, b""))

    async def _create(*_a, **_k):
        return mock_proc

    return patch("asyncio.create_subprocess_exec", side_effect=_create)


def _mock_cfg(vault_root: Path):
    cfg = MagicMock()
    cfg.require_path.return_value = vault_root
    return patch("src.oligo.tools.vault_query.get_config", return_value=cfg)


def test_vault_query_type_filter(vault_dir):
    k1 = vault_dir / "k1.md"
    t1 = vault_dir / "t1.md"
    t2 = vault_dir / "t2.md"
    with _mock_cfg(vault_dir), _mock_rg([k1, t1, t2]):
        result = asyncio.run(vault_query(type="knowledge"))
    assert "PaperA" in result
    assert "HypB" not in result


def test_vault_query_status_filter(vault_dir):
    t1 = vault_dir / "t1.md"
    t2 = vault_dir / "t2.md"
    with _mock_cfg(vault_dir), _mock_rg([t1, t2]):
        result = asyncio.run(vault_query(type="thought", status="dead_end"))
    assert "HypC" in result
    assert "HypB" not in result


def test_vault_query_no_filter():
    result = asyncio.run(vault_query())
    assert "[Tool Error]" in result


def test_vault_query_latency_smoke(vault_dir):
    k1 = vault_dir / "k1.md"
    with _mock_cfg(vault_dir), _mock_rg([k1]):
        start = time.perf_counter()
        asyncio.run(vault_query(type="knowledge"))
        elapsed = time.perf_counter() - start
    assert elapsed < 2.0
