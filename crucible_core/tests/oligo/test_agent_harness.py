"""Manual end-to-end smoke harness for ChimeraAgent (no LLM calls)."""
from __future__ import annotations

from typing import Any, AsyncGenerator

import pytest

from src.oligo.core.agent import ChimeraAgent
from src.oligo.tools.vault_tools import set_vault_adapter


class _HarnessVault:
    async def search_notes(self, query: str, top_k: int = 3) -> str:
        return f"[HarnessVault] snippets for {query!r}"

    async def search_by_attribute(self, key: str, value: str, top_k: int = 5) -> str:
        return f"[HarnessVault] attr {key}={value!r}"

    async def query_graph(
        self,
        node_type: str | None = None,
        link_pattern: str | None = None,
        max_depth: int = 2,
    ) -> list[dict[str, Any]]:
        return []

    def read_file(self, path: str) -> str:
        raise FileNotFoundError(path)


class _HarnessMockLLMClient:
    async def generate_raw_text(self, messages: list[dict[str, str]]) -> str:
        full_conv = " ".join(m.get("content", "") for m in messages)
        sys0 = messages[0].get("content", "") if messages else ""
        if "Chimera OS local router" in sys0:
            if "[SYSTEM TOOL RESULTS]" in full_conv:
                return "Senpai, based on the vault: Titans is flawed. That is all."
            return '<CMD:search_vault({"query": "Titans"})> Searching...'
        if "[SYSTEM TOOL RESULTS]" in full_conv:
            return "Senpai, based on the vault: Titans is flawed. That is all."
        return "Hello from BB."

    async def stream_generate(
        self, messages: list[dict[str, str]]
    ) -> AsyncGenerator[str, None]:
        import asyncio
        for c in "Senpai, Titans is flawed. That is all.":
            yield c
            await asyncio.sleep(0)

    async def generate_structured_data_async(self, system_prompt: str, user_prompt: str, response_model: type) -> Any:
        raise NotImplementedError


@pytest.mark.asyncio
async def test_theater_smoke_with_tool_call():
    set_vault_adapter(_HarnessVault())
    try:
        agent = ChimeraAgent(
            raw_messages=[{"role": "user", "content": "Fetch Titans."}],
            system_core="You are BB, a dramatic waifu persona.",
            skill_override=None,
            llm_client=_HarnessMockLLMClient(),
        )
        chunks: list[str] = []
        async for chunk in agent.run_theater():
            chunks.append(chunk)
        output = "".join(chunks)
        assert "bb-stream-done" in output or len(chunks) > 0
    finally:
        set_vault_adapter(None)
