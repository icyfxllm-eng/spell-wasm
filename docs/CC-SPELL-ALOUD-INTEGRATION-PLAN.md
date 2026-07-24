# CC-SPELL-ALOUD-INTEGRATION — plan

**Status: DRAFT — REVIEW-GATED.** Branch `feature/spell-aloud-integration`. Nothing
merges, ships, or enters the submission pipeline without Eric's sign-off. **Two hard
conflicts with the parent CC-SPELL-ALOUD block the start** (below) — the spec says
"where the two conflict, STOP AND ASK; do not infer," so no implementation until they
are ruled.

## The important context: most of this already exists (just not where I've been building)

Grounding the spec against the tree changes the shape of the work:

- **Feature 6 (registry gate) is essentially done.** `consts::VOICE_SPELL_LANGS =
  [EN, ES]` + `voice_spell(lang)` is the single language gate; `config/modes.json`
  `spell_aloud` already carries `languages: ["en","es"]` and is `status: "live"`. No
  `if (lang === …)` exists or is needed (I3).
- **Features 1–3, 7, 8 are already implemented by the shipped INPUT METHOD**
  (`src/spell_aloud.rs` `mic_tap`/`on_final`): it serves the **current game word**,
  spells into the **answer field** (so it renders as the **same typed tiles**), and
  submits through the **standard path** (`game::set_answer` → normal submit → standard
  scoring / streaks / missed-words). That is exactly the spec's "standard-mode input
  variant … same submit path … tiles exactly like typed input."
- **What I built this session (CC-SPELL-ALOUD Phases 1–6, builds 60–63)** is a
  *different* thing: a **standalone `#spellAloud` overlay** with its **own** buffer,
  tiles, chip UI, and push-to-talk. That surface **conflicts** with this spec's
  "tiles exactly like typed input / same submit path / standard-mode input variant."

So the integration is mostly: **(a) route the hub tile into real gameplay with the
input method active** (not a separate overlay), **(b) add the pieces the input method
lacks** — the D3 whole-word rule, reserved undo/clear commands with a CI collision
check, the hub "unavailable" state + "type instead" fallback, and the A1–A10
fixtures — **(c) decide the fate of the standalone overlay.**

---

## ⛔ Gate before anything: two conflicts with CC-SPELL-ALOUD (STOP AND ASK)

| Gate | The conflict | Needs from Eric |
|---|---|---|
| **G-INT-1 · Architecture: overlay vs. input-method** | This spec wants the mode to be the **standard game flow with voice input** (typed tiles, same submit, "standard-mode input variant"). CC-SPELL-ALOUD Phases 1–6 built a **standalone `#spellAloud` overlay**. | ✅ **RULED 2026-07-24 — promote the INPUT METHOD.** The hub tile drops into normal play with the voice mic on; the standalone overlay (+ chip/push-to-talk, Phases 1–6) is **retired as superseded**. |
| **G-INT-2 · D3 (target-based rejection) reverses G-A (answer-leak, Phase-0 ruling)** | **Feature 4 / D3:** reject an utterance by matching it **against the target word**; **G-A (Phase 0):** the target is **NEVER** given to the matcher. | ✅ **RULED 2026-07-24 — D3 wins.** Reintroduce the target ONLY to **reject** whole-word utterances (exact/embedded), never to **disambiguate** letters — so G-A's anti-leak intent still holds for letter resolution. Reverses the Phase-0 drop-the-target change in the input method. |

**✅ Both gates ruled — implementation may proceed on this branch (still REVIEW-GATED: no merge/ship without sign-off).**

## Retiring the overlay (per G-INT-1)
The standalone `#spellAloud` mode surface (`src/spell_aloud/screen.rs`, its DOM in
`index.html`, the `dev.rs` Spell-Aloud entry) is **superseded**. It ships hidden behind
the dev door (builds 60–63), so it harms nothing in production; it will be removed as a
cleanup step on this branch. The **parser core** it exercised (`parse`/`events`/lexicons)
is kept — the input method uses it.

---

## Decisions checklist (honored / stop-and-ask)

- **D1** Letter unit = en A–Z; es A–Z + ñ + accent grammar. ✅ already in the lexicons. No jamo.
- **D2** Reject the **target only**; "dog" while spelling "cat" is an ignored token. ✅ (needs the target-compare from G-INT-2; only the exact target rejects).
- **D3** Discard-whole-utterance on embedded target. ⛔ **G-INT-2** — reverses G-A, confirm.
- **D4** No leaderboard / Climb entry. ✅ (add nothing).
- **D5** Kid Mode: available, typed-Kid word filter, Kid-register rejection string. Plan: reuse the typed-Kid gating; **verify** the rejection copy has a Kid variant.
- **D6** Homophones: lexicon authoritative for mapping; Feature 4 wins for the exact target. ✅ lexicons already hold the homophone set.
- **D7** Correction = voice undo/clear + on-screen backspace (typed backspace). Plan: add `undo`/`clear` reserved commands; the current lexicon has `delete`/`clear`/`done` (Phase 2) — **reconcile** (the spec's submit is the *standard control*, not a voice `done`; drop/keep `done`?). Stop-and-ask if `done` should go.

## Features → implementation (after the gates)

1. **Bootstrap from app lang+tier** — read the active game language + tier; no in-mode picker. (This is why the ad-hoc difficulty picker I started was reverted.)
2. **Word serving via the engine** — `words::tier_for(lang, tier)` + the standard selection/no-repeat path; TTS via the existing `api::play_word`; replay button. No mode word list (I1).
3. **Letter capture loop** — the input method's parser fills the answer buffer; tiles are the typed tiles; one standard submit control.
4. **Whole-word rejection (D3)** — per G-INT-2.
5. **Non-letter tokens ignored + reserved undo/clear** — from the lexicon; **CI collision check** (no reserved command equals a letter name).
6. **Registry gate** — `voiceSpell` already = `VOICE_SPELL_LANGS`/`modes.json languages`; add the hub **"unavailable / coming soon"** tile state for non-en/es.
7. **Scoring/streaks/review** — untouched; reached only via the standard submit (I4).
8. **Permission flow** — inline request on entry; denied → existing copy + a **live "type instead"** button that drops into typed standard mode holding the same word.

## Acceptance A1–A10 → where
A1/A4 letter→buffer→correct · A2/A3 D3 rejection (G-INT-2) · A5 D2 ignore · A6 reserved-command CI collision test · A7 registry unavailable tile · A8 word-source parity (200-word) · A9 permission fallback + type-instead · A10 streak-delta parity vs typed control. Harness: the whisper loopback (`tools/spell-aloud-loopback/`) + Rust fixtures.

## Invariants
I1 no own word list · I2 target utterance = 0 letters · I3 registry is the only gate ·
I4 scoring untouched (standard submit only) · I5 NFC before lookup · I6 on-device only.
All either already hold or are covered above.
