# CC-SPELL-ALOUD-INTEGRATION — wire the stub into real gameplay

**Branch:** `feature/spell-aloud-integration` → (target: release branch)
**Status: REVIEW-GATED — DO NOT MERGE/SHIP without Eric's sign-off.** No TestFlight,
no submission-pipeline entry. Awaiting the review-gate **demo video** (below).

## Summary

Build 55 shipped Spell-It-Out-Loud as a dead-end tile. This branch makes it a real
mode: it serves words from the **standard word engine**, renders **typed tiles**,
submits through the **standard submit path**, and scores through **standard scoring** —
the only difference from typed standard mode is the input (spoken letter names). Saying
the whole word is explicitly rejected; you must spell it.

The key realization (grounded in the tree): most of this **already existed** as the
shipped *input method* (the mic beside the answer field) + the `voice_spell` language
gate. So the integration **promoted the input method to a hub mode** and added the
missing rules — rather than building a parallel surface.

## Gate rulings (were blocking; both conflicted with the parent CC-SPELL-ALOUD)

- **G-INT-1 — architecture.** Ruled: **promote the input method.** The hub tile enters
  normal play with the voice mic; the standalone `#spellAloud` overlay (CC-SPELL-ALOUD
  Phases 1–6) is **retired as superseded**.
- **G-INT-2 — D3 vs G-A.** Ruled: **D3 wins.** The target word is used **only to reject**
  whole-word utterances, **never to disambiguate letters** (G-A's anti-leak intent still
  holds for letter resolution). Reversed the Phase-0 "drop the target" change in the
  input method.

## Decisions — honored / stop-and-ask

- [x] **D1** — Letter unit = the language's typing unit (en A–Z; es +ñ +accent grammar). en/es only; no jamo.
- [x] **D2** — Whole-word rejection matches the **target only**; a non-target word ("dog") is an ignored token, not a rejection (A5).
- [x] **D3** — Discard-whole-utterance on embedded target ("cat see ay tee" → void). Strict, no salvage (A3). *(Honored via G-INT-2.)*
- [x] **D4** — No leaderboard, no Climb entry. Nothing added.
- [x] **D5** — Kid Mode: mode available (`kidSafe:true`), word filtering via the standard source; *(follow-up: the rejection copy could use the Kid-register string set — flagged, not blocking).*
- [x] **D6** — Homophones: lexicon authoritative for mapping; Feature 4 wins for the exact target ("sea"==target → reject; "see" → C).
- [x] **D7** — Correction = voice undo/clear + on-screen backspace; undo behaves like typed backspace (grapheme-aware). Submission is the on-screen control (voice `done` is *not* a command).

**No decision was silently inferred; the two conflicts (G-INT-1/2) were stopped-and-asked.**

## Invariants

- [x] **I1** — Mode owns no word list (A8: structural + 200-draw parity over the standard pool).
- [x] **I2** — A target utterance contributes zero letters (`says_target`, A2/A3).
- [x] **I3** — `voice_spell` registry gate is the only language gate; no new `if lang===`.
- [x] **I4** — Scoring untouched; voice reaches it only via the standard submit (A10: the module has no scoring/streak call; same submitted string as typing).
- [x] **I5** — NFC before lexicon lookup (A4: "niño" byte-exact, ñ = U+00F1).
- [x] **I6** — On-device recognition only; no audio/transcript leaves the device (parent spec; `requiresOnDeviceRecognition = true`).

## Acceptance tests

| # | Test | Where | Status |
|---|---|---|---|
| A1 | "cat" ← see/ay/tee → correct | `loopback::a1_cat_en_letter_by_letter` | ✅ |
| A2 | "cat" utterance → rejected, no miss | `tests::d3_says_target_…` | ✅ |
| A3 | embedded "cat see ay tee" → whole utterance void | `tests::d3_says_target_…` | ✅ |
| A4 | "niño" ← ene/i/eñe/o, NFC | `loopback::a4_nino_es_letter_by_letter_nfc` | ✅ |
| A5 | "dog" vs "cat" → ignored token | `tests::d3_says_target_…` | ✅ |
| A6 | reserved-command CI collision + voice undo/clear | `tests::a6_*` | ✅ |
| A7 | `voiceSpell:false` lang → unavailable tile, unreachable | `play_hub::a7_*` | ✅ |
| A8 | word-source parity (200 words in standard list) | `tests::a8_word_source_parity_en_medium` | ✅ |
| A9 | permission fallback + "type instead" holds the word | `tests::a9_*` + browser-verified | ✅ |
| A10 | correct voice word increments streak == typed | `tests::a10_*` (mechanism) | ✅ |

**All offline-provable acceptance is green.** The end-to-end streak delta (A10) and the
per-utterance capture (A1/A4) are proven at the parser/mechanism level here; the live
on-device capture is exercised by the **demo video** (below) and the whisper harness
(`tools/spell-aloud-loopback/`, offline/manual, gated on whisper.cpp being installed).

## Constraints / non-goals — respected

- [x] Scoring / streak / missed-words implementations untouched (consumed via existing interfaces only).
- [x] Say-It pronunciation mode unmodified; no shared-mic-plumbing changes beyond the additive `letterFinal` payload already shipped.
- [x] No languages beyond en/es; no speculative jamo/kana/akshara.
- [x] No leaderboard, no Climb hook, no share cards.
- [x] No server-side recognition.
- [x] Entitlement depth unchanged (follows standard mode; this branch grants nothing).

## Changes

`git log build-56-activation..feature/spell-aloud-integration`:

- `99f0426` plan (REVIEW-GATED, two blocking conflicts)
- `8818bcc` D3 whole-word rejection + A6 command CI gate
- `8c0bfdb` hub routing + unavailable tile (A7)
- `a6d393c` kidSafe true (D5) + retire the standalone overlay (G-INT-1)
- `e729d49` A8 word-source parity (I1)
- `e08e11b` A10 streak parity — the mechanism (I4)
- `18afc96` A6 runtime — voice undo/clear (D7)
- `314acf3` A9 permission fallback + live "type instead"
- `f63ada8` A1/A4 loopback fixtures

Net: **+532 / −444** across 23 files; the overlay `src/spell_aloud/screen.rs` (−374)
removed. Parser core (`parse`/`interpret`/`events`/`says_target` + lexicons) kept.

## Test status

`cargo test` **full lib suite 284/0** (spell_aloud 47/47, play_hub 3/3). i18n 325-key
parity; entitlement checks + `npm run build` green; wasm release builds clean.

## ⛔ Review gate — still owed before Eric reviews

- [ ] **Demo video** (en + es): one clean spell, one undo, one whole-word rejection.
  Requires a device/build — I cannot capture it. (Ship a build on request.)

## Reviewer notes / open follow-ups (non-blocking)

- **D5 rejection copy**: currently the standard `voiceSpell.spellItOut`; a Kid-register
  variant would be a nicety.
- The **confusable/chip parser features** (Phase 3/4: `decide_letter`, `Event::Chip`,
  ambiguous b/v) remain in the parser as tested API but are **not surfaced** by the
  input method (no chip UI in typed tiles). D6 auto-resolves es "be"→b. Kept for a
  possible future chip; remove if you'd rather not carry them.
