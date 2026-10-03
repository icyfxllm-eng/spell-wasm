# CC-SNAP-CLEAN v1.1 — Unbunching (amendment)

> **Provenance and status, added when this was committed on 2026-10-03.**
> Everything between the rules below is Eric's, verbatim. It had lived only in
> a chat session since 2026-10-01. Nothing in it has been edited; the review
> findings and the build status are kept separate, after the spec.
>
> **BUILT 2026-10-01** (commit 4ef812f2), with **auto-split switched off** —
> see "What was built, and the one switch that is off" at the foot of this
> file. Census C1 no longer HALTs: `docs/CC-SNAP-BOXES.md` provides the
> geometry.

---

**Status:** REVIEW-GATED. §0 census executable on Eric's signature. Phase A blocked on census review.
**Layering:** amendment atop CC-SNAP-CLEAN v1, under CC-SNAP-ROADMAP v1 (Level 1.2). Everything in v1 stands; this file adds one stage and two invariants.
**Owns:** splitting a merged OCR token into its printed words. Nothing else.
**Does not own:** word-box extraction from Vision / ML Kit (CC-SNAP-LIST must provide it — see C1), bank lookup (CLEAN v1 C4), lemma matching (CC-SNAP-LEMMA, Level 2).

---

## Intent

OCR sometimes drops the spaces between printed words, so `big red dog` arrives as `bigreddog`. Today that imports as one junk word the parent must retype. Every printed word should become its own entry.

Two different causes need two different fixes:
1. **The page has gaps but OCR dropped them.** This is a transport bug. Fix it by reading the word boxes the OCR engine already produced. No guessing.
2. **The page truly has no gap** (a handwritten run-on, a decorative font, a merged line). Only here is a dictionary split justified, and only when it is unambiguous.

When this spec is incomplete, prefer: box evidence over dictionary evidence; no split over a doubtful split; a one-tap suggestion over a silent change.

---

## §0 Census (read-only; report before any code)

**C1. Word boxes.** Confirm each OCR line reaching `clean_ocr_lines` carries its per-word bounding boxes (`Vec<WordBox { text, x0, x1, confidence }>`), on both iOS (Vision `VNRecognizedText` candidates → word-level boxes) and Android (ML Kit `Text.Element`).
- **HALT** if only line-level text arrives. That is a CC-SNAP-LIST change and must land first; this file does not add box plumbing.

**C2. Gap statistics.** On the 10 worksheet photos from CLEAN v1 C5, record inter-word gap width as a fraction of median glyph width. Report the distribution. This sets the F1 gap threshold (D-U4).

**C3. Bank word-set lookup.** Confirm a case-folded, NFC exact-match set per language exists (CLEAN v1 C4). Record the lookup cost; F2 may call it O(n²) times per token where n = token length.

**C4. Frequency list.** For each language, is a licensed word-frequency list already in the repo (used by any existing tier calibration)? If yes, record it; F2 uses it for tie-breaks. If no, F2 tie-breaks on piece count alone. **Do not add a frequency list in this file.**

**C5. Compound languages.** Confirm the per-language policy table (D-U1) has an entry for all 15 languages. Any language missing from it HALTs.

**C6. Test command.** Same as CLEAN v1 C7.

---

## Features

### F1. Box-gap split (runs before dictionary split)
- For a line with ≥2 word boxes, treat each box's text as its own token. A gap exists between two boxes when `x0_next − x1_prev ≥ gap_threshold × median_glyph_width` (threshold from D-U4).
- Tokens separated by a gap are separate candidates before any dictionary logic runs.
- Where the OCR engine returned a line as one box with no internal gaps, F1 does nothing; the token passes to F2.
- F1 is **always on** for every language, including zh/ja (a box gap on a CJK page is still a real gap and is honored; D-U1 only governs F2).

### F2. Dictionary split (fallback)
Input: one token `t` that is not a bank word (case-folded, NFC).
- Compute every segmentation of `t` into pieces where **every piece is a bank word** for the line's `lang`. Dynamic programming over positions; memoize.
- **Candidate ranking:** fewest pieces first; then, if C4 found a frequency list, highest summed log-frequency; then no further tie-break.
- **Auto-split only when all hold:**
  1. `t` itself is not a bank word (I-U2),
  2. at least one full segmentation exists,
  3. the best segmentation has strictly fewer pieces than the runner-up, **or** there is exactly one segmentation,
  4. the language's policy (D-U1) is `auto`.
- Otherwise, if ≥1 segmentation exists and policy is `auto` or `suggest`, attach the best segmentation as a `SplitSuggestion` on the candidate (F3). Policy `never` attaches nothing.
- Pieces shorter than `min_piece_len` (D-U3) are excluded from segmentation, except a whitelist of legitimate one-letter words per language (`a`, `I` for en; `y`, `a`, `o` for es; `à`, `y` for fr; `и`, `в`, `с`, `к`, `а`, `о`, `у` for ru; etc. — the whitelist is a per-language table in the core, populated from the bank's own one-letter entries, never hand-typed).

### F3. Suggestion surface
- A `Candidate` gains an optional `split: Option<SplitSuggestion { pieces: Vec<String>, applied: bool }>`.
- `applied = true` when F2 auto-split; the review screen shows the pieces as separate rows with a one-tap **Rejoin** on the first piece.
- `applied = false` when suggest-only; the review screen shows the merged token with a one-tap **Split into a · b · c?** chip.
- This file only guarantees the field. CC-SNAP-LIST renders it.

### F4. Pipeline position
F1 runs immediately after CLEAN v1 F1 (marker strip) and before F3 (punctuation trim), because markers attach to the first box only. F2 runs after CLEAN v1 F4 (split/drop) and before F5 (bank canonicalize), so each piece is then canonicalized independently.

---

## Invariants (each one is a test)

- **I-U1 Concatenation.** For every auto-split or suggested split, `pieces.concat() == token` byte-for-byte after NFC. Proptest, 10k cases, every language.
- **I-U2 Whole-word wins.** A token that is itself a bank word is never split and never gets a suggestion (`therapist`, `nowhere`, `carpet`, `Haustür`).
- **I-U3 Box gap is authoritative.** When F1 split a line on a box gap, F2 never re-merges across that gap.
- **I-U4 No invented pieces.** Every piece of an applied split is a bank word for the line's `lang`.
- **I-U5 Policy respected.** A language whose policy is `never` produces zero `SplitSuggestion`s; `suggest` produces zero `applied = true`. Table-driven test over all 15 languages.
- **I-U6 Deterministic.** Same input → identical output. Extends CLEAN v1 I5.
- **I-U7 Idempotent.** Running the pipeline on the pieces produces the same pieces. If proptest finds a counterexample, HALT and report (CLEAN v1 I7 rule).

---

## Decisions

**Signed:** none yet.

**Recommended, applied unless Eric reverses:**
- **D-U1 Per-language policy.**
  - `auto` (auto-split + Rejoin): en, es, fr, pt, pl, ru, fil, sw, hi, ar.
  - `suggest` (chip only, never auto): de, ko, vi.
  - `never` (F2 off; F1 box gaps still honored): zh, ja.
  - *Why:* German compounds and Vietnamese syllable spacing are real words even when absent from the bank; Korean spacing is inconsistent in the wild. Hindi and Arabic are `auto` because word boundaries are orthographically marked by spaces; if the census shows OCR unreliability on these scripts, drop them to `suggest` and say so.
- **D-U2 Ranking.** Fewest pieces, then frequency if available, then stop. No ML, no language model, no LLM.
- **D-U3 `min_piece_len` = 2**, with the per-language one-letter whitelist derived from the bank (F2).
- **D-U4 Gap threshold.** Set from C2 so that no gap on the 10 clean worksheets is missed and no intra-word glyph spacing is counted as a gap. Starting value 0.35 × median glyph width; the census may move it.
- **D-U5 Ambiguity.** When two segmentations tie on piece count and no frequency list exists, suggest rather than apply (F2 condition 3). *Why:* `catsup` vs `cat`+`sup` style ties are exactly where auto-split embarrasses the app.
- **D-U6 Rejoin is one tap and whole.** Rejoining restores the original token; no partial rejoin in v1.

**If Claude Code disagrees with any decision, stop and ask. Never infer a reversal.**

---

## Non-goals

- No change to word-box extraction (CC-SNAP-LIST).
- No lemmatization or inflection handling (CC-SNAP-LEMMA).
- No spell-correction inside tokens (`bigredcog` with `cog` ∉ bank is not split and is flagged `NotInBank`).
- No frequency list added in this file.
- No telemetry about splits.

---

## Acceptance table (`tests/snap_clean_unbunch_golden.rs`)

| # | Input (lang, boxes) | Expected |
|---|---|---|
| 1 | `bigreddog` (en, one box) | `big` `red` `dog`, applied |
| 2 | `big│red│dog` (en, three boxes with gaps) | `big` `red` `dog` via F1, no suggestion field |
| 3 | `catdog` (en) | `cat` `dog`, applied |
| 4 | `applepie` (en) | `apple` `pie`, applied |
| 5 | `therapist` (en) | unchanged, no suggestion |
| 6 | `nowhere` (en) | unchanged |
| 7 | `carpet` (en) | unchanged |
| 8 | `Haustür` (de) | unchanged, `suggest` policy, `Haus`+`Tür` chip only if both in bank |
| 9 | `catsup` (en, tie `cat`+`sup` vs whole-word present) | unchanged (I-U2) |
| 10 | `redcar` (en, tie `red`+`car` vs no other seg) | applied |
| 11 | `bigredcog` (en, `cog` ∉ bank) | unchanged, `NotInBank` |
| 12 | `котсобака` (ru) | `кот` `собака`, applied |
| 13 | `일이삼` (ko, `suggest`) | unchanged, chip if pieces ∈ bank |
| 14 | `一样朋友` (zh, `never`) | unchanged, no suggestion |
| 15 | `aman` (en, `a`+`man` vs whole-word absent) | `a` `man` only if `a` is in the one-letter whitelist; else unchanged |

---

## Phases

- **A.** Census → review → F1–F4, I-U1–I-U7, golden rows 1–15, D-U1 table, D-U4 constant.

## Done when

1. C6 test command passes including all golden rows and proptests.
2. On TestFlight, a worksheet printed in a tight font that build 219 imported as one merged line now imports as separate words, zero edits. Eric verifies by hand on the same photo.

---

## What was built, and the one switch that is off

Added 2026-10-03. The spec above is unedited; this is the build record and the
review findings, kept apart from it.

**Phase A built 2026-10-01, commit 4ef812f2.** `WordBox`, box-gap splitting,
the dictionary splitter, the suggestion field, D-U1's fifteen-language table,
and I-U1 through I-U7 as tests — 25 of them in
`tests/snap_clean_unbunch_golden.rs`.

**`AUTO_SPLIT_ENABLED` is false and needs Eric.** Every D-U decision says
"Signed: none yet", so auto-split would ship an unsigned behaviour — and the
behaviour is not safe at the bank's current size. Measured against the
36,000-word frequency list already in `tools/wordpipe/sources/`: of the 33,068
real English words the 3,165-word bank does NOT contain, **2,218 (6.71%) have a
unique segmentation into bank words** and would be split without anyone being
asked:

```text
maybe    -> may + be        tomorrow -> tom + or + row
listen   -> list + en       asking   -> as + king
yourself -> your + self     ladies   -> la + dies
everyone -> every + one     nobody   -> no + body
```

D-U5 does not help: it suspends on a TIE, and every one of these is
unambiguous. The cause is not the ranking — a 3,165-word bank is about 2% of
English and makes a poor segmentation dictionary. With the switch false the
stage still runs and still attaches its `SplitSuggestion`, so the reader is
offered the split and can take it in one tap, which is what the Intent above
asks for.

**Three acceptance rows do not survive the real bank**, and the tests assert
reality with both values recorded:

- **Row 4** `applepie` cannot split — neither `apple` nor `pie` is in the en
  bank, and F2 requires every piece to be one.
- **Row 8** `Haustür` IS in the de bank, and so are `Haus` and `Tür`. I-U2 says
  a bank word gets no suggestion, so the chip the row asks for violates the
  invariant three lines above it.
- **Row 15** `aman` reasons about a one-letter whitelist that is **empty in all
  fifteen languages** — no bank holds a one-letter entry — and misses the
  unambiguous split it actually has: `am`+`an`.

**Row 6 is right, for a reason the spec does not give.** `nowhere` is not a
bank word, so I-U2 does not protect it; it survives because it has TWO
segmentations (`no`+`where`, `now`+`here`) and D-U5 suspends a tie. Rows 5, 7
and 9 pass only because their second piece is missing from the bank, and one
bank addition flips them.

**Two deviations from the spec, both deliberate:**

- The field is `Candidate.unbunch`, not `Candidate.split`. `FlagSet.split`
  already means "this line was divided on a separator" from v1 F4.
- F1 runs before the fixed-point loop rather than at the position F4 names. A
  marker attaches to the first box, so splitting first leaves it on the first
  unit where the loop strips it exactly as before.

**F2 does not enumerate segmentations.** The spec says to compute every one,
which is exponential; the ranking only needs the minimum piece count and
whether more than one way reaches it, so one dynamic program carries both.

**C1 and C4, answered:** C1 HALTed as written — there was no box plumbing
anywhere. `docs/CC-SNAP-BOXES.md` now owns it and Phases A–C are built, so C1
is satisfied. C4 is partly yes: `en`, `ja` and `ko` have a frequency list under
`tools/wordpipe/sources/`, but build-time only, not in `assets/`, so not
reachable at runtime.
