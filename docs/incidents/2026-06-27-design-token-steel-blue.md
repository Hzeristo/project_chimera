# Incident 2026-06-27 — Astrocyte design tokens: purple → Swiss Steel Blue

**Status:** Fixed (token-source swap)

## Symptom

The Astrocyte palette was built on neural-purple `#bb9af7` /
`rgba(187, 154, 247, …)` with purple-tinted surfaces (`(10,10,15)`,
`(20,18,30)`) and over-saturated accents (`#22d3ee` cyan / `#7c3aed`
violet). Off-brand for the intended restrained, Swiss/Internationalist
steel look.

## Root cause

All color values originate from the `:root` token block in
`astrocyte/src/app.css`. Components consume them only via `var(--…)`,
so the palette is fully centralized — the purple identity lived in that
single block, not in the component tree.

## Fix

Swapped the token source in `astrocyte/src/app.css:6`:

- Primary `--astrocyte-neural-purple` `#bb9af7` → `#8aa4b8` (steel blue).
- Entire `--astrocyte-purple-a-*` α-ladder `rgba(187,154,247,n)` →
  `rgba(138,164,184,n)`.
- Accents desaturated: cyan `#22d3ee` → `#b8c8d4`, violet `#7c3aed` →
  `#4a6278` (progress-bar light/dark ends).
- Surfaces de-purpled to neutral grey `(12,14,18)` family; borders to
  steel-grey; LED-off `#4a2060` → `#2a3a4a`.
- Semantic colors (good/bad/error/warning) lowered in saturation.

One non-token straggler aligned for consistency: the `[SYSTEM] Neural
link` devtools `console.log` `%c` style in
`astrocyte/src/routes/+page.svelte:1044` (`#bb9af7` → `#8aa4b8`); it is
a console style string, not a CSS custom property, so it cannot use a
`var(--…)` token.

## Verification

`:root` block re-confirmed well-formed (spacing/radius/font/compat
tokens after `--warning` preserved). No `rgba(187,154,247)` / `#bb9af7`
literals remain in `astrocyte/src` outside descriptive comments.
