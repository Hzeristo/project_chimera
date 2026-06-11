"""Tests for ChimeraAgent.archive_segment / unarchive_segment."""
from __future__ import annotations

import json
import pytest

from src.crucible.core.schemas import ChatMessage
from src.oligo.core.agent import ChimeraAgent


class _PassthroughClient:
    async def generate_raw_text(self, messages):
        return ""


@pytest.fixture
def agent():
    a = ChimeraAgent(
        raw_messages=[
            {"role": "user", "content": "msg1"},
            {"role": "assistant", "content": "msg2"},
            {"role": "user", "content": "msg3"},
        ],
        system_core="test",
        skill_override=None,
        llm_client=_PassthroughClient(),
    )
    return a


def test_archive_reduces_length_by_segment_plus_tombstone(agent, tmp_path, monkeypatch):
    import src.crucible.core.platform as plat
    monkeypatch.setattr(plat, "get_chimera_root", lambda: tmp_path)

    original_len = len(agent.messages)  # 4: system + 3 user/assistant
    agent.archive_segment(1, 3, "superseded proposal")
    # replaced 2 messages with 1 tombstone → net -1
    assert len(agent.messages) == original_len - 2 + 1


def test_archive_writes_audit_log(agent, tmp_path, monkeypatch):
    import src.crucible.core.platform as plat
    monkeypatch.setattr(plat, "get_chimera_root", lambda: tmp_path)

    agent.archive_segment(1, 3, "reason")

    log_dir = tmp_path / "archive_log"
    logs = list(log_dir.glob("*.jsonl"))
    assert len(logs) == 1
    lines = logs[0].read_text(encoding="utf-8").strip().splitlines()
    assert len(lines) == 2
    assert json.loads(lines[0])["content"] == "msg1"
    assert json.loads(lines[1])["content"] == "msg2"


def test_tombstone_format(agent, tmp_path, monkeypatch):
    import src.crucible.core.platform as plat
    monkeypatch.setattr(plat, "get_chimera_root", lambda: tmp_path)

    agent.archive_segment(1, 2, "old idea")
    tombstone = agent.messages[1]
    assert tombstone.content.startswith("[ARCHIVED]")
    assert "superseded" in tombstone.content


def test_unarchive_restores_original(agent, tmp_path, monkeypatch):
    import src.crucible.core.platform as plat
    monkeypatch.setattr(plat, "get_chimera_root", lambda: tmp_path)

    original_msgs = [m.model_copy(deep=True) for m in agent.messages]
    agent.archive_segment(1, 3, "reason")
    agent.unarchive_segment(1)

    assert len(agent.messages) == len(original_msgs)
    for a, b in zip(agent.messages, original_msgs):
        assert a.content == b.content


def test_archive_protects_slot_zero(agent, tmp_path, monkeypatch):
    import src.crucible.core.platform as plat
    monkeypatch.setattr(plat, "get_chimera_root", lambda: tmp_path)

    with pytest.raises(ValueError, match="system slot"):
        agent.archive_segment(0, 2, "bad")
