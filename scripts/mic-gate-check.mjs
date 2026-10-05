#!/usr/bin/env node
// CC-SPELLIT-MIC-FIX — the speech gate may never be set ABOVE the fixed value
// it replaced, and it may never be decided once and kept.
//
// The letter VAD compares each buffer's RMS against an ADAPTIVE threshold. That
// threshold is what decides where one spoken letter ends, so getting it wrong
// does not look like a threshold bug from outside — it looks like a letter the
// mic "didn't pick up" and a letter it misheard, because several letters land in
// one recognition cycle and the recognizer returns a word-ish guess.
//
// Two ways it has actually gone wrong on Eric's phone:
//   * build 263: a FIXED 0.010 against a microphone delivering 0.002. Nothing
//     ever crossed it, no boundary ever fired, nothing was finalized.
//   * mic10: calibrated from the first four buffers (~0.34 s, about the length
//     of one spoken letter) while he was already speaking, giving thr=0.0166 —
//     66% ABOVE the fixed value — and he had to scream at the phone.
//
// So two invariants, both cheap to state and both expensive to rediscover:
//   1. the adaptive threshold is CLAMPED into [floor, ceiling], and the ceiling
//      is no higher than the legacy fixed value: adaptation may only ever make
//      the gate more sensitive, never less;
//   2. the floor keeps adapting after the opening window, so a calibration
//      window polluted by speech self-corrects instead of poisoning the session.
//
//   node scripts/mic-gate-check.mjs            # check the real tree
//   node scripts/mic-gate-check.mjs --selftest # prove it bites
import fs from 'node:fs';
import path from 'node:path';

const ROOT = path.dirname(new URL(import.meta.url).pathname) + '/..';
export const PLUGIN = 'ios/App/App/NativeLanguageKitPlugin.swift';

const num = (src, name) => {
  const m = src.match(new RegExp(`${name}\\s*:\\s*Float\\s*=\\s*([0-9.]+)`))
    || src.match(new RegExp(`${name}\\s*=\\s*([0-9.]+)`));
  return m ? Number(m[1]) : null;
};

export function check(src) {
  const bad = [];
  const legacy = num(src, 'speechRMS');          // the fixed value, kept for reference
  const ceiling = num(src, 'speechRMSCeiling');
  const floor = num(src, 'speechRMSFloor');

  if (ceiling === null) {
    bad.push('speechRMSCeiling is gone — nothing stops the adaptive gate climbing above the fixed value');
  } else if (legacy !== null && ceiling > legacy) {
    bad.push(`speechRMSCeiling ${ceiling} is above the legacy fixed ${legacy} — adaptation must only ever make the gate MORE sensitive`);
  }
  if (floor !== null && ceiling !== null && floor > ceiling) {
    bad.push(`speechRMSFloor ${floor} is above speechRMSCeiling ${ceiling} — the clamp is inverted`);
  }

  // Every assignment of the live threshold must be clamped by the ceiling.
  const assigns = src.split('\n')
    .map((line, i) => [i + 1, line])
    .filter(([, l]) => /speechRMSAdaptive\s*=/.test(l) && !l.trim().startsWith('//'));
  if (!assigns.length) bad.push('speechRMSAdaptive is never assigned — the adaptive gate has been removed');
  for (const [n, line] of assigns) {
    // Assigning a bare LITERAL is a default or a per-session reset, not a
    // computed gate: there is nothing to clamp, but the constant itself must
    // still sit inside the band. Only computed values need the clamp.
    const lit = line.match(/speechRMSAdaptive\s*=\s*([0-9.]+)\s*$/);
    if (lit) {
      const v = Number(lit[1]);
      if (ceiling !== null && v > ceiling) {
        bad.push(`${PLUGIN}:${n} sets speechRMSAdaptive to ${v}, above the ceiling ${ceiling}`);
      }
      continue;
    }
    if (!/speechRMSCeiling/.test(line)) {
      bad.push(`${PLUGIN}:${n} sets speechRMSAdaptive without clamping to speechRMSCeiling (${line.trim().slice(0, 60)})`);
    }
  }

  // The floor must keep adapting past the opening window.
  const seeds = /noiseSamples\s*<\s*noiseWindow/.test(src);
  if (seeds && !/}\s*else\s*{\s*\n\s*noiseFloor\s*=/.test(src)) {
    bad.push('noiseFloor is seeded from the opening window with no else branch — a window polluted by speech would poison the whole session');
  }

  // Closing the recognition request per letter is what keeps the recognizer
  // from straddling letter boundaries ("OK" for a spoken K). But closing it too
  // eagerly is build 265: partials=0 on loud, clean audio, because every cycle
  // was killed before it could produce a result. Both halves are load-bearing.
  if (/endAudio\(\)/.test(src)) {
    const minReq = num(src, 'minRequestSecs');
    if (minReq === null) {
      bad.push('the request is closed with no minRequestSecs guard — this is build 265, where every cycle was killed before it could recognize anything');
    } else if (minReq < 0.5) {
      bad.push(`minRequestSecs ${minReq}s is too little context for on-device recognition (keep it >= 0.5)`);
    }
    if (!/guard reqSecs >= minRequestSecs/.test(src)) {
      bad.push('minRequestSecs exists but nothing guards on it — the request can still be closed with too little audio');
    }
    if (!/guard boundaryBySilence else/.test(src)) {
      bad.push('the request is closed without first requiring a SILENCE boundary — the safety timer would close it too');
    }
  }

  // The safety boundary must stay short enough that it cannot swallow letters.
  const maxSeg = num(src, 'maxSegment');
  if (maxSeg === null) bad.push('maxSegment is gone — nothing forces a boundary when the VAD fails');
  else if (maxSeg > 1.5) bad.push(`maxSegment ${maxSeg}s is long enough to swallow several letters into one recognition cycle (keep it <= 1.5)`);

  return { bad };
}

const GOOD = `
    private let speechRMSFloor: Float = 0.0015
    private let speechRMSCeiling: Float = 0.010
    private let speechRMS: Float = 0.010
    private var speechRMSAdaptive: Float = 0.010
    private let maxSegment = 1.4
    private let minRequestSecs = 0.7
        guard boundaryBySilence else { return }
        guard reqSecs >= minRequestSecs else { boundaryHandler?(); return }
        request?.endAudio()
        if noiseSamples < noiseWindow {
            noiseFloor = noiseSamples == 0 ? level : min(noiseFloor, level)
            noiseSamples += 1
        } else {
            noiseFloor = min(noiseFloor * noiseLeak, max(level, speechRMSFloor))
        }
        speechRMSAdaptive = min(max(noiseFloor * noiseMult, speechRMSFloor), speechRMSCeiling)
`;

if (process.argv.includes('--selftest')) {
  const cases = {
    clean: { r: check(GOOD), want: null },
    ceiling_removed: { r: check(GOOD.replace(/private let speechRMSCeiling.*\n/, '').replace(/, speechRMSCeiling\)/, ')')), want: /speechRMSCeiling is gone/ },
    ceiling_above_legacy: { r: check(GOOD.replace('speechRMSCeiling: Float = 0.010', 'speechRMSCeiling: Float = 0.050')), want: /above the legacy fixed/ },
    // The exact mic10 regression: the clamp dropped from the assignment.
    unclamped_assign: { r: check(GOOD.replace('min(max(noiseFloor * noiseMult, speechRMSFloor), speechRMSCeiling)', 'max(noiseFloor * noiseMult, speechRMSFloor)')), want: /without clamping to speechRMSCeiling/ },
    floor_frozen_after_window: { r: check(GOOD.replace(/ } else \{\n            noiseFloor = min\(noiseFloor \* noiseLeak, max\(level, speechRMSFloor\)\)\n        \}/, ' }')), want: /poison the whole session/ },
    inverted_clamp: { r: check(GOOD.replace('speechRMSFloor: Float = 0.0015', 'speechRMSFloor: Float = 0.10')), want: /clamp is inverted/ },
    long_safety_boundary: { r: check(GOOD.replace('maxSegment = 1.4', 'maxSegment = 2.5')), want: /swallow several letters/ },
    reset_to_literal_is_fine: { r: check(GOOD + '\n        speechRMSAdaptive = 0.010\n'), want: null },
    reset_above_ceiling: { r: check(GOOD + '\n        speechRMSAdaptive = 0.030\n'), want: /above the ceiling/ },
    close_without_min_guard: { r: check(GOOD.replace(/ *private let minRequestSecs = 0\.7\n/, '')), want: /this is build 265/ },
    close_with_too_little_context: { r: check(GOOD.replace('minRequestSecs = 0.7', 'minRequestSecs = 0.2')), want: /too little context/ },
    close_on_the_safety_timer: { r: check(GOOD.replace(/ *guard boundaryBySilence else \{ return \}\n/, '')), want: /requiring a SILENCE boundary/ },
    guard_constant_but_unused: { r: check(GOOD.replace('guard reqSecs >= minRequestSecs else { boundaryHandler?(); return }', 'if true { }')), want: /nothing guards on it/ },
    adaptive_removed: { r: check(GOOD.replace(/speechRMSAdaptive.*\n/g, '')), want: /never assigned/ },
  };
  let failed = 0;
  for (const [name, c] of Object.entries(cases)) {
    const ok = c.want === null ? c.r.bad.length === 0 : c.r.bad.some((b) => c.want.test(b));
    const why = c.want === null ? (c.r.bad[0] ?? '') : (c.r.bad.find((b) => c.want.test(b)) ?? `wanted ${c.want}, got ${c.r.bad.join(' | ') || 'nothing'}`);
    console.log(`  ${ok ? (c.want ? 'caught ' : 'clean  ') : 'MISSED '} ${name}${why ? ' — ' + why.slice(0, 70) : ''}`);
    if (!ok) failed++;
  }
  if (failed) { console.error(`mic-gate-check selftest: ${failed} case(s) wrong`); process.exit(1); }
  console.log('mic-gate-check selftest: OK');
  process.exit(0);
}

const src = fs.readFileSync(path.join(ROOT, PLUGIN), 'utf8');
const { bad } = check(src);
if (bad.length) {
  console.error('mic-gate-check: FAILED');
  for (const b of bad) console.error('  ✗ ' + b);
  process.exit(1);
}
console.log('mic-gate-check: OK — the speech gate is clamped at or below the fixed value, the floor keeps adapting, and the safety boundary stays short.');
