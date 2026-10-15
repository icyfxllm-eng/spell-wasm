# CC-QUICKBEE-DRAW §0 recon

Run 2026-10-15 against main. Read-only: nothing was changed, no phase started.
The spec's header says "post-build-56"; production is at build 282, so that line
is stale and nothing here is racing a release.

## 1. What exists, and what the spec assumes

| the spec assumes | what the repo has |
|---|---|
| "Quick Bee" is a fixed-time run scored in words per minute, with run-length options (O2) | "Quick Bee" is the localised name of the base game's **timed mode** (`state.timed`, `game.rs::start_timer`, `tier_time(tier)`): a **per-word countdown** that scores a chain (`timed10`, "Reach a 10-chain in Quick Spell"). There is no fixed-time run and no run-length option. CC-BLITZ, which the placement doc cites, is not in the repo. |
| an existing draw plugin that recognises with **ML Kit Digital Ink** and downloads a model per language | The one pad is `src/drawing.rs` + `src/ink_probe.rs`, recognised by **Apple Vision** `VNRecognizeTextRequest` through `NativeLanguageKitPlugin+Ink.swift` (`recognizeInk`). No ML Kit Digital Ink anywhere (the only ML Kit hits are photo-import docs). No model download exists. |
| Korean has a draw flag that is default OFF | There is **no** draw flag for any language. The pad is gated by `ink_probe::ink_allowed(lang)`, which is `zh || ja`, with a test that asserts `!ink_allowed(KO)` (`ink_probe.rs`). Korean is explicitly excluded, not dormant. |
| a per-locale `inputModes` config | No such config. Per-language gating is functions on `consts` (and flags in `flags.rs`, named `spell_flag_<name>`, never per language). |
| flag `quickbee.draw.<lang>` | Flags here are snake_case functions; there is no per-language flag mechanism. |
| a shared no-repeat history that Bee, Word Chains, Impostor and Definition Match use | The shared ledger (`wordsearch::ledger`, key `spell_wordgrid_seen_v1`) is used by Spell Search, Spell Cross, SpellDoku, Board Game and Randospell. Bee, Chains, Impostor and Definition Match do **not** use it (no `ledger` reference in their sources). |
| leaderboards, mode-local, per language | The Climb leaderboard (`backend/climb.py`) is segmented by difficulty and locale. No timed-mode leaderboard exists, and there is no "drawn" segment. |
| culturally matched cell grids (田字格 / grey square / faint cross) | `drawing.rs::draw_guides` draws two dashed Latin guides (baseline and x-height). CC-CJK-INK F2 draws a square frame for CJK. No 田字格, no per-language grid. |

## 2. Conflicts with signed files (each is a STOP-AND-ASK)

1. **Recogniser (CC-CJK-INK D2, Invariants 1 and 3).** The signed design is Vision
   first, stroke-matching as the fallback, ink never leaves the device, and "one
   recognizer path... a second path is a build failure". The spec's ML Kit Digital
   Ink is a second path, and its on-first-use model download is a network request
   the invariant's wording rules out. Either the spec runs on Vision, or CC-CJK-INK
   is amended.
2. **Korean (CC-CJK-INK D1 and Invariant 5).** D1 is "for zh and ja only" and
   Invariant 5 says the pad is never accepted outside CJK scripts, tested for ko.
   Korean was never measured by F1 (zh 96%, ja 88%). Enabling it reverses D1 and
   needs its own F1-style measurement first; it is also the language with no stroke
   table (see census).
3. **Spell Jr (CC-CJK-INK D6, Invariant 6).** "Little Speller does not get the pad"
   and "absent, not disabled". The spec's D6 gives Jr the pad at Easy. That reverses a
   signed decision.
4. **Auto-advance with no confirm button (F1).** The existing flow shows candidates
   and never marks a written character wrong without showing them (CC-CJK-INK
   F3/F4, Done 1: "a drawn round shows the candidates and never marks a correctly
   written character wrong"). A timed loop that ends a word on the first wrong cell
   is the opposite posture and needs a ruling.
5. **Scoring A1 vs measured accuracy.** A1 wants 90% accepted at Medium. F1 measured
   zh 96% and ja 88% (build 202). ja is under the bar before any tuning.
6. **"Same clock options as Quick Bee" and O2.** There are none (per-word timer by
   tier). A fixed-time run scored in cells is a new clock, not a variant of an old one.

## 3. Census: words by cell count and tier

Cells are the stored unit (D3 of Randospell, same here): Hangul syllable blocks for
ko, kana for ja (the bank stores kana, so D4's "stored target form" is kana), hanzi
for zh. Each cell is `total / writing-ready`, where writing-ready is
`stroke_counts::writing_ready` (CC-CJK-INK F4: tier-4 characters of 13+ strokes
are held out because every F1 miss lived there). Pending-purity rows are not
excluded here (the bank files as shipped).

| lang | tier | 1 | 2 | 3 | 4 | 5 | 6 | 7+ |
|---|---|---|---|---|---|---|---|---|
| ko | easy | 77/0 | 379/0 | 89/0 | 13/0 | | | |
| ko | medium | 28/0 | 119/0 | 24/0 | 1/0 | 36/0 | 9/0 | |
| ko | hard | 35/0 | 678/0 | 173/0 | 31/0 | 1/0 | | 16/0 |
| ko | expert | 38/0 | 1006/0 | 332/0 | 67/0 | 7/0 | 1/0 | 4/0 |
| ja | easy | 0 | 51/51 | 67/67 | 70/70 | 17/17 | 4/4 | |
| ja | medium | 0 | 106/106 | 218/218 | 415/415 | 164/164 | 39/39 | 5/5 |
| ja | hard | 0 | 143/143 | 328/328 | 633/633 | 293/293 | 48/48 | 34/34 |
| ja | expert | 0 | 286/286 | 885/885 | 630/630 | 395/395 | 119/119 | 31/31 |
| zh | easy | 160/145 | 136/118 | | | | | |
| zh | medium | 178/162 | 731/642 | 33/28 | 2/2 | | | |
| zh | hard | 274/235 | 1550/1336 | 116/98 | **18/17** | | | |
| zh | expert | 319/262 | 2321/1982 | 267/228 | 77/64 | | | |

Against the spec's tier caps (Easy 1-2, Medium up to 3, Hard and Expert up to 4):

* Every served cell at every tier and language has at least 20 words **except zh
  Hard at 4 cells (18; 17 writing-ready)**. Per F2 that is reported, not padded.
* ko Easy under the cap is 456 words (cells 1 and 2); ja Easy is 51 (two kana only,
  since the bank has no one-kana words); zh Easy is 296. ja Easy is the thin one.
* **ko has no stroke-count entries at all**, so `writing_ready` is false for every
  Hangul block (0 of 12,200). The F4 gate cannot be applied to Korean, and a
  stroke-count-based "this character is complete" signal is not available for it.
  Korean auto-advance would have to be a timing heuristic.
* ja, by contrast, is 100% writing-ready because the bank is kana (kanji, where F1
  was weakest, never appears).

## 4. What I did not run

* A1 (50 recorded drawings per language) and A2 (150 ms p95 on an SE-class phone):
  they need real handwriting and a device. Phase D by the spec.
* Anything that changes a language gate, a flag or the pad.

## 5. Decisions needed before Phase A

1. Recogniser: Vision (as signed) or amend CC-CJK-INK for ML Kit?
2. Korean: reverse CC-CJK-INK D1 for ko, and run an F1-style measurement first?
3. Spell Jr: reverse CC-CJK-INK D6 for this mode?
4. Is the wrong-cell-ends-the-word rule acceptable against CC-CJK-INK F3/F4?
5. The clock: a new fixed-time run (what lengths?) or the existing per-word timer?
6. History: join the shared `wordsearch::ledger`, or a Bee-family store that does
   not exist yet?
7. Leaderboard: a new backend segment ("drawn") on `climb.py`, or local-only?
