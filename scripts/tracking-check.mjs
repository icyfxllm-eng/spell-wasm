// CC-LOCALE-TYPESET F2 (the static half) — the joined-script tracking law.
//
// Tracking on a joining script is a CORRECTNESS bug, not a taste one: it
// inserts space BETWEEN characters, which prises Arabic cursive joins apart
// and detaches Devanagari matras from their base consonant. index.html says
// so itself, on `.ltr.joined`.
//
// That was known and fixed twice — the answer path and `.feedback .reveal` —
// and nowhere else, because nothing was checking. Eleven localized surfaces
// shipped tracked, including the Arabic home-screen tagline
// (اسمعها · تهجَّاها · واصِل السلسلة). Per-bug fixes do not generalise; a lint
// does.
//
// The law: if an element carries localized text and its rule sets a nonzero
// letter-spacing, a companion rule must reset it to normal for every joining
// script. Localized means EITHER a data-i18n attribute OR an id written by
// dom::set_text — the second matters, because #brandTag is set that way and a
// data-i18n-only scan misses it, which is how the tagline survived.
//
// This is the static approximation of F2. The real thing resolves through the
// F1 token cascade and can check computed values; that needs D4. This catches
// the bug class today without pre-empting it.
import fs from "node:fs";
import path from "node:path";

const ROOT = path.dirname(new URL(import.meta.url).pathname) + "/..";
const JOINED = ["ar", "fa", "ur", "hi"];

const raw = fs.readFileSync(`${ROOT}/index.html`, "utf8");
// Strip comments first: an earlier draft captured a comment block as part of a
// selector and reported it as a violation.
const html = raw.replace(/\/\*[\s\S]*?\*\//g, "");

// ── every rule that touches letter-spacing ──────────────────────────────────
const tracked = new Map();   // base selector -> the declaration, nonzero only
const covered = new Map();   // base selector -> Set(langs reset to normal)

for (const m of html.matchAll(/(^|[}\n])\s*([^{}@\n][^{}]*)\{([^}]*)\}/g)) {
  const body = m[3];
  if (!/letter-spacing/.test(body)) continue;
  const isZero = /letter-spacing\s*:\s*(normal|0)\b/.test(body);
  for (const sel of m[2].split(",").map((s) => s.trim()).filter(Boolean)) {
    const lang = sel.match(/:lang\(([a-z-]+)\)/);
    const base = sel.replace(/:lang\([a-z-]+\)/g, "").trim();
    if (isZero && lang) {
      if (!covered.has(base)) covered.set(base, new Set());
      covered.get(base).add(lang[1]);
    } else if (!isZero) {
      tracked.set(base, sel);
    }
  }
}

// ── every element that carries localized text ───────────────────────────────
const locIds = new Set();
const locCls = new Set();
for (const tag of html.matchAll(/<[^>]*data-i18n[^>]*>/g)) {
  const id = tag[0].match(/id="([^"]*)"/);
  const cl = tag[0].match(/class="([^"]*)"/);
  if (id) locIds.add(id[1]);
  if (cl) cl[1].split(/\s+/).forEach((c) => c && locCls.add(c));
}
const rs = fs.readdirSync(`${ROOT}/src`).filter((f) => f.endsWith(".rs"))
  .map((f) => fs.readFileSync(`${ROOT}/src/${f}`, "utf8")).join("\n");
for (const m of rs.matchAll(/set_text\("([A-Za-z0-9_]+)"/g)) locIds.add(m[1]);
// resolve those ids to their classes in the shell
for (const id of [...locIds]) {
  const tag = html.match(new RegExp(`<[^>]*id="${id}"[^>]*>`));
  const cl = tag && tag[0].match(/class="([^"]*)"/);
  if (cl) cl[1].split(/\s+/).forEach((c) => c && locCls.add(c));
}

// ── the law ─────────────────────────────────────────────────────────────────
let failed = false;
let checked = 0;
for (const [base, sel] of [...tracked].sort()) {
  const toks = [...base.matchAll(/[.#]([\w-]+)/g)].map((t) => t[1]);
  if (!toks.length) continue;
  const localized = toks.some((t) => locCls.has(t) || locIds.has(t));
  if (!localized) continue;
  checked++;
  const have = covered.get(base) ?? new Set();
  const missing = JOINED.filter((l) => !have.has(l));
  if (missing.length) {
    console.error(
      `FAIL ${sel} sets tracking on localized text with no reset for: ${missing.join(", ")}`
    );
    failed = true;
  }
}

if (!checked) {
  console.error("FAIL: found no localized tracked rules at all — this check is vacuous");
  failed = true;
}
if (!failed) {
  console.log(`tracking-check: OK — ${checked} localized tracked rules, all reset for ${JOINED.join("/")}`);
}
process.exit(failed ? 1 : 0);
