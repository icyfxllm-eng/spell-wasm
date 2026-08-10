# The arcade batch — four new game modes

Pasted by Eric 2026-08-09. **All four REVIEW-GATED. Not started.**
Build order: **IMPOSTOR → LETTER-FORGE → WORD-CHAINS → BEE-SIM.**

Saved verbatim because a pasted spec was already lost to context compaction in
this work; the transcript is not a safe home for law. Session notes are at the
bottom, clearly separated from Eric's text.

---

# 1. CC-IMPOSTOR — "The Impostor": spot the real spelling

STATUS: REVIEW-GATED. Build order for the arcade batch: IMPOSTOR → LETTER-FORGE
→ WORD-CHAINS → BEE-SIM. This file is #1 and exercises the trap-class rule
tables the later files reuse.

## Intent (read first)

Players need a fast, thumb-only, recognition-based way to engage spelling — the
low-friction on-ramp production modes can't be. The Impostor plays the engine in
reverse: hear the word, see four spellings (one real, three generated
misspellings built from that language's trap classes), tap the real one. Speed
and flow are the fun; the "reveal why" moment after a miss is the learning.
Every design call this file doesn't cover should optimize round-to-round flow —
any friction between rounds is a bug against intent.

## Decisions (decide as written; conflicts with repo state or other CC files → STOP AND ASK)

- **D1** — Distractors are generated, never authored. Distractors come from the per-language trap-class rule tables (the same data behind tier calibration): apply a trap transform (silent-letter drop/add, diacritic swap, soft-sign drop, hamza seat shift, homophone-letter substitution, etc.) to the real word. Deterministic: seeded by (wordId, roundIndex) so the same round is reproducible in tests and bug reports.
- **D2** — Difficulty = distractor distance. Easy: one near-miss + two obviously-wrong distractors. Medium: all plausible, different trap classes. Hard: all plausible, same trap-class family. Expert: single-seat/single-diacritic differences only. Kid Mode caps at medium (matching the standing kid ceiling).
- **D3** — Fairness invariant (non-negotiable). A generated distractor is rejected and regenerated if it is a valid word in the target language OR in any lineup language written in the same script. A "wrong" answer that is actually a real word somewhere the player might study is a fairness bug no audit will catch — the generator must check, not hope. If regeneration fails 10× for a word, that word is excluded from Impostor rotation and logged to a build-time report.
- **D4** — Streaks, not lives. 10-round sets, auto-advancing. Wrong tap: streak resets, set continues. Visible streak counter with escalating flair at 5/10/20. End screen: accuracy, best streak, one replay button. No elimination anywhere in this mode.
- **D5** — Reveal-why on every miss. Wrong tap → correct card highlights + a one-line trap label ("silent H — French loves these") drawn from a per-trap-class name/quip string table. These strings are displayed content: hard-capped, and they ride the standing Fiverr audit round; a language's reveal-why lines render only post-audit (English pre-audit exempt). Until audited, non-English languages show the highlight with no quip — never machine-fallback text.
- **D6** — Analytics. Each miss emits (trapClass, tier) to the existing on-device analytics store only — same COPPA posture as the trouble-spot heatmap, which becomes this mode's downstream consumer. Nothing leaves the device. If you believe any network write is needed, you have misread this file.
- **D7** — Gating. Free at PREVIEW: easy tier, standard 10-round sets. FULL: medium/hard/expert. Registers in the hub via modes.json per CC-MODE-HUB. Fully offline.

## Features

1. **Round engine.** TTS plays the word (replay free/unlimited), four large cards render in randomized positions, tap → sub-300ms feedback (correct: chime + advance; wrong: reveal-why per D5, tap-anywhere to advance). Set of 10, then end screen.
2. **Distractor generator (Rust core).** Trap transforms per D1 with D3 validity screening against the relevant validity indexes; distance selection per D2; deterministic seeding; build-time exclusion report for D3 failures.
3. **Trap label table.** Per-language, per-trap-class one-liners in the instructional-strings bucket (hard cap enforced by schema CI).
4. **First-launch how-to.** 3-card intro (audited instructional strings, same pattern as CC-PRACTICE intro cards), shown once.
5. **Streak/end-screen UI.** Portrait, one-thumb; streak flair thresholds as config constants.

## Constraints and non-goals

- Do NOT touch Climb, Daily Challenge, leaderboards, streak (app-level), spaced-repetition, or entitlement code.
- No `if (lang === ...)`; trap tables + registry only. No new authored words or definitions.
- No timers in v1 (a timed variant is a config door, not a build target).
- No network calls from this mode.

## Invariants

- **I1**: Every rendered distractor fails validity lookup in the target language and all same-script lineup languages (property test: 1,000 generated rounds per live language, zero violations).
- **I2**: Determinism: same (wordId, roundIndex, tier) → identical card set across runs/platforms (golden test, WASM + iOS).
- **I3**: Mode writes only to its own progress record + the on-device analytics store (store-diff test, byte-identical elsewhere).
- **I4**: Reveal-why quip renders in language L ⇔ L's trap-label strings cleared audit (English exempt); no fallback text path exists.
- **I5**: Kid Mode never receives hard/expert rounds (Playwright assertion + unit test on the tier resolver).
- **I6**: All strings NFC-normalized; card comparison uses the same normalization as gameplay.

## Done when (all checkable)

1. CI green: property test I1, golden test I2, string-schema hard-cap check, store-diff I3.
2. Playwright (en + es + one CJK): complete a 10-round set; deliberate miss shows reveal-why with correct trap label; streak resets and set continues; end screen renders; replay starts a fresh seeded set.
3. D3 exclusion report attached to the review diff (per-language count of words excluded; >2% excluded in any language → flag in review notes, don't silently accept).
4. Per-language readiness report: trap-table coverage / label-audit status / mode visibility, one row per lineup language.
5. Screenshots: easy vs expert card sets for the same word (visually demonstrating D2), plus one reveal-why moment.

## Eval note

Behavioral bar: a tester should finish a 10-round set and immediately start
another without being asked. If sets end and testers stop, the friction is in
feedback timing or end-screen pull — fix feel before features. Secondary bar: a
miss should feel like learning ("oh, silent H!"), never like a trick; if testers
report gotcha-feel, the D2 distance tuning is wrong.

---

# 2. CC-LETTER-FORGE — "Letter Forge": per-writing-system honeycomb word-finding

STATUS: REVIEW-GATED. Arcade batch build order: IMPOSTOR → LETTER-FORGE →
WORD-CHAINS → BEE-SIM. This file is #2; it introduces the validity-index and
daily-seed plumbing WORD-CHAINS reuses.

## Intent (read first)

This is the retention-depth mode: seven typing units in a honeycomb, spell as
many valid words from the language's audited list as possible, one required
center unit. The format is the most replayable spelling game ever shipped, and
no one has built it per writing system — a Korean jamo forge that assembles
syllable blocks is both the differentiator and the shareable moment. The fun
lives in submission feel (hundreds of submissions per session, each reaction
sub-300ms, none scolding) and in a rank ladder that lets a beginner feel
finished at "Good" while a completionist chases the top. Where this spec is
silent, protect flow and generosity.

## Decisions (decide as written; conflicts → STOP AND ASK)

- **D1** — Units are registry typing units. Latin letters, Korean jamo, Japanese kana, pinyin syllables for zh — whatever the registry already defines for input. No Latin assumptions anywhere. Korean submissions assemble jamo into syllable blocks with a snap animation; the assembled block is what's matched.
- **D2** — Puzzle generation (Rust core, deterministic). A puzzle = 7 units + designated center unit, generated from the language's audited source-of-record list. Validity: word uses only the 7 units, includes the center unit, meets the per-language minimum length (new registry field `forgeMinUnits`; PROPOSED defaults: 4 for Latin-script, 2 blocks for ko, 2 units for ja/zh — review this table). Acceptance gates per puzzle: pool ≥ 20 valid words AND ≥ 1 pangram-equivalent (a word using all 7 units). Failing seeds walk deterministically (seed+1) until gates pass.
- **D3** — Daily Forge + practice forges. Daily Forge: one puzzle per language per day, date+language seeded (same determinism doctrine as Daily Challenge; shares NO code or storage with it). Practice forges: endless, seeded, generated on demand. Gating: Daily Forge free at PREVIEW in the player's free languages; practice forges and the daily archive are Complete. Fully offline.
- **D4** — Rank ladder, hidden totals. Ranks (Good/Great/Amazing/Forge Master — final names are localized strings, review the set) at percentage thresholds of the puzzle's maximum score. The total pool count is NEVER shown during play; it appears only after the player taps "Reveal remaining," which ends the run for ranking purposes. Intent: "12 of 214" demoralizes; "Great" motivates.
- **D5** — Three submission reactions, zero sting. Valid → chime + score flies. Duplicate → gray shake, no penalty. Invalid → soft bounce, no penalty, no red. One free shuffle button, prominent, unlimited. Pangram-equivalent → golden celebration.
- **D6** — Scoring. Word score = unit count, pangram bonus = +7 (config constants). Mode-local only; no Climb/Daily/leaderboard writes. A future Forge leaderboard is explicitly out of scope.
- **D7** — Content safety. The honeycomb can only ever match words already in the audited list, so no generation-side profanity risk exists; the "Reveal remaining" list therefore renders list words only. Kid Mode plays the same puzzles but "Reveal remaining" filters to the kid-visible subset of the list (existing kid filter, not new logic).

## Features

1. **Forge screen.** Honeycomb (center unit visually distinct), tap-to-compose with the current word displayed large, delete/submit/shuffle controls sized for one thumb, found-words drawer, rank progress bar, replayable puzzle-independent TTS OFF (this mode is visual; no audio dependency).
2. **Validity index (Rust core).** Per-language compiled index over the source-of-record list supporting exact-match lookup at input speed; built at content-build time, versioned with the list.
3. **Puzzle generator + seed walker** per D2, with a build-time generability report per language (how many of 365 test seeds pass gates on first try).
4. **Daily Forge surface.** Today's puzzle per language, local-midnight rollover, resumable same-day; archive (Complete) lists past dailies.
5. **First-launch how-to.** 3-card intro, audited instructional strings, shown once.

## Constraints and non-goals

- Do NOT touch Daily Challenge code or storage, Climb, leaderboards, entitlement resolution, or spaced repetition.
- No `if (lang === ...)`; registry + per-language config fields only. No new authored content beyond rank names + how-to cards (audited strings).
- No hints/definitions surface in v1. No leaderboard. No network calls.

## Invariants

- **I1**: Determinism: (date, lang) → identical Daily Forge puzzle across platforms (golden test, WASM + iOS).
- **I2**: Every shipped puzzle satisfies pool ≥ 20 and pangram ≥ 1 (property test over ≥ 365 seeds per live language).
- **I3**: Every acceptable submission exists in the audited list; every list word meeting D2 validity for the puzzle is acceptable (index completeness test: exhaustive cross-check for 10 sampled puzzles per language).
- **I4**: Pool total is unreachable in the UI before "Reveal remaining" (Playwright assertion + no data binding exposing it).
- **I5**: Mode writes only its own progress/found-words records (store-diff test).
- **I6**: NFC normalization on composition and matching; Korean matching operates on assembled blocks.

## Done when (all checkable)

1. CI green: golden I1, property I2, completeness I3, store-diff I5.
2. Playwright (en + ko minimum): compose/submit valid, duplicate, invalid — three distinct reactions; pangram celebration fires; shuffle rearranges without state loss; rank advances at thresholds; kill/reopen resumes same-day daily.
3. Generability report attached (per-language first-try seed pass rate; any language under 80% → flag in review notes with the failing constraint).
4. ko-specific capture: video or screenshot sequence of jamo → block assembly for Eric's review (this animation is a headline feature, not polish).
5. Per-language readiness report row set, as in CC-IMPOSTOR.

## Eval note

Behavioral bar: in dogfooding, a session should end because the player chooses
to stop, not because the game ran out of pull — "one more word" is the target
sensation. If testers stall early, suspect D2 gates (pool too small), unit
selection quality, or reaction feel, in that order. Secondary bar: a ko tester
should describe block assembly as satisfying without prompting.

---

# 3. CC-WORD-CHAINS — "Word Chains": per-language chaining (shiritori done right)

STATUS: REVIEW-GATED. Arcade batch build order: IMPOSTOR → LETTER-FORGE →
WORD-CHAINS → BEE-SIM. This file is #3; it reuses LETTER-FORGE's validity
indexes.

## Intent (read first)

Spell any valid word beginning with the designated final unit of the previous
word; keep the chain alive. In Japanese this is shiritori — a real, beloved game
— and shipping the ja ruleset authentically while generalizing per language is a
cultural-credibility move as much as a mechanic. The fun is momentum: the next
hook unit should dominate the screen ("what starts with 리?"), dead-ends must
always have an exit, and losing to your own memory of the chain must never
happen. Where silent, protect momentum and fairness.

## Decisions (decide as written; conflicts → STOP AND ASK)

- **D1** — Chain unit is a registry field. New per-language registry field `chainUnit`: final letter (Latin scripts, ru), final jamo-normalized syllable (ko — PROPOSED: chain on the final syllable block, not final jamo; flag if the word lists make this too sparse), final kana with shiritori conventions (ja), final pinyin syllable (zh), final letter with diacritic-sensitivity flag (vi — PROPOSED: diacritic-insensitive hooks; review). The full per-language table ships in this file's diff as a reviewable artifact. No chaining logic outside this field + the rules engine.
- **D2** — Authentic ja ruleset as reference implementation. Official ja rules: ん-ending loses (official mode) / gray-shake warning (casual mode); small-kana conventions (word ending ゃ/ゅ/ょ chains on the full-size kana); long-vowel ー chains on the preceding kana; dakuten-flexibility ON (が hook accepted by か-initial words) as PROPOSED default — review. State in the mode's intro card that ja follows real shiritori rules.
- **D3** — Two moods, one engine. Untimed (learning, no pressure) and Timed (per-turn clock that shrinks as the chain grows; shrink curve = config constants). Solo-vs-list in v1. Pass-and-play two-player on one device is Phase 2 of this same file — build the turn abstraction now, ship the second player later; do not build networking ever under this file.
- **D4** — Always an exit. In Timed mode the engine validates, before accepting a word, that at least one unused successor exists; if a submitted word would dead-end the chain, it is rejected with a "dead end!" bounce (not a loss). "Pass" is always available: costs points (config), draws a random valid successor to keep momentum. Running out of all options ends the run with celebration of chain length — framed as a completed chain, never a failure.
- **D5** — Memory is the game's job, not the player's. The chain history is a visible scrolling ribbon; submitting a used word gets the gray-shake duplicate reaction and consumes nothing. No penalty for the player's memory.
- **D6** — Scoring + gating. Score = chain length, with per-word unit-count bonus in Timed (config constants). Mode-local storage only. Free at PREVIEW: untimed mode; Timed and Phase-2 pass-and-play: Complete. Registers via modes.json. Fully offline. Kid Mode: untimed only, kid-visible word subset for validation and Pass suggestions.

## Features

1. **Chain screen.** Hook unit rendered dominant (largest element on screen), input via existing typing-unit input, chain ribbon with auto-scroll, Pass button, Timed-mode clock ring, run-end screen (chain length, longest word, replay).
2. **Chain rules engine (Rust core).** `chainUnit` resolution, ja convention handling per D2, successor-existence validation (successor index derived from the LETTER-FORGE validity index, built at content-build time), duplicate tracking, deterministic Pass selection (seeded per run).
3. **Per-language `chainUnit` table** as reviewable data with schema CI.
4. **First-launch how-to.** 3-card intro (audited instructional strings); ja version includes the authenticity note per D2.

## Constraints and non-goals

- Do NOT touch Climb, Daily Challenge, leaderboards, entitlements, spaced repetition, or any other mode's stores.
- No `if (lang === ...)`; `chainUnit` field + rules engine only. No new authored words/definitions. No networking, no online multiplayer, no leaderboard.
- Phase 2 (pass-and-play) ships only after Eric reviews Phase 1 in TestFlight.

## Invariants

- **I1**: Timed mode can never accept a word with zero unused successors (property test: simulate 500 runs per live language, assert no accepted dead-end).
- **I2**: ja conventions match D2 exactly (unit-test suite with a fixture table of canonical shiritori cases: ん, small kana, ー, dakuten pairs — fixture table itself attached to the review diff for Eric's check).
- **I3**: Duplicate submission never ends or penalizes a run (unit + Playwright).
- **I4**: Determinism: (seed, lang, mode) → identical Pass draws across platforms.
- **I5**: Mode-local writes only (store-diff test).
- **I6**: NFC normalization on hook computation and matching.

## Done when (all checkable)

1. CI green: property I1, ja fixture suite I2, `chainUnit` schema check, store-diff I5.
2. Playwright (en + ja + ko): play a 10-word chain; duplicate gray-shakes; Pass advances with point cost; Timed clock shrinks per turn; dead-end submission rejected with bounce; run-end celebration frames length positively.
3. `chainUnit` table + ja fixture table attached to the review diff (these two artifacts are the review's center of gravity).
4. Per-language readiness report rows, as in prior arcade files.
5. Screenshots: hook-unit-dominant layout in ja and ko.

## Eval note

Behavioral bar: a Japanese-speaking tester (auditor pool) should confirm
unprompted that this "is shiritori" — authenticity is pass/fail, not polish.
General bar: dead-ends should feel like the game saving you, never blocking you;
if testers report the rejection as annoying, tune the "dead end!" feedback
copy/feel, not the rule.

---

# 4. CC-BEE-SIM — "Bee Simulator": the staged classic spelling bee

STATUS: REVIEW-GATED. Arcade batch build order: IMPOSTOR → LETTER-FORGE →
WORD-CHAINS → BEE-SIM. This file is #4; it reuses the synthetic-competitor
pattern from Spell Racing's pace ghosts.

## Intent (read first)

The ritual is the product: podium walk-up, hushed room, "Your word is…", and the
contestant's ceremonial requests — Repeat, Slower, Definition. Elimination stakes
create a tension The Climb doesn't have, placement narrative softens the landing,
and for English this mode is a direct hook into real bee-prep families and the
education edition. Mechanically it is a dramatized tier ladder over the existing
engine — that's why it's cheap; keep it that way. Where silent, protect ceremony
and dignity: a player who goes out in round 2 should feel like a contestant,
never like a failure.

## Decisions (decide as written; conflicts → STOP AND ASK)

- **D1** — Bee structure. Rounds escalate by existing difficulty tiers: rounds 1–2 easy, 3–4 medium, 5–6 hard, 7+ expert (config table). One word per contestant per round; a miss eliminates (standard mode). Bee ends at last-contestant-standing or word-pool exhaustion for the tier ladder.
- **D2** — Synthetic co-contestants. 8–12 named contestants with flags/avatars, deterministic per bee seed (same doctrine as Spell Racing pace ghosts: no free text, generated from a curated name/flag component table — table is a reviewable artifact). Their per-round survival follows deterministic skill curves tuned so the player's placement feels earned: early rounds cull weak synthetics, finals are close. No randomness at play time; all resolved from the seed.
- **D3** — Ceremonial requests. Repeat (unlimited), Slower (unlimited, slow TTS preset), Definition (renders the audited definition). Requests are strategy, never weakness: no penalty, but the end screen tracks "clean words" (no requests used) as a bragging stat only. Definition-request gate: the Definition button exists in language L only if L's definitions cleared the definitions audit (standing invariant: no definition renders pre-audit). No "sentence, please" — sentences are new authored content and are out of scope permanently unless Eric opens it.
- **D4** — Elimination softened by narrative. Miss → the authentic ding bell, the correct spelling shown respectfully (no red X), then "You placed 4th of 12" with the final-standings board, and one button: "Next bee starts at round {N-1}" (re-entry one round below where you fell). Placement, hook, dignity — in that order.
- **D5** — Official Rules toggle. Optional bee-authentic input: letters committed one at a time, no backspace, spoken confirmation of each unit by TTS. Default OFF; the toggle lives inside the mode. Spoken letter input (via Spell-It-Out-Loud capture) is a v2 door — design the input abstraction so it can plug in, build zero dependency on it now.
- **D6** — Kid Mode bee. Same full ceremony, no elimination: every contestant (player included) spells every round; ends with a ribbon screen for all. The ceremony is what kids love; the stakes are what they don't. Kid tier ceiling (medium) applies to the round table.
- **D7** — Gating + placement. Free at PREVIEW: bees through the medium rounds. FULL: hard/expert rounds + Official Rules toggle. Mode-local storage only; registers via modes.json; fully offline; education builds include the mode as-is (already offline, zero purchase surface).

## Features

1. **Bee stage screen.** Podium/stage scene, contestant strip with elimination states, announcer text + TTS ("Your word is…" strings are localized, hard-capped, audited instructional strings), the three request buttons rendered large and ceremonial, input via existing engine (or D5 committed input), ding + standings flow per D4.
2. **Bee engine (Rust core).** Seeded contestant generation + survival curves (D2), round/tier table (D1), word selection from the language's audited list without repeats within a bee, re-entry round computation (D4).
3. **Contestant component table** (names/flags/avatars) as reviewable data with schema CI — names must be culturally plausible and inoffensive across markets; flag any generation approach that can produce free text (prohibited).
4. **First-launch how-to.** 3-card intro (audited strings) explaining rounds, requests, and the clean-word stat.

## Constraints and non-goals

- Do NOT touch Climb, Daily Challenge, leaderboards, entitlements, spaced repetition, Spell-It-Out-Loud capture, or Spell Racing code (pattern reuse ≠ code coupling; if sharing a ghost/contestant abstraction is cheap, STOP AND ASK first).
- No sentences, no hints beyond D3, no networking, no leaderboard, no free-text generation anywhere.
- No `if (lang === ...)`.

## Invariants

- **I1**: Definition button renders in L ⇔ L's definitions audit cleared (unit test on the gate + Playwright presence/absence check in one audited and one unaudited language).
- **I2**: Determinism: (seed, lang) → identical contestant roster, survival outcomes, and word sequence across platforms (golden test).
- **I3**: No word repeats within a bee (property test, 200 simulated bees per live language).
- **I4**: Kid Mode: no elimination state ever renders; round tiers ≤ medium (unit + Playwright).
- **I5**: Official Rules input allows zero backspace/edit events once a unit is committed (unit test on the input abstraction).
- **I6**: Mode-local writes only (store-diff test). NFC normalization throughout.

## Done when (all checkable)

1. CI green: golden I2, property I3, contestant-table schema check, store-diff I6.
2. Playwright (en + es): full bee to elimination — verify ding → respectful reveal → placement → re-entry button targets round N-1; a full clean win; Definition button present (en) and absent in an unaudited language; Kid Mode bee ends in ribbons with zero eliminations; Official Rules run rejects backspace.
3. Contestant component table + survival-curve constants attached to the review diff.
4. Per-language readiness report rows, as in prior arcade files.
5. Screenshots: stage scene, request buttons, ding/placement sequence, Kid Mode ribbon screen.

## Eval note

Behavioral bar: an eliminated tester should immediately tap "Next bee" — the
placement + re-entry hook is the retention engine; if testers close the app after
a ding, D4's feel has failed regardless of green tests. Ceremony bar: a tester
should describe the mode with the word "bee" or "competition," not "quiz." Kid
bar: a kid-mode session must contain zero moments a parent would read as the app
judging their child.

---

## Session notes, 2026-08-09 (NOT part of Eric's text)

Three things to resolve before IMPOSTOR starts.

**1. The build order has a dependency inversion.** IMPOSTOR is #1, but its D3
fairness invariant requires validity lookup "in the target language OR in any
lineup language written in the same script" — and the validity index is
introduced by LETTER-FORGE, which is #2. IMPOSTOR cannot satisfy its
non-negotiable invariant without #2's plumbing. Either the index moves earlier,
or the order swaps.

**2. "The audited list" does not exist yet in any language.** LETTER-FORGE D2
generates puzzles from "the language's audited source-of-record list", and D7
rests content safety entirely on that list being audited. As of today all 14
gloss files are `audited: false`, and the 705 bank words added this session are
unreviewed. English may be exempt as frozen-reference; every other language is
not. BEE-SIM D3 has the same shape for definitions, which are dark per
CC-DEF-MATCH.

**3. The trap-class rule tables IMPOSTOR D1 calls "the same data behind tier
calibration" may not exist as a general per-language artifact.** Trap sets are
defined for Persian in CC-PERSIAN-FOUNDATION F5 (the five homophone-letter sets),
but that is one language in a spec that is itself not started. Worth a step-0
dump before D1 is treated as reading existing data rather than authoring new
data.
