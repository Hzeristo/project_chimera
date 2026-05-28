# EXT.3 — Rich ToolSpec: user_aliases + common_mistakes

- **Commit:** `a79fa0f`
- **Status:** Sealed
- **Files changed:**
  - `crucible_core/src/crucible/core/schemas.py` — added `user_aliases: list[str]` and `common_mistakes: list[str]` fields to `ToolSpec`
  - `crucible_core/src/oligo/core/prompt_composer.py` — `_format_one_tool_verbose` renders aliases and common mistakes after example lines
  - `crucible_core/src/oligo/tools/registry.py` — populated 4 priority tools

## What was done

**Schema additions** (`schemas.py:640–649`): two new optional fields with `default_factory=list`. `extra="forbid"` preserved.

**Verbose renderer** (`prompt_composer.py`): after the existing example lines, appends:
- `Aliases: alias1, alias2, ...` if `user_aliases` non-empty
- `Common mistake: ...` per item if `common_mistakes` non-empty
Compact and micro renderers untouched.

**Priority tool population** (`registry.py`):

| Tool | Aliases | Common mistakes | Examples |
|---|---|---|---|
| `daily_paper_pipeline` | 爬取papers, 爬取论文, fetch papers, run pipeline, daily pipeline, 跑一下日报, 早上的论文, 今天的论文 | Don't call if pipeline already running | `daily_paper_pipeline({})` |
| `arxiv_miner` | 爬取, mine arxiv, fetch arxiv, 搜索论文, 找论文, arxiv搜索 | — | `arxiv_miner({"query": "..."})` |
| `check_task_status` | 任务状态, check status, pipeline status, is it done, 跑完了吗 | Don't call before starting a long-running tool | `check_task_status({"task_id": "..."})` |
| `search_vault` | 搜索, 查找, find notes, vault search, 搜笔记, 找笔记 | — | `search_vault({"query": "..."})` |

**Dual-registry sync verified**: `TOOL_REGISTRY` is a lazy proxy over `get_tool_registry()` via `__getattr__` in `tools/__init__.py` — both populated from the same `_register_default_tools()` call. No consolidation needed.

## HSC verification

- `pytest tests/oligo/` → 8 pre-existing failures only, no new failures ✓
- `grep "user_aliases" src/oligo/tools/registry.py` → 4 hits ✓
