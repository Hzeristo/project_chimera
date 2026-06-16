"""vault_query tool — ripgrep vault frontmatter, no index, no daemon."""

from __future__ import annotations

import asyncio
from pathlib import Path
from typing import Any

import yaml

from src.crucible.core.config import get_config


def _parse_frontmatter(text: str) -> dict:
    if not text.startswith("---"):
        return {}
    parts = text.split("---", 2)
    if len(parts) < 3:
        return {}
    try:
        return yaml.safe_load(parts[1]) or {}
    except Exception:
        return {}


async def vault_query(
    type: str | None = None,
    status: str | None = None,
    linked_to: str | None = None,
    **kwargs: Any,
) -> str:
    if not any([type, status, linked_to]):
        return "[Tool Error]: vault_query requires at least one of: type, status, linked_to."

    settings = get_config()
    try:
        vault_root = settings.require_path("vault_root")
    except Exception as e:
        return f"[Tool Error]: vault_root not configured ({e})."

    rg_pattern = f"^type: {type}$" if type else (f"^status: {status}$" if status else str(linked_to))

    try:
        proc = await asyncio.create_subprocess_exec(
            "rg", "--files-with-matches", "-m", "1", "--glob", "*.md",
            rg_pattern, str(vault_root),
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.DEVNULL,
        )
        stdout, _ = await asyncio.wait_for(proc.communicate(), timeout=10.0)
    except FileNotFoundError:
        return "[Tool Error]: ripgrep (rg) not found in PATH."
    except asyncio.TimeoutError:
        return "[Tool Error]: vault_query timed out (>10s)."

    paths = [Path(p) for p in stdout.decode().splitlines() if p.strip()]
    results: list[str] = []

    for path in paths:
        try:
            fm = _parse_frontmatter(path.read_text(encoding="utf-8", errors="replace"))
        except Exception:
            continue

        if type and fm.get("type") != type:
            continue
        if status and fm.get("status") != status:
            continue
        if linked_to and linked_to not in str(fm.get("graph_edges", {})):
            continue

        title = fm.get("title") or fm.get("short_moniker") or fm.get("arxiv_id") or path.stem
        excerpt = f"type={fm.get('type', '?')}  status={fm.get('status', '?')}"
        results.append(f"- {title}\n  {path}\n  {excerpt}")

    if not results:
        return "[vault_query] No matching notes."
    return f"[vault_query] {len(results)} match(es):\n" + "\n".join(results)
