# Unified Configuration (`~/.chimera/config.toml`)

**Phase:** I M2 (Configuration Unification) (`18:18:project_chimera/docs/ROADMAP.md`) | **Status:** Active  
**Updated:** 2026-05-17 (against commit `48b2b2a`)

## Purpose

All cross-language Chimera binaries share one user-owned TOML file under **`~/.chimera`** for durable paths and LLM slot definitions. Companion JSON files keep **pure UI/ephemeral shells** (`provider_config.json`) while cryptographic material may be outsourced to environment variables. Python adds optional legacy-repo bootstrap YAML; Rust mirrors path resolution for Tauri HUD + task streaming.

Extended field-by-field commentary also exists in-repository at `crucible_core/config.example.toml` (`1:95:project_chimera/crucible_core/config.example.toml`) — see human-oriented comments there while this ARCHITECTURE page tracks **behavioral contracts**.

## Architecture

```text
~/.chimera/
  ├── config.toml          ← single structural truth (slots, paths)
  └── provider_config.json ← Astrocyte UI state only (locks: shared/exclusive reads/writes)

Load path (Python `ChimeraConfig`):
  init defaults → OS env (+ CHIMERA_*) → PROJECT_ROOT `.env` → TomlConfigSource(get_config_path) → extras

Rust HUD subset:
  `config::load_config()` → merges builtin provider defaults + `[llm.providers.*]` (`159:171:project_chimera/astrocyte/src-tauri/src/config.rs`)
```

## API / Schema

### Top-level sections (Python `ChimeraConfig`)

Rendered from class fields (`307:314:project_chimera/crucible_core/src/crucible/core/config.py`):

| Section | Purpose | Structural model |
|---------|---------|------------------|
| `[system]` | Vault + shared dirs / log level (`131:143:project_chimera/crucible_core/src/crucible/core/config.py`) | `SystemConfig`; `vault_root` **validated required** (`395:402:project_chimera/crucible_core/src/crucible/core/config.py`) |
| `[oligo]` | Host/port/agent bounds (`146:151:project_chimera/crucible_core/src/crucible/core/config.py`) | Host default `127.0.0.1`, port `33333`, `tool_execution_deadline_seconds` etc. (`148:151`) |
| `[llm.working]` / `[llm.wash]` / `[llm.router?]` | Runtime model routing (`209:226:project_chimera/crucible_core/src/crucible/core/config.py`; example `39:61:project_chimera/crucible_core/config.example.toml`) | `LLMModelConfig` rows + merged `providers` map (`223:226`) |
| `[llm.providers.*]` | Named HUD-compatible slots Astrocyte mirrors (`169:226:project_chimera/crucible_core/src/crucible/core/config.py`) | Merges builtins (`181:226`) → disk overrides |
| `[wash]` | Wash compressor policy (`252:266:project_chimera/crucible_core/src/crucible/core/config.py`; example `66:74:project_chimera/crucible_core/config.example.toml`) | Drives bundled `ChimeraConfig.oligo_agent` (`482:489`) |
| `[vault]` | Read-cache knobs (`267:274:project_chimera/crucible_core/src/crucible/core/config.py`; example `78:83:project_chimera/crucible_core/config.example.toml`) | `VaultRuntimeConfig` |
| `[astrocyte]` | Desktop shell toggles (`275:279:project_chimera/crucible_core/src/crucible/core/config.py`; example `87:94`) | Mirrors Rust `ChimeraAstrocyteSection` (`65:82:project_chimera/astrocyte/src-tauri/src/config.rs`) |
| `[paper_miner]` | Optional corpus paths/query (`282:294:project_chimera/crucible_core/src/crucible/core/config.py`) | Coerced flatten keys accepted (`362:367:project_chimera/crucible_core/src/crucible/core/config.py`; defaults ensured `414:430`) |
| Telegram + Wash env bridging | Tokens / auxiliary wash identifiers via fields `tg_bot_token`, `tg_chat_id`, `WASH_MODEL_*` (`319:324:project_chimera/crucible_core/src/crucible/core/config.py`) | See validation aliases |

**Rust `crate::config::ChimeraConfig` currently parses** `system`, `oligo`, `llm`, `astrocyte` only (`177:185:project_chimera/astrocyte/src-tauri/src/config.rs`) — **`[wash]` / `[vault]` / `[paper_miner]` are intentionally ignored during Rust deserialize** (`115:126` lumps extra LLM subtrees into `TomlValue` placeholders).

### Platform abstraction (`platform.py` ↔ `platform.rs`)

| Concern | Python | Rust |
|---------|--------|------|
| Home dir `.chimera` | `_migrate_legacy_dotless_chimera` then `mkdir` (`15:46:project_chimera/crucible_core/src/crucible/core/platform.py`) | `rename ~/chimera → ~/.chimera` gate + `canonicalize` (`76:98:project_chimera/astrocyte/src-tauri/src/platform.rs`) |
| Config absolute path | `get_config_path()` (`50:53:project_chimera/crucible_core/src/crucible/core/platform.py`) | Same join (`101:103:project_chimera/astrocyte/src-tauri/src/platform.rs`) |
| Skills / logs / caches | Convenience helpers (`55:76:project_chimera/crucible_core/src/crucible/core/platform.py`) | Matching join + `mkdir` (`106:123:project_chimera/astrocyte/src-tauri/src/platform.rs`) |

**Additional Rust bootstrap:** `migrate_legacy_app_data()` copies historical files from **`dirs::data_local_dir()/chimera`** **only when destination missing**, including **`provider_config.json`**, scratchpad JSON/MD, `skills/` tree, chat `history/` (`33:71:project_chimera/astrocyte/src-tauri/src/platform.rs`). Invoked before config snapshot load (`1379:1382:project_chimera/astrocyte/src-tauri/src/lib.rs`).

Python **does not** mirror `data_local_dir` merges—only **`~/chimera` directory rename**.

### Migration: `~/chimera` (no dot) → `~/.chimera`

- **Python:** `shutil.move` when `.chimera` absent (`17:21:project_chimera/crucible_core/src/crucible/core/platform.py`).
- **Rust:** `fs::rename` identical predicate before `create_dir_all` (`84:90:project_chimera/astrocyte/src-tauri/src/platform.rs`; comment warns ordering—must run **before** unconditional mkdir).

### API key / credential resolution chains

#### Astrocyte runtime provider HUD (`resolved_provider_api_key`)

1. If `[llm.providers.<id>].api_key` trims non-empty → **use TOML** (`133:136:project_chimera/astrocyte/src-tauri/src/settings.rs`).
2. Else map lowercase id → **`OPENAI_API_KEY`**, `DEEPSEEK_API_KEY`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY` (`137:143`).
3. Missing/blank env → **`None`** (caller supplies empty HUD string) (`145:149`).

#### Python `ChimeraConfig`

- Registers **env**, **nested `CHIMERA_` prefix** (`298:305:project_chimera/crucible_core/src/crucible/core/config.py`).
- Ships **explicit post-merge shred/restore**: drop case-insensitive top-level `*_api_key` duplicates then copy canonical env vars **`os.environ` first**, else **`PROJECT_ROOT/.env`** values (`346:354:project_chimera/crucible_core/src/crucible/core/config.py`, `122:129:project_chimera/crucible_core/src/crucible/core/config.py`).

**Note:** pydantic merges multiple `SettingsSources` (`329:344:project_chimera/crucible_core/src/crucible/core/config.py`); authoritative precedence vs nested `[llm.working]` for every field combination is governed by pydantic-settings + this validator hook—when in doubt grep `CHIMERA_LLM__` usage or hydrate via `ChimeraConfig()` in tests.

### `PROJECT_ROOT`-relative `.env`

`dotenv_values(PROJECT_ROOT / ".env")` resolved from module anchor (`35:39:project_chimera/crucible_core/src/crucible/core/config.py`, `_restore` invocation `122:129`). `PROJECT_ROOT = Path(__file__).parents[3]`.

Also registered as pydantic **`env_file`** (`301:301:project_chimera/crucible_core/src/crucible/core/config.py`).

### Provider JSON vs TOML (single semantic source)

`settings.rs` documents JSON **stores UI bookkeeping only** (`66:76:project_chimera/astrocyte/src-tauri/src/settings.rs`): `active_provider_id`, `is_oligo_mode`, `active_skill_id`, legacy `oligo_base_url`; **credential bodies live in `[llm.providers]`** consistent with HUD copy (`2399:2403:project_chimera/astrocyte/src/routes/+page.svelte`). Legacy JSON `providers` array stripped on ingest (`190:193:project_chimera/astrocyte/src-tauri/src/settings.rs`).

**Accepted partial:** provider deletion UI deferred because editing TOML is the sanctioned workflow (`67:69:project_chimera/docs/ACCEPTED_PARTIALS.md`).

### File-lock strategy (`provider_config.json`)

| Operation | Lock |
|-----------|------|
| Read (`load_astrocyte_config`) | `.read(true)`, `fs2::FileExt::lock_shared()` (`207:214:project_chimera/astrocyte/src-tauri/src/settings.rs`) |
| Write (`save_astrocyte_config`) | `.write(true).create(true).truncate(true)`, `lock_exclusive()`, write + flush via **same locked handle** (`232:246:project_chimera/astrocyte/src-tauri/src/settings.rs`) avoiding double-handle races documented in docstring (`225:225`). |

Atomic rename pattern is absent (truncate-in-place guarded by advisory lock).

### Template emission / secret hygiene (Python-only)

`_write_chimera_toml` scrubs `[llm.*].api_key` + top-level `_API_KEY` keys prior to serialization (`544:557:project_chimera/crucible_core/src/crucible/core/config.py`) invoked when migrating legacy YAML to new TOML (`729:736`).

### `effective_oligo_base_url`

Priority **environment `OLIGO_BASE_URL`** → derived TOML `http://host:port` → JSON legacy field only when non-empty mismatch (`17:31:project_chimera/astrocyte/src-tauri/src/settings.rs`; Rust TOML URL builder `199:206:project_chimera/astrocyte/src-tauri/src/config.rs`).

### Legacy repo `config.yaml` bootstrap

If `~/.chimera/config.toml` absent, Loader reads flat `PROJECT_ROOT/config.yaml`, maps into nested dict (`618:743:project_chimera/crucible_core/src/crucible/core/config.py`), attempts **`_write_chimera_toml`** template (`735:739`), feeds `InitSettingsSource` fallback (`739:743`). Rust HUD **still requires** authored TOML for `load_config` success (`219:224:project_chimera/astrocyte/src-tauri/src/config.rs`) — bootstrap is Python-side concern.

## Decision Points

- **User-owned dotdir** anchors secrets/paths portable across crates (`25:47:project_chimera/crucible_core/src/crucible/core/platform.py`).
- **Split persistence:** heavy structure in **TOML**, lightweight ephemeral toggles **`provider_config.json`** with locks (`225:246:project_chimera/astrocyte/src-tauri/src/settings.rs`).
- **Dangerous migrations run once:** both languages gate rename/copy on missing destination to avoid nesting corruption (`84:90:project_chimera/astrocyte/src-tauri/src/platform.rs`).
- **`vault_root` non-optional:** fail-fast after normalization (`395:402:project_chimera/crucible_core/src/crucible/core/config.py`).

## Checklist: Extending Configuration

1. Add fields to pydantic models in `config.py` with `extra="forbid"` symmetry (`295:314` pattern).
2. Mirror deserialization subset in **`astrocyte/src-tauri/src/config.rs`** if HUD must read values; widen structs deliberately—Rust ignores unknown sections today only where `Deserialize` skips them.
3. Update **`config.example.toml`** human comments + regenerate docs pointer (`1:95:project_chimera/crucible_core/config.example.toml`).
4. If secrets enter new buckets, extend `_CANONICAL_LLM_SECRET_ENV_NAMES` / shredding lists as needed (`50:56`, `116:129`, `544:557:project_chimera/crucible_core/src/crucible/core/config.py`).
5. For UI-visible toggles kept outside TOML, extend **`AstrocyteConfig`** + lock-aware load/save (`66:246:project_chimera/astrocyte/src-tauri/src/settings.rs`).
6. Re-run **`normalize_astrocyte_with_chimera`** equivalents after slot changes (`114:125:project_chimera/astrocyte/src-tauri/src/settings.rs`).

## Known Issues / References

- **Rust vs Python parity gap:** HUD parser omits `[wash]` / `[vault]` / `[paper_miner]` semantics—Python-only until desktop needs surface them (`177:185:project_chimera/astrocyte/src-tauri/src/config.rs`).
- **I.M2.1** deletion UI omission documented as accepted (`67:69:project_chimera/docs/ACCEPTED_PARTIALS.md`).
- **Typo-risk:** Editing `effective_oligo_base_url` interplay can resurrect stale JSON-origin URLs (`17:31:project_chimera/astrocyte/src-tauri/src/settings.rs`).
- **`CHIMERA_` nesting** mistakes (`299:305:project_chimera/crucible_core/src/crucible/core/config.py`) — verify with integration tests before relying in production scripting.

## Cross-references

- [`TASK_PROGRESS_SYSTEM.md`](./TASK_PROGRESS_SYSTEM.md) — `tasks/` + `metrics.json` siblings under Chimera root.
- [`SSE_PROTOCOL.md`](./SSE_PROTOCOL.md) — Oligo base URL wiring from `[oligo]` snapshot.
- `crucible_core/docs/CONFIG_SCHEMA.md` — supplementary prose schema table (maintain coherence when fields drift).
