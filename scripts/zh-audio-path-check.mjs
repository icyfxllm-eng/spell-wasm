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
// and knows about zh has to prove it excludes zh first.
for (const f of fs.readdirSync(`${ROOT}/src`).filter((n) => n.endsWith(".rs"))) {
  const src = fs.readFileSync(`${ROOT}/src/${f}`, "utf8");
  if (!/speech_out::speak\s*\(/.test(src)) continue;
  const knowsZh = /consts::ZH|"zh"/.test(src);
  if (!knowsZh) continue;
  // Compliant shape: the zh case routes to the forced-reading clip before it
  // can reach the device voice.
  //
  // Proximity is measured on CODE, with line comments stripped first. F5 added
  // a sandhi lookup and four lines explaining it between the guard and the
  // call, which pushed them past the window and failed this check on a change
  // that was entirely correct. A gate that fires on comment length is a gate
  // people learn to route around.
  const code = src.replace(/^\s*\/\/.*$/gm, "");
  const guarded = /==\s*crate::consts::ZH[\s\S]{0,600}?play_word_with/.test(code);
  if (!guarded)
    problems.push(
      `src/${f} can reach speech_out::speak for zh — the device voice cannot be ` +
        `given a reading, so it would guess the polyphone (F6, Invariant 4)`,
    );
}

if (problems.length) {
  console.error(`zh-audio-path-check: FAILED — ${problems.length} problem(s):`);
  for (const p of problems) console.error("  ✗ " + p);
  process.exit(1);
}
console.log(
  "zh-audio-path-check: OK — zh synthesizes only through <phoneme alphabet=\"pinyin\">, " +
    "keyed on reading + voice, and never reaches an on-device voice.",
);

// Selftest: the scan must catch the thing it exists to catch. Feed it the shape
// Bee had before F6 — a zh-aware file speaking through the device voice with no
// forced reading — and confirm it fires.
{
  const VIOLATION = `
fn speak(w: &str, lang: &str) {
    if lang == crate::consts::ZH {
        speech_out::speak(w.split('|').next().unwrap_or(w), 1.0, lang);
    }
}`;
  const knowsZh = /consts::ZH|"zh"/.test(VIOLATION);
  const guarded = /==\s*crate::consts::ZH[\s\S]{0,600}?play_word_with/.test(VIOLATION);
  if (!(knowsZh && /speech_out::speak\s*\(/.test(VIOLATION) && !guarded)) {
    console.error("zh-audio-path-check: SELFTEST FAILED — the scan no longer detects a");
    console.error("  zh word handed to the on-device voice. It is not protecting anything.");
    process.exit(1);
  }
  console.log("zh-audio-path-check: selftest OK — a bare-Hanzi zh path fails the build.");
}
