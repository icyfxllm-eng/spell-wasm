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
