# crucible_core/tests/oligo/test_lifecycle_after_long_task.py
"""A.6: lifecycle integrity after a long-task AWAITING_TASK turn."""
from __future__ import annotations

import asyncio
import json
from datetime import datetime
from pathlib import Path
from typing import Any

import pytest

from src.crucible.core.schemas import TaskEventType
from src.crucible.services.task_service import TaskService, TaskStatus, Task, set_task_service
from src.oligo.core.agent import ChimeraAgent
from src.oligo.core.schemas import AgentPhase


_FAKE_TASK_ID = "deadbeef"
_REAL_RESULT = "3 papers found: arXiv:2401.00001, arXiv:2401.00002, arXiv:2401.00003"


def _make_smart_client(probe_with_tool: str, final_response: str) -> Any:
    class _Client:
        calls: list = []
        probe_call_count = 0
        final_call_count = 0

        async def generate_raw_text(self, messages: list[dict[str, Any]]) -> str:
            self.calls.append(list(messages))
            sys = (messages[0].get("content", "") or "") if messages else ""
            full = " ".join(m.get("content", "") for m in messages)
            if "Chimera OS local router" in sys:
                self.probe_call_count += 1
                if "[SYSTEM TOOL RESULTS]" in full:
                    return "<PASS>"
                return probe_with_tool
            self.final_call_count += 1
            return final_response

    return _Client()


def _mk_completed_task_file(tasks_dir: Path, task_id: str) -> None:
    tasks_dir.mkdir(parents=True, exist_ok=True)
    task = Task(
        id=task_id,
        type="arxiv_fetch",
        status=TaskStatus.PENDING,
        progress=0.0,
        result=None,
        error=None,
        created_at=datetime.now().isoformat(),
        completed_at=None,
    )
    (tasks_dir / f"{task_id}.json").write_text(
        json.dumps(task.model_dump()), encoding="utf-8"
    )


async def test_history_contains_real_result_after_long_task(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """After AWAITING_TASK, self.messages last tool-result message has real output."""
    svc = TaskService(tmp_path / "tasks")
    _mk_completed_task_file(tmp_path / "tasks", _FAKE_TASK_ID)
    set_task_service(svc)

    # Monkeypatch _execute_tool_with_deadline so arxiv_miner returns "Task started: deadbeef"
    async def _fake_execute(self_agent: Any, tool_name: str, args: dict) -> tuple[str, None]:
        return f"Task started: {_FAKE_TASK_ID}", None

    monkeypatch.setattr(ChimeraAgent, "_execute_tool_with_deadline", _fake_execute)

    try:
        # arxiv_miner is a real long_running=True tool in the registry
        probe = '<CMD:arxiv_miner({"query": "agent memory", "max_results": 5})>'
        client = _make_smart_client(probe, "Summary of papers.")

        agent = ChimeraAgent(
            raw_messages=[{"role": "user", "content": "爬取论文 agent memory 5篇"}],
            system_core="You are BB.",
            skill_override=None,
            llm_client=client,
            max_turns=5,
            allowed_tools=["arxiv_miner"],
        )

        async def _emit_completed() -> None:
            await asyncio.sleep(0.05)
            await svc.emit_completed(_FAKE_TASK_ID, summary=_REAL_RESULT)

        asyncio.create_task(_emit_completed())
        frames: list[str] = [chunk async for chunk in agent.run_theater()]

        # HSC-1: message history has real result, not "Task started:"
        tool_result_msgs = [
            m for m in agent.messages
            if m.role == "user" and "[SYSTEM TOOL RESULTS]" in (m.content or "")
        ]
        assert tool_result_msgs, "expected a tool result message in history"
        assert _REAL_RESULT in tool_result_msgs[-1].content
        assert "Task started:" not in tool_result_msgs[-1].content

        # HSC-2: AWAITING_TASK phase event in SSE stream
        assert '"phase": "awaiting_task"' in "".join(frames)

        # HSC-3: no subscription leak
        assert len(svc._subscribers) == 0

    finally:
        set_task_service(None)  # type: ignore[arg-type]


async def test_no_subscription_leak_on_fast_task(tmp_path: Path) -> None:
    """Fast task (already COMPLETED) resolves via pre-subscribe check, no subscription leak."""
    svc = TaskService(tmp_path / "tasks")
    task = Task(
        id=_FAKE_TASK_ID,
        type="arxiv_fetch",
        status=TaskStatus.COMPLETED,
        progress=1.0,
        result=_REAL_RESULT,
        error=None,
        created_at=datetime.now().isoformat(),
        completed_at=datetime.now().isoformat(),
    )
    (tmp_path / "tasks").mkdir(parents=True, exist_ok=True)
    (tmp_path / "tasks" / f"{_FAKE_TASK_ID}.json").write_text(
        json.dumps(task.model_dump()), encoding="utf-8"
    )

    event = await svc.await_completion(_FAKE_TASK_ID, timeout_s=1.0)
    assert event.event_type == TaskEventType.COMPLETED
    assert event.message == _REAL_RESULT
    assert len(svc._subscribers) == 0

