# Astrocyte Visual Design System (HUD / shell)

**Related:** Phase III.C structured contract (`88:88:project_chimera/docs/ROADMAP.md` FC.3 attachment UI calls out design-token compliance) | **Status:** Active  
**Updated:** 2026-05-17 (against commit `48b2b2a`)

## Purpose

Astrocyte is the **desktop shell** over Oligo + task/tool telemetry. This document pins the **CSS custom property vocabulary** so new UI matches the existing cyberpunk-dark HUD: one `:root` ledger in `app.css`, component classes layered on top, and Svelte `<style>` blocks that should **prefer tokens** over raw hex. It operationalizes `chimera-code-taste` core principle **“Design tokens exclusive”** — interpreted here as: **new work must use `var(--astrocyte-*)`, `var(--surface-*)`, spacing/radius/font tokens, and semantic feedback colors** from the global sheet; **documented exceptions** below are legacy or domain-specific (system log rail, danger guillotine).

## Architecture

```text
Fonts (Google CDN) + :root token ledger
  → `1:148:project_chimera/astrocyte/src/app.css`

Global HUD patterns (buttons, dropdowns, chat, tool/task overlays, scrollbars)
  → `150:1300:project_chimera/astrocyte/src/app.css`

Route / lib local `<style>`
  → Prefer `var(--*)`; may still contain ad-hoc `#rrggbb` (drift — see Known Issues)

Timeline webview host
  → `166:181:project_chimera/astrocyte/src/app.css` strips body chrome for OS glass
```

**Single source of numeric rhythm:** spacing uses a **4 px base** (`112:118:project_chimera/astrocyte/src/app.css`); radii are `2–8px` (`120:125`).

## API / Schema

### Brand & α ladder (purple)

| Token | Role | Definition |
|-------|------|------------|
| `--astrocyte-neural-purple` | Primary accent / focus | `#bb9af7` (`10:10:project_chimera/astrocyte/src/app.css`) |
| `--astrocyte-purple-a-**` | Borders, glows, muted text | `rgba(187, 154, 247, α)` steps `0.04–0.95` (`15:49`) |
| `--astrocyte-accent-violet` / `--astrocyte-accent-cyan` | Miner / multi-segment progress gradients | `53:53`; `52:52` |
| `--astrocyte-bb-fg`, `--astrocyte-user-fg`, `--astrocyte-read-fg*` | Chat role colors | `143:146` |

### Surfaces & chrome

| Family | Examples | Lines |
|--------|----------|-------|
| Depth stack | `--surface-0` … `--surface-3`, `--surface-body` | `56:60` |
| Modals / rows | `--surface-modal`, `--surface-row-muted`, `--surface-scrim` | `70:77` |
| HUD specialty | `--surface-code`, `--surface-sidebar`, `--surface-floating`, `--surface-card-frost` | `68:84` |
| Status chrome | `--surface-chrome-88`, `--surface-progress-track`, `--status-led-*` | `66:67`, `87:92` |

### Semantic feedback

| Token | Use |
|-------|-----|
| `--feedback-good` / `--feedback-bad` | Non-destructive OK / soft warn tint (`105:106`) |
| `--error`, `--error-surface`, `--error-fg` | Fatal / destructive (`.error-card`, `.btn-danger`) (`107:109`, `295:311`, `1156:1169`) |
| `--surface-danger-surface` | Timeline danger affordance background (`94:94`, used in `timeline/+page.svelte` via token) |

### Layout tokens

| Category | Tokens |
|----------|--------|
| Space | `--space-1` … `--space-6` (`112:118`) |
| Radius | `--radius-xs` … `--radius-lg`, `--radius-3` (`120:125`) |
| Border | `--border-hud`, `--border-neutral`, `--border-muted`, `--border-active` (`97:102`) |
| Type scale | `--font-xs` … `--font-lg`, `--line-tight|normal|relaxed` (`127:135`) |
| Families | `--font-body` (Inter stack), `--font-mono` (JetBrains / Fira Code) (`139:140`) |

### Global components (non-Svelte)

Representative **canonical patterns** to copy:

- **Buttons:** `.btn`, `.btn--hud`, `.btn--icon`, `.btn--primary`, `.btn-danger` (`223:317`)
- **Selectors:** `.persona-trigger`, `.skill-trigger`, dropdown panels (`319:520`)
- **Chat:** `.hud-output article.msg-row …` user/bb/system senders (`750:814`)
- **System tool trace:** `.system-log-raw` + sub-elements (uses **local** `--system-log-fg`/`--system-log-glow`, not `:root` — `882:967`)
- **Task overlay:** `.active-task-panel`, `.task-row`, `.task-elapsed`, progress bar (`1183:1272`)
- **Tool overlay shell:** `.active-tool-telemetry` container (`1282:1300`); row grid lives in component (below)
- **Scrollbar:** universal thin purple thumb (`724:741`)

## Typography rules

1. **Sans (Inter + CJK fallbacks)** for conversational chat bubbles (`756:761:project_chimera/astrocyte/src/app.css`).
2. **Monospace** for HUD rails, fire selector, tool/system telemetry, stage cards, archive banner (`200:221`, `716:722`, `627:667`, `834:858`, `1002:1011`).
3. **Sizing:** prefer `--font-*`; legacy `10px` / `0.62rem` literals remain inside skill/persona HUD blocks (`475:603`) — new code should align to the nearest `--font-xs` / `--font-sm` step when touching those files.

## `tabular-nums` policy

Apply **`font-variant-numeric: tabular-nums`** anywhere **elapsed seconds**, **counts**, or **tabular metrics** jitter would distract (monospace alone is insufficient for proportional digit fonts):

| Location | Evidence |
|----------|----------|
| Skill trigger inline stats | `360:364:project_chimera/astrocyte/src/app.css` |
| Skill card stat values | `588:594`, `612:617` |
| Settings / metrics panels (main HUD) | e.g. `3243:3244`, `3288:3288`, `3331:3331`, `3380:3380` in `routes/+page.svelte` scoped CSS |
| Task overlay elapsed | `1248:1253:project_chimera/astrocyte/src/app.css` (`.task-elapsed` — consumed by `ActiveTaskPanel.svelte` `260:260`) |
| Tool telemetry elapsed | `180:184:project_chimera/astrocyte/src/lib/ActiveToolTelemetry.svelte` (`.tool-tel-elapsed`) |

## Decision Points

| ID | Decision | Rationale |
|----|----------|-----------|
| DS.1 | One `:root` block holds all cross-route tokens (`6:148`) | Matches sprint-1 ledger comment; avoids token fork per route (`3:5`) |
| DS.2 | Timeline route nulls body background | OS glass timeline requires transparent host (`166:181`) |
| DS.3 | `.system-log-raw` cyan/blue rail is tokenized **locally** | Distinct sci-fi readability for streaming trace without polluting global purple scale (`882:904`) |
| DS.4 | Primary CTA text uses literal `#000` on purple | Contrast on `#bb9af7` (`281:285`) |

## Checklist (before merging UI)

1. **Colors:** new surfaces/borders use `var(--surface-*)` / `var(--astrocyte-purple-a-*)` / `var(--border-*)`; no new stray `#rgb` unless matching an existing exception pattern.
2. **Spacing:** pad/margin/gap via `--space-*` or `calc()` on those tokens (`112:118`).
3. **Radii:** `--radius-*` only (`120:125`).
4. **Monospace HUD:** telemetry/time-code rows use `var(--font-mono)` (`139:140`, `716:722`).
5. **Animations:** honor `prefers-reduced-motion` when adding motion (see `.system-log-raw` pattern `990:999`).
6. **Telemetry numerals:** durations / stats get `tabular-nums` (`tabular-nums policy` table).

## Known Issues / Drift

- **Hex literals in Svelte routes:** `routes/+page.svelte`, `sidebar/+page.svelte`, `timeline/+page.svelte` still embed `#…` for one-off tints (e.g. `2675`, `445:528` bands) — should migrate toward α-ladder tokens over time **FC.3 alignment** (`88:88:project_chimera/docs/ROADMAP.md`).
- **`app.css` legacy literals:** skill description `#888`, `.skill-name` `#d0c8e0`, guillotine reds, Katy TeX tweaks (`518:558`, `1054:1083`, `821:824`) — same migration story.
- **Dev-only console styling:** `console.log` with `#bb9af7` (`1031:1031:project_chimera/astrocyte/src/routes/+page.svelte`) — not end-user UI but duplicates brand hex outside tokens.

## Cross-references

- **Tool / task HUD behavior (not styling source of truth):** [`INTENT_AND_DEGRADATION.md`](./INTENT_AND_DEGRADATION.md), [`TASK_PROGRESS_SYSTEM.md`](./TASK_PROGRESS_SYSTEM.md)
- **SSE taxonomy driving telemetry:** [`SSE_PROTOCOL.md`](./SSE_PROTOCOL.md)
- **Prompt-side budgets (indirectly affects HUD content volume):** [`PROMPT_MIDDLEWARE.md`](./PROMPT_MIDDLEWARE.md)
