#!/usr/bin/env node
// Every game mode must offer a way out, at EVERY point — the end screen
// included. Eric's law, 2026-10-09, and never an X in the top-left corner.
//
// THE BUG THIS EXISTS FOR. Impostor's end-of-run overlay is
// `position:fixed; inset:0`, so it covers the top bar and the mode-exit
// button with it. The only control it carried was "Again". Once the ten
// words were done there was no way out of the mode at all, short of killing
// the app — and for a child that reads as the game being broken, not as a
// missing button. Eric found it in Impostor; Chains, Bee and Boardgame all
// had the same shape.
//
// WHAT IT CHECKS. For each mode's end-of-run panel: the panel exists, it
// carries at least one control that LEAVES the mode, and that control's id
// is actually wired to the mode's close in Rust. A button nobody wired is
// the dead-id law's problem (dom-id-live-check) but a silent trap here, so
// this checks the wiring too rather than trusting the markup.
//
// WHY A LIST AND NOT A HEURISTIC. "Which div is an end-of-run panel" cannot
// be read off the HTML — it is a fact about the mode. A heuristic would
// either miss panels or flag every modal in the file. The list is the
// honest shape, and a new mode is a deliberate line here.
//
//   node scripts/mode-exit-check.mjs
//   node scripts/mode-exit-check.mjs --selftest

import { readFileSync, existsSync, mkdtempSync, writeFileSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { pathToFileURL } from 'node:url';

// panel id -> [the Rust file that owns the mode, the exit button's id]
const PANELS = {
  impOver: ['src/impostor_screen.rs', 'impOverExit'],
  cnOver: ['src/chains_screen.rs', 'cnOverExit'],
  beeOver: ['src/bee_screen.rs', 'beeOverExit'],
  bgOver: ['src/boardgame_screen.rs', 'bgOverExit'],
  dmRecap: ['src/defmatch_screen.rs', 'dmRecapClose'],
};

/** The balanced <div> block beginning at the panel's own tag. */
export function panelBlock(html, id) {
  const open = new RegExp(`<div[^>]*id="${id}"`).exec(html);
  if (!open) return null;
  const re = /<(\/?)div\b/g;
  re.lastIndex = open.index;
  let depth = 0;
  let m;
  while ((m = re.exec(html))) {
    depth += m[1] ? -1 : 1;
    if (depth === 0) return html.slice(open.index, re.lastIndex);
  }
  return html.slice(open.index);
}

export function problems(root = '.') {
  const out = [];
  const htmlPath = join(root, 'index.html');
  if (!existsSync(htmlPath)) return [`${htmlPath} is missing`];
  const html = readFileSync(htmlPath, 'utf8');

  for (const [panel, [rs, exitId]] of Object.entries(PANELS)) {
    const block = panelBlock(html, panel);
    if (!block) {
      out.push(`#${panel} is gone — if the mode was removed, drop it from PANELS here too`);
      continue;
    }
    if (!new RegExp(`id="${exitId}"`).test(block)) {
      out.push(`#${panel} has no #${exitId}: the end screen covers the top bar, `
        + 'so a player who finishes the run cannot leave the mode');
      continue;
    }
    const rsPath = join(root, rs);
    if (!existsSync(rsPath)) {
      out.push(`${rs} is missing, so #${exitId} cannot be wired`);
      continue;
    }
    const src = readFileSync(rsPath, 'utf8');
    if (!new RegExp(`on_click\\(\\s*"${exitId}"`).test(src)) {
      out.push(`#${exitId} exists in index.html but ${rs} never wires it — `
        + 'a button that does nothing is worse than no button');
    }
  }

  // AND THE EXIT MUST BE A WORD, NOT A GLYPH. Eric, 2026-10-09: never an X
  // in the top-left corner. Changing only the POSITION would not satisfy
  // that — the top bar is a plain flex row, so its leading edge is
  // top-left in the 14 LTR languages and top-right in Arabic, and any
  // trailing placement simply moves the violation to Arabic. Changing WHAT
  // the control is works in every direction, so that is what is enforced:
  // every mode exit carries a translated text label and no bare glyph.
  // Both kinds: the exit that LEAVES A MODE and the close that dismisses a
  // panel. Eric swept the second on 2026-10-09 after the first. The panel
  // closes were `.hub-x`, pinned with inset-inline-end — top-right in the
  // 14 LTR languages but top-left in Arabic — plus listsClose, which sat on
  // the leading edge and so was top-left in English too.
  const GLYPHS = /[\u00d7\u2715\u2716\u274c\u2573xX]/;
  const WAYS_OUT = /<button([^>]*(?:\bmode-exit\b|\bhub-close\b|id="\w*(?:Close|Exit)")[^>]*)>([\s\S]*?)<\/button>/g;
  for (const m of html.matchAll(WAYS_OUT)) {
    const [, attrs, body] = m;
    const id = (/id="([^"]+)"/.exec(attrs) || [, '(no id)'])[1];
    const text = body.replace(/<[^>]+>/g, '').trim();
    const kind = /\bmode-exit\b/.test(attrs) ? 'mode exit' : 'way out';
    if (!/data-i18n="/.test(attrs)) {
      out.push(`#${id} is a ${kind} with no data-i18n label — it must say a `
        + 'translated word, so it reads the same in every writing direction');
    }
    if (text.length <= 2 && GLYPHS.test(text)) {
      out.push(`#${id} is a ${kind} showing the glyph ${JSON.stringify(text)} — `
        + 'it must be a word. An X is a convention a child has not learned, and '
        + 'the corner it sits in flips between LTR and RTL.');
    }
  }

  return out;
}

function selftest() {
  const mk = (html, rs) => {
    const d = mkdtempSync(join(tmpdir(), 'modeexit-'));
    mkdirSync(join(d, 'src'), { recursive: true });
    writeFileSync(join(d, 'index.html'), html);
    for (const [f, body] of Object.entries(rs)) writeFileSync(join(d, f), body);
    return d;
  };
  const full = (extra = '') => {
    let h = '';
    const rs = {};
    for (const [panel, [f, id]] of Object.entries(PANELS)) {
      // The fixtures must satisfy the label rule too, or the selftest is
      // testing a world the real check would reject.
      h += `<div id="${panel}"><div><button id="${id}" data-i18n="aria.close">Close</button></div></div>\n`;
      rs[f] = `dom::on_click("${id}", close);`;
    }
    return [h + extra, rs];
  };
  const cases = [
    ['a panel with a wired exit passes', ...full(), 0],
    ['a panel missing its exit fails', (() => {
      const [h, rs] = full();
      return [h.replace('<button id="impOverExit" data-i18n="aria.close">Close</button>', ''), rs];
    })(), 1],
    ['an exit nobody wired fails', (() => {
      const [h, rs] = full();
      return [h, { ...rs, 'src/impostor_screen.rs': '// nothing' }];
    })(), 1],
    ['a deleted panel fails', (() => {
      const [h, rs] = full();
      return [h.replace(/<div id="cnOver">[\s\S]*?<\/div><\/div>\n/, ''), rs];
    })(), 1],
    ['a way out showing a bare glyph fails', (() => {
      const [h, rs] = full();
      return [`${h}<button class="mode-exit" id="someExit" data-i18n="aria.exit">\u2715</button>`, rs];
    })(), 1],
    ['a way out with no translated label fails', (() => {
      const [h, rs] = full();
      return [`${h}<button class="mode-exit" id="otherExit">Exit</button>`, rs];
    })(), 1],
  ];
  let bad = 0;
  for (const c of cases) {
    const name = c[0];
    const want = c[c.length - 1];
    const [html, rs] = c.length === 4 ? [c[1], c[2]] : [c[1][0], c[1][1]];
    const got = problems(mk(html, rs)).length;
    const ok = got === want;
    if (!ok) bad += 1;
    console.log(`  ${ok ? 'ok    ' : 'FAILED'} ${name} (want ${want}, got ${got})`);
  }
  if (bad) { console.error('mode-exit-check selftest: FAILED'); process.exit(1); }
  console.log('mode-exit-check selftest: OK');
}

const RUN_AS_CLI = import.meta.url === pathToFileURL(process.argv[1] || '').href;
if (RUN_AS_CLI && process.argv.includes('--selftest')) {
  selftest();
} else if (RUN_AS_CLI) {
  const bad = problems();
  if (bad.length) {
    console.error('mode-exit-check: FAILED');
    for (const b of bad) console.error(`  ${b}`);
    console.error('\n  Every mode must have a way out at every point, the end');
    console.error('  screen included. A mode a child cannot leave reads as a');
    console.error('  broken game, not as a missing button.');
    process.exit(1);
  }
  console.log(`mode-exit-check: OK — ${Object.keys(PANELS).length} end screens, all with a wired way out`);
}
