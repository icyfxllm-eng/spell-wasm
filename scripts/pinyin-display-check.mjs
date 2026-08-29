// CC-ZH-PINYIN-DISPLAY L1 — no illegal codepoint may reach a player as pinyin.
//
// The device showed 游戏 as `you` with a mark floating beside it. The reveal was
// building display text inline as `segment + <sup>spacing-accent</sup>`, so the
// tone mark was never on a vowel -- it was a separate element holding U+00B4.
// There was no PinyinKey -> display function anywhere to fix; the fix was a
// merge, and this lint is what stops a second builder reappearing.
//
// THREE LAWS.
//
//   L1a  Spacing accents and combining marks are illegal ON THE DISPLAY PATH.
//        U+00AF U+00B4 U+02C7 U+02CA U+02CB U+02D9 and U+0300-U+036F.
//
//        SCOPED TO OUTPUT, DELIBERATELY. The first draft of this lint scanned
//        all of src/ and flagged viet.rs, keyboard.rs, editor.rs and pinyin's
//        own parser tables -- every one of them correct. Combining marks are
//        REQUIRED on the input side: the canonicalizer must accept a player
//        who types a decomposed `yóu`, and the keyboard's long-press builds
//        marks to send. Vietnamese is out of scope for this file entirely.
//        A lint that fails the build on correct code gets disabled, and then
//        it guards nothing. The exhaustive alphabet guarantee lives where it
//        can actually be exhaustive: `l1_only_precomposed_reaches_the_player`
//        runs display_syllable over the whole inventory x 5 tones and asserts
//        every emitted codepoint is legal.
//
//   L1b  ONE builder. `display_syllable` / `display_key` are the only things
//        that turn a syllable into display text. The retired spacing-accent
//        table must stay test-only.
//
//   L1c  No detached mark element. `<sup class="zh-tone">` was the bug made
//        visible; if it comes back, so does the bug.
import fs from "node:fs";
import path from "node:path";

const ROOT = path.dirname(new URL(import.meta.url).pathname) + "/..";
const SELFTEST = process.argv.includes("--selftest");

const SPACING = [0x00af, 0x00b4, 0x02c7, 0x02ca, 0x02cb, 0x02d9];
const NAMES = {
  0x00af: "MACRON", 0x00b4: "ACUTE ACCENT", 0x02c7: "CARON",
  0x02ca: "MODIFIER ACUTE", 0x02cb: "MODIFIER GRAVE", 0x02d9: "DOT ABOVE",
};

/** Strip `#[cfg(test)] mod ... { }` blocks -- brace-matched, not regexed. */
function stripTests(src) {
  let out = "", i = 0;
  while (true) {
    const m = src.indexOf("#[cfg(test)]", i);
    if (m < 0) { out += src.slice(i); break; }
    out += src.slice(i, m);
    let b = src.indexOf("{", m);
    if (b < 0) { break; }
    let depth = 0, j = b;
    for (; j < src.length; j++) {
      if (src[j] === "{") depth++;
      else if (src[j] === "}" && --depth === 0) { j++; break; }
    }
    i = j;
  }
  return out;
}

function evaluate(files) {
  const fails = [];
  for (const [name, raw] of files) {
    const code = stripTests(raw);
    // L1a — the illegal alphabet, as literal chars and as \u escapes.
    for (const cp of SPACING) {
      const lit = String.fromCharCode(cp);
      const esc = new RegExp(`\\\\u\\{${cp.toString(16)}\\}`, "i");
      if (code.includes(lit) || esc.test(code)) {
        fails.push(
          `${name}: spacing accent U+${cp.toString(16).toUpperCase().padStart(4, "0")} ` +
          `(${NAMES[cp]}) in shipped source. A spacing accent is a standalone glyph, ` +
          `not a tone mark on a vowel — this is what rendered 游戏 as "you" + a ` +
          `floating mark. Use pinyin::display_syllable.`
        );
      }
    }
    for (let cp = 0x300; cp <= 0x36f; cp++) {
      const esc = new RegExp(`\\\\u\\{${cp.toString(16)}\\}`, "i");
      if (code.includes(String.fromCharCode(cp)) || esc.test(code)) {
        fails.push(
          `${name}: combining mark U+${cp.toString(16).toUpperCase().padStart(4, "0")} ` +
          `in shipped source. F1 is precomposed-only: one encoding of "yóu", or the ` +
          `match key and the cache key disagree.`
        );
      }
    }
    // L1c — the detached mark element.
    if (/<sup[^>]*class=\\?"zh-tone/.test(code)) {
      fails.push(
        `${name}: <sup class="zh-tone"> is back. The mark belongs ON its vowel, ` +
        `precomposed, inside the syllable span — not in an element beside it.`
      );
    }
  }
  return fails;
}

/** Just the OUTPUT path: the display builders and the surface that renders
 *  them. Everything else in src/ is input handling and is none of F1's
 *  business. */
function displayPathFiles() {
  const out = [];
  const grab = (file, fnNames) => {
    const src = fs.readFileSync(path.join(ROOT, file), "utf8");
    for (const fn of fnNames) {
      const at = src.indexOf(fn);
      if (at < 0) continue;
      let b = src.indexOf("{", at), depth = 0, j = b;
      for (; j < src.length; j++) {
        if (src[j] === "{") depth++;
        else if (src[j] === "}" && --depth === 0) { j++; break; }
      }
      out.push([`${file}::${fn.replace(/fn |const |\(/g, "").trim()}`, src.slice(at, j)]);
    }
  };
  grab("src/pinyin.rs", ["const PRECOMPOSED", "fn is_legal_display_char", "fn tone_vowel_index",
                        "pub fn display_syllable", "pub fn display_key"]);
  grab("src/game.rs", ["fn zh_reveal_html"]);
  out.push(["index.html", fs.readFileSync(path.join(ROOT, "index.html"), "utf8")]);
  return out;
}

const files = displayPathFiles();

// L1b — one builder, and the retired table stays test-only.
const pinyin = fs.readFileSync(path.join(ROOT, "src/pinyin.rs"), "utf8");
const structural = [];
if (!/pub fn display_syllable\(/.test(pinyin))
  structural.push("src/pinyin.rs: display_syllable is gone — F1 requires exactly ONE PinyinKey->display function");
if (!/#\[cfg\(test\)\]\s*pub const TONE_MARKS_DISPLAY/.test(pinyin))
  structural.push("src/pinyin.rs: TONE_MARKS_DISPLAY must stay #[cfg(test)] — it is the retired spacing-accent table");

if (SELFTEST) {
  // The spec's fixture: `you` + U+0301 must fail. A lint that cannot fail is
  // decoration, and this one guards a defect that already shipped once.
  const lesions = [
    ["you + U+0301 (combining acute)", [["fixture.rs", 'let s = "you\\u{301}";']]],
    ["a spacing acute back in source", [["fixture.rs", 'let s = "you\\u{b4}";']]],
    ["the detached <sup> element", [["fixture.rs", '<sup class="zh-tone">x</sup>']]],
    ["a literal combining mark", [["fixture.rs", "let s = \"you\u0301\";"]]],
  ];
  let bad = 0;
  for (const [name, f] of lesions) {
    if (evaluate(f).length === 0) { console.error(`  SURVIVED: ${name}`); bad++; }
  }
  if (bad) {
    console.error(`pinyin-display-check: FAILED\n  ${bad} of ${lesions.length} lesions not caught`);
    process.exit(1);
  }
  console.log(`pinyin-display-check: selftest OK — all ${lesions.length} lesions fail the build`);
  process.exit(0);
}

const fails = [...structural, ...evaluate(files)];
if (fails.length) {
  console.error("pinyin-display-check: FAILED");
  for (const f of fails) console.error("  " + f);
  process.exit(1);
}
console.log(`pinyin-display-check: OK — one display builder, no spacing accents, no combining marks, no detached mark element`);
