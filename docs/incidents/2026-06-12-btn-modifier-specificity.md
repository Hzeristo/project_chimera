# Incident 2026-06-12 — `.btn--icon` Specificity Tie / Square Enforcement

**Status:** Fixed (manual patch after Claude could not resolve)

## Symptom

`.btn--icon` buttons (E/D/R) rendered non-square despite Claude
applying `width`/`height`/`min-*`/`flex: 0 0`/`aspect-ratio` across
multiple rounds. Sharp corners (`border-radius: 0`) declared in
`.btn--icon` also failed to take effect.

## Root cause (specificity tie + load-bearing padding)

`.btn` and `.btn--icon` both have specificity (0,1,0). For properties
defined in BOTH classes, source order decides the winner — and `.btn`
appeared after `.btn--icon` in the cascade for `padding` and
`border-radius`, so the base class won:

- `.btn` `padding: var(--space-1) var(--space-3)` (4px 12px) overrode
  `.btn--icon` `padding: 0`. Horizontal padding 24px exceeded the
  18px width target with `box-sizing: border-box`, forcing the
  browser to inflate the box → not square.
- `.btn` `border-radius: var(--radius-sm)` (4px) overrode `.btn--icon`
  `border-radius: 0`. Buttons stayed rounded.
- `width` / `height` / `flex` / `aspect-ratio` were defined ONLY in
  `.btn--icon` (no tie) → those took effect, but padding inflation
  defeated them.

## Fix (manual)

After Claude's CSS-text-reasoning rounds failed to identify the tie,
F12 computed-style inspection identified the cascade winner. Manual
patch added a double-class selector to `app.css`:

    .btn.btn--icon {
      padding: 0;
      border-radius: 0;
    }

Specificity (0,2,0) wins over either single class regardless of
source order.

## What did NOT fix the bug (for the record)

- `box-sizing: border-box` on `.btn--icon` — base class already had it
- `flex: 0 0 var(--control-h-xs)` — buttons weren't being stretched;
  they were being inflated by their own padding
- `aspect-ratio: 1 / 1` — defeated by padding-driven width inflation
- `line-height: 1` — irrelevant; height was correct, width was wrong
- `align-items: center` on `.msg-actions` — irrelevant; flex stretch
  was not the cause

These changes were applied during diagnosis rounds and remain in the
codebase, but they did not address the actual root cause.

## Lesson

When a modifier class overrides properties already defined in its
base class, **specificity tie is the default trap** — source order
decides the winner, not declaration intent. Modifier classes that
override base properties MUST use `.base.modifier` double-class
selectors `(0,2,0)` to guarantee precedence.

Diagnostic lesson: when LLM-driven CSS edits don't take effect after
multiple rounds, dump computed style (F12) and identify the cascade
winner manually. Text-level reasoning over CSS rules cannot detect
which value actually wins after the cascade resolves.

## Cross-reference

This incident motivated `anti_patterns.md` rule `modifier_specificity_tie`
(land in chimera-code-taste during Phase III.F or next debt cleanup).
