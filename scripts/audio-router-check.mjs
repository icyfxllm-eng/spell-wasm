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
    check(/set_text\(\s*"voiceNote"\s*,\s*""\s*\)/.test(setSource[1]),
      "set_source never clears voiceNote — a word that recovers on a later source keeps\n" +
      "  showing the failure banner until the word changes");
    check(/"none"/.test(setSource[1]),
      'set_source paints the banner without testing for the "none" source — it would fire\n' +
      "  on success too");
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

const api = fs.readFileSync(`${ROOT}/src/api.rs`, "utf8");

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
    ["play()'s promise discarded",
      (s) => s.replace(/match audio\.play\(\) \{/, "let _ = audio.play();\n    match audio.play() {")],
    ["AbortError treated as a source failure",
      (s) => s.replace(/"AbortError"/, '"SomeOtherError"')],
    ["onerror is a Closure::once",
      (s) => s.replace(/let err_cb = Closure::wrap/, "let err_cb = Closure::once")],
  ];

  let bad = 0;
  for (const [name, lesion] of LESIONS) {
    const mutated = lesion(api);
    if (mutated === api) {
      console.error(`  LESION DID NOT APPLY: ${name} — the code shape changed and this\n` +
                    "    selftest is no longer testing anything. Re-target the lesion.");
      bad++;
      continue;
    }
    if (evaluate(mutated).length === 0) {
      console.error(`  SURVIVED: ${name}`);
      bad++;
    }
  }
  if (bad) {
    console.error(`audio-router-check: FAILED\n  ${bad} of ${LESIONS.length} lesions were not caught —\n` +
                  "  those laws are decoration and would pass a broken router");
    process.exit(1);
  }
  console.log(`audio-router-check: selftest OK — all ${LESIONS.length} lesions fail the build`);
  process.exit(0);
}

const fails = evaluate(api);
if (fails.length) {
  console.error("audio-router-check: FAILED");
  for (const f of fails) console.error("  " + f);
  process.exit(1);
}
console.log("audio-router-check: OK — the banner is the router's alone, cleared on success, and no source can strand it");
