# FC.3a — Rust `open_vault_note` + path-containment guard

- **Commit:** `4261a84`
- **Status:** Pass
- **Files changed:** `astrocyte/src-tauri/src/lib.rs`, `astrocyte/src-tauri/Cargo.toml`

## What was done

Added `vault_contains_path` helper and `open_vault_note` Tauri command to the Rust backend.

**`vault_contains_path(vault_root, raw) -> Result<PathBuf, String>`**
- Layer 1: pre-filesystem `..` component scan — rejects traversal before any I/O
- Layer 2: `fs::canonicalize` prefix check — rejects symlink escapes post-canonicalize
- Returns the canonical `PathBuf` on success; error string (bracket-prefixed) on failure

**`open_vault_note` Tauri command**
- Reads `vault_root` from `state.chimera.read().system.vault_root` — errors if unconfigured
- Calls `vault_contains_path` — errors if path outside root
- Manual percent-encoding (byte-by-byte) — avoids new dep; encodes everything except `[A-Za-z0-9\-_.~/:]`
- Constructs `obsidian://open?path=<encoded>` URI and calls `app.opener().open_url()`
- Registered in `invoke_handler!` after `sublimate_scratchpad`

**Tests** (`path_containment_tests` module, 5 tests):
- `inside_root_accepted` — normal vault path passes
- `outside_root_rejected` — sibling path outside vault root rejected
- `traversal_rejected` — `../` prefix rejected at layer 1
- `nested_traversal_rejected` — `notes/../../../etc/passwd` rejected at layer 1
- `symlink_escape_rejected` — symlink outside root rejected at layer 2 (`#[cfg(not(target_os = "windows"))]`)
- `symlink_escape_skipped_on_windows_without_devmode` — documented skip (Windows symlink creation requires elevated privileges)

`tempfile = "3"` added to `[dev-dependencies]` for test vault isolation.

## Verification

- `cargo test path_containment` — 5/5 PASS
- `cargo build` implicit (clean compile on test run)
- `svelte-check` deferred (no `node_modules`); no Svelte changes in this sprint

## Accepted partials

None.

## Planning deviation

None.
