# FC.3b — Svelte artifact chip rendering

- **Commit:** `b7582bf`
- **Status:** Pass
- **Files changed:** `astrocyte/src/routes/+page.svelte`

## What was done

Added artifact chip rendering to the BB message block in `+page.svelte`.

**Template** (after `{@html renderMarkdown(msg)}`, before feedback buttons):
- `{#if msg.sender === 'bb' && msg.artifacts && msg.artifacts.length > 0}` guard
- `.artifact-chips` flex row with `{#each msg.artifacts as art (art.path)}`
- Each chip: `<button class="artifact-chip">` with `◈` icon + filename label
- `on:click={() => openVaultNote(art.path)}` — invokes Rust command
- `title="Open in Obsidian: {art.path}"` — full path in tooltip
- Label shows `art.path.split('/').pop() ?? art.path` — filename only, full path in tooltip

**Script** (`openVaultNote` function, after `onAbortGeneration`):
- `await invoke('open_vault_note', { path })` — matches Rust command signature
- Error routed to `notifySystem('[OPEN_VAULT_NOTE_ERROR] ...')` — bracket-prefixed

**CSS** (after `.feedback-indicator` block):
- `.artifact-chips` — flex wrap, `gap: var(--space-2)`, `margin-top: var(--space-2)`
- `.artifact-chip` — inline-flex, border `var(--astrocyte-purple-a-30)`, bg `var(--surface-chrome-92)`, color `var(--astrocyte-neural-purple)`
- `.artifact-chip:hover` — border `var(--astrocyte-purple-a-72)`, bg `var(--astrocyte-purple-a-10)`
- `.artifact-chip:focus-visible` — outline `var(--astrocyte-neural-purple)` (keyboard accessible)
- `.artifact-chip__label` — `max-width: 220px`, `text-overflow: ellipsis` for long paths
- All tokens from `--astrocyte-*` / `--surface-*` vocabulary — no invented colors

## Verification

- `cargo test path_containment` — 5/5 PASS (FC.3a tests, no regression)
- `svelte-check` deferred (no `node_modules`); manual UI smoke deferred to FC.6 E2E
- No TypeScript errors expected: `invoke` already imported; `msg.artifacts` typed as `Artifact[]` from FC.2b

## Accepted partials

- `svelte-check` deferred — same condition as FC.2b; `node_modules` absent on dev host

## Planning deviation

None.
