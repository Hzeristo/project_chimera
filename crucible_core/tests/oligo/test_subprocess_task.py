"""Tests for TaskService.run_subprocess_task and _extract_failure_lesson."""
from __future__ import annotations

import asyncio
import sys
from pathlib import Path
from unittest.mock import AsyncMock

import pytest

from src.crucible.services.task_service import TaskService, _extract_failure_lesson


@pytest.fixture
def svc(tmp_path):
    return TaskService(tmp_path / "tasks")


def _task(svc):
    task_id = svc.create_task("test")
    return task_id


@pytest.mark.asyncio
async def test_success_exit_emits_completed(svc):
    task_id = _task(svc)
    cmd = [sys.executable, "-c", "print('hello')"]
    await svc.run_subprocess_task(task_id, cmd)
    t = svc.get_task_status(task_id)
    assert t.status.value == "completed"


@pytest.mark.asyncio
async def test_failure_exit_emits_failed_with_exit_code(svc):
    task_id = _task(svc)
    cmd = [sys.executable, "-c", "import sys; sys.exit(1)"]
    await svc.run_subprocess_task(task_id, cmd)
    t = svc.get_task_status(task_id)
    assert t.status.value == "failed"
    assert "exit 1" in t.error


@pytest.mark.asyncio
async def test_failure_with_wash_client_includes_lesson(svc):
    task_id = _task(svc)
    mock_wash = AsyncMock()
    mock_wash.generate_raw_text = AsyncMock(return_value="Wrong tool chosen first.")
    cmd = [sys.executable, "-c", "import sys; print('fail log'); sys.exit(2)"]
    await svc.run_subprocess_task(task_id, cmd, wash_client=mock_wash)
    t = svc.get_task_status(task_id)
    assert t.status.value == "failed"
    assert "lesson:" in t.error
    assert "Wrong tool chosen first." in t.error


@pytest.mark.asyncio
async def test_stall_timeout_emits_failed(svc):
    task_id = _task(svc)
    # Python process that writes nothing then hangs — we use a short timeout
    cmd = [sys.executable, "-c", "import time; time.sleep(60)"]
    await svc.run_subprocess_task(task_id, cmd, stall_timeout_s=0.3)
    t = svc.get_task_status(task_id)
    assert t.status.value == "failed"
    assert "stall" in t.error


@pytest.mark.asyncio
async def test_lesson_extraction_failure_degrades_gracefully():
    mock_wash = AsyncMock()
    mock_wash.generate_raw_text = AsyncMock(side_effect=RuntimeError("boom"))
    result = await _extract_failure_lesson("some log", mock_wash)
    assert result == "lesson extraction failed"


@pytest.mark.asyncio
async def test_lesson_in_failed_error_field(svc):
    task_id = _task(svc)
    mock_wash = AsyncMock()
    mock_wash.generate_raw_text = AsyncMock(return_value="Bad import at start.")
    cmd = [sys.executable, "-c", "import sys; sys.exit(3)"]
    await svc.run_subprocess_task(task_id, cmd, wash_client=mock_wash)
    t = svc.get_task_status(task_id)
    assert "exit 3" in t.error
    assert "Bad import at start." in t.error
