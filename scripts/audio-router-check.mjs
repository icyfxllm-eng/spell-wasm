// CC-AUDIO-REPLAY F1 — the audio router's banner law.
//
// Eric, on device: tapping "hear it again" showed "Couldn't reach the audio
// server for this word" while the word was, in fact, audible.
//
// The router's order is Pack -> ServerCache -> NativeTts. The <audio>
// element's onerror painted the failure banner ITSELF, so a ServerCache miss
// announced total failure and NativeTts then played the word perfectly. The
// banner was true about one source and false about the outcome. Nothing
// cleared it either, so it sat there for the rest of the word.
//
// THE LAW: the banner belongs to the router's wall, not to a source.
// Only `set_source` may paint it, only on "none" (every source exhausted),
// and it must clear on any source that succeeds.
//
// Four corollaries, each a real defect found on that same path:
//   - a cached <audio> element LATCHES its error; reusing one without
//     checking .error() means every replay of that word re-fails against the
//     same corpse and never re-enters the router.
//   - play() returns a Promise that rejects. Discarding it strands the router
//     and records a success that never happened.
//   - but AbortError is NOT a source failure: it means a newer play on the
//     same element superseded ours, so the word IS playing. Advancing the
//     router there stacks native TTS on top of audible speech.
//   - a media element can fire `error` more than once, so onerror must not be
//     a Closure::once — the second fire invokes a consumed FnOnce and aborts
//     wasm.
//
// NOT a law here, deliberately: audio-native.js's LRU skips the unload when
// the evicted clip is the currently-playing one, which reads like a
// map/native desync. It is unreachable — every insertion into `preloaded`
// happens in the same step that sets `current`, so `current` is always the
// most-recently-used entry and can never be the oldest. Confirmed by running
// the bridge against a fake iOS that rejects duplicate preloads. Left alone
// rather than "fixed", because a fix would have been change without a defect.
import fs from "node:fs";
import path from "node:path";

const ROOT = path.dirname(new URL(import.meta.url).pathname) + "/..";
const SELFTEST = process.argv.includes("--selftest");

// Every law, as a pure function of the source text, so the selftest can run
// them against deliberately broken variants.
function evaluate(api) {
  const fails = [];
  const check = (cond, msg) => { if (!cond) fails.push(msg); };
  // Comments discuss the banner at length; strip them so prose neither
  // satisfies nor trips a law.
  const code = api.replace(/^[ \t]*\/\/.*$/gm, "");

  const painters = [...code.matchAll(/voice\.audioFail/g)].length;
  check(painters > 0, "nothing paints voice.audioFail — the failure banner has gone missing entirely");

  const setSource = code.match(/fn set_source\(s: &'static str\) \{([\s\S]*?)\n\}/);
  check(!!setSource,
    "could not find `fn set_source(s: &'static str)` in src/api.rs — the banner law is\n" +
    "  anchored to that function; if it was renamed, re-anchor this gate");

  if (setSource) {
    check(/voice\.audioFail/.test(setSource[1]),
      "voice.audioFail is painted outside set_source — one source's failure must not announce\n" +
      "  total failure; the router still has NativeTts to try after ServerCache");
    check(painters === 1,
      `voice.audioFail is painted in ${painters} places; only set_source's "none" arm may paint it`);
    // Anchored to the GUARDED PAINT, not merely to the presence of "none"
    // somewhere in the body — the debug readout also matches on "none", and
    // matching that instead let two lesions through.
    check(/if s == "none"\s*\{[^}]*set_text\(\s*"voiceNote"\s*,\s*&banner\s*\)/.test(setSource[1]),
      'the banner is not painted under `if s == "none"` — only the router\'s wall, where every\n' +
      "  source has been tried and none played, may announce total failure");
    check(/else[^{]*\{[^}]*set_text\(\s*"voiceNote"\s*,\s*""\s*\)/.test(setSource[1]),
      "set_source never clears voiceNote on a non-\"none\" source — a word that recovers on a\n" +
      "  later source keeps showing the failure banner until the word changes");
  }

  const reuse = code.match(/let already_current[\s\S]{0,500}?\n\n/);
  check(reuse && /\.error\(\)/.test(reuse[0]),
    "the already_current fast path reuses a cached <audio> without checking .error();\n" +
    "  a media element latches its error, so every replay of that word re-fails and the\n" +
    "  router is never re-entered");

  // Scoped to the ROUTER. play_sentence_audio has no fallback chain, so it has
  // nowhere to route a failure; these laws are about the path that does.
  const router = code.match(/fn play_word_html\([\s\S]*?\n\}/);
  check(!!router, "could not find fn play_word_html in src/api.rs — re-anchor this gate");
  check(router && !/let\s+_\s*=\s*audio\.play\(\)/.test(router[0]),
    "audio.play()'s promise is discarded in play_word_html — a rejection never reaches\n" +
    "  on_fail, and the source is recorded as having succeeded before playback started");
  check(router && /"AbortError"/.test(router[0]),
    "play_word_html treats every play() rejection as a source failure. AbortError means a\n" +
    "  NEWER play superseded ours — the word IS playing, and advancing the router there\n" +
    "  stacks native TTS on top of it");

  check(!/let err_cb\s*=\s*Closure::once/.test(code),
    "onerror is a Closure::once — a media element can fire `error` more than once, and the\n" +
    "  second fire invokes a consumed FnOnce, aborting wasm. Use an FnMut.");

  return fails;
}

// ---------- Law 2: the audio session is configured, awaited, and retried ----
//
// focus:true puts AVAudioSession on .playback, which is the ONLY reason word
// audio survives the ring/silent switch. The original fired configure() and
// forgot it — latch set before the promise resolved, rejection swallowed — so
// one failure left the session on .ambient for the whole app session with no
// retry and no log. Silent, permanent, and invisible from outside a debugger.
function evaluateSession(js) {
  const fails = [];
  const check = (cond, msg) => { if (!cond) fails.push(msg); };
  const code = js.replace(/^[ \t]*\/\/.*$/gm, "");

  const fn = code.match(/function ensureSession\([\s\S]*?\n  \}/);
  check(!!fn, "no ensureSession() in audio-native.js — the audio session must be configured in\n" +
    "  one awaited place, not fired and forgotten from plugins()");

  check(/focus:\s*true/.test(code),
    "configure() is not called with focus:true — the session stays .ambient and every word\n" +
    "  is silenced for anyone with their ring/silent switch on");

  if (fn) {
    check(/_sessionReady = null/.test(fn[0]),
      "ensureSession never clears its latch on failure — one failed configure would leave the\n" +
      "  session on .ambient for the entire app session, with no retry");
    check(/warn\(/.test(fn[0]),
      "a failed configure is swallowed silently — this failure mutes the game, so it must be\n" +
      "  logged rather than hidden");
  }

  const play = code.match(/playWord: function[\s\S]*?\n    \},/);
  check(!!play, "could not find playWord in audio-native.js — re-anchor this gate");
  check(play && /ensureSession\(/.test(play[0]),
    "playWord does not await ensureSession — the first word can go out before the category\n" +
    "  is set, which is silent on a muted phone");

  check(!/if \(!_configured\) \{\s*_configured = true;/.test(code),
    "the latch is set before configure() resolves — that is the original fire-and-forget bug");

  return fails;
}

const api = fs.readFileSync(`${ROOT}/src/api.rs`, "utf8");
const js = fs.readFileSync(`${ROOT}/audio-native.js`, "utf8");

if (SELFTEST) {
  // Each law gets its own targeted lesion. A law that survives its lesion is
  // decoration: it would pass a router that lies to the player.
  const LESIONS = [
    ["banner painted by a source, not the router",
      (s) => s.replace(/if s == "none" \{[\s\S]*?\n    \}/, "if s == \"never\" { }")],
    ["banner never cleared on recovery",
      (s) => s.replace(/dom::set_text\("voiceNote", ""\);/, "();")],
    ['banner painted without testing for "none"',
      (s) => s.replace(/if s == "none"/, "if s == \"server-cache\"")],
    ["a dead <audio> element is replayed",
      (s) => s.replace(/&& a\.error\(\)\.is_none\(\)/, "")],
    // These two laws are scoped to play_word_html, so their lesions must land
    // there too. src/api.rs now has a second player (the human-audio path,
    // CC-HUMAN-AUDIO Phase C) with the same code, earlier in the file; a lesion
    // on the first match would mutate a function the law never reads.
    ["play()'s promise discarded",
      (s) => inRouter(s, (r) => r.replace(/match audio\.play\(\) \{/, "let _ = audio.play();\n    match audio.play() {"))],
    ["AbortError treated as a source failure",
      (s) => inRouter(s, (r) => r.replace(/"AbortError"/, '"SomeOtherError"'))],
    ["onerror is a Closure::once",
      (s) => s.replace(/let err_cb = Closure::wrap/, "let err_cb = Closure::once")],
  ]

  // Apply a lesion inside fn play_word_html only.
  function inRouter(src, f) {
    const m = src.match(/fn play_word_html\([\s\S]*?\n\}/);
    return m ? src.replace(m[0], f(m[0])) : src;
  }

  const JS_LESIONS = [
    ["configure latch never cleared on failure",
      (s) => s.replace(/_sessionReady = null; \/\/ not configured.*/, "")],
    ["playWord no longer awaits the session",
      (s) => s.replace(/return ensureSession\(P\)\.catch\(function \(\) \{\}\)\n        \.then\(function \(\) \{ return stopPrev; \}\)/, "return stopPrev")],
    ["session configured without focus:true",
      (s) => s.replace(/focus: true/, "focus: false")],
    ["a failed configure is swallowed silently",
      (s) => s.replace(/warn\("audio session configure FAILED[^;]*;/, ";")],
  ];

  let bad = 0;
  const ALL = LESIONS.map((l) => [...l, api, evaluate])
    .concat(JS_LESIONS.map((l) => [...l, js, evaluateSession]));
  for (const [name, lesion, source, evaluator] of ALL) {
    const mutated = lesion(source);
    if (mutated === source) {
      console.error(`  LESION DID NOT APPLY: ${name} — the code shape changed and this\n` +
                    "    selftest is no longer testing anything. Re-target the lesion.");
      bad++;
      continue;
    }
    if (evaluator(mutated).length === 0) {
      console.error(`  SURVIVED: ${name}`);
      bad++;
    }
  }
  if (bad) {
    console.error(`audio-router-check: FAILED\n  ${bad} of ${ALL.length} lesions were not caught —\n` +
                  "  those laws are decoration and would pass a broken router");
    process.exit(1);
  }
  console.log(`audio-router-check: selftest OK — all ${ALL.length} lesions fail the build`);
  process.exit(0);
}

const fails = [...evaluate(api), ...evaluateSession(js)];
if (fails.length) {
  console.error("audio-router-check: FAILED");
  for (const f of fails) console.error("  " + f);
  process.exit(1);
}
console.log("audio-router-check: OK — the banner is the router's alone, cleared on success, and no source can strand it");
