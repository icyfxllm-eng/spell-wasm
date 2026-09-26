# CC-FEEDBACK §0 census

Run 2026-09-25 against `cc-sd-fit` at the merge of origin/main. Phase A only:
no code changed, nothing built.

**Two HALT conditions fire (C1, C3) and one near-miss is worth your eye (C4).**
C1 is the one that changes the shape of the work.

---

## C1 — Outcome enums — **HALT**

> HALT if two modes produce outcomes from different enums with no shared type.

There is no shared type. There is no outcome type *at all* for the base game.

| Path | What it produces |
|---|---|
| `game.rs::submit_guess` (base, Daily, Climb) | **nothing.** An internal `bool`, plus meaning carried in thread-locals (`VALID_OTHER_SENSE`, `ZH_VERDICT`, `ZH_VIA_SURFACE`), then a branch to `on_correct` / `finalize_incorrect_ex(.., feedback_class: &str, ..)` |
| `spelldoku/play.rs::judge` | `Verdict { Correct, Misspelled, WrongValue, WrongSystem }` |
| `spell_aloud.rs::interpret` | `SpellOutcome { Insert, WholeWord, Nothing }` — an input-interpretation type, not a verdict |
| `defmatch.rs::climb_outcome` | `bool` |
| `pinyin.rs` | `WordVerdict` / `SyllableVerdict` (zh detail, consumed inside `game.rs`) |
| `reports.rs::classify_miss` | `MissClass { Substitution, Omission, Insertion, Transposition }` — orthogonal: it classifies a miss after the fact |

Six paths, six shapes, no common supertype.

**Why this is structural, not cosmetic.** F1 specifies "an exhaustive Rust
`match` in the core" with "no `_ =>` wildcard arm". There is nothing to match
on. Phase B would first have to *create* a unified outcome type and thread it
through every grading path — and the file's own header says "Grading logic is
NOT touched". Those two cannot both hold. Unifying outcomes IS a change to
grading code, even if the verdicts it computes stay identical.

The closest thing to an existing carrier is the `feedback_class: &str` argument
already threaded into `finalize_incorrect`, which is a CSS class name chosen at
the call site — the very "modes invent their own feedback" that F1 exists to
end, but also proof the plumbing route exists.

**Two variants have no row in F1's table.** SpellDoku's `WrongValue` (correctly
spelled, wrong cell) and `WrongSystem` (right value, wrong counting system) are
not CORRECT, not MISSPELLED, not near-miss, not `valid_other_sense`, not
TRAP_MISS, not VOIDED. The player spelled correctly and placed wrong. Mapping
them to `miss` would tell a player their spelling was wrong when it was not —
which is the exact lie the four-state grammar was designed to avoid. My read is
`close`, but it is your call and it needs a row before Phase B.

## C2 — Existing feedback

Scattered across at least four owners; Phase B has to absorb all of them.

- `game.rs` — `feedback_class` string set at **five** sites via
  `dom::el("feedback").set_class_name(..)` (2230, 2238, 2244, 2251, 2254)
- `index.html` — `@keyframes shake` (882), `.feedback` / `.feedback .reveal`
  (1035–1036), `.dm-card.wrong` (357), `.imp-card.right` / `.imp-card.wrong`
  (434–435), `.sd-cell.flash` + `@keyframes sd-flash` (1371, 1400)
- `forge_screen.rs`, `chains_screen.rs` — a "grey shake" each, documented as
  deliberately costless
- `haptics.rs` — already has `correct()`, `key_tap()`, `incorrect(kid: bool)`

**No answer-feedback SOUND exists today.** F3 is entirely new; F6 is largely
already built, and `incorrect(kid)` already takes the Jr flag, so D2 has
precedent in the codebase.

## C3 — Audio session split — **HALT (cannot be answered here)**

> Prove with a device test, not documentation.

I cannot run it. The split needs a physical device with a real ring/silent
switch; the iOS Simulator has no such switch to toggle, so a simulator result
would prove nothing. Per §0 this is a stop-and-ask rather than a fallback
choice, so I am not picking one.

What I can report: the app uses `@capacitor-community/native-audio` plus an
`AudioContext`/`GainNode` in `src/audio_boost.rs`, whose own comment notes that
creating an AudioContext is treated as a fingerprinting signal by hardened
browsers — so F3's "pre-decoded AudioContext buffers" must reuse that shared
context rather than open a second one.

## C4 — Word-audio hook — passes, with one qualification

For bank words in built-in languages there is exactly one choke point:
`api::play_word_with` → `play_chain` (`src/api.rs`). This was established
independently two days ago and is now enforced by
`scripts/zh-audio-path-check.mjs`: `game.rs::speak_word` routes every
`is_builtin_lang` language through `play_word_with` and returns before any
device-voice path.

Three `speech_out::speak` sites do bypass the resolver, and all three are
non-bank audio, each justified at the site by a `zh-ok(audio)` annotation:

- My Words — the player's own text, on-device only by the COPPA posture
- the meaning panel's English example sentence (`game.rs:1020`, hardcoded en-US)
- the Spanish syllable reveal on web (`game.rs:2321`, hardcoded es-ES)

So F3's gap rule can be enforced at one place for bank words. The three above
are sounds the gap rule should also respect, and they are not reachable from
the resolver's observation point.

## C5 — Normalizer

`tools/audio-loudness/loudness.py`. Also `tools/human-audio/qc.py` and
`bundle.py` reference the loudness metrics. Config lives at
`config/audio-loudness.json`. A4's "≥6 LU below word audio" measures against the
per-language word clips, which exist only on the server for most languages —
so A4 is a CI check against the bank, not something reproducible offline today.

## C6 — Jr resolver

Merged. `src/experience.rs` resolves `Experience::Junior` / `Standard` with a
`Resolved { experience, locked, source }`, including an `AgeGate` source that
locks it. F2/F8 can depend on it directly; no fixture resolver needed.

## C7 — Theme tokens

The red token is `--bad` (fallback `#e0564f`, and `#ff6b7a` in one place). Used
in game surfaces at `index.html` 326, 357, 783, 1225, 1371, 1400 — including
SpellDoku's flash and Definition Match's wrong card. There are no
`--feedback-*` tokens yet; F2 introduces all four.

**I6 will need every one of those six sites checked under a Jr profile**, not
just the new tokens, because `.sd-cell.flash` and `.dm-card.wrong` already
render red today in modes a Jr player can reach.

## C8 — Motion + haptics

- `@capacitor/haptics ^8.0.2` is installed, and `src/haptics.rs` already wraps
  it through `window.Capacitor.Plugins.Haptics`, fire-and-forget.
- Reduce Motion is read in CSS at `index.html` 482, 516, 669, and once from Rust
  in `defmatch_screen.rs:54` via `match_media`. There is no central accessor —
  F2 needs one, or the fourth reader will drift from the other three.

## C9 — Reveal surface

Partial. `.feedback .reveal` exists in the base game (styled at
`index.html:1036`, with a `:lang(hi)` tracking reset already). `forge_screen.rs`
has `reveal()`, `impostor_screen.rs` has `reveal_why()`. `void_round.rs` has
structural tests pinning that the VOIDED reveal never reaches an outcome path or
the answer field — F5 must not break those.

No reveal in Spell Search, Spell Cross, or SpellDoku spelling commits.

## C10 — Mode list — larger than the file assumes

`config/modes.json` registers **20** modes, not the ~10 §0 expected:

practice, ghost_racing, syllable_replay, say_it, photo_list, spell_aloud,
word_stories, online_spelloff, def_match, letter_forge, word_chains, impostor,
bee_sim, word_picture, reports, calendar, translate, spelldoku, spell_search,
spell_cross

Removing the non-grading ones (reports, calendar) still leaves ~18 grading or
choosing surfaces. Since C10 "becomes the Phase B test matrix", A1's fixture
grid is roughly twice the size the file budgeted for.

---

## What I recommend before Phase B

1. **Decide C1.** Either the spec's "grading logic is NOT touched" relaxes to
   permit introducing a shared outcome type, or F1 becomes an adapter per mode
   (each grading path maps its own result to a state), which keeps grading
   untouched but gives up the exhaustive-match guarantee of I1.
2. **Rule on `WrongValue` / `WrongSystem`.**
3. **Run the C3 device test**, or accept that D4 ships unproven.
4. **Confirm the real mode matrix** for A1, given 18 rather than 10.
