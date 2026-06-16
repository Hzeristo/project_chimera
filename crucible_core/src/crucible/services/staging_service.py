"""Staging protocol: create / promote / reject candidate K/T/I/D nodes."""

from __future__ import annotations

import re
from datetime import datetime
from pathlib import Path

import yaml

_TYPE_DEST = {"thought": "Thoughts", "insight": "Insight", "decision": "Decision"}
_TYPE_EDGES: dict[str, dict[str, list]] = {
    "thought":  {"derives_from": [], "supersedes": [], "contradicts": [], "dead_ends": []},
    "insight":  {"synthesizes": [], "verified_with": [], "derives_from": [], "supersedes": [], "contradicts": []},
    "decision": {"depends_on": [], "dead_ends": [], "supersedes": [], "contradicts": []},
}
_SLUG_RE = re.compile(r'[\\/:*?"<>|\s]+')


class StagingService:
    def __init__(self, staging_dir: Path, vault_root: Path) -> None:
        self.staging_dir = staging_dir
        self.vault_root = vault_root
        staging_dir.mkdir(parents=True, exist_ok=True)

    def create_staging_node(
        self,
        type: str,
        title: str,
        body: str,
        edges: dict | None = None,
    ) -> Path:
        node_type = type.lower()
        if node_type not in _TYPE_DEST:
            raise ValueError(f"Unknown node type: {type!r}")
        graph_edges = dict(_TYPE_EDGES[node_type])
        if edges:
            for k, v in edges.items():
                if k in graph_edges:
                    graph_edges[k] = v
        fm = {
            "type": node_type,
            "status": "PENDING_REVIEW",
            "title": title,
            "created_at": datetime.now().strftime("%Y-%m-%d"),
            "tags": [node_type],
            "graph_edges": graph_edges,
        }
        slug = _SLUG_RE.sub("_", title)[:60].rstrip("_")
        stamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        path = self.staging_dir / f"{stamp}-{slug}.md"
        content = f"---\n{yaml.dump(fm, allow_unicode=True, default_flow_style=False)}---\n\n# {title}\n\n{body}\n"
        path.write_text(content, encoding="utf-8")
        return path

    def promote_node(self, staging_path: Path) -> Path:
        text = staging_path.read_text(encoding="utf-8")
        _, fm_raw, body = text.split("---", 2)
        fm = yaml.safe_load(fm_raw)
        node_type = fm.get("type", "thought")
        dest_sub = _TYPE_DEST.get(node_type, "Thoughts")
        fm["status"] = "active"
        slug = _SLUG_RE.sub("_", fm.get("title", "untitled"))[:60].rstrip("_")
        dest_dir = self.vault_root / dest_sub
        dest_dir.mkdir(parents=True, exist_ok=True)
        dest_path = dest_dir / f"{slug}.md"
        dest_path.write_text(
            f"---\n{yaml.dump(fm, allow_unicode=True, default_flow_style=False)}---\n{body}",
            encoding="utf-8",
        )
        staging_path.unlink()
        return dest_path

    def reject_node(self, staging_path: Path) -> None:
        staging_path.unlink()
