# CC-SPELL-KEYBOARD v1 — §0 Recon report

**Date:** 2026-10-20. **Tree:** origin/main `575cc38a`. **Read-only:** nothing under `src/`, `assets/`, `config/` or `index.html` was changed.
The bank charset scan was a throwaway script run outside the repo. Its numbers are reproduced in R4.

**Status: STOPPED for Eric's review.** Phases A–E have not started.

**Gate note.** §0 was meant to wait on CC-VISUAL-ECLIPSE being signed. Eclipse is itself stopped at its own §0 (`reports/visual-eclipse-recon.md`), and Eric chose to run this recon anyway. Three Eclipse answers bear directly on this file: Q3 (per-key haptics), Q4 (play-screen fold) and Q6 (feedback registry).

**Baseline.** Per Eric's instruction, the existing on-screen keyboards (`src/keyboard.rs` + `assets/keyboards/*.json`) are the baseline. The spec's description of "today" is wrong in several places, and §1 lists them first.

---

## 1. Where the spec's premise has to change

| # | Spec says | Repo says | Effect on the spec |
|---|---|---|---|
| P1 | Play screens use either the iOS system keyboard or BoardGame's keyboard. | **No play screen uses the system keyboard.** There is no `<input>` in the answer flow (`keyboard.rs:1-6`). The only answer `<input>`s, `#prInput` and `#wpInput`, are `inputmode="none"` (index.html:2909, 3008). The system keyboard opens only on names, auth, parent gate, search, lists, translator and reminders (R2 §8). The custom keyboard exists precisely to keep the system keyboard out of a spelling test (`docs/CC-CJK-INK.md:33`). | KF1's intent ("replace the system keyboard", "layout jumps hide the input") does not apply. Behaviours KF1.2–KF1.3 already hold. **KF1 becomes a consolidation:** fold ~8 separate key grids into one component (P2). Eclipse AT-4 has the same wrong premise (Eclipse Q4). |
| P2 | There is "one shared keyboard" to build, plus BoardGame's. | There is one shared main keyboard (K1, `#gameKeyboard`, lent to Spell Picture) **and seven other key grids**: Bee and Chains (`unit_rows`, no long-press), `boardgame_input` (shared by BoardGame, RandoSpell and Spelluzzle), SpellDoku, Spell Search, Spell Cross, and Practice's tile tray. | The migration surface is ~8 implementations, not 1. KT-19 needs a regression suite per surface, not just BoardGame's. |
| P3 | KF2 gives a "starting table" of layouts to build. | **All 15 shipped languages already have layouts.** They are hardcoded in the Rust `layout_for` registry (`keyboard.rs:178-204`), with JSON mirrors held in sync by a test (`json_layouts_match_rust`, `keyboard.rs:648`). The ja layout is a hiragana gojūon grid that has already shipped (`keyboard.rs:88-100`). | KF2 is a review of shipped layouts, not new work. Differences from the spec's table are in R2-b. K-D8 / O3 ("design ja") is moot unless Eric wants to *change* the shipped ja grid. |
| P4 | KT-1 (typeability) is new. | **It is already a build gate.** `scripts/build-wordlists.py` fails the build on any bank character not on the layout. `every_word_char_is_typeable` and `src/typeable_sweep.rs` check composition. BoardGame has its own typeable test (`boardgame_input.rs:390-418`). The R4 re-scan found **0 unreachable characters in all 15 languages**. | KT-1 already passes on current layouts. K-D27 (a flag per layout, gated on KT-1) gates nothing new. |
| P5 | Keys commit on touch-up, long-press opens at 350 ms (K-D15, K-D16). | Already true. Plain keys commit on `click` (`keyboard.rs:337-348`), `HOLD_MS = 350` (`:370`), with slide-and-release on the popover (`:411-421`). **Missing:** the preview balloon, explicit rollover (one global `HOLD` state, `:366`), and slide-to-correct between plain keys. | K-D15 narrows to: add the balloon, rollover and slide-to-correct. |
| P6 | K-D4 introduces a keystroke haptic class. | `key_tap()` already fires a light impact on every key (`haptics.rs:44-50`), from 47 call sites in 13 files. Some of those are not keystrokes (wrong-answer taps in bee and impostor). There is no settings toggle. | K-D4 *is* the answer to Eclipse Q3. Signing K-D4 and Eclipse F6 has to be one decision. The 47 call sites need sorting into keystroke and non-keystroke before the class can exist. |
| P7 | Shift exists "only where K-D18 says". BoardGame "has no Shift". | A one-shot German Shift already exists, built hidden and shown only for de Impossible (`keyboard.rs:297-300`, `game.rs:654-691`). BoardGame has none, so de is kept out of its Impossible trap pool (`boardgame_pools.rs:144-150`). | Matches K-D18 for de. **Note:** de banks hold 2,809 capitalised rows, but case is graded only in Impossible (`engine.rs:28-37`). Elsewhere `fold_strict` lowercases. Shift is therefore meaningful only in Impossible, unless grading changes, which this file forbids. |
| P8 | KF2 zh: "tone keys 1–4 … place its mark … by the pinyin rule". | zh already has **two** tone affordances: a `12345` digit row inside the keyboard (`keyboard.rs:105-108`) and a five-button tone row `#toneRow` outside it (index.html:2609-2615). The tone row re-tones the last syllable (`pinyin.rs:756-768`) and is **absent at Expert** (`game.rs:586`, CC-ZH-TONE F4). Mark placement already exists, but for display only: the buffer keeps digits and `display_partial` renders `hai2` as `hái` (`pinyin.rs:114-213`). | "1–4" would drop neutral tone 5, which 143 bank rows use. The spec must say whether it replaces the CC-ZH-TONE row, and whether the Expert-hidden rule survives. Changing the buffer from digits to marks touches `pinyin::grade_sandhi_aware` inputs. Grading accepts both forms, but KT-3 parity must be re-proven. |
| P9 | vi: a tone key marks the last typed vowel, and "both conventions are accepted by existing grading". | The first half matches today (`viet::retone`, `viet.rs:17-31`). **The second half is false.** The base game, BoardGame and Bee accept **only the stored placement** (`fold_strict` is accent-strict; the vi homophone table is empty). Only SpellDoku's `vi_split` accepts both. The bank mixes conventions (9 old-style tokens, 74 new-style) and holds both spellings as **separate rows** ("hàng hoá" / "hàng hóa", "văn hoá" / "văn hóa", "sức khoẻ" / "sức khỏe"). | **KT-7 fails as written** unless grading changes, and the Non-goals forbid that. This is **stop-and-ask trigger 6** in spirit. See Q2. |
| P10 | KT-7 "interior spaces are preserved". D1: "the space is part of the answer". | The vi layout has a space key (`keyboard.rs:70-79`), but `fold_strict` **removes all whitespace** for every language (`norm.rs:53-60`), so `họcsinh` = `học sinh`. SpellDoku alone keeps spaces significant (`spelldoku/table.rs:50-52`). The vi census raised this as an open conflict, VI D8 (`reports/vi-bank-census.md:93-114`). | The keyboard preserves spaces, so KT-7's "preserved" holds at the keyboard. Grading ignores them. The spec should say which one it means. |
| P11 | K-D7: Korean backspace should remove the last jamo. | Already true (`hangul.rs:194-223`; 값→갑→가→ㄱ). | Drop K-D7 as a change. KT-6 can reuse the existing hangul tests. |
| P12 | Desktop web IME for ko/ja/zh (R12, KF9.2). | **The web build serves English only** (`is_active_lang`, `consts.rs:352`). | KF9.2's IME work has no web audience today. On iPad with a hardware keyboard it matters (see R12). |
| P13 | "Starts after build 56." | Same as Eclipse Q1: the iOS project is at 266, and builds 280–285 have shipped. | Sequencing can't be checked against build 56. |
| P14 | Builds on CC-IMPOSSIBLE-LAUNCH L-D4, CC-BOARD-GAME D6, CC-VI-BANK-GROWTH D1/D2, CC-BANK-PURITY, CC-QUICKBEE-DRAW. | **None of those CC files is in the repo.** Only their recon or census outputs are. The text of L-D4, D6 and D2 couldn't be checked. | P9 relies on the vi census's reading of D2, not D2 itself. |

---

## 2. Decisions needed before Phase A

| # | Question | Evidence |
|---|---|---|
| Q1 | **Re-scope KF1/KF2** from "build a keyboard and layouts" to "consolidate ~8 key grids onto K1 and review the 15 shipped layouts". The phase plan and flags change as a result: `keyboard.layout.<lang>` gates nothing new. | P1–P4 |
| Q2 | **vi tone placement.** KT-7 needs both conventions graded correct. Grading accepts only the stored one, and the bank holds both spellings as duplicate rows. Options: (a) drop the "both accepted" clause from KT-7 and keep grading as is; (b) a separate grading change outside this file, which would also need a decision on the duplicate rows; (c) another rule. This file can't do (b), because the Non-goals forbid it. Note the typing-order trap: the tone marks the *last* vowel, so h-o-a-tone produces old-style **hoá**, which is wrong against the majority **hóa**. | P9, R7 |
| Q3 | **zh tone keys.** Keep the CC-ZH-TONE five-button row (including 5, absent at Expert) as the zh tone affordance, or replace it with KF2's 1–4? Should the buffer stay as digits with marks shown on display (today), or become marks? | P8, R6 |
| Q4 | **K-D4 and Eclipse F6 together.** Is keystroke haptics a separate class exempt from the cap? Then define keystroke against the 47 `key_tap` sites. | P6, R2 §4 |
| Q5 | **Mode `input` values for mixed modes.** Practice (tap tiles, then typed), Spell Search (tap the grid, then typed), Letter Forge (tile composition) and Quick Bee Draw (drawn only for zh/ja) don't fit one value. Proposal: allow `input` to be a per-phase or per-language override, or add `mixed`. | R1 |
| Q6 | **Where the `input` and `verdict` fields live.** Recommend `config/modes.json` columns gated by `modes-check.mjs`, the same recommendation as Eclipse Q6. Impossible isn't a modes.json entry: it's a level of the base game, so its policy needs a level-scoped field. | R1 |
| Q7 | **KF8 hidden vs disabled.** Today a spent Impossible cap *disables* `replayBtn` and `slowBtn` (`game.rs:1465-1468`), and RandoSpell Ear disables `rsOrb`. KF8 says hidden. Should the orb key follow KF8 while the existing buttons stay disabled, or should everything become hidden? | R9 |
| Q8 | **Shift beyond Impossible.** Case isn't graded outside Impossible, so is Shift shown only in de Impossible (today), or in all de play (it would type capitals that `fold_strict` then folds)? K-D19 says labels show the case typed. Today keycaps display uppercase but type lowercase (`keyboard.rs:271-272`), which violates K-D19 for every language. | P7 |
| Q9 | **K-D6 (Latin legends on non-Latin keys)** has no existing code. Confirm it is still wanted. It is the only K-D item that adds key content depending on tier. | — |

---

## 3. Pre-existing defects found in passing (none fixed)

- **Spell Aloud bypasses the Impossible replay cap.** `spellAloudEnter` calls `game::speak_current` without `note_replay` (`spell_aloud.rs:1058-1063`, `game.rs:1208`). The mode is hidden, so this is latent.
- **SpellDoku Tier-Mode submit bypasses pinyin and `vi_split`**: plain `norm(typed) == spelling` (`spelldoku_ui.rs:1152`).
- **Practice Audio phase may be untypeable on touch.** `#prInput` is `inputmode="none"` and no K1 `surface_hooks` is installed for it. Only Word Picture installs one (`wordpic_screen.rs:1989`). Inferred, not tested on device.
- **`wire_physical` ignores `isComposing`** (`keyboard.rs:576-609`). IME "Process" keys are dropped as multi-character. Only the submit guard (`game.rs:2096`) is composition-aware.
- **The physical keyboard accepts ASCII digits in every language**, not only zh (`keyboard.rs:600`). A vi player typing Telex/VNI digits gets literal digits.
- **All five vi tone keys are labelled "tone mark"** (`keyboard.rs:539`), so VoiceOver can't tell them apart. vi and zh tone taps aren't announced. Backspace announces the English word "delete" in every language (`game.rs:809`).
- **Long-press popovers have no ARIA** and need a pointer hold, so accents are likely unreachable with VoiceOver or Switch Control. Inferred.
- **No Reduce Motion rule for keys.** `.kb-key` transitions and the `:active` translate are outside the reduced-motion block (index.html:1569-1571 vs 2424-2435).
- **Bee and Chains keys have no `aria-label`.** The locked keyboard state is only `opacity`/`pointer-events`, with no `aria-disabled` (index.html:1563).
- **Stale comments:** `keyboard.rs:119-122` and `:155-156` still say ar/hi are unreachable, but both are Active. `docs/rtl-keyboard-split.md` cites tests at `keyboard.rs:655-681` that are now at `:690-721`.
- **Shift on ß** yields `'S'`, not `SS` (`game.rs:688`). It's moot unless a capital-ß word enters de Impossible.
- **The English server grader** does only `.lower().strip()`, with no NFC (`backend/app.py:768-778`). That matters only for en, which is ASCII.

---

## 4. Recon items

### R1 — Modes and how answers are entered

`config/modes.json` has 29 entries: 23 modes plus 6 core surfaces. There is **no input, feedback, verdict or replay field**, and the only behaviour-like field is `exitStyle`. Adding fields is safe: there is no `deny_unknown_fields`, and `modes-check.mjs` checks values, not keys. `src/feedback.rs` holds a 4-state grammar with converters for three outcome types only.

| Mode | Entry today | Proposed `input` |
|---|---|---|
| standard, Misses, Translate "Spell it" | K1 (optional Spell Aloud mic; zh/ja ink pad is practice only, never graded, `game.rs:548-554`) | typed |
| Impossible / Mix (a level, not a mode) | K1 + de Shift | typed |
| climb, daily (incl. Impossible Daily), versus, online_spelloff\*, ghost_racing\* | K1 | typed |
| word_picture | K1 lent into `#wpKbSlot` | typed |
| bee_sim, word_chains | own `unit_rows` grid, no long-press | typed |
| boardgame, randospell, spelluzzle | `boardgame_input` grid | typed |
| spelldoku | own grid + flattened long-press row + vi tone row, plus tap for pencil marks | typed (+tap) |
| spell_cross | own grid (`wordcross_ui.rs:352`) | typed |
| spell_search | drag/tap the grid to find, then type in the lock-in grid | tap + typed (Q5) |
| practice | tap tiles (phases 1–2), `#prInput` (phase 3) | tap → typed (Q5) |
| letter_forge | tap honeycomb tiles | tap (Q5) |
| def_match, impostor | tap a card | tap |
| quickbee_draw | Vision pad, zh/ja only | drawn |
| spell_aloud\* | letter names into the base-game answer | voice |
| syllable_replay\*, word_stories\*, photo_list, my_words, reports, calendar, translate | no answer entry | — |

\* hidden. `wordsearch_ui::layout` covers only en/es/fr/de/pt/pl/ru/fil, and every other language gets the en layout in Spell Search and Spell Cross (`wordsearch_ui.rs:105-116`). Those modes are presumably not offered in other languages; their `languages` field wasn't cross-checked.

### R2 — Existing keyboards (baseline)

**a. Architecture.** The runtime source of truth is Rust `Layout{rows, long_press}` consts (`keyboard.rs:24-162`), chosen by the single `layout_for` match. Unknown locales fall back to `&EN`, which is a **silent fallback**. My Words uses the list's "Speak in" language, and zh falls back to en (`keyboard.rs:208-222`). The JSON format is only `rows` + `longPress`. Shift, the vi tone row, apostrophe/hyphen (My Words only) and the vi space are hardcoded in `keyboard.rs`.

**b. Shipped layouts vs the spec's starting table.**

| Lang | Shipped | Differs from spec |
|---|---|---|
| en, sw | QWERTY | — |
| es | QWERTY + ñ; long-press á é í ó ú/ü | — |
| fr | AZERTY; long-press é è ê ë, à â, ç, î ï, ô œ, ù û ü, ÿ | ç by long-press (spec: same) |
| de | QWERTZ + ü ö ä; ß on long-press s; Shift (Impossible only) | spec: ß as a key |
| pt | QWERTY + ç key; long-press á à â ã, é ê, í, ó ô õ, ú | spec: ç by long-press |
| pl | QWERTY; long-press ą ć ę ł ń ó ś ź ż | — |
| fil | QWERTY + ñ key + hyphen key | spec: ñ by long-press; no hyphen (bank has 114 hyphenated rows) |
| ru | ЙЦУКЕН; ё on long-press е | spec: ё as a key |
| vi | QWERTY + space; ă â ê ô ơ ư đ by long-press; 5-mark tone row | spec: those letters as keys |
| zh | QWERTY + `12345` row; ü on long-press v; separate `#toneRow` | P8 |
| ko | Dubeolsik, tense consonants + ㅒ ㅖ on long-press; live composition | — |
| ja | 5-row hiragana gojūon; dakuten, handakuten and small kana as long-press alternates | spec: blocked on R5 |
| ar | 11/11/7; long-press أ إ آ, ؤ, ئ ى, ة | — |
| hi | 6-row Devanagari incl. matras and signs; aspirates on long-press | — |
| fa | layout exists, not shipped (not in `BUILTIN_LANGS`) | — |

`archive/keyboards/` holds retired it, nb, nl, sv, tr and ur layouts.

**c. BoardGame grid** (`src/boardgame_input.rs`, also used by RandoSpell and Spelluzzle). It must preserve:
- the `data-k` `u:`/`t:`/`m:` encoding (`:97-120`), which e2e clicks rely on
- **bank-derived extra rows** in place of long-press (`:208-242`)
- the ja modifier row ゛ ゜ 小, a toggle on the last kana (`:27-65`), which differs from K1's long-press
- the vi tone row via `retone`
- ko `hangul::feed` / backspace
- zh ü shown as `v`
- keys live only in `AwaitSpelling` and not on an NPC turn (`boardgame_screen.rs:1659-1683`)
- `#bgDel` / `#bgGo` outside the grid
- `#bgField` `aria-live=polite`
- lowercase key faces (K1 shows uppercase)
- 46 px keys (e2e asserts ≥ 45.5 px)

It has **no** Shift, space, apostrophe/hyphen, hardware keys or key `aria-label`s. Spelluzzle renders only unit keys, with no tone or modifier keys (`spelluzzle_screen.rs:954-975`). **Trigger 3 check:** every behaviour listed is reproducible on a shared component, but the ja modifier-row semantics and bank-derived rows are a different model from K1's long-press. Consolidating means choosing one model per language, or supporting both. This is not yet a stop, but it is a design choice Eric should see. Tests: `boardgame_input.rs:390-485`, `tests/e2e/specs/boardgame-layout.mjs`, `boardgame-boost.mjs`.

**d. Key heights.** K1 is min 46 px (Kid/Big Text 56 px). bg/rs keys are 46 px, sz keys 40 px. The e2e floor is only w ≥ 20, h ≥ 30, and that spec tests en only (`tests/e2e/specs/keyboard.mjs:25`). The spec's K-D17 (44 / 52 / 56 pt) is close to the current values. 40 px Spelluzzle keys are below the proposed 44.

**e. Theming.** K1 uses `--panel --edge --ink --amber --amber-soft --muted --body`, with a hardcoded `#36210a` on active keys. `data-lang` overrides `--amber` for ja and zh. The other grids hardcode `#ffffff14`. There are no keycap tokens and no per-theme keycap styling.

### R3 — Input pipeline

- **Key → buffer:**
  - `emit_key` (`game.rs:684-696`) applies the one-shot Shift, then sends ko to `type_jamo` → `hangul::feed` (live recomposition of the whole buffer) and everything else to `type_char` (append).
  - `apply_vi_tone` → `viet::retone` acts on the last character only, returns NFC, and toggles off on a repeat tap.
  - zh `tap_tone` → `pinyin::apply_tone` replaces the trailing digit.
- **Submit:** `answer.trim()` (`game.rs:2105`), then the provenance check, then:
  - en → server `.lower().strip()`
  - Impossible → `engine::grade` (de case-strict; others `fold_strict` + alternates)
  - zh → `pinyin::grade_sandhi_aware`
  - ko → `jamo::grade` (NFC, whitespace dropped, exact)
  - others → `norm::answer_matches`: `fold_strict` (NFC, lowercase, **whitespace removed**, tashkeel/tatweel/ZWNJ removed, accents kept), or `fold_lenient` in Kid Mode (strips combining marks, ß→ss, etc.)
- **Per mode:**
  - BoardGame: same branches, minus Impossible and provenance.
  - Bee: `fold_strict` only, with no Kid leniency and no `jamo::grade`.
  - SpellDoku: own `table::norm` (trim, NFC, lowercase, **interior spaces kept**); zh `pinyin::matches`, vi `vi_split`.
  - Word Search / Cross: `answer_matches`.
- **NFC:** it is applied in the graders (`fold_strict`, `jamo::grade`, `pinyin`, `table::norm`), not at keystroke time. vi `retone` emits NFC by construction. **I1/I3 implication:** a shared keyboard that emits the same characters as `type_char` today keeps parity. KT-3 can be a direct string-vs-engine harness over `game.rs`'s branch order.
- **`input_provenance.rs`** counts keystrokes and the largest chunk per field. It **does not distinguish on-screen from hardware keys**: both log `insertText,1`. So KT-3's three input paths already converge before grading.
- **`fa_canon.rs`** is dead code (fa not shipped).

### R4 — Bank character census (after NFC; source rows are already NFC)

| Lang | Distinct codepoints | Uppercase rows | Space / hyphen / other | Outside script |
|---|---|---|---|---|
| en | 26 | 0 | — | — |
| es | 33 | 0 | — | — |
| fr | 37 (no ë ù ü ÿ æ in banks) | 0 | — | — |
| de | 59 | **2,809** (939 in Impossible) | — | — |
| pt | 37 | 0 | — | — |
| pl | 34 (has v, x; no q) | 0 | — | — |
| fil | 27 | 0 | **hyphen: 114 rows** | — |
| sw | 25 (no q) | 0 | — | — |
| vi | 89 letters + space | 0 | **space: 4,418 of 5,607** | — |
| zh (pinyin half) | 32 (a–z, ü, 1–5) | 0 | tone digit on every syllable; 143 rows use 5; 45 rows spell ü as `v`, 5 as `ü` | — |
| ru | 33 | 0 | — | — |
| ar | 36 | 0 | — | — |
| hi | 57 codepoints / ~371 akshara clusters | 0 | — | — |
| ko | 807 precomposed syllables, no bare jamo | 0 | — | — |
| ja | 79, all hiragana | 0 | — | — |

- **Typeability on shipped layouts: 0 unreachable characters in any language.**
  - de capitals are reached through the fold, or Shift in Impossible.
  - vi tones come from the tone row.
  - ko syllables come from the Dubeolsik automaton, including compound medials and finals.
  - zh ü is on long-press v.
  - hi conjuncts are typed as C + ् + C.
- **Shift derivation (K-D18):** only **de** has uppercase. The spec's table already says so.
- **Trigger 4 / 5 check:** none fire. No row needs a new key type, and KT-1 needs no bank edits on current layouts.

### R5 — Japanese

- **Answers are 100% hiragana in every tier.** There are no katakana, no ー, no kanji, no romaji, and no reading/surface split.
- ja is Active in the app and off on the web.
- The shipped keyboard is a gojūon grid; every bank kana is ≤ 2 taps.
- ja Impossible is locked as `drawn-entry`.
- Quick Bee Draw ja draws the kana row itself (flag `quickbee_draw_ja`, on in build 283 per the release branch).
- **Per trigger 1, no ja layout design was started.** The question for Eric: is the shipped gojūon grid the ja layout, or should ja mirror iOS (12-key flick or romaji)? Flick is arguably a new input method (K-D2).

### R6 — Chinese

- Typed answer = the pinyin half of `pinyin|hanzi` rows, with tone digits. The hanzi is spoken, revealed and drawn.
- **The grader accepts** tone marks (incl. ǖ–ǜ), digits 1–5 (0 → 5), bare syllables as neutral 5 (a value, not a wildcard), `v` and `u:` for ü, spaces and hyphens (dropped), apostrophe as a syllable boundary, full-width, any case. It rejects a mark and a digit on one syllable, and syllables outside the 426-syllable inventory.
- Kid Mode is tone-blind. Sandhi surface forms are accepted at Easy and Medium.
- ü: `lv3`, `lü3` and `lu:3` all canonicalise the same.
- Neutral tone is used in 143 bank rows.

### R7 — Vietnamese

Covered in P9 and P10. Tone entry is the on-screen tone row only (no Telex or VNI). Backspace deletes a toned vowel whole. SpellDoku appends a raw combining mark instead, so its backspace removes only the tone.

### R8 — Arabic and Hindi marks

- **ar:** 0 harakat (U+064B–0652, U+0670) and 0 tatweel in all 6,279 rows. **No harakat keys are needed** (they would be folded out anyway). Hamza and alef forms are all on the layout.
- **hi:** nukta U+093C in 30 rows, always decomposed, with no precomposed nukta letters (consistent with NFC exclusions). Every mark used is a key: ा ि ी ु ू े ै ो ौ ं ़ ् ः ँ, plus ृ on long-press ु. ॅ and ॉ are absent from the banks. ङ ळ ऽ ऋ are on the layout but unused.

### R9 — Replay per mode

| Mode | Control → handler | Rule |
|---|---|---|
| base game family | `orbWrap` / `replayBtn` / `slowBtn` / `rescueSlow` → `game::note_replay` (`game.rs:1455`) | unlimited |
| **Impossible** | same path | `MAX_REPLAYS = 1` on top of the first play (`engine.rs:23`). After the cap, replay and slow are **disabled** (`game.rs:1465-1468`). Bypassed by Spell Aloud (§3). |
| boardgame | `bgOrb` → `orb` (`boardgame_screen.rs:1566-1598`) | unlimited; an Impossible trap word gets 1 via `IMP_REPLAYS` |
| randospell | `rsOrb` → `orb` | unlimited; an **Ear** pick gets 1 play (`randospell/mod.rs:542-551`), orb disabled after |
| **spelluzzle** | tap slot → `Play::select` → `can_hear` (`spelluzzle/play.rs:110-126`) | **silent** slots need a bought Listen; **secret** word is never audible until solved |
| **Par boards** (Spelluzzle F15) | Listen = a stroke | all words silent at load; par = fewest Listens (Expert 3, Hard 2–3, `par.rs:28-33`); re-hearing a listened word is free (`play.rs:91`) |
| def_match | `dmOrb` → `speak_target` | unlimited; **silent in reverse rounds** |
| bee_sim | `beeRepeat` / `beeSlower` | unlimited, but the word stops counting as "clean" |
| word_picture, spelldoku, spell_search, spell_cross, impostor, practice, quickbee_draw | per-mode orb or play buttons | unlimited (Slow restricted by tier in some) |
| word_chains, letter_forge | — | no audio |

Replay caps are **not centralised**: there are four separate mechanisms. KF8 / K-D24 ("call the existing replay action") therefore needs a per-mode adapter, not one call.

**Verdict timing (for K-D23 / KT-15):** per-word verdicts are withheld **only** in Spelluzzle (always) and in SpellDoku logic errors on Hard/Expert. Every other mode shows its verdict immediately. Streak counters exist in the base game (warmth at 3 and 6), Climb shields, Versus, Def Match, Impostor, RandoSpell (×1.5 at 3) and BoardGame (hot streak 5). Bee counts clean words instead.

### R10 — CC files assuming the system keyboard

**Trigger 2 does not fire for play screens.** No CC file mandates the system keyboard there. Hits, all outside play or historical:
- `docs/CC-TRANSLATE-SCREEN.md:75-77, 175` says the Translate search uses the system keyboard and "forbids new keyboard code". This is scoped to Translate.
- `docs/CC-PERSIAN-FOUNDATION.md:32` and `docs/CC-AR-HI-PREAUDIT.md:81` assume the iOS keyboard for fa and ar. They are stale but harmless (fa isn't shipped; ar has a custom layout).

Constraints any keyboard change must respect:
- **CC-SPELLDOKU** reachability invariant for all 15 languages (`docs/CC-SPELLDOKU.md:46-47`), and "reads shared keyboard, modifies none" (`:7`).
- **CC-ZH-TONE F4:** five tone buttons, absent at Expert (P8).
- **CC-ARCADE-BATCH I5:** Official Rules has "no backspace" once a unit is committed. Jamo-level backspace has to respect it.
- **CC-IMPOSSIBLE-v1.2-MIX:28:** "no new input method".
- **`docs/rtl-keyboard-split.md`:** the keyboard JSON doubles as the content charset gate. Consolidation must keep that role.

### R11 — Accessibility today

- Keys are `<button>`s with `aria-label` = the lowercase character. The keyboard is `role=group` with an i18n label, and backspace has a localised label.
- The spellbox is `role=group aria-label="Your spelling"`, with letters as spans. Typed characters are announced through the sr-only `#kbAnnounce` (aria-live polite).
- **Gaps:** see §3 (tone key labels, unannounced tones, English "delete", no popover ARIA, no Reduce Motion for keys, Bee/Chains unlabelled, locked state without `aria-disabled`).
- There is no Switch Control-specific code. Long-press is likely unreachable for switch users, so **trigger 10 is not tested and needs a device pass**.
- The answer field is already non-focusable today, so KF1.2 introduces no new VoiceOver risk.

### R12 — Desktop web and hardware keys

- **Web build serves English only** (P12).
- **Main game:** a window `keydown` handler (`keyboard.rs:576-609`) is active. Enter submits and Backspace deletes. Any single alphabetic, ASCII-digit, `'` or `-` character is emitted, lowercased unless de-Impossible Shift. Ctrl/Meta/Alt are skipped, events from real inputs are ignored, and paste is blocked.
- **No other mode** has hardware typing.
- **IME:** a window `compositionstart` / `compositionend` pair sets `app.composing`, and submit refuses while it is set. `keydown` doesn't check `isComposing`, so composed ko/ja/zh text from an IME is never inserted into the answer (§3). **Inferred from code, not tested on device:** on iPad with a hardware keyboard, ko players likely can't type Hangul through the OS IME today. The on-screen keyboard still works. This is the real KF9.2 target, not desktop web.

---

## 5. Stop-and-ask triggers: status

| # | Trigger | Status |
|---|---|---|
| 1 | Japanese layout | **Raised** (R5): no design done; question posed |
| 2 | CC file mandates the system keyboard | Not fired for play screens (R10) |
| 3 | BoardGame behaviour not reproducible | Not fired; a model choice is noted (R2-c) |
| 4 | Bank row needs a new key type | Not fired (R4) |
| 5 | KT-1 needs bank edits | Not fired (R4) |
| 6 | Grading difference between paths | Not fired between input paths. **KT-7 as written assumes grading that doesn't exist** (P9 / Q2) |
| 7 | Native-speaker report | n/a |
| 8 | Latency budget | Not measured (no device run in §0) |
| 9 | Proposal to add hints / flags / glide | None |
| 10 | Non-focusable field breaks VO / Switch | Field is already non-focusable. Switch Control long-press is unverified (R11) |
