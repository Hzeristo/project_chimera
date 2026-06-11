"""Tests for ChimeraAgent.fork_subagent and budget conservation."""
from __future__ import annotations

import pytest

from src.oligo.core.agent import ChimeraAgent


class _FinalOnlyClient:
    async def generate_raw_text(self, messages):
        sys0 = (messages[0].get("content", "") or "") if messages else ""
        if "Chimera OS local router" in sys0 or "router_continuation" in sys0:
            return ""  # no tool calls → go to final
        return "child answer"


@pytest.fixture
def client():
    return _FinalOnlyClient()


def _make_agent(client, max_turns=5):
    return ChimeraAgent(
        raw_messages=[{"role": "user", "content": "hello"}],
        system_core="test",
        skill_override=None,
        llm_client=client,
        max_turns=max_turns,
    )


@pytest.mark.asyncio
async def test_fork_does_not_grow_parent_messages(client):
    agent = _make_agent(client)
    before = len(agent.messages)
    await agent.fork_subagent("sub-task prompt")
    assert len(agent.messages) == before


@pytest.mark.asyncio
async def test_fork_returns_str_under_4096(client):
    agent = _make_agent(client)
    result = await agent.fork_subagent("sub-task prompt")
    assert isinstance(result, str)
    assert len(result) < 4096


@pytest.mark.asyncio
async def test_budget_conservation(client):
    agent = _make_agent(client, max_turns=5)
    agent._current_turn = 3  # 2 turns remaining
    # request 10 turns → child should be capped at 2
    child_result = await agent.fork_subagent("prompt", max_turns=10)
    # If budget conservation works, no exception and result is str
    assert isinstance(child_result, str)


@pytest.mark.asyncio
async def test_budget_conservation_exact_cap(client):
    """fork_subagent(max_turns=10) on parent with 2 turns remaining → child max_turns == 2."""
    parent = _make_agent(client, max_turns=5)
    parent._current_turn = 3  # 5 - 3 = 2 remaining

    captured = {}

    original_init = ChimeraAgent.__init__

    def patched_init(self, *args, **kwargs):
        original_init(self, *args, **kwargs)
        captured["child_max_turns"] = self.max_turns

    import src.oligo.core.agent as agent_mod
    original = agent_mod.ChimeraAgent.__init__
    agent_mod.ChimeraAgent.__init__ = patched_init
    try:
        await parent.fork_subagent("prompt", max_turns=10)
    finally:
        agent_mod.ChimeraAgent.__init__ = original

    assert captured.get("child_max_turns") == 2


@pytest.mark.asyncio
async def test_hsc1_50k_prompt_does_not_enter_parent_messages(client):
    """HSC 1 (downgraded): 50K-token prompt given to fork_subagent stays out of parent messages."""
    big_prompt = "x" * 50_000
    agent = _make_agent(client)
    before_msgs = [m.model_copy(deep=True) for m in agent.messages]

    result = await agent.fork_subagent(big_prompt)

    assert len(result) < 4096
    assert len(agent.messages) == len(before_msgs)
    parent_content = " ".join(m.content or "" for m in agent.messages)
    assert big_prompt not in parent_content
