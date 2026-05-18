# UI Design Tokens

## Colors

| Token | Use case |
|---|---|
| `--astrocyte-neural-purple` | Primary interactive color |
| `--astrocyte-accent-cyan` | Decorative gradients only |
| `--astrocyte-accent-violet` | Decorative gradients only |
| `--surface-0` | Lightest background |
| `--surface-1` | Default surface |
| `--surface-2` | Elevated surface |
| `--surface-3` | Highest elevation |

## Spacing (4px grid)

| Token | Pixels |
|---|---|
| `--space-1` | 4px |
| `--space-2` | 8px |
| `--space-3` | 12px |
| `--space-4` | 16px |
| `--space-5` | 20px |
| `--space-6` | 24px |

## Radii

| Token | Pixels |
|---|---|
| `--radius-sm` | 4px |
| `--radius-md` | 6px |
| `--radius-lg` | 8px |

## Required for number displays
```css
font-variant-numeric: tabular-nums;
```

## Forbidden
- Hex literals (`#a88cf5`)
- Off-grid spacing (5px, 7px, 10px, 14px)
- Emoji as UI icons (use border-left or text glyphs)
- New colors without design rationale