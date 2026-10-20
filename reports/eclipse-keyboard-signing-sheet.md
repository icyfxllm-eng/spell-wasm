# Signing sheet: CC-VISUAL-ECLIPSE v1 + CC-SPELL-KEYBOARD v1

**Date:** 2026-10-20, origin/main `bed81537`.
**Sources:**
- `reports/visual-eclipse-recon.md` (Eclipse Q1–Q11)
- `reports/spell-keyboard-recon.md` (Keyboard P1–P14, Q1–Q9)

**How to use it:**
- Each row has a recommendation. To accept it, tick the box; to change it, write your call beside it.
- **Part 1 is the minimum to start Eclipse Phase A.** Everything after that can wait until the phase that needs it.
- Once you sign, the agreed amendments go into both spec files and Phase A starts.

**Already done, nothing to sign:** both §0 recons, and the pre-existing bug fixes on main:

| Commit | Fix |
|---|---|
| `91ae1edf` | fonts |
| `e538468b` | Kid Mode sharing and the Yearbook (this was Eclipse Q10) |
| `a210fa93` | Bee and Impostor |
| `bed81537` | screenshot generators and gates |

---

## Part 1 — Needed to start Eclipse Phase A

| ✓ | ID | Question | Recommendation |
|---|---|---|---|
| ☐ | E-Q1 / K-P13 | "After build 56": the iOS project is at 266 and builds up to 285 have shipped. Which build is meant? | *Your call.* If "56" is a typo for a current build, name it. If it's obsolete, delete the clause from both files. |
| ☐ | E-Q4 | The main play screen doesn't fit on SE-size phones: keyboard and Check are below the fold at 320×568 and 375×667, and the result line is below the fold at every size. AT-4 assumes the iOS keyboard, which play screens never use. | **Approve a compact play layout for short screens** (smaller orb, Replay/Slow and glow rows folded, keyboard docked). **Rewrite AT-4:** spellbox, in-app keyboard, Check and result line fully visible at 320, 375, 390 and 440 pt widths. This is a structural change (stop-and-ask 10), so it needs your signature. |
| ☐ | E-Q4b | Landscape is allowed on iPhone, and the play screen is unusable there. | **Lock iPhone to portrait.** iPad keeps all orientations. Making landscape work is a larger layout than this file should take on. |
| ☐ | E-Q4c | Should the fold fix ride along with the current build as a P0 (V-D8 exception)? | **No.** Ship it in Phase A with the token baselines, so the zero-diff migration has one clean "before". |
| ☐ | E-Q5 | The app supports iPad (`TARGETED_DEVICE_FAMILY = 1,2`), but the spec's device matrix is iPhone-only. | **Add iPad 13" to AT-4, AT-19 and the screenshot matrix.** App Store Connect requires an iPad set anyway. |
| ☐ | V-D7 | Icon source. | **Keep the default:** code-authored SVGs, swappable later. |
| ☐ | V-D9 | Token schema. | **Sign as written.** The recon found 33 existing `:root` tokens and no spacing, radius or duration tokens; A1 maps them across at today's values. |
| ☐ | V-D10, V-D11 | Font rules; ≤ 2 MB of bundled fonts. | **Sign.** About 1 MB is in use today (Nastaliq 446 KB of it, for unregistered Urdu: a candidate to drop in A3). |
| ☐ | V-D12 | Motion: durations only from tokens; animate only transform and opacity. | **Sign.** |
| ☐ | V-D21, V-D22, V-D23 | Web parity, flag rules, idle-orb pause. | **Sign.** |
| ☐ | AT-19 device | Frame budget on the "oldest supported device", which is iPhone 6s / SE 1st gen (A9, iOS 15). | **Your call:** supply one, or name the oldest device you have as the stand-in. |

---

## Part 2 — Needed before Eclipse Phase B (orb + feedback) and Keyboard Phase A

| ✓ | ID | Question | Recommendation |
|---|---|---|---|
| ☐ | E-Q2 | **Stop-and-ask 1.** On iOS a cache-hit clip never reaches JS, and the native player exposes no playback clock. | **Option (a):** a side read of the cached mp3, decoded in JS, synced to `play()` resolving plus an offset measured on a device. Playback and the server stay unchanged. If the decode fails, the orb breathes and the failure is logged. AT-8 is then measured on a device. |
| ☐ | V-D13 | Orb sync via a precomputed RMS envelope. | **Sign, with E-Q2 (a)** as the iOS source of the bytes. |
| ☐ | E-Q6 / K-Q6 | Where the feedback policy and `input` fields live. | **Columns in `config/modes.json`,** gated by `modes-check.mjs`. One registry serves both files. **Impossible gets a level-scoped override**, because it's a level of the base game, not a mode. |
| ☐ | K-Q5 | Mixed-entry modes (Practice, Spell Search, Letter Forge, and Quick Bee Draw, which is drawn only for zh and ja). | **Add `input: mixed`,** with an optional per-language override. A mixed mode shows the keyboard only in its typed phase, and that phase logic stays in the mode. |
| ☐ | V-D14, V-D15 | `display_diff` in the Rust core; F4 reveals nothing new. | **Sign.** |
| ☐ | E-Q7 | Red is shipped on wrong answers in DefMatch, Spelldoku, Impostor, Spell Cross and the base orb. | **Amber (`color.caution`) everywhere,** including those four. They aren't protected mode art. Mode-owned scenes (Randospell, BoardGame, Spelluzzle) keep their own palettes. |
| ☐ | E-Q7b | The base orb shakes on a miss. F4 says "a red shake punishes". | **Replace the shake with the F3 `wrong` dim.** Reduce Motion already removes it. |
| ☐ | Base-game Hint | The opt-in Hint shows "N letters" before answering. | **Record it as `lengthBeforeAnswer: on_request`** (a third value). It's existing behaviour, so F4/I6 must neither remove it nor extend it. |

---

## Part 3 — Keyboard re-scope (Keyboard Phase A)

| ✓ | ID | Question | Recommendation |
|---|---|---|---|
| ☐ | K-Q1 | KF1/KF2 assume a keyboard that doesn't exist. The app has its own keyboard (K1), seven other key grids, and layouts for all 15 languages. | **Re-scope:** KF1 = consolidate the ~8 key grids onto K1 (each grid keeps its regression suite). KF2 = review the shipped layouts. `keyboard.layout.<lang>` and K-D27 are dropped, because KT-1 is already a build gate and passes with 0 unreachable characters. |
| ☐ | K-D7 | Korean backspace removes the last jamo. | **Drop as a change:** it already does. KT-6 reuses the existing hangul tests. |
| ☐ | K-D8 / O3 | Japanese layout. | **The shipped hiragana gojūon grid is the ja layout.** Flick or romaji would be a new input method (K-D2) and is out of scope. |
| ☐ | K-D15 | Touch-up commit, slide-to-correct, rollover. | **Narrow it to what's missing:** the preview balloon, rollover, and slide-to-correct between plain keys. Touch-up commit and the 350 ms long-press already ship. |
| ☑ | K-Q2 | vi: KT-7 wants both tone placements graded correct. Grading accepts only the stored one, and the bank holds both spellings as duplicate rows. | ~~(a) Drop "both accepted" from KT-7.~~ **Eric, 2026-10-20: accept both.** A separate grading fix, outside both spec files, makes vi grading placement-insensitive (hoá = hóa) and removes the duplicate rows. KT-7 stays as written and passes once that fix lands. The tone row is unchanged: no auto-placement, so old-style rows stay typeable. |
| ☐ | K-P10 | vi spaces: the keyboard types them, but grading strips all whitespace (VI D8 is still open). | **KT-7 asserts spaces are preserved at the keyboard only.** VI D8 stays with the vi census. |
| ☐ | K-Q3 | zh tone keys: KF2 says 1–4. Today there's a 5-button row (neutral tone 5 is used by 143 rows), and it's absent at Expert. | **Keep the CC-ZH-TONE row as is,** including 5 and hidden at Expert. The buffer stays as digits with marks shown on display. Drop KF2's 1–4. |
| ☐ | K-Q8 / K-D18 / K-D19 | Shift beyond Impossible; key labels show uppercase but type lowercase in every language. | **Shift stays de-Impossible only.** Make key labels **lowercase**, switching to uppercase only while Shift is armed, so labels show what the key types (K-D19). |
| ☐ | K-D17 | Key heights 44 / 52 / 56 pt. | **Sign.** It's close to today's values. Spelluzzle's 40 px keys come up to 44 in its consolidation step. |
| ☐ | K-D1–K-D3, K-D16, K-D20, K-D21 (vi half), K-D22–K-D26, K-D28 | Remaining defaults. | **Sign.** K-D21's zh half follows K-Q3. |
| ☐ | K-Q7 | The orb key is hidden where replay is spent; the existing Replay/Slow buttons are disabled. | **The orb key hides (KF8); the existing buttons stay disabled** as they are. No scope creep into them. |
| ☐ | K-Q9 / K-D6 | Latin legends on non-Latin keys, varying by tier. | **Defer.** It's the only item that puts tier-dependent content on keys, so it sits closest to "the keyboard never helps". Revisit after native feedback (O2). |
| ☐ | O1 | "Use device keyboard" as an accessibility setting. | **Sign the build default,** with autocorrect, autocapitalise and spellcheck off. It also covers the VoiceOver/Switch Control gap on long-press accents (K-R11). |
| ☐ | O2 | Gig B "layout feels natural" checklist row. | **Sign** (advisory only). |

---

## Part 4 — Needed before Eclipse Phase C (sound + haptics) and Keyboard Phase B

| ✓ | ID | Question | Recommendation |
|---|---|---|---|
| ☐ | E-Q3 + K-D4 + K-Q4 | F6 caps haptics at 2 per word, but every keypress fires one today (47 `key_tap` sites, some of them not keystrokes). | **One decision:** keystrokes become their own haptic class, controlled by the "Key feel" toggle and exempt from the cap. Phase C first sorts the 47 sites into keystroke and non-keystroke. Amend Eclipse F6/AT-17 accordingly. |
| ☐ | V-D17 | Haptic table. A wrong answer is two light impacts in the spec; today it's an ERROR notification (MEDIUM for kids). | **Sign the spec table,** and fix the three hard-coded kid flags in Randospell, Quick Bee Draw and Spelluzzle in the same step. |
| ☐ | V-D16 | Game sounds on at 40% (consumer), off in Education Edition. | **Sign.** |
| ☐ | K-D5 | Key clicks muted while TTS plays. | **Sign.** |
| ☐ | Eclipse I7 | Does the keyboard count as part of the input zone (it animates during a word)? | **Yes** (the Keyboard amendment). |

---

## Part 5 — Needed before Eclipse Phases D/E (icons, celebrations, cards, themes)

| ✓ | ID | Question | Recommendation |
|---|---|---|---|
| ☐ | E-Q8 / V-D19 | Unlock criteria vs stored data. Only the Daily streak is retroactive-exact. The lifetime counts are base-game only, and the reset button wipes them. | **Store each unlock once it's earned, and never revoke it.** Grant retroactively from existing data (Daily `best_streak`, base-game counts). Add a new all-modes lifetime counter that the reset button can't wipe, counting from now on. |
| ☐ | E-Q9 / V-D18 | Nothing stores a perfect run, and Climb rank lives on the server only. | **Add two small local records,** for perfect run and rank-up. They're display-only and touch no grading. |
| ☐ | E-Q11 | 🪭 🪬 🪺 can't render on iOS 15.0–15.3 (🪭 needs 16.4). | **Swap the three picture icons.** Don't raise the minimum iOS. |
| ☐ | F11 card | The share card is 1080×1080 with hard-coded English text; F11 says 1080×1350. | **Sign F11 as written.** The new renderer replaces the old card and uses i18n text. |
| ☐ | V-D5, V-D6, V-D20 | Already signed or default. | **No change.** |

---

## Part 6 — Yours alone (outside both files)

| ✓ | Item | What's needed |
|---|---|---|
| ☐ | Telemetry login expired (wrangler 7403) | Run `wrangler login` yourself; I can't enter credentials. |
| ☐ | iOS server clips skip the 1.35 volume boost | Bug or fine? AVAudioPlayer can't go above 1.0, so matching it means routing differently. |
| ☐ | FAIL-verdict words are silent on the web (e.g. "discussed") | Fall back to Web Speech, or keep them silent? |
| ☐ | Batch 5: the app ships three wasm copies (~6.8 MB uncompressed) | Approve dropping the `.br` / `.gz` copies from the app build only. Needs a simulator smoke test. |
| ☐ | Store screenshots | The 2026-07-11 set predates the drawer. The generators now work; regenerate from a release build when you're ready. Eclipse F12 does this properly later. |

**Signature:** ______________________ **Date:** __________
