#!/usr/bin/env node
// Nothing that can BLOCK may run on the audio render thread.
//
// AVAudioEngine delivers microphone buffers on a real-time thread. Work there
// has a hard deadline: miss it and buffer delivery simply stops. Twice now that
// has shipped, and both times the symptom pointed somewhere else entirely:
//
//   * build 266 added `recognizer?.isAvailable` to a diagnostic line inside the
//     tap. That property is backed by a daemon connection; it blocked the render
//     thread and capture stalled at EXACTLY four buffers on loud, clean audio.
//     The reading it produced then became the evidence for a spec revision
//     written against the damage.
//   * `print()` in the same place is file I/O, and had to be reverted too.
//
// Neither was caught by a test, because the code is correct in isolation --
// it is correct everywhere EXCEPT here. So this gate is about a REGION, not an
// expression: the tap closures, and the helpers they call synchronously.
//
// Deliberately NOT forbidden, with reasons:
//   * Self.mlog / os.Logger  — os_log is designed for this and the call sites
//                              are throttled. Changing that is a real decision,
//                              not a lint.
//   * DispatchQueue.main.async — handing work OFF the thread is the fix, not the
//                              problem. `.sync` is forbidden: that is a deadlock.
//   * pendingLock            — a bounded, uncontended NSLock around an array
//                              swap, documented where it is used.
//
//   node scripts/audio-thread-check.mjs            # check the real tree
//   node scripts/audio-thread-check.mjs --selftest # prove it bites
import fs from 'node:fs';
import path from 'node:path';

const ROOT = path.dirname(new URL(import.meta.url).pathname) + '/..';
export const PLUGIN = 'ios/App/App/NativeLanguageKitPlugin.swift';

// Helpers the tap calls synchronously: their bodies are the audio thread too.
const SYNC_HELPERS = ['observeLevel', 'rms', 'yieldToAnalyzer', 'appendServerPCM'];

const BANNED = [
  [/\bprint\s*\(/, 'print() is file I/O on the render thread — it shipped here once and had to be reverted'],
  [/\.isAvailable\b/, 'isAvailable is daemon-backed — this is build 266, where capture stalled at exactly four buffers'],
  [/\.supportsOnDeviceRecognition\b/, 'supportsOnDeviceRecognition is daemon-backed, same hazard as isAvailable'],
  [/AVAudioSession\s*\./, 'AVAudioSession calls can block — route them through AudioSessionOwner, off this thread'],
  [/DispatchQueue\.main\.sync/, 'main.sync from the render thread is a deadlock — use async'],
  [/FileManager|String\(contentsOf|\.write\(to:/, 'file I/O on the render thread'],
  [/Thread\.sleep|usleep\s*\(/, 'sleeping on the render thread'],
];

// Brace-match a body starting from the line that opens it.
function bodyFrom(lines, startIdx) {
  let depth = 0, started = false, out = [];
  for (let i = startIdx; i < lines.length; i++) {
    const l = lines[i];
    out.push([i + 1, l]);
    for (const ch of l) {
      if (ch === '{') { depth++; started = true; }
      else if (ch === '}') depth--;
    }
    if (started && depth <= 0) break;
  }
  return out;
}

export function check(src) {
  const bad = [];
  const lines = src.split('\n');
  const regions = [];

  lines.forEach((l, i) => {
    if (/installTap\s*\(/.test(l)) regions.push(['the audio tap', bodyFrom(lines, i)]);
    for (const h of SYNC_HELPERS) {
      if (new RegExp(`func\\s+${h}\\s*\\(`).test(l)) regions.push([`${h}() (called from the tap)`, bodyFrom(lines, i)]);
    }
  });

  if (!regions.length) {
    bad.push('no audio tap found — if capture moved, this gate must move with it');
    return { bad };
  }
  // The three capture paths plus the helpers; a missing region means a silent hole.
  const taps = regions.filter(([n]) => n === 'the audio tap').length;
  if (taps < 3) bad.push(`only ${taps} audio tap(s) found; there are three capture paths (legacy, analyzer, server) and each one is this thread`);

  for (const [name, body] of regions) {
    // Work handed to another queue has LEFT this thread, and that is the fix
    // rather than the problem: a print inside DispatchQueue.main.async is a
    // main-thread print. Skip those bodies, or the gate flags the remedy.
    let offThread = 0;
    for (const [lineNo, line] of body) {
      const code = line.split('//')[0];          // a comment may NAME the hazard
      if (offThread > 0) {
        for (const ch of code) {
          if (ch === '{') offThread++;
          else if (ch === '}') offThread--;
        }
        if (offThread > 0) continue;
        continue;
      }
      if (/DispatchQueue\.[\w.]+\.async\s*(\(.*\))?\s*\{/.test(code)) {
        offThread = 1;
        // the opening brace is counted above; count any others on this line
        let seen = false;
        for (const ch of code) {
          if (ch === '{') { if (seen) offThread++; else seen = true; }
          else if (ch === '}') offThread--;
        }
        // Skip this line either way: a one-liner that opens AND closes here is
        // entirely off-thread, and offThread has already returned to 0 so the
        // next line is scanned normally.
        if (offThread < 0) offThread = 0;
        continue;
      }
      for (const [re, why] of BANNED) {
        if (re.test(code)) {
          bad.push(`${PLUGIN}:${lineNo} in ${name}: ${why} (${line.trim().slice(0, 48)})`);
        }
      }
    }
  }
  return { bad };
}

const GOOD = `
    private func observeLevel(_ level: Float, seconds: Double) {
        sessionSecs += seconds
        DispatchQueue.main.async { [weak self] in self?.paint() }
    }
    private static func rms(_ buffer: AVAudioPCMBuffer) -> Float {
        return 0
    }
    private func yieldToAnalyzer(_ b: AVAudioPCMBuffer) {
        stream?.yield(b)
    }
    private func appendServerPCM(_ b: AVAudioPCMBuffer) {
        pcm.append(b)
    }
    func a() {
        input.installTap(onBus: 0, bufferSize: 1024, format: format) { [weak self] buffer, _ in
            guard let self = self else { return }
            self.tapCount += 1
        }
    }
    func b() {
        input.installTap(onBus: 0, bufferSize: 1024, format: format) { [weak self] buffer, _ in
            self?.yieldToAnalyzer(buffer)
        }
    }
    func c() {
        input.installTap(onBus: 0, bufferSize: 1024, format: format) { [weak self] buffer, _ in
            self?.appendServerPCM(buffer)
        }
    }
`;

if (process.argv.includes('--selftest')) {
  const cases = {
    clean: { r: check(GOOD), want: null },
    // The exact build-266 regression.
    availability_in_the_tap: { r: check(GOOD.replace('self.tapCount += 1', 'let a = self.recognizer?.isAvailable')), want: /build 266/ },
    print_in_the_tap: { r: check(GOOD.replace('self.tapCount += 1', 'print("buf")')), want: /file I\/O on the render thread/ },
    // The helper bodies are the audio thread too.
    print_in_a_sync_helper: { r: check(GOOD.replace('sessionSecs += seconds', 'print("level")')), want: /observeLevel\(\) \(called from the tap\)/ },
    session_call_in_the_tap: { r: check(GOOD.replace('self.tapCount += 1', 'try? AVAudioSession.sharedInstance().setActive(true)')), want: /AVAudioSession calls can block/ },
    main_sync: { r: check(GOOD.replace('self.tapCount += 1', 'DispatchQueue.main.sync { }')), want: /deadlock/ },
    sleep_in_the_tap: { r: check(GOOD.replace('self.tapCount += 1', 'Thread.sleep(forTimeInterval: 0.1)')), want: /sleeping on the render thread/ },
    // A comment may name the hazard without being the hazard.
    // Handing the work to main is the REMEDY. The gate must not flag it.
    print_inside_main_async_is_fine: { r: check(GOOD.replace('self.tapCount += 1', 'DispatchQueue.main.async { print("ok") }')), want: null },
    multiline_main_async_is_fine: { r: check(GOOD.replace('self.tapCount += 1', 'DispatchQueue.main.async { [weak self] in\n                print("ok")\n                self?.stop()\n            }')), want: null },
    // ...but what follows the async block is on the thread again.
    hazard_after_an_async_block: { r: check(GOOD.replace('self.tapCount += 1', 'DispatchQueue.main.async { print("ok") }\n            print("not ok")')), want: /file I\/O on the render thread/ },
    comment_naming_it_is_fine: { r: check(GOOD.replace('self.tapCount += 1', 'self.tapCount += 1 // never read isAvailable here')), want: null },
    a_tap_went_missing: { r: check(GOOD.replace(/func c\(\)[\s\S]*$/, '')), want: /there are three capture paths/ },
    capture_moved: { r: check('let x = 1'), want: /no audio tap found/ },
  };
  let failed = 0;
  for (const [name, c] of Object.entries(cases)) {
    const ok = c.want === null ? c.r.bad.length === 0 : c.r.bad.some((b) => c.want.test(b));
    const why = c.want === null ? (c.r.bad[0] ?? '') : (c.r.bad.find((b) => c.want.test(b)) ?? `wanted ${c.want}, got ${c.r.bad.join(' | ') || 'nothing'}`);
    console.log(`  ${ok ? (c.want ? 'caught ' : 'clean  ') : 'MISSED '} ${name}${why ? ' — ' + why.slice(0, 70) : ''}`);
    if (!ok) failed++;
  }
  if (failed) { console.error(`audio-thread-check selftest: ${failed} case(s) wrong`); process.exit(1); }
  console.log('audio-thread-check selftest: OK');
  process.exit(0);
}

const { bad } = check(fs.readFileSync(path.join(ROOT, PLUGIN), 'utf8'));
if (bad.length) {
  console.error('audio-thread-check: FAILED');
  for (const b of bad) console.error('  ✗ ' + b);
  process.exit(1);
}
console.log('audio-thread-check: OK — nothing that can block runs on the audio render thread.');
