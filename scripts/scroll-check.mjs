// CC-AUG6-AUDITPASS F4 — the scroll law.
//
// Eric's Aug 6 device audit: "no scrolling at all in Spell Picture",
// which blocked the rest of the audit. The cause was not Spell Picture.
// `.wp-grid` declares `overflow-y:auto` but sits inside a fixed
// flex-column screen, and a flex item's default `min-height:auto`
// refuses to shrink below its content — so the scroller never engages
// and content spills off a fixed parent with nowhere to go. Big Text
// (1.5x) is what pushes a surface over the line, which is why this
// showed up on device and never in a desktop viewport.
//
// Two laws:
//   1. Every `overflow-y:auto|scroll` rule declares `min-height:0`.
//      Harmless when the element is not a flex child, load-bearing when
//      it is — and which one it is depends on runtime layout, so it is
//      required everywhere rather than guessed at.
//   2. Every fixed flex-column surface either scrolls itself or has a
//      DESCENDANT that scrolls. Checked against the real element tree,
//      not against selector names: an earlier draft of this file
//      matched scroller selectors by substring, which passed
//      vacuously and would have shipped the bug it exists to catch.
//
// ALLOW lists surfaces that clip on purpose. Each needs a reason, in a
// diff, written by a person — never a silent default.
import fs from "node:fs";
import path from "node:path";

const ROOT = path.dirname(new URL(import.meta.url).pathname) + "/..";
const css = fs.readFileSync(`${ROOT}/index.html`, "utf8");

const ALLOW = new Map([
  [".scrim", "full-viewport backdrop; never holds flowing content"],
]);

const rules = [...css.matchAll(/\n\s*([.#][\w.,#\s:>-]+)\{([^}]*)\}/g)].map((m) => ({
  sel: m[1].trim(),
  body: m[2].replace(/\s+/g, " "),
}));
const scrolls = (b) => /overflow(-y)?:\s*(auto|scroll)/.test(b);

// Map a bare class/id token -> does any rule for it scroll?
const scrollTokens = new Set();
for (const { sel, body } of rules) {
  if (!scrolls(body)) continue;
  for (const tok of sel.split(",")) {
    const m = tok.trim().match(/^([.#][\w-]+)/);
    if (m) scrollTokens.add(m[1]);
  }
}

const problems = [];
let checked = 0;

// ── Law 1 ────────────────────────────────────────────────────────────
// TWO halves, because the first half ALONE shipped a P0. `min-height:0`
// removes the min-content floor that was holding a flex child open; with
// default flex-shrink and nothing to size it, the element can then
// collapse to zero. That is what happened to `.wp-grid` in ship 138 —
// Spell Picture rendered nothing on build 147. So a scroller must also
// be able to HOLD space: a definite size (max-height/height/flex-basis)
// or flex-grow. Requiring the release valve without the floor is worse
// than requiring neither.
for (const { sel, body } of rules) {
  if (!scrolls(body) || ALLOW.has(sel)) continue;
  checked++;
  if (!/min-height:\s*0/.test(body)) {
    problems.push(
      `${sel} scrolls but never declares min-height:0 — as a flex child its ` +
        `default min-height:auto stops overflow-y from ever engaging`
    );
    continue;
  }
  const sized = /max-height:\s*[^;]/.test(body)
    || /(^|;)\s*height:\s*[^;]/.test(body)
    || /flex(-grow)?:\s*[1-9]/.test(body)
    || /flex:\s*\d+\s+\d+/.test(body)
    // A fixed element pinned on both edges is sized BY THE VIEWPORT, so
    // it has no flex parent to shrink it and cannot collapse.
    || (/position:\s*fixed/.test(body) && /(inset:\s*0|top:[^;]*;[^}]*bottom:)/.test(body));
  if (!sized) {
    problems.push(
      `${sel} declares min-height:0 with nothing to hold its space — no ` +
        `max-height, height, or flex-grow. min-height:0 removes the ` +
        `min-content floor, so as a flex child this can COLLAPSE TO ZERO ` +
        `(that is how .wp-grid emptied Spell Picture in ship 138)`
    );
  }
}

// ── Law 2 ── walk the real element tree ──────────────────────────────
// Minimal well-formed-enough walker: we only need ancestry, and the
// shell is hand-authored HTML with closed containers.
const VOID = new Set(["br", "img", "input", "meta", "link", "hr", "source", "use", "path", "circle"]);
const tags = [...css.matchAll(/<(\/?)([a-zA-Z][\w-]*)([^>]*?)(\/?)>/g)];
const stack = [];
const nodes = []; // {tokens, depth, openIndex}
for (const t of tags) {
  const [, close, name, attrs, selfClose] = t;
  const lname = name.toLowerCase();
  if (VOID.has(lname) || selfClose) continue;
  if (close) {
    for (let i = stack.length - 1; i >= 0; i--) {
      if (stack[i].name === lname) { stack.length = i; break; }
    }
    continue;
  }
  const id = (attrs.match(/\bid="([^"]+)"/) || [])[1];
  const cls = (attrs.match(/\bclass="([^"]+)"/) || [])[1];
  const tokens = [
    ...(id ? [`#${id}`] : []),
    ...(cls ? cls.trim().split(/\s+/).map((c) => `.${c}`) : []),
  ];
  const node = { name: lname, tokens, depth: stack.length, children: [] };
  if (stack.length) stack[stack.length - 1].children.push(node);
  nodes.push(node);
  stack.push(node);
}

const anyDescendantScrolls = (n) =>
  n.children.some((c) => c.tokens.some((t) => scrollTokens.has(t)) || anyDescendantScrolls(c));

for (const { sel, body } of rules) {
  if (!/position:\s*fixed/.test(body) || !/flex-direction:\s*column/.test(body)) continue;
  if (ALLOW.has(sel) || scrolls(body)) continue;
  checked++;
  const token = sel.split(",")[0].trim().match(/^([.#][\w-]+)/)?.[1];
  if (!token) continue;
  const instances = nodes.filter((n) => n.tokens.includes(token));
  if (!instances.length) continue; // styled but never used
  const trapped = instances.filter((n) => !anyDescendantScrolls(n));
  if (trapped.length) {
    problems.push(
      `${sel} is a fixed flex-column surface with no scroller anywhere inside ` +
        `it (${trapped.length}/${instances.length} instance(s)) — content taller ` +
        `than the viewport is unreachable, and Big Text makes that the common case`
    );
  }
}

// ── Law 3 ── a modal must not be nested inside a mode screen ─────────
// #plcCard lived inside #wpPicker, whose .wp-screen is display:none
// until the picker opens. On the game screen the card took `.show` and
// rendered nothing, while next_word() had already handed it the turn —
// the Aug 6 orb dead-tap. A modal outlives the screen it is declared
// in, so it belongs at body level, always.
const SCREEN_TOKENS = [".wp-screen", ".pr-screen", ".dm-screen"];
const walk = (n, ancestors) => {
  for (const c of n.children) {
    if (c.tokens.includes(".wp-modal")) {
      const host = ancestors.find((a) => a.tokens.some((t) => SCREEN_TOKENS.includes(t)));
      if (host) {
        const id = c.tokens.find((t) => t.startsWith("#")) || ".wp-modal";
        const hid = host.tokens.find((t) => t.startsWith("#")) || host.tokens[0];
        problems.push(
          `${id} is a modal nested inside mode screen ${hid} — that screen is ` +
            `display:none until it opens, so the modal can be "shown" while ` +
            `rendering nothing. Move it to body level.`
        );
      }
    }
    walk(c, [c, ...ancestors]);
  }
};
for (const root of nodes.filter((n) => n.depth === 0)) walk(root, [root]);
checked += nodes.filter((n) => n.tokens.includes(".wp-modal")).length;

if (problems.length) {
  console.log("scroll-check: FAILED\n  " + problems.join("\n  "));
  process.exit(1);
}
console.log(
  `scroll-check: OK — ${checked} scrolling surfaces obey the scroll law ` +
    `(${ALLOW.size} allowed to clip by design)`
);
