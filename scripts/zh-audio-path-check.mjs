#!/usr/bin/env node
// CC-ZH-TONE F6 — no zh audio is ever synthesized from bare Hanzi.
//
// An isolated word carries no context, so a TTS frontend guesses polyphone
// readings: 行 xíng/háng, 长 cháng/zhǎng, 重 zhòng/chóng, 了 le/liǎo. Naming the
// reading removes the guess. Invariant 4 says any code path that CAN synthesize
// zh without one is a build failure, so this scan looks for the paths, not for
// the symptom.
//
// Three leaks existed when F6 started, and only one was the obvious one:
//   1. the backend synthesized zh as plain text,
//   2. Bee spoke through the browser voice — and handed it the PINYIN, so a
//      Mandarin voice read romanization aloud,
//   3. the audio router fell back to on-device TTS, which cannot be given a
//      reading at all.
import fs from "node:fs";
import path from "node:path";

const ROOT = path.dirname(new URL(import.meta.url).pathname) + "/..";
const problems = [];

// ---- backend: zh must go through <phoneme>, and refuse without a reading ----
const app = fs.readFileSync(`${ROOT}/backend/app.py`, "utf8");
if (!/alphabet=\\?["']pinyin\\?["']/.test(app))
  problems.push("backend/app.py no longer emits <phoneme alphabet=\"pinyin\"> (F6)");
if (!/zh synthesis requires a pinyin reading/.test(app))
  problems.push(
    "backend/app.py no longer REFUSES zh synthesis without a reading — a missing " +
      "reading must be an error, never a fall back to guessing (Invariant 4)",
  );
if (!/def validate_pinyin/.test(app))
  problems.push("backend/app.py lost validate_pinyin — the ph value is interpolated into SSML");
// The cache key must carry the reading and the voice, not the hanzi.
if (!/def zh_cache_key/.test(app) || !/f"zh:\{py\}:\{voice_name\}"/.test(app))
  problems.push("backend/app.py zh cache key must be reading + voice id (F6)");

// ---- client: zh never reaches an on-device voice ----
const api = fs.readFileSync(`${ROOT}/src/api.rs`, "utf8");
if (!/order\.retain\(\|s\| \*s != Source::NativeTts\)/.test(api))
  problems.push(
    "src/api.rs no longer drops the native-TTS rescue for zh — on-device voices " +
      "cannot be handed a reading, so they speak bare Hanzi (Invariant 4)",
  );
if (!/fn play_word_with/.test(api))
  problems.push("src/api.rs lost play_word_with — nothing can pass a reading");
if (!/&py=/.test(api))
  problems.push("src/api.rs no longer sends the py parameter");

// ---- no zh word may be handed to the browser voice ----
// speech_out::speak is the on-device path. Any file that both speaks through it
// and knows about zh must justify EACH call site, at the site:
//
//     // zh-ok(audio): the zh branch above returns through play_word_with
//
// This replaced a proximity heuristic on 2026-09-23. The old scan asked "is
// there a zh guard somewhere within N characters of a play_word_with?", which
// is not a question about the call site being checked. It was wrong in both
// directions: it failed twice on correct code (F5's sandhi lookup, then a
// refactor that moved the guard past the window), and because one matching
// pair anywhere in a file cleared the WHOLE file, it would have passed an
// unguarded call that happened to share a file with a guarded one. Widening
// the window would only have made the second problem worse.
//
// An annotation is a weaker guarantee than a proof and a stronger one than a
// guess: it cannot be satisfied by accident, it is visible in review, it dies
// with the code, and a stale one fails the build. The invariant itself is still
// enforced structurally by the api.rs and backend checks above -- that zh drops
// the native rescue and that the server refuses a reading-less zh clip.
const ANNOT = /\/\/\s*zh-ok\(audio\):\s*(\S.*?)\s*$/;
const ANNOT_ANY = /\/\/\s*zh-ok\(audio\)/;
const SPEAK = /speech_out::speak\s*\(/;
const MIN_REASON = 12;

function scanSpeak(name, text, acc) {
  const lines = text.split("\n");
  lines.forEach((l, i) => { if (ANNOT_ANY.test(l)) acc.all.add(`${name}:${i + 1}`); });
  if (!SPEAK.test(text)) return;
  if (!/consts::ZH|"zh"/.test(text)) return;   // the file cannot route a zh word
  for (let i = 0; i < lines.length; i++) {
    if (!SPEAK.test(lines[i]) || ANNOT_ANY.test(lines[i])) continue;
    acc.sites++;
    // An annotation applies to the next line of CODE, so the search walks up
    // through contiguous comment lines and stops at the first one that is not a
    // comment. An earlier version looked back a fixed three lines, which meant a
    // reason long enough to wrap onto a fourth silently stopped applying -- the
    // same class of magic-number bug this whole change exists to remove.
    let ok = false;
    for (let k = i; k >= 0; k--) {
      if (k !== i && !/^\s*\/\//.test(lines[k])) break;
      const m = lines[k].match(ANNOT);
      if (!m) continue;
      acc.used.add(`${name}:${k + 1}`);
      if (m[1].length < MIN_REASON)
        acc.problems.push(`${name}:${k + 1} zh-ok(audio) needs a real reason, not ${JSON.stringify(m[1])}`);
      ok = true;
      break;
    }
    if (!ok)
      acc.problems.push(
        `${name}:${i + 1} can hand a zh word to the on-device voice — it cannot be ` +
          `given a reading, so it would guess the polyphone (F6, Invariant 4). If zh ` +
          `cannot reach this line, say why at the site: // zh-ok(audio): <reason>`,
      );
  }
}

const acc = { problems: [], sites: 0, used: new Set(), all: new Set() };
for (const f of fs.readdirSync(`${ROOT}/src`).filter((n) => n.endsWith(".rs")))
  scanSpeak(f, fs.readFileSync(`${ROOT}/src/${f}`, "utf8"), acc);
for (const a of acc.all)
  if (!acc.used.has(a)) acc.problems.push(`${a} zh-ok(audio) matches no call site — stale exemption, delete it`);
problems.push(...acc.problems);

if (problems.length) {
  console.error(`zh-audio-path-check: FAILED — ${problems.length} problem(s):`);
  for (const p of problems) console.error("  ✗ " + p);
  process.exit(1);
}
console.log(
  `zh-audio-path-check: OK — zh synthesizes only through <phoneme alphabet="pinyin">, ` +
    `keyed on reading + voice, and never reaches an on-device voice ` +
    `(${acc.sites} device-voice call site(s) justified at the site).`,
);

// Selftest: the scan must catch what it exists to catch, and the exemption must
// be harder to abuse than the window it replaced.
{
  const SPEAKS = `fn say(w: &str, lang: &str) {\n    if lang == crate::consts::ZH {\n        speech_out::speak(w, 1.0, lang);\n    }\n}`;
  const cases = [
    ["a zh-aware file speaking through the device voice", SPEAKS, true],
    ["the same call justified at the site",
      SPEAKS.replace("        speech_out::speak", "        // zh-ok(audio): fixture, zh cannot reach this line at all\n        speech_out::speak"), false],
    ["a justification with no real reason",
      SPEAKS.replace("        speech_out::speak", "        // zh-ok(audio): fine\n        speech_out::speak"), true],
    ["a stale justification matching no call site",
      `// zh-ok(audio): this points at nothing any more\nfn f(lang: &str) { let _ = crate::consts::ZH; }`, true],
    ["a reason long enough to wrap over several comment lines",
      SPEAKS.replace("        speech_out::speak",
        "        // zh-ok(audio): a reason long enough that it wraps, which used to push\n" +
        "        // the marker past a fixed three-line lookback and silently stop\n" +
        "        // applying to the call below it. Four continuation lines,\n" +
        "        // deliberately: three still landed inside the old window\n        speech_out::speak"), false],
    ["a file that speaks but knows nothing of zh",
      `fn say(w: &str) {\n    speech_out::speak(w, 1.0, "en-US");\n}`, false],
  ];
  const bad = [];
  for (const [label, text, shouldFail] of cases) {
    const a = { problems: [], sites: 0, used: new Set(), all: new Set() };
    scanSpeak("fixture.rs", text, a);
    for (const x of a.all) if (!a.used.has(x)) a.problems.push("stale");
    if (a.problems.length > 0 !== shouldFail)
      bad.push(`${label}: expected ${shouldFail ? "a failure" : "a pass"}, got ${a.problems.length} problem(s)`);
  }
  if (bad.length) {
    console.error("zh-audio-path-check: SELFTEST FAILED — it is not protecting anything:");
    for (const b of bad) console.error("  ✗ " + b);
    process.exit(1);
  }
  console.log(`zh-audio-path-check: selftest OK — ${cases.length} fixtures, including a stale exemption.`);
}
