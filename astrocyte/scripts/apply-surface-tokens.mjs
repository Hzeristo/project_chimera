import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(__dirname, '..');

/** 仅替换样式区，避免改写 :root 中的字面量定义 */
function patchAppCss() {
  const p = path.join(root, 'src/app.css');
  let t = fs.readFileSync(p, 'utf8');
  const marker = '\n\nhtml,';
  const idx = t.indexOf(marker);
  if (idx === -1) throw new Error('app.css: marker not found');
  let head = t.slice(0, idx);
  let rest = t.slice(idx);

  const pairs = [
    [/background-color: rgba\(15, 15, 20, 0\.8\)/g, 'background-color: var(--surface-body)'],
    [/color: #d4d4d4/g, 'color: var(--astrocyte-surface-text)'],
    [/background: rgba\(187, 154, 247, 0\.15\)/g, 'background: var(--astrocyte-purple-a-15)'],
    [/border: 1px solid rgba\(100, 90, 130, 0\.35\)/g, 'border: 1px solid var(--border-hud-emphasis)'],
    [/border: 1px solid rgba\(100, 90, 130, 0\.3\)/g, 'border: 1px solid var(--border-hud)'],
    [/border: 1px solid rgba\(100, 90, 130, 0\.2\)/g, 'border: 1px solid var(--border-muted)'],
    [/background: rgba\(30, 27, 40, 0\.6\)/g, 'background: var(--surface-skill-tile-hover)'],
    [/background: rgba\(30, 27, 40, 0\.4\)/g, 'background: var(--surface-skill-tile)'],
    [/background: rgba\(22, 20, 32, 0\.85\)/g, 'background: var(--surface-3)'],
    [/background: rgba\(20, 18, 30, 0\.95\)/g, 'background: var(--surface-2)'],
    [/background: rgba\(12, 10, 18, 0\.95\)/g, 'background: var(--surface-inset)'],
    [/background: rgba\(12, 10, 18, 0\.6\)/g, 'background: var(--surface-inset-soft)'],
    [/border-left: 2px solid #bb9af7/g, 'border-left: 2px solid var(--astrocyte-neural-purple)'],
    [/border-color: #bb9af7/g, 'border-color: var(--astrocyte-neural-purple)'],
    [/color: #bb9af7/g, 'color: var(--astrocyte-neural-purple)'],
    [/background: #050505/g, 'background: var(--surface-code)'],
    [/background: rgba\(220, 38, 38, 0\.1\)/g, 'background: var(--error-surface)'],
    [/border-left: 3px solid #dc2626/g, 'border-left: 3px solid var(--error)'],
    [/color: #fecaca/g, 'color: var(--error-fg)'],
    [/border: 1px solid rgba\(51, 51, 51, 0\.9\)/g, 'border: 1px solid var(--border-scrollbar)'],
  ];
  for (const [re, rep] of pairs) {
    rest = rest.replace(re, rep);
  }

  const layout = [
    [/border-radius: 4px/g, 'border-radius: var(--radius-sm)'],
    [/border-radius: 6px/g, 'border-radius: var(--radius-md)'],
    [/border-radius: 3px/g, 'border-radius: var(--radius-3)'],
    [/border-radius: 2px/g, 'border-radius: var(--radius-xs)'],
    [/line-height: 1\.5(?=[;\s}])/g, 'line-height: var(--line-normal)'],
    [/line-height: 1\.6(?=[;\s}])/g, 'line-height: var(--line-relaxed)'],
  ];
  for (const [re, rep] of layout) {
    rest = rest.replace(re, rep);
  }

  const space = [
    [/padding: 6px 12px/g, 'padding: var(--space-2) var(--space-3)'],
    [/padding: 4px 12px/g, 'padding: var(--space-1) var(--space-3)'],
    [/padding: 4px 10px/g, 'padding: var(--space-1) var(--space-3)'],
    [/padding: 4px 8px/g, 'padding: var(--space-1) var(--space-2)'],
    [/padding: 4px 2px/g, 'padding: var(--space-1) var(--radius-xs)'],
    [/padding: 8px 12px/g, 'padding: var(--space-2) var(--space-3)'],
    [/padding: 10px 12px/g, 'padding: var(--space-3) var(--space-3)'],
    [/padding: 12px 16px/g, 'padding: var(--space-3) var(--space-4)'],
    [/padding: 12px 8px 12px 12px/g, 'padding: var(--space-3) var(--space-2) var(--space-3) var(--space-3)'],
    [/padding: 12px;/g, 'padding: var(--space-3);'],
    [/gap: 2px/g, 'gap: var(--radius-xs)'],
    [/gap: 4px/g, 'gap: var(--space-1)'],
    [/gap: 8px/g, 'gap: var(--space-2)'],
    [/gap: 10px/g, 'gap: var(--space-3)'],
    [/gap: 12px/g, 'gap: var(--space-3)'],
    [/margin-top: 2px/g, 'margin-top: var(--radius-xs)'],
    [/margin-top: 4px/g, 'margin-top: var(--space-1)'],
    [/margin-top: 6px/g, 'margin-top: var(--space-2)'],
    [/margin-top: 8px/g, 'margin-top: var(--space-2)'],
    [/margin-bottom: 4px/g, 'margin-bottom: var(--space-1)'],
    [/margin-bottom: 8px/g, 'margin-bottom: var(--space-2)'],
    [/margin-bottom: 6px/g, 'margin-bottom: var(--space-2)'],
    [/margin-bottom: 10px/g, 'margin-bottom: var(--space-3)'],
    [/padding-left: 10px/g, 'padding-left: var(--space-3)'],
  ];
  for (const [re, rep] of space) {
    rest = rest.replace(re, rep);
  }

  fs.writeFileSync(p, head + rest);
  console.log('patched app.css (post-:root)');
}

function patchSvelte(rel) {
  const p = path.join(root, rel);
  let t = fs.readFileSync(p, 'utf8');
  const start = t.indexOf('<style>');
  const end = t.indexOf('</style>');
  if (start === -1 || end === -1) return;
  let head = t.slice(0, start);
  const styleOpen = t.slice(start, start + 7);
  let mid = t.slice(start + 7, end);
  const tail = t.slice(end);
  const pairs = [
    [/border: 1px solid #333/g, 'border: 1px solid var(--border-neutral)'],
    [/border: 1px dashed #333/g, 'border: 1px dashed var(--border-neutral)'],
    [/background: #0a0a0f/g, 'background: var(--surface-0)'],
    [/background: #000000/g, 'background: var(--surface-absolute-black)'],
    [/color: #bb9af7/g, 'color: var(--astrocyte-neural-purple)'],
    [/border-color: #bb9af7/g, 'border-color: var(--astrocyte-neural-purple)'],
    [/caret-color: #bb9af7/g, 'caret-color: var(--astrocyte-neural-purple)'],
    [/color: #d3b8ff/g, 'color: var(--astrocyte-bb-fg)'],
    [/background: #050505/g, 'background: var(--surface-code)'],
    [/color: #e2e8f0/g, 'color: var(--astrocyte-read-fg)'],
    [/background: rgba\(6, 6, 10, 0\.95\)/g, 'background: var(--surface-archive)'],
    [/background: rgba\(12, 12, 20, 0\.92\)/g, 'background: var(--surface-row-muted)'],
    [/background: rgba\(8, 8, 12, 0\.94\)/g, 'background: var(--surface-modal)'],
    [/background: rgba\(8, 8, 12, 0\.92\)/g, 'background: var(--surface-modal-inner)'],
    [/background: rgba\(10, 10, 15, 0\.92\)/g, 'background: var(--surface-chrome-92)'],
    [/background: rgba\(10, 10, 15, 0\.88\)/g, 'background: var(--surface-chrome-88)'],
    [/background: linear-gradient\(90deg, #7c3aed, #22d3ee\)/g,
      'background: linear-gradient(90deg, var(--astrocyte-accent-violet), var(--astrocyte-accent-cyan))',
    ],
    [/color: #ff8fa3/g, 'color: var(--feedback-bad)'],
    [/color: #8ef1b6/g, 'color: var(--feedback-good)'],
  ];
  for (const [re, rep] of pairs) {
    mid = mid.replace(re, rep);
  }
  /* 样式块内残余主色短 hex */
  mid = mid.replace(/#bb9af7/g, 'var(--astrocyte-neural-purple)');
  fs.writeFileSync(p, head + styleOpen + mid + tail);
  console.log('patched', rel);
}

patchAppCss();
patchSvelte('src/routes/+page.svelte');
patchSvelte('src/routes/sidebar/+page.svelte');
patchSvelte('src/routes/timeline/+page.svelte');
