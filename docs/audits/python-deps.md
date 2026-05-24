# Python Dependency Audit — `crucible_core/src/`

**Generated**: 2026-05-24
**Method**: Static scan of all `^(from|import)` statements under `crucible_core/src/`, grouped by top-level package. Read-only — no installation, no `pyproject.toml` mutation.
**Files scanned**: 51 `.py` files across `src/crucible/**` and `src/oligo/**`.

---

## 1. Standard library only (no action)

These are bundled with CPython 3.11+ — no lock entry needed.

| Module | Notes |
|---|---|
| `__future__` | `annotations` import is the dominant pattern across the codebase |
| `asyncio` | task_service, agent, server, optics_service, daily_chimera_service, vault_read_adapter, openai_compatible_client, miner_tools, web_search, skill_stats_service |
| `collections` | `OrderedDict`, `defaultdict` — vault_read_adapter |
| `collections.abc` | `Awaitable`, `Callable`, `Collection`, `Mapping` — registry, prompt_composer, agent, optics_service, task_service |
| `contextlib` | `asynccontextmanager` — server |
| `copy` | config |
| `csv` | arxiv_fetch, paper_archive_adapter |
| `dataclasses` | `dataclass` — tool_protocol, mineru_pipeline |
| `datetime` | schemas, agent, vault_note_writer, paper_archive_adapter, arxiv_fetch, skill_stats_service, daily_chimera_service, task_service |
| `enum` | `Enum` — schemas, task_service |
| `errno` | agent (used for client-gone detection) |
| `html` | daily_chimera_service |
| `json` | tool_protocol, prompt_composer, sse, agent, task_service, json_janitor, openai_compatible_client, optics_lens_registry, metrics_service, filter_service, skill_stats_service |
| `logging` | pervasive |
| `os` | config |
| `pathlib` | `Path` — pervasive |
| `re` | tool_protocol, text_sanitizer, prompt_composer, agent, vault_read_adapter, naming, json_janitor, arxiv_fetch |
| `shutil` | platform, paper2md, mineru_pipeline, paper_archive_adapter, paper_loader |
| `subprocess` | paper2md (Mineru CLI invocation) |
| `sys` | platform, cli_presenter |
| `textwrap` | cli_presenter |
| `threading` | vault_read_adapter |
| `time` | server, agent, task_service |
| `typing` | `Any`, `Literal`, `Protocol`, `Final`, `Self`, `TYPE_CHECKING`, `AsyncGenerator`, `overload`, `get_args`, `get_origin` |
| `urllib.parse` | `quote` — daily_chimera_service |
| `uuid` | tool_protocol, task_service |
| `xml.etree.ElementTree` | tool_protocol, prompt_composer, arxiv_fetch |

---

## 2. Third-party packages — must lock

Top-level packages actually imported from `crucible_core/src/`. Order: stable → version-sensitive (those flagged in §4).

| Package | Imported as | Used in | Min surface |
|---|---|---|---|
| `PyYAML` | `yaml` | `core/config.py:16`, `services/optics_lens_registry.py:10`, `ports/vault/vault_read_adapter.py:13` | `yaml.safe_load` |
| `tomlkit` | `tomlkit` | `core/config.py:15` | TOML config persistence (write-preserving) |
| `python-dotenv` | `dotenv` | `core/config.py:17` (`dotenv_values`) | `.env` file ingestion |
| `Jinja2` | `jinja2` | `ports/prompts/jinja_prompt_manager.py:8` | Prompt templating |
| `tenacity` | `tenacity` | `ports/llm/openai_compatible_client.py:17`, `ports/notify/telegram_notifier.py:9` | `retry`, `stop_after_attempt`, `wait_exponential`, `RetryCallState` |
| `requests` | `requests` | `ports/arxiv/arxiv_fetch.py:12`, `ports/notify/telegram_notifier.py:8` | Sync HTTP (arXiv API + Telegram bot) |
| `pydantic` | `pydantic` | see §4 — flagged | v2-only API surface (`model_validator`, `ConfigDict`, `AliasChoices`, `SecretStr`) |
| `pydantic-settings` | `pydantic_settings` + `pydantic_settings.sources` | `core/config.py:19,23` | Pulled in transitively by pydantic v2 split |
| `openai` | `openai` | see §4 — flagged | v1+ async client + exception classes |
| `httpx` | `httpx` | see §4 — flagged | Custom transport injected into `AsyncOpenAI` |
| `fastapi` | `fastapi` (+ `.exceptions`, `.middleware.cors`, `.responses`) | see §4 — flagged | App, request validation, CORS, streaming responses |

**Total unique third-party top-level packages: 11** (`PyYAML`, `tomlkit`, `python-dotenv`, `Jinja2`, `tenacity`, `requests`, `pydantic`, `pydantic-settings`, `openai`, `httpx`, `fastapi`).

---

## 3. Internal imports (ignore)

All `from src.crucible.*` and `from src.oligo.*` imports are first-party. No external resolution. Skipped.

Sample (representative, not exhaustive):
- `src.crucible.core.config`, `src.crucible.core.schemas`, `src.crucible.core.platform`, `src.crucible.core.naming`
- `src.crucible.bootstrap`
- `src.crucible.ports.{arxiv,ingest,llm,notify,papers,prompts,vault}.*`
- `src.crucible.services.*`
- `src.oligo.{api,core,tools}.*`

These resolve via the `src/` layout (the `src.` prefix indicates the canonical import root for this package).

---

## 4. Ambiguous / version-sensitive — flagged with file:line

These four packages have breaking-change history, parallel API generations, or transitive coupling that demand explicit version pinning. Locking them loosely (`>=`) is a known foot-gun.

### 4.1 `pydantic` — **must be v2.x**

The codebase uses pydantic v2 idioms exclusively. v1 will not resolve `model_validator`, `ConfigDict`, `AliasChoices`, or the new `SecretStr` semantics.

| File | Line | Statement |
|---|---|---|
| `crucible_core/src/crucible/core/schemas.py` | 9 | `from pydantic import BaseModel, ConfigDict, Field, model_validator` |
| `crucible_core/src/crucible/core/config.py` | 18 | `from pydantic import AliasChoices, BaseModel, ConfigDict, Field, SecretStr, model_validator` |
| `crucible_core/src/crucible/bootstrap.py` | 17 | `from pydantic import SecretStr` |
| `crucible_core/src/crucible/services/task_service.py` | 15 | `from pydantic import BaseModel, Field` |
| `crucible_core/src/crucible/services/optics_service.py` | 12 | `from pydantic import BaseModel, ValidationError` |
| `crucible_core/src/crucible/ports/llm/openai_compatible_client.py` | 16 | `from pydantic import BaseModel, ValidationError` |
| `crucible_core/src/crucible/ports/llm/base.py` | 7 | `from pydantic import BaseModel` |

**Companion**: `pydantic-settings` (v2-split package). Used at `core/config.py:19` and `core/config.py:23` (`from pydantic_settings.sources import ...`). Must move in lockstep with the pydantic major version.

**Pin recommendation**: `pydantic ~= 2.x`, `pydantic-settings ~= 2.x` — single coordinated bump.

### 4.2 `openai` — **must be v1+ (async client era)**

Uses `AsyncOpenAI`, `OpenAI`, and the new exception class hierarchy (`APIError`/`APIConnectionError`/`APITimeoutError`). All introduced in `openai>=1.0`. The legacy `openai.ChatCompletion` namespace is **not** used.

| File | Line | Statement |
|---|---|---|
| `crucible_core/src/crucible/ports/llm/openai_compatible_client.py` | 15 | `from openai import APIConnectionError, APIError, APITimeoutError, AsyncOpenAI, OpenAI` |
| `crucible_core/src/crucible/services/optics_service.py` | 11 | `from openai import APIConnectionError, APIError, APITimeoutError` |

**Pin recommendation**: `openai ~= 1.x`. Avoid floating to v2 without a migration sprint — the SDK has signaled a v2 cutover.

### 4.3 `httpx` — **transport coupling with `openai`**

Used to inject a custom transport into `AsyncOpenAI` (timeout / proxy control). httpx 0.27 → 0.28 changed `Transport` constructor signatures; openai SDK has constraints on supported httpx ranges.

| File | Line | Statement |
|---|---|---|
| `crucible_core/src/crucible/ports/llm/openai_compatible_client.py` | 14 | `import httpx` |

**Pin recommendation**: pin to whatever range the chosen `openai` version declares as compatible. Do not float independently.

### 4.4 `fastapi` — **Pydantic v2 era required**

FastAPI before 0.100 ran on pydantic v1. The codebase is pydantic v2, so fastapi must be `>=0.100`. Uses `Request`, `RequestValidationError`, `CORSMiddleware`, `JSONResponse`, `StreamingResponse` — all stable surface but pydantic-coupled.

| File | Line | Statement |
|---|---|---|
| `crucible_core/src/oligo/api/server.py` | 10 | `from fastapi import FastAPI, Request` |
| `crucible_core/src/oligo/api/server.py` | 11 | `from fastapi.exceptions import RequestValidationError` |
| `crucible_core/src/oligo/api/server.py` | 12 | `from fastapi.middleware.cors import CORSMiddleware` |
| `crucible_core/src/oligo/api/server.py` | 13 | `from fastapi.responses import JSONResponse, StreamingResponse` |

**Pin recommendation**: `fastapi >= 0.110, < 1.0` to stay safely inside pydantic-v2 support. **Note**: no ASGI server (`uvicorn`/`hypercorn`) is imported in `src/` — runtime serving is presumably launched by an out-of-tree entrypoint. That dependency is **not** captured by this static scan; to confirm before locking, check `crucible_core/`'s top-level scripts / Makefile / shell wrappers.

---

## 5. Cross-cutting observations

- **Two HTTP clients in play**: `requests` (sync, used by `arxiv_fetch` and `telegram_notifier`) and `httpx` (async, used by the LLM client). Both are legitimate — collapsing them is a separate refactor, not a lockfile concern.
- **No test framework imports detected in `src/`**. Test deps (pytest, pytest-asyncio, etc.) live elsewhere or are not yet wired.
- **No linter/type-checker imports** (ruff/mypy run as external tools, not imports). Out of scope for this scan.
- **No ASGI server import in `src/`** — see §4.4 note.
- The `tomlkit` + `PyYAML` + `python-dotenv` triad in `core/config.py` is the layered-config story; all three are load-bearing, none are optional.

---

## 6. Lockfile candidate set (summary)

For when the user authorizes the next step:

```
PyYAML
tomlkit
python-dotenv
Jinja2
tenacity
requests
pydantic           # ~= 2.x  (flagged)
pydantic-settings  # ~= 2.x  (flagged, lockstep with pydantic)
openai             # ~= 1.x  (flagged)
httpx              # range tied to openai  (flagged)
fastapi            # >= 0.110, < 1.0  (flagged, pydantic-v2 era)
```

11 direct dependencies. Transitive closure (e.g., `anyio`, `starlette`, `idna`, `certifi`, `sniffio`, `MarkupSafe`, `typing-extensions`, `annotated-types`) will be resolved by the package manager — not enumerated here.

**Not done in this audit** (per instructions):
- No installation.
- No `pyproject.toml` write.
- No `requirements*.txt` write.
- No version resolution against PyPI.
