# crucible_core/tests/oligo/test_theater_stream.py
"""FC.2a: artifact aggregation + bb-message-artifacts SSE emit."""
from __future__ import annotations

import json
from typing import Any

from src.crucible.core.schemas import Artifact, ToolOutput
from src.oligo.core.agent import ChimeraAgent
from src.oligo.tools.vault_tools import set_vault_adapter


class _SmartMockClient:
    """Mock that returns PASS once tool results have landed, so theater exits to Final.

    Distinct from ``conftest.MockLLMClient`` which always returns the same
    probe_response and exhausts max_turns when tools are involved.
    """

    def __init__(
        self,
        probe_response: str,
        final_response: str = "Final answer.",
    ) -> None:
        self.probe_response = probe_response
        self.final_response = final_response
        self.probe_call_count = 0
        self.final_call_count = 0

    async def generate_raw_text(self, messages: list[dict[str, Any]]) -> str:
        sys_content = (messages[0].get("content", "") or "") if messages else ""
        full_conv = " ".join(m.get("content", "") for m in messages)
        if "Chimera OS local router" in sys_content:
            self.probe_call_count += 1
            if "[SYSTEM TOOL RESULTS]" in full_conv:
                return "<PASS>"
            return self.probe_response
        self.final_call_count += 1
        return self.final_response


class _GraphFirstAdapter:
    """Adapter whose query_graph returns one row per call (cycling), so multi-turn yields distinct artifacts."""

    def __init__(self) -> None:
        self._calls = 0

    async def search_notes(self, query: str, top_k: int = 3) -> str:
        return f"[MockVault] {query}"

    async def search_by_attribute(self, key, value, top_k=5):
        return f"[MockVault] {key}={value}"

    async def query_graph(self, node_type=None, link_pattern=None, max_depth=2):
        self._calls += 1
        idx = self._calls
        return [
            {
                "title": f"Note{idx}",
                "path": f"/vault/Note{idx}.md",
                "type": "thought",
                "links": [],
            }
        ]

    def read_file(self, path: str) -> str:
        return f"# {path}"


def _parse_artifacts_event(frame: str) -> list[dict] | None:
    """Return the artifacts list if ``frame`` is the bb-message-artifacts event, else None."""
    if "event: bb-message-artifacts" not in frame:
        return None
    for line in frame.splitlines():
        if line.startswith("data: "):
            payload = json.loads(line[len("data: ") :])
            artifacts = payload.get("artifacts")
            assert isinstance(artifacts, list)
            return artifacts
    raise AssertionError("bb-message-artifacts frame had no data line")


async def test_artifacts_event_emitted_once_with_dedup(mock_client):
    """Two-turn run with same vault row deduplicates and emits exactly one frame."""

    class _StaticAdapter(_GraphFirstAdapter):
        async def query_graph(self, node_type=None, link_pattern=None, max_depth=2):
            return [
                {
                    "title": "DupNote",
                    "path": "/vault/DupNote.md",
                    "type": "thought",
                    "links": [],
                }
            ]

    client = _SmartMockClient(
        probe_response='<CMD:obsidian_graph_query({"node_type": "thought"})>',
        final_response="Done.",
    )

    set_vault_adapter(_StaticAdapter())
    try:
        agent = ChimeraAgent(
            raw_messages=[{"role": "user", "content": "graph"}],
            system_core="You are BB.",
            skill_override=None,
            llm_client=client,
            max_turns=3,
            allowed_tools=["obsidian_graph_query"],
        )
        frames = [chunk async for chunk in agent.run_theater()]
    finally:
        set_vault_adapter(None)

    artifact_events = [_parse_artifacts_event(f) for f in frames]
    artifact_events = [e for e in artifact_events if e is not None]
    assert len(artifact_events) == 1
    artifacts = artifact_events[0]
    assert len(artifacts) == 1
    assert artifacts[0]["kind"] == "vault_note"
    assert artifacts[0]["path"] == "/vault/DupNote.md"


async def test_no_artifacts_event_when_empty(mock_client):
    """Run with no tool calls produces no bb-message-artifacts frame."""
    client = _SmartMockClient(probe_response="<PASS>", final_response="Hi.")

    agent = ChimeraAgent(
        raw_messages=[{"role": "user", "content": "ping"}],
        system_core="You are BB.",
        skill_override=None,
        llm_client=client,
        max_turns=3,
        allowed_tools=None,
    )
    frames = [chunk async for chunk in agent.run_theater()]
    assert all(_parse_artifacts_event(f) is None for f in frames)


async def test_no_artifacts_event_when_only_search_vault(mock_client):
    """search_vault produces ToolOutput.artifacts=None → no frame."""
    client = _SmartMockClient(
        probe_response='<CMD:search_vault({"query": "RAG"})>',
        final_response="Done.",
    )

    set_vault_adapter(_GraphFirstAdapter())
    try:
        agent = ChimeraAgent(
            raw_messages=[{"role": "user", "content": "search"}],
            system_core="You are BB.",
            skill_override=None,
            llm_client=client,
            max_turns=3,
            allowed_tools=["search_vault"],
        )
        frames = [chunk async for chunk in agent.run_theater()]
    finally:
        set_vault_adapter(None)

    assert all(_parse_artifacts_event(f) is None for f in frames)


async def test_artifacts_event_after_final_chunks_no_python_done(mock_client):
    """Success-path: artifacts frame follows the last bb-stream-chunk; Python emits no bb-stream-done.

    Per FC.0 cross-finding 4: Python only emits bb-stream-done on error;
    Rust emits the success "DONE". Artifacts ride before Rust's DONE.
    """
    client = _SmartMockClient(
        probe_response='<CMD:obsidian_graph_query({"node_type": "thought"})>',
        final_response="Result text.",
    )

    set_vault_adapter(_GraphFirstAdapter())
    try:
        agent = ChimeraAgent(
            raw_messages=[{"role": "user", "content": "graph"}],
            system_core="You are BB.",
            skill_override=None,
            llm_client=client,
            max_turns=3,
            allowed_tools=["obsidian_graph_query"],
        )
        frames = [chunk async for chunk in agent.run_theater()]
    finally:
        set_vault_adapter(None)

    artifact_idx = next(
        (i for i, f in enumerate(frames) if _parse_artifacts_event(f) is not None),
        None,
    )
    assert artifact_idx is not None, "expected one bb-message-artifacts frame"
    chunk_idxs = [
        i for i, f in enumerate(frames) if "event: bb-stream-chunk" in f
    ]
    assert chunk_idxs, "expected at least one bb-stream-chunk frame"
    assert artifact_idx > max(chunk_idxs)
    assert not any("event: bb-stream-done" in f for f in frames)


async def test_session_artifacts_never_in_messages(mock_client):
    """After a tool turn, self.messages must not carry any artifact path or kind."""
    client = _SmartMockClient(
        probe_response='<CMD:obsidian_graph_query({"node_type": "thought"})>',
        final_response="Done.",
    )

    set_vault_adapter(_GraphFirstAdapter())
    try:
        agent = ChimeraAgent(
            raw_messages=[{"role": "user", "content": "graph"}],
            system_core="You are BB.",
            skill_override=None,
            llm_client=client,
            max_turns=3,
            allowed_tools=["obsidian_graph_query"],
        )
        async for _ in agent.run_theater():
            pass
    finally:
        set_vault_adapter(None)

    blob = " ".join(m.content for m in agent.messages)
    assert "/vault/Note1.md" not in blob
    assert "vault_note" not in blob
    # accumulator populated though
    assert agent._session_artifacts
    assert agent._session_artifacts[0].path == "/vault/Note1.md"


def test_accumulate_artifacts_dedup_by_kind_and_path(mock_client):
    """Sync unit test: _accumulate_artifacts dedups by (kind, path) across calls."""
    from src.crucible.core.schemas import ExecutedToolResult, ToolCallStatus

    agent = ChimeraAgent(
        raw_messages=[{"role": "user", "content": "x"}],
        system_core="x",
        skill_override=None,
        llm_client=mock_client(),
    )
    art_a = Artifact(kind="vault_note", path="/a.md", metadata={"v": 1})
    art_a_again = Artifact(kind="vault_note", path="/a.md", metadata={"v": 2})
    art_b = Artifact(kind="vault_note", path="/b.md")
    er = ExecutedToolResult(
        call_id="c1",
        tool_name="obsidian_graph_query",
        args={},
        status=ToolCallStatus.SUCCESS,
        raw_result="ok",
        artifacts=[art_a, art_a_again, art_b],
    )
    agent._accumulate_artifacts([er])
    agent._accumulate_artifacts([er])  # second turn identical → no growth
    paths = [a.path for a in agent._session_artifacts]
    assert paths == ["/a.md", "/b.md"]

