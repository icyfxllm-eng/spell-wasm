# CC-RUSSIAN-STRESS v2 — Phase 0 result

**Status:** Phase 0 RUN and REPORTED. Verdict RESPONSIVE (machine). Awaiting the
auditor's listen and Eric's Feature 6 signature. Phase 1 not started.

## Phase 0 — voice probe, 2026-08-30, `ru-RU-Wavenet-D`

`tools/ru_stress_probe.py`. Minimal pairs whose members differ ONLY by stress.

    determinism  замок vs замок     0.0000   floor
    ceiling      замок vs молоко    0.8607   plainly different words

    за́мок vs замо́к                 0.7449   87% toward a different word
      за́мок vs bare                0.6067
      замо́к vs bare                0.7132

    мо́лодец vs молоде́ц             0.6994   81%
      мо́лодец vs bare              0.7075
      молоде́ц vs bare              0.4435

**VERDICT: RESPONSIVE.** U+0301 reaches the front end and moves the stress.
Feature 4 takes the simple branch: send `mark_stress()` output as synthesis
text. No SSML, works on every tier.

**The asymmetry is the strongest evidence, and it is more than the file asked
for.** `молоде́ц` is the CORRECT natural stress and sits 0.4435 from the bare
form, while the wrong-stress `мо́лодец` sits 0.7075. An engine mangling the mark
would push both roughly equally far from bare. Instead the marked form agreeing
with the engine's own default stays close and the one fighting it moves away —
which is what a front end genuinely relocating stress looks like. The same
holds for `замок`, where bare is nearer `за́мок`, implying the engine defaults to
*castle*.

**THE SPEC'S CONTROL IS DEGENERATE AND WAS REPLACED.** It says to compare the
pair distance against "either and its own repeat synthesis". This API is
deterministic — the same request twice returns byte-identical audio, measured
as the 0.0000 floor above — so that control is always zero and ANY difference
would read as responsive. The floor and ceiling above put the verdict on a
scale instead.

**THE FIRST RUN OF THIS PROBE WAS WRONG.** It reported RESPONSIVE at 89% using
stress indices 2 and 4 for `замок` — both CONSONANTS — and synthesized `зам́ок`
and `замоќ`. The engine was reacting to two different nonsense placements, not
to stress. Right verdict, worthless evidence. Feature 1 already states the rule
("index pointing at one of аеёиоуыэюя") and `mark()` now asserts it, so a mark
on a consonant fails loudly instead of producing a confident number.

**Not closed.** The machine cannot separate RESPONSIVE from DISTORTING — the
asymmetry is suggestive, not proof the stress lands on the right vowel. Clips
are in `build/ru-stress-probe/` for the auditor.

## Feature 6 — the homograph schema question has NO INSTANCES

Measured over the shipped `ru` bank:

    6210  entries
       0  DUPLICATE SPELLINGS
    5961  polysyllabic — need a stress index
     249  monosyllabic — null by rule
      42  contain ё — index forced to the ё

**Zero collisions.** Every spelling in the bank is unique, so the schema cannot
be ambiguous at the row level and there is no rarer sense to drop. Eric's
recommended (b), one sense per spelling, is already the de facto state at zero
cost. The decision still wants a signature — a future bank addition could
introduce the first collision — but it blocks nothing today.

**Scope of the audit job: 5961 rows.** That is the number to weigh before
committing to a `ru` release date, and it is the cheapest audit column in the
project: binary, no judgment, hundreds of rows an hour for a native speaker.

## Phase 3 — ingest (BUILT 2026-08-31)

`tools/ru_stress_ingest.py` is the only write path for stress data, as with
definitions. It reads the auditor's filled sheet, converts `stressed_form` back
to a character `stress_index`, and emits `src/ru_stress_data.rs`.

**Four gates, each rejecting the WHOLE sheet rather than dropping a row.**

| gate | what it catches |
|---|---|
| AGREEMENT | `strip_stress(stressed_form) != word` — the answer key was edited (typo, autocorrect, stray space). I1's promise rests on the key being untouched. |
| WELL-FORMED | not exactly one U+0301, or a mark on a consonant. This project has produced that defect by hand three times. |
| Ё | an entry containing ё indexed anywhere but the ё. |
| DECOY | more than one of the six planted rows left uncorrected. Rejects the sheet and gates payment (v2 Feature 2). One miss is tolerated; two is a pattern. |

A rejected sheet writes nothing. There is no partial ingest: I5 excludes an
uncovered entry from selection, so an absent row degrades safely and a wrong
row does not.

Verified by construction, not assertion — the unaudited sheet (6 decoys intact)
is REJECTED; a corrected sheet is ACCEPTED; and three lesions each trip their
own gate with a message naming the actual cause.

### The table ships DARK

`config/ru-stress-audit.json` holds `audited: false`. **The ingest reads that
file and never writes it**, following `gloss_docs` in `src/translate.rs`:
`audited` is a HUMAN claim, no tool sets it. The ingest's gates prove the sheet
is well-formed and the auditor was awake; they cannot prove the answers are
right, and the 973 pre-filled rows are Wiktionary — a good source, not an audit.

While the claim is false, `ru_stress.rs::stress_index` returns `None` for every
word, including words in the table. The data is present and diffable; it is
inert. `scripts/ru-stress-ingest-check.mjs` (5 lesions, in the gate) catches the
forgery that a unit test cannot: flipping `AUDITED` in the *generated* file
instead of the config, which would take 973 unread rows live.

Eric flips it after his own pass (he expects ~a month of study; stated
2026-08-29). Flipping it makes `the_table_is_dark_until_a_human_signs_off` fail
until the lookup is expected live — deliberate, and visible in a diff.

**Still open:** the 112 stress-homographs need `senseDiscriminator` + two
definitions each before they can be covered at all; D20 (Phase 6 report
content) and D21's threshold remain unsigned.

## Phase 4 — Feature 5, visual stress on the reveal (BUILT 2026-08-31)

**Why the reveal and not a pre-answer surface.** Feature 5 says "the
hint/prompt surface", and in this game that can only be the reveal. Every
pre-answer surface deliberately withholds spelling: `mask_word` shows one
letter and bullets, and `show_sentence_hint` masks the word inside its own
example sentence on the stated grounds that "hearing it doesn't give away the
spelling the way reading it would". Rendering молоко́ before the answer would
hand over the whole word, which is the one thing this game never does.
Post-answer the word is already on screen, so the mark is pure teaching.

`ru_reveal_html` sits in `finalize_incorrect_ex` beside `zh_reveal_html`, the
exact precedent — Chinese already puts its tone marks on that surface.

**Gating.** Russian, easy tier, and AUDITED coverage. Tier comes from the new
`effective_tier`, because Climb reports a *band* and a check against the raw
`level` string would be false for every Climb player. `preload_pool` had the
same logic inline and now shares it, so the two cannot drift.

**I1 holds.** `ru_reveal_html` builds a display string and returns it. It never
touches `reveal`, which stays bare for grading, for the audit-preview
attribute, and for every store keyed on the word.

**L3, new gate law.** The marking functions are callable from `ru_reveal_html`
and nowhere else in shipped code. Feature 5: "Never on the answer field, never
on anything the player types into." Grading compares against the bare answer
key, so a marked string on the input path would mark a CORRECT answer wrong.
Pinyin's lint is the mirror image — it *forbids* combining marks on its output
path because pinyin is precomposed; U+0301 has no precomposed Cyrillic form, so
here the mark is required on output and banned on input. 8 lesions total.

### The rendering note, measured

The spec flags that U+0301 must render as a true combining mark. The bundled
webfont subsets declare U+0300-0301 (for Vietnamese) but carry **no Cyrillic**,
so the letters fall back to a system font while the mark could be matched from
a different face — mark and base from different fonts is exactly when combining
marks break. Measured in the browser against the real `.reveal` stack
("Space Mono", ui-monospace, monospace): adding U+0301 adds **zero advance
width** both to a whole word and to a lone vowel, and a visual check renders
молоко́, за́мок / замо́к, хорошо́ and ёж with the acute on the correct vowel.
The browser stack is clear. **iOS WKWebView remains a device-pass item.**

**Ships invisible.** `marked()` returns None while the table is dark, so this
changes nothing a player sees until `config/ru-stress-audit.json` is signed.
`ru_reveal_is_dark_until_a_human_signs_off` asserts exactly that.
