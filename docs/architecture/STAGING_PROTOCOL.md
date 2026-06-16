# Staging Protocol

Candidate nodes live in `docs/staging/` (repo, not vault) with `status: PENDING_REVIEW`. Promotion is an explicit human-triggered action — no automatic promotion.

---

## Directory Layout

```
docs/staging/
  20260615_143000-My_Hypothesis.md   ← PENDING_REVIEW candidate
  20260615_150000-Key_Insight.md
```

Files are named `{YYYYMMDD_HHMMSS}-{slug}.md`. The slug is the sanitized title, max 60 chars.

---

## Candidate Frontmatter Contract

All candidates have:

```yaml
type: thought | insight | decision
status: PENDING_REVIEW
title: "..."
created_at: "YYYY-MM-DD"
tags: [thought]
graph_edges:
  ...       # type-specific spec edges (see NODE_ONTOLOGY.md)
```

Edge fields match the authoritative set in `NODE_ONTOLOGY.md`. Knowledge nodes (`type: knowledge`) are not staged — they are written directly by `VaultNoteWriter`.

---

## Lifecycle State Machine

```
create_staging_node()
        │
        ▼
  PENDING_REVIEW  (lives in docs/staging/)
        │
   ┌────┴────┐
promote()   reject()
   │             │
   ▼             ▼
 active       deleted
(vault/)
```

`promote_node(path)` → `status: active` for all types. `cross_verified` is a separate manual upgrade, not a promotion consequence.

---

## Promote Destination by Type

| Node type  | Vault subfolder  |
|------------|------------------|
| `thought`  | `Thoughts/`      |
| `insight`  | `Insight/`       |
| `decision` | `Decision/`      |

`vault_root` is sourced from `SystemConfig.vault_root` — never hardcoded.

---

## API

```python
from crucible.services.staging_service import StagingService
from crucible.core.platform import get_project_root

svc = StagingService(
    staging_dir=get_project_root() / "docs" / "staging",
    vault_root=settings.require_path("vault_root"),
)

path = svc.create_staging_node("thought", "My Hypothesis", body="...", edges={"derives_from": ["2501.12345"]})
vault_path = svc.promote_node(path)   # writes to vault, removes staging file
svc.reject_node(path)                 # deletes staging file
```

Astrocyte HTTP endpoints wrapping this service are wired in V.A.4.
