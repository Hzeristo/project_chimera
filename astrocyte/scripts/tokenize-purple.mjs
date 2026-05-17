import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(__dirname, '..');

const rels = [
  'src/app.css', // only replace after :root (see below)
  'src/routes/+page.svelte',
  'src/routes/sidebar/+page.svelte',
  'src/routes/timeline/+page.svelte',
];

const opacities = [
  0.95, 0.92, 0.9, 0.86, 0.85, 0.82, 0.8, 0.78, 0.75, 0.72, 0.66, 0.65, 0.6, 0.56, 0.55, 0.52,
  0.5, 0.45, 0.42, 0.4, 0.35, 0.32, 0.3, 0.28, 0.25, 0.22, 0.2, 0.16, 0.15, 0.12, 0.1, 0.08,
  0.06, 0.05, 0.04,
];

/** 0.35 → 35, 0.04 → 04, 0.3 → 30 — 与 :root 中 --astrocyte-purple-a-XX 命名一致 */
function suffix(a) {
  const s = String(a);
  const m = s.match(/^0\.(\d+)$/);
  if (!m) return s;
  const f = m[1];
  if (f.length === 1) return `${f}0`;
  if (f.length === 2) return f;
  return f.slice(0, 2);
}

function replaceRgba(t) {
  for (const a of opacities) {
    const esc = a.toString().replace(/\./, '\\.');
    const re = new RegExp(`rgba\\(187,\\s*154,\\s*247,\\s*${esc}\\)`, 'g');
    const n = suffix(a);
    t = t.replace(re, `var(--astrocyte-purple-a-${n})`);
  }
  return t;
}

for (const rel of rels) {
  const p = path.join(root, rel);
  let t = fs.readFileSync(p, 'utf8');
  if (rel === 'src/app.css') {
    const marker = '\n\nhtml,';
    const idx = t.indexOf(marker);
    if (idx === -1) throw new Error('app.css: expected marker before html,');
    t = t.slice(0, idx) + replaceRgba(t.slice(idx));
  } else {
    t = replaceRgba(t);
  }
  fs.writeFileSync(p, t);
  console.log('tokenized', rel);
}
