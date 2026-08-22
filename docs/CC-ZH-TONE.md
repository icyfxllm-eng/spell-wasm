# CC-ZH-TONE

**Status:** D1–D7 SIGNED by Eric 2026-08-19. ALL FEATURES COMPLETE.
Done 3 signed 2026-08-20. Outstanding: Done 6 (needs keys and whisper.cpp)
and Done 9, Eric's device pass, which closes the file.

**Depends on:** the shared-canonicalizer pattern from CC-PERSIAN-FOUNDATION F1.
**Blocks:** zh bank tooling, zh Climb pools, the drawn-character stage.

## Intent

Mandarin answers in standard mode must carry tone. Toneless pinyin is not
Mandarin -- *ma* is not a word -- and accepting it teaches a fossilized error
adult learners never shake. The audio carries tone; grading that discards it
throws away the only information the audio contained.

The risk is not difficulty, it is the keyboard. Requiring tone *marks* on a
phone means fighting a long-press diacritic picker that may not offer pinyin
vowels at all. So: require the tone, accept any encoding of it, and give the
player a one-tap way to express it.

## Decisions (signed 2026-08-19)

- **D1.** Tone required in standard mode at all tiers including the free tier-1
  preview. Little Speller stays tone-blind. *The ja analogue is pitch accent and
  the answer there is never grade it -- kana does not encode pitch, it varies by
  region, and there is no writing convention for it. Recorded so a future ja
  file inherits the reasoning rather than re-litigating it.*
- **D2.** Input is encoding-agnostic: diacritics, trailing digits, `v` and `u:`
  for ü, any separator. All normalize to one key.
- **D3.** Neutral is written bare and parses to tone 5. Bare input where the
  answer is tone 1–4 is a TONE_MISS, not a pass.
- **D4.** Sandhi: grade citation, synthesize surface, accept both at tiers 1–2
  with a teaching note on reveal, citation only at tier 3+.
- **D5.** A TONE_MISS-only answer at tier 1 grants one free retry before
  scoring. Tier 2+ scores partial credit with no retry.
- **D6.** Ambiguous segmentation resolves by charitable parse.
- **D7.** Tone colours are **Pleco's scheme**: 1 red, 2 green, 3 blue,
  4 purple, 5 grey. One map on the orb, the input buttons and the reveal.
  Colour is never the sole carrier.

**Amendments made at signing:**

- **Neutral stays `bao3bao5` in the bank.** D3 describes canonical neutral as
  written bare; the bank writes it explicitly. Both are true at once because
  tone is a *value* in the key -- `bao3bao5` and `bao3bao` produce the same
  PinyinKey, so the stored spelling stops mattering. No bank rewrite.
- **The apostrophe is a hard syllable boundary**, not a discarded separator.
  It is pinyin's own disambiguator and nobody types one by accident, so
  `xi'an` is never read back as `xian`. Space and hyphen stay discarded: a
  stray space is a plausible typo and must not wreck a good answer.

## F0 — Step 0 findings

The live path is `submit()` in src/game.rs, branching at the language check to
`crate::pinyin::matches`. Comparison was never raw-string equality and NFC was
already applied. `s.word` holds the pinyin half; `s.spoken` holds the hanzi.

Four findings that changed the shape of the work:

1. **A tone-tolerant branch already existed.** The old `normalize` dropped every
   `5`, so `de5` and `de` were equal. It agreed with D3 by accident, not by
   design -- it was a global character filter with no notion of syllables. Its
   header cited a "spec §Mandarin normalization" that does not exist in docs/.
2. **The bank already carried pinyin**, unnamed, as the left half of a
   `pinyin|hanzi` string (`bao3bao5|宝宝`). No `pinyinSurface`, no `syllables`.
3. **The bank stores ü two ways** -- 45 entries as `v` (`lv3xing2|旅行`), 5 as
   `ü` (`lü3ke4|旅客`), including the same syllable both ways. Both key
   identically after D2 folding, but F5's named `pinyinCitation` must pick one.
4. **Invariant 4 is violated today.** zh synthesizes from bare Hanzi:
   `SynthesisInput(text=word)` in backend/app.py with `word` = the hanzi, via
   Google rather than the Azure SSML builder. F6 is a fix, not a hardening.
   Separately the TTS cache key carries no voice id, so changing the zh voice
   serves stale clips -- a latent bug independent of this file.

**A second zh grading path already exists.** src/bee_screen.rs grades with
`norm::fold_strict` and never touches the canonicalizer, so `lv4` does not match
`lü4` in Bee mode. F2's deliberate-failure lint has a real violation to catch on
day one. (src/photo_import.rs also compares zh, but that is dictionary
membership, not grading.)

**Stale dependency.** This file's header blocks a drawn-character stage, but
REVIEW_zh_metadata.md records that drawing is retired app-wide and zh is
typed-only. src/drawing.rs still exists at 434 lines. Resolve before CC-CJK-INK
is written against a removed stage.

## F1 — the canonicalizer

`src/pinyin.rs` exports `canonicalize_pinyin(input, expected_syllable_count)
-> Result<PinyinKey, ParseError>`, total, never throwing and never silently
returning its input. `PinyinKey` is an ordered `Vec<Syllable { segment, tone }>`
and is the only thing zh grading compares.

**F1a — the pinned inventory.** `src/pinyin_inventory.rs`, generated by
`tools/build-pinyin-inventory.py`, 426 syllables, sha256-pinned. Three sources
reconciled rather than trusted: pypinyin's reading table, the shipped zh bank,
and a standard-Mandarin review list. Bank coverage is a hard gate -- a syllable
the bank can ask for but the inventory lacks makes that word unanswerable, so
the generator fails rather than shipping the gap. After D2's v→ü fold, all 383
bank syllables are covered. `scripts/pinyin-inventory-check.mjs` recomputes the
pin in the gate, so editing the list by hand fails CI.

**F1b — segmentation.** Greedy longest-match with backtracking against the
expected count. Two rules were forced by test failures rather than chosen:

- *Prefer parses using no flagged syllable.* pypinyin's inventory includes bare
  interjections (`n`, `m`, `ng`, `hm`, `hng`) and dialect readings. Greedy
  longest-match read `xian` at two syllables as `xia` + `n`, because the
  interjection is found before `xi` + `an`. The 14 flagged syllables stay legal
  as answers but lose ties. **This preference is invented, not specified.**
- *Over-inclusion in the inventory is NOT harmless.* The original reasoning --
  that a too-large inventory only changes a typo's error class -- was wrong, and
  the `xian` failure is the counterexample.

**Deviations from the letter of the spec:**

- **Full-width folding.** D2 says NFC, "universal baseline, unchanged", but NFC
  does not fold full-width forms and Done 2 requires them in the fixture. Only
  NFKC would, and NFKC mangles unrelated text, so the `U+FF01–FF5E` block is
  folded explicitly. A full-width `３` from a Chinese IME means tone three.
- **D6 cannot be implemented at F1's signature.** The charitable parse scores
  candidates *against the expected answer*, but `canonicalize_pinyin` receives
  only a count. Grading therefore calls `canonicalize_against(input,
  &expected_key)`; the spec'd signature returns the deterministic greedy parse.
  Either the signature grows a parameter or D6 belongs to the matcher.

**`ê` is a legal pinyin letter** (欸/诶) and its circumflex is not a tone mark.
Lifting tone marks off vowels rejected it until the base-letter guard learned
the difference.

## F2 — the Tone Law

`ToneMode` is `Graded` or `Blind`, derived from the `kid` flag callers already
hold (`modes.rs` calls it "Little Speller / Kid Mode"). `matches_with(typed,
answer, mode)` is the single zh grading entry point. Blind ignores tone rather
than stripping it -- the key still carries tone, so the reveal can display it.
Blind is not blind to spelling or to syllable count.

**The violation was real, and it was live.** Bee lists zh among its languages,
and graded with `norm::fold_strict`, so `lv4` did not match `lü4` and a
tone-perfect answer could be marked wrong. Bee now routes through
`matches_with`. Its reveal shows the stored form with tone digits intact, which
is F2.3 working: Little Speller sees tone even though grading ignored it.

`scripts/zh-grading-path-check.mjs` keys on the pipe -- Mandarin is the only
language whose entries are `pinyin|hanzi`, so a site that splits on `|` and then
compares against typed input is grading Mandarin. It carries a selftest, and
reverting Bee to `fold_strict` fails it by name.

**The scan found four sites the manual inventory missed** -- chains.rs,
impostor.rs, keyboard.rs, translate.rs. All four turned out to be non-grading
(chain identity, distractor cards, a keyboard-reachability assertion, OCR
lookup) and are allowlisted with reasons. The exemption is per FILE, which is
coarser than it looks: a real grading path added to an allowlisted file would
inherit the pass, so the two true grading surfaces are also checked by name.

**Two bugs found while wiring F2, both left for their own file:**

- **Bee speaks the wrong half.** `bee_screen.rs` synthesizes
  `split('|').next()` -- the *pinyin* -- where the main game speaks the hanzi.
  A zh Bee round currently reads romanization aloud to a Mandarin voice. F6
  territory; the invariant it breaks is the spirit of "never synthesize zh from
  bare Hanzi", from the other side.
- **Camera lookup can never match zh.** `translate.rs` `camera_lookup` compares
  OCR output against the pinyin half, but a camera pointed at Chinese text
  recognizes hanzi, so no zh word can ever be found. Out of scope here.

## F3 — error classification and routing

`grade(typed, answer, mode) -> WordVerdict` returns a verdict PER SYLLABLE:
Exact, ToneMiss, SegmentMiss, Both, or a whole-word LengthMismatch. The word is
correct iff every syllable is Exact. `is_tone_only` is the routing predicate and
is deliberately strict: any segment error anywhere makes it false.

**The queue is a sibling, not a flag.** `src/tone_drill.rs` mirrors `misses.rs`
-- same Leitner ladder, its own storage key and cap. A tone-only miss goes there
and never into the general queue (Invariant 6), and the converse is enforced
too: a word that later comes back with a segment wrong is evicted from the drill
on its way into the general queue, so it cannot sit in both being drilled for
the wrong reason.

The routing law is a pure function, `tone_drill::route`, so Invariant 6 is
testable without a running app. misses.rs learned half this lesson -- it split
the clock out for tests but left the storage write in, so its own spaced-rep
rules are still untested today.

**Two rules the tests forced, both about forcing a reading nobody typed.**
Constraining a parse to the answer's syllable count always finds SOME reading if
one exists, which made LengthMismatch unreportable:

- `ping2` against `ping2guo3` came back as `pi` + the interjection `ng`. When
  the constrained parse leans on a flagged syllable the answer does not use, the
  player's own unconstrained reading wins and it is a length mismatch.
- `suo3` against `suo3yi3` came back as `su` + `o`, and `o` is not flagged, so
  the rule above did not catch it. The real signal is simpler: **a tone digit
  terminates a syllable**, so input where every syllable carries one has already
  declared its own count. Re-reading it at another count invents an answer.
  Input with bare syllables stays ambiguous and still resolves by the expected
  count, which is what keeps `xian` readable as `xi` + `an`.

**The reveal names the failure.** Each syllable is tinted by its tone from the
one Pleco map, marked with its verdict, and carries its tone mark in a `sup`;
underneath, a sentence names the failing syllable by index and class in all 15
locales. Invariant 7 holds three ways over -- colour, mark, and words -- so the
message survives a colour-blind player and a greyscale screenshot.

**D5, and where partial credit went.** At tier 1 (`TIER_ORDER[0]`, "easy") a
tone-only answer earns one free retry and NOTHING is scored first -- unlike the
extra-attempts path, which records the miss before granting its retry. The
budget is `aids.retry_used`, cleared per word by `attempts::start_word`, so it
is once per word. Head-to-head is excluded: a free swing one side gets and the
other does not is not a fair match. Little Speller cannot reach it at all,
since tone-blind grading never yields a tone-only verdict.

The rule lives in `tone_drill::tier1_retry_earned` rather than in the app, so
Done 7 can fuzz it; game.rs decides only the runtime context (language, versus)
that a pure function cannot see.

Scoring in this app was boolean everywhere -- `TierStat` counted `seen` and
`correct` and nothing else -- so partial credit had no home and the constant
would have shipped unused. `TierStat` now carries `tone_partial` beside
`correct`, and `credited()` weights it by `ZH_TONE_MISS_CREDIT` (0.5). The
displayed pair stays the honest whole-correct count; only the percentage is
credited, and the tone credit is NAMED next to it in all 15 locales rather than
silently inflating the number.

## F4 — tone input affordance

Five buttons under the answer: ¯ ´ ˇ ` ˙, each carrying its numeral as well as
its mark and tinted from the same Pleco map as the reveal, so Invariant 7 holds
without leaning on colour. Mandarin only, hidden at expert tier — the row is
ABSENT there rather than disabled, which is the same doctrine the mode registry
uses. Typed digits and typed diacritics keep working in parallel; the buttons
are an affordance, never the only path.

`pinyin::apply_tone` is a pure transform: it replaces any tone already on the
syllable being typed rather than appending, so a wrong choice costs one tap and
never a retype, and it is a no-op on an empty answer so a stray tap cannot leave
a dangling digit the parser would reject.

Scope worth stating: "the active syllable" is the one at the END of the answer.
The spec says any syllable can be re-toned without retyping, and for the
syllable being typed that holds — but re-toning an EARLIER one still costs a
backspace, because there is no syllable-selection affordance to hang it on.

**The settings-truth gate had a hole, and this feature found it.** AUG6 scanned
`<input>` only, because switches and sliders were the only control kinds that
existed; a `<button>` sailed straight past it. Worse, once declared, the five
buttons passed with a dead handler — the effect tests exercise the transform,
not the wiring, so a rendered, declared, fully-tested button was still allowed
to be inert. Both are closed now: the scan reads `.tone-btn` buttons, and the
click wiring is asserted by name. Unwiring the handler fails the build, which
was verified by actually unwiring it.

## F5 — sandhi

你好 is spoken ni2 hao3; written pinyin keeps ni3 hao3. We tell the player to
spell what they hear and then mark it wrong, so the conflict had to be designed
rather than discovered on device. Grading compares the CITATION, TTS speaks the
SURFACE, and tiers 1–2 accept both with a teaching note that explains the
difference. Tier 3+ takes the citation only, and a surface answer there reads as
a tone miss — which is exactly what it is.

**Where the three fields live, and why not in the bank string.** The spec says
every entry gains three fields. `src/words.rs` entries are parsed POSITIONALLY
in a dozen places and inconsistently -- game.rs takes field 2 as the hanzi while
kid_filter.rs takes the LAST field -- so any extension of `pinyin|hanzi` breaks
one or the other silently. The fields therefore live in `src/zh_sandhi.rs`,
generated, keyed 1:1 off the whole bank entry. `pinyinCitation` is not
duplicated: it is the left half of the key, which is already what grading
compares. Nothing that reads the bank had to change, and the lint still requires
a row for every entry, so a missing field is a build failure exactly as
Invariant 3 asks.

**The tagger.** `tools/build-zh-sandhi.py` applies three rules: 一 shifts to yi2
before a falling tone and yi4 before the rest, 不 shifts to bu2 only before a
falling tone, and in a run of third tones every one but the last rises to
second. 一 and 不 are CHARACTER-specific -- a syllable that merely sounds like
yi1 does not take 一's sandhi -- so the tagger aligns syllables to characters
positionally and reports any entry where that does not hold rather than
guessing.

Of 6,182 entries: 5,888 none, 185 third-third, 83 yi, 26 bu.

**The audit is hand-derived, and that is the point.** Done 3 asks the tagger and
a human to agree 50/50. The fifty cases in `config/zh-sandhi-audit.json` were
worked out from the rules, never copied from the tagger's output -- an audit
built from the thing it audits proves nothing, which is the same trap as grading
a generator against its own product. They agreed 50/50 on the first run. The
fixture is the authority: on a disagreement the TAGGER is what is wrong.

**Done 3 is closed.** Eric signed the fixture 2026-08-20 and the tagger matched
it 50/50. The signature is load-bearing rather than decorative: the lint fails on
`confirmed_by: null`, so changing any expectation re-opens the gate and needs
signing again. Agreement with an unsigned fixture would only be the tagger
agreeing with its own author.

## F6 — forced-pinyin TTS

Google Cloud TTS supports `alphabet="pinyin"` and in fact REQUIRES it for
Chinese: numeric tones at the end of each syllable, whitespace between
syllables, their own example being `wo3 de5`. Verified against the docs
2026-08-20 rather than assumed, because the whole feature turns on it — had it
been IPA-only, zh would have had to move providers.

`pinyin::phoneme_reading` builds the `ph` value from the canonicalizer, not
from the stored string, so the bank's two ü spellings and its unspaced
syllables all come out in one form. A word whose stored form will not parse
gets NO reading, and the caller must then refuse to speak it.

The reading rides in its own `py` parameter. It cannot share `word`:
`validate_word` rejects digits and whitespace by design, so `bao3 bao5` would be
thrown out before it reached synthesis.

**The cache key is the reading plus the voice id**, never the hanzi. Pinyin
keeps clips valid across bank edits that do not change pronunciation, and the
voice id fixes a latent bug older than this file — without it, changing any
language's voice silently kept serving the old one's clips.

**Invariant 4 had FOUR leaks, and only one was the obvious one.** The scan found
the last one by itself:

1. the backend synthesized zh as plain text;
2. Bee spoke through the browser voice and handed it the PINYIN, so a Mandarin
   voice read romanization aloud — zh Bee audio was simply wrong;
3. the audio router fell back to on-device TTS, which cannot be given a reading
   at all (and a player set to "native-only" got nothing else);
4. the word-picture replay sent the pinyin as `word`, which the server rejects,
   so every zh replay fell through to the device voice reading romanization.
   Its feed keeps only the typed half, so the character had to be looked up
   again before it could be spoken.

`scripts/zh-audio-path-check.mjs` holds all four shut and carries a selftest.

Its proximity check measures CODE, with comments stripped. F5 added a sandhi
lookup and four lines explaining it between the zh guard and the call, which
pushed them past the window and failed the gate on a change that was entirely
correct. A gate that fires on comment length is a gate people learn to route
around.

**A deliberate consequence, stated plainly:** zh no longer falls back to the
on-device voice, so with no pack and no server, Mandarin has NO audio rather
than wrong audio. That is what Invariant 4 asks for, but it is a real behaviour
change for offline play and Eric should know it rather than discover it.

**One leak left open on purpose.** A My Words import with "Speak in: Chinese"
still reaches the device voice. There is no bank entry and therefore no reading
to force, so honouring the invariant there would delete the feature rather than
fix it. Out of scope for this file; recorded rather than silently closed.

**Done 6 cannot run here.** The loopback needs live synthesis and whisper.cpp:
this machine has neither TTS key set nor whisper.cpp installed. Everything the
gate can check is checked; the 30-word polyphone verification is outstanding and
needs Eric's machine. Until it runs, one detail is unconfirmed: whether Google's
pinyin alphabet wants `ü` or `v`. The reading sends `ü` (standard pinyin) and the
server's validator accepts both.

## Invariants

1. No raw-string equality anywhere in zh grading; comparison is on PinyinKey.
2. The canonicalizer is total.
3. Every zh bank entry carries `pinyinCitation`, `pinyinSurface`, `sandhiClass`.
4. No zh audio is ever synthesized from bare Hanzi.
5. Tone-blind mode is reachable only via the explicit matcher flag.
6. TONE_MISS-only words never enter the general missed-words queue.
7. Colour is redundant with a textual tone indicator on every surface.
8. Freshness is not implemented here; zh consumes the global invariant.

## Done

1. **Canonicalizer coverage** — PASSING. Every syllable in the inventory, in
   every accepted encoding, maps to exactly one key: 0 unmapped, 0 collisions
   across all 2,130 syllable-tone pairs.
2. **Adversarial fixture** — PASSING. `lv3 / lü3 / lu:3 / lǜ` with the last
   staying tone 4, `ma / ma5 / ma0`, uppercase, trailing whitespace, full-width,
   `mǎ3` erroring, `xian` vs `xi'an` at both counts.
3. **Second-grading-path lint** — PASSING (F2's deliberate-failure gate).
   Reverting Bee to fold_strict fails the scan by name.
3b. **Bank field lint** — PASSING, both halves. 6,182 entries carry all three
    fields; tagger and hand audit agree 50/50; the audit is signed (Eric,
    2026-08-20) and the lint enforces the signature.
4. **Error-class routing** — PASSING. 50 tone-wrong, 50 segment-wrong and 50
   length-mismatch synthetics classify correctly; all 50 tone-only words route
   to the drill and none to the general queue.
5. **Settings-truth effect test** — PASSING. 5/5 tone buttons produce an
   observable tone change on the active syllable, and a dead handler now
   fails the build (it did not before this feature).
6. **TTS loopback** — BLOCKED, not failed. Needs a TTS key and whisper.cpp,
   neither present on this machine. The forced-reading path itself is built,
   gated and tested; only the acoustic verification is outstanding.
7. **Tier-1 recoverability fuzz** — PASSING. 100+ randomized tone-only
   submissions at tier 1, deterministic seed so a failure reproduces: zero
   unrecoverable zeros on first encounter.
8. **Deliberate-failure piece** — PASSING, both halves. A missing surface form
   fails zh-bank-sandhi-check's selftest; a bare-Hanzi synthesis path fails
   zh-audio-path-check's.
9. **Eric's device pass** — this gate closes the file. The tests do not.
