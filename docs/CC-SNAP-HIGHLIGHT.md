# CC-SNAP-HIGHLIGHT v1 — Highlighted words only

> **Provenance and status, added when this was committed on 2026-10-03.**
> Everything between the rules below is Eric's, verbatim. It had lived only in
> a chat session since 2026-10-01. Nothing in it has been edited; the review
> findings are kept separate, after the spec.
>
> **Scoring core built 2026-10-04; still not reachable, and C1 still HALTs.**
> `src/snap_highlight.rs` implements F1/F2 scoring and F3 rejoin, with
> `tests/snap_highlight_core.rs` covering I-H2, I-H3, I-H4 and I-H5. What the
> C2 fixture blocks is CALIBRATION, not the arithmetic, so the arithmetic is
> written and the thresholds are honest about not being calibrated:
> `min_paper_px` is null in `config/snap-highlight.json`, and while it is null
> the detector scores NOTHING, which is the Intent's own fall-back. Nothing is
> wired to the review screen and no Swift shim exists.
> **The 20-photo folder is still the single thing blocking Level 1.3.**

---

**Status:** REVIEW-GATED. §0 census executable on Eric's signature. Phases A–B blocked on census review.
**Layering:** new file under CC-SNAP-ROADMAP v1 (Level 1.3). Depends on CC-SNAP-LIST v1 (capture, crop/rotate, word boxes per CLEAN v1.1 C1) and CC-SNAP-CLEAN v1/v1.1 (all text goes through `clean_ocr_lines`).
**Owns:** deciding which OCR'd words sit on a highlighter mark, and the "Highlighted (N)" default filter on the review screen. Nothing else.
**Does not own:** OCR, cleanup, bank matching, the review screen's layout (only the filter state it opens in), My Words storage.

---

## Intent

A reader working through a book marks the words they don't know with a highlighter. The page holds 300 words; they care about 5. Importing all 300 and asking the parent to delete 295 is the opposite of least editing.

The feature should find the marked words with no model, no cloud, and no settings, and should fail toward **showing everything** (the v1 behaviour) rather than toward hiding a word the reader wanted.

When this spec is incomplete, prefer: a false positive (one extra word shown as highlighted) over a false negative (a marked word hidden); the whole-page list always one tap away; heuristics the census can tune over any learned model.

---

## §0 Census (read-only; report before any code)

**C1. Boxes + pixels.** Confirm CC-SNAP-LIST passes the cropped, rotation-corrected bitmap and per-word boxes (CLEAN v1.1 C1) to the core together. **HALT** if boxes are available only on the platform side with no pixel access, or vice versa — the two must meet in one place.

**C2. Fixture.** Eric assembles a 20-photo folder: 10 worksheets (no highlights), 5 book pages with yellow highlights, 3 book pages with two colours, 2 e-reader screenshots with highlights. Each photo gets a sidecar `expected.json` listing the highlighted words. This file's acceptance tests run over the fixture; the census only confirms it exists and loads.

**C3. Paper estimate.** Measure per-photo: median background colour of non-text pixels (text pixels = luminance below an Otsu threshold), its saturation (HSV S), and the saturation distribution of every word box's background. Report the two histograms (highlighted vs non-highlighted boxes, from `expected.json`). This sets D-H2.

**C4. E-reader screenshots.** Confirm Apple Books / Kindle highlight colours in the two fixture screenshots. Report their S values; they are flatter than physical marker and may need their own threshold (D-H3).

**C5. Hyphenation.** Confirm the OCR engine marks a trailing `-` at line end as part of the word box text. F3 relies on it.

**C6. Test command.** Same as CLEAN v1 C7.

---

## Features

### F1. Paper model
- Convert the cropped bitmap to HSV (on-device, in the core or a thin platform shim that returns per-box statistics — census C1 decides which; either way the pixels never leave the device).
- `paper_S` = median S over all pixels not classified as text and not inside any word box's dilated region.
- `paper_V` similarly. Both are per photo, recomputed every capture; nothing is cached across photos.

### F2. Per-word highlight score
- For each word box, dilate by 15% of box height, mask out text pixels (luminance below Otsu), and take the remaining background pixels.
- `frac_sat` = fraction of those pixels with `S − paper_S > ΔS` (D-H2) **and** `V ≥ V_min` (excludes shadows and pen ink).
- A word is `Highlighted` when `frac_sat ≥ coverage` (D-H1, default 0.50).
- Store per word: `highlighted: bool`, `hue_bucket: Option<u8>` (dominant hue of the saturated pixels, quantized to 12 buckets), `frac_sat` (for debug display in DEV_PREVIEW only).

### F3. Line-break rejoin
- When a highlighted word ends in `-` and the next line's first word is highlighted too, join them (`remem-` + `ber` → `remember`) **before** `clean_ocr_lines`. The join is recorded in `Candidate.flags` as `HyphenJoin` with the two originals, so the review screen can show the seam if tapped.
- Only highlighted-to-highlighted joins. A hyphenated word with only one half marked is left as two entries with `Blank`-style unchecked state; the reader decides.

### F4. Filter state on the review screen
- If `N = count(highlighted) > 0`, the review screen opens on **Highlighted (N)** with **All words** as a visible one-tap toggle.
- If `N = 0`, the screen opens exactly as CC-SNAP-LIST v1 does today. No empty "Highlighted (0)" tab.
- Highlighted candidates are checked by default **unless** a CLEAN flag unchecks them (`LowConfidence`, `HasDigits`, `Blank`). `MultiWord` does not uncheck a highlighted phrase — a reader who marks "in spite of" wants it (D-H5).

### F5. Colour chips
- When highlighted words span ≥2 `hue_bucket`s, show one chip per bucket (named by nearest of yellow / green / pink / orange / blue; otherwise "Colour N"), all selected. Deselecting a chip hides that colour's words from the Highlighted filter only; All words is unaffected.
- Single-colour pages show no chips.

### F6. Pass-through
- Every word, highlighted or not, still runs through `clean_ocr_lines` (DOC-5). This file adds `highlighted`, `hue_bucket` and `HyphenJoin` to `OcrLine`/`Candidate`; it adds no second text path.

---

## Invariants (each one is a test)

- **I-H1 Nothing hidden from All words.** The All words list is byte-identical to what CC-SNAP-LIST v1 produced without this file. Golden on the 10 worksheet fixtures.
- **I-H2 No highlights on plain paper.** The 10 worksheet fixtures and any synthetic page tinted uniformly warm (lamp) or aged yellow produce `N = 0`. The tint test is synthetic: take a fixture, apply a global hue/sat shift within the C3-observed lamp range, assert `N = 0`.
- **I-H3 Partial counts.** A word with ≥50% box coverage counts (D-H1); a word with <50% does not. Synthetic test paints a controlled fraction.
- **I-H4 Rejoin is exact.** `HyphenJoin` pieces concatenate (minus the `-`) to the joined text. Proptest.
- **I-H5 Determinism.** Same bitmap + boxes → identical output. No randomness, no time.
- **I-H6 On-device.** Symbol/egress scan finds no network call reachable from this module, and no bitmap persisted beyond the capture session.
- **I-H7 Jr unreachable.** The Highlighted filter and chips are never constructed for a Jr profile (inherits ROADMAP DOC-4; the whole From a photo flow is already gated, this is a belt-and-braces assertion).

---

## Decisions

**Signed:** none yet.

**Recommended, applied unless Eric reverses:**
- **D-H1 Coverage = 0.50.** A half-highlighted word counts. *Why:* readers overshoot and undershoot the word edges; 50% catches a sloppy stroke without catching a neighbour's stroke bleed.
- **D-H2 ΔS and V_min from the census.** Starting values ΔS = 0.25, V_min = 0.35; C3 sets final values so the fixture scores 0 false negatives and ≤1 false positive per page.
- **D-H3 E-reader screenshots** get their own ΔS if C4 shows they need it; the two thresholds live in one config struct, keyed by `source: Camera | Screenshot` (CC-SNAP-LIST already knows which).
- **D-H4 Heuristic, not ML.** No classifier in v1. Revisit only if the fixture cannot be made to pass with D-H2 tuning. If a model is ever proposed, it is a new file and must still satisfy I-H6.
- **D-H5 Highlighted phrases stay checked.** `MultiWord` does not uncheck a highlighted run. The reader marked it on purpose.
- **D-H6 Pen underlines, circles, brackets, margin notes: out of v1.** Flag for a v2 file; do not partially implement.
- **D-H7 Colour chips default all-on;** no persisted colour preference in v1.
- **D-H8 DEV_PREVIEW overlay.** In DEV_PREVIEW and TestFlight only, a long-press on the review screen shows each box tinted by `frac_sat`. This is Eric's gradable artifact for tuning D-H2; it is compiled out of production.

**If Claude Code disagrees with any decision, stop and ask. Never infer a reversal.**

---

## Non-goals

- No ML model, no cloud vision, no photo upload.
- No change to OCR, cleanup, bank matching or My Words.
- No underline / circle / bracket detection.
- No per-user colour preferences or highlight history.
- No telemetry about highlights.
- No change to the worksheet (N = 0) path — it must remain pixel-identical to v1.

---

## Acceptance table (`tests/snap_highlight_fixture.rs`, runs over the C2 folder)

| # | Fixture | Expected |
|---|---|---|
| 1 | Book page, 5 yellow words among ~300 | `N = 5`, exactly those words, all checked; All words lists ~300 |
| 2 | Worksheet, no highlights | `N = 0`, screen identical to v1 |
| 3 | Book page under warm lamp, no highlights | `N = 0` |
| 4 | Aged yellowed page, no highlights | `N = 0` |
| 5 | One word 60% covered | counted |
| 6 | One word 30% covered | not counted |
| 7 | Highlight wraps two lines, 4 words | `N = 4`, one entry per word |
| 8 | `remem-` / `ber` both highlighted | `remember`, `HyphenJoin` |
| 9 | `remem-` highlighted, `ber` not | two entries, unchecked |
| 10 | Page with yellow + pink | two chips; deselect pink → only yellow words in Highlighted |
| 11 | Apple Books screenshot, 3 highlights | `N = 3` |
| 12 | Kindle screenshot, 2 highlights | `N = 2` |
| 13 | Highlighted phrase "in spite of" | one candidate, `MultiWord` set, **checked** |
| 14 | Highlighted word with low OCR confidence | present in Highlighted, unchecked (CLEAN rule wins) |
| 15 | Pen-underlined word, no highlighter | `N = 0` (D-H6; documents the limit) |

---

## Phases

- **A.** Census → review → F1, F2, F4, I-H1–I-H3, I-H5–I-H7, fixture rows 1–7, 11–15, D-H8 overlay.
- **B.** F3 rejoin, F5 chips, I-H4, fixture rows 8–10.

## Done when

1. C6 test command passes with every fixture row and invariant.
2. On TestFlight, Eric photographs a book page he has highlighted himself and the review screen opens on exactly his words, with zero edits, and All words is one tap away.

---

## Where this is blocked

Added 2026-10-03. The spec above is unedited; this is the review record.

**Built since this review, 2026-10-04.** `src/snap_highlight.rs` (F1/F2
scoring, F3 rejoin), `config/snap-highlight.json` (the one source for every
threshold), `scripts/snap-highlight-check.mjs` (in gate.sh and pre-push) and
`tests/snap_highlight_core.rs` (14 tests incl. two proptests). Three notes on
how it treats the findings below:

- **D-H1 vs rows 5/6 is resolved in favour of D-H1.** The code thresholds
  `frac_sat` — background pixels after text masking — because that is what the
  signed-style decision says. Rows 5 and 6 describe box AREA, which for a bold
  word is a different number. The acceptance rows need rewording, not the code.
- **I-H1 vs F3 is left open, and nothing depends on it.** The core does not
  decide whether a HyphenJoin is an exception to "byte-identical to v1"; it
  just reports the join with both originals.
- **D-H8's overlay is NOT built.** It collides with `seam-absence-check.mjs`
  and the review record says that is Eric's call. `frac_sat` is computed and
  returned regardless, so the overlay is a UI job when he rules.

The histogram-across-the-bridge choice is new and is argued in the module's
own header: if Swift applied ΔS, every re-tune after the fixture would need an
app build and the two copies would drift. Swift sends the distribution; every
threshold stays on the Rust side in one file.

**The first two fixture shots are filed, 2026-10-06, and C4 is answered.**
Two e-reader screenshots, as `tests/fixtures/snap-highlight/11-ereader-pale`
and `12-ereader-saturated`. Measured at the fixture's own 1600px:

| | paper S | mark S median | mark S range | hue |
|---|---|---|---|---|
| 11, **Kindle** | 0.000 | 0.549 | 0.27 – 0.60 | 53° |
| 12, **Apple Books** | 0.000 | 1.000 | 0.49 – 1.00 | 50° |

Both the pairing and the assignment are Eric's, confirmed 2026-10-06. The
assignment was first read off the chrome — Kindle's "Learning reading
speed" indicator on one, Apple Books' "Sample" and page number on the
other — and then confirmed correct, so nothing in this table is inferred.
Note it inverts the original row names, which had 11 as Apple Books.

**Kindle is the tight one.** Its palest highlight pixels sit at S=0.27
against a ΔS of 0.25, so the two hundredths of margin belong to the reader
with the larger installed base, not to an edge case.

So C4's question — do screen highlights need their own ΔS — has a provisional
answer of **no**: D-H2's starting 0.25 separates both. But row 11's palest
pixels sit at 0.27, which clears it by two hundredths, so D-H3's
per-source threshold should stay available rather than be deleted as unused.

Two physical pages were measured alongside them and are not filed (they are
not fixture rows): a form marked in cyan, paper S 0.039, ink at hue 180-210°;
and an aged book page with a red strikethrough, **paper S 0.153**, ink at hue
0-15°. That second number is the one worth keeping: an ABSOLUTE saturation
threshold would have called that entire page highlighted. F1's
paper-relative cut is load-bearing, not a refinement.

**F2's text mask is not sufficient, and the fixture caught it before any code
ran.** Row 11 has two saturated clusters, not one: the yellow highlights at
hue ~53° (28k px) and the blue "1.2.2 Jupyter Notebook" section heading at
hue ~200° (15k px). F2 says to mask text pixels by "luminance below an Otsu
threshold"; that cut fell at L=143 and mid-tone coloured type is brighter, so
the heading survives as background and reads as strongly saturated. On a
technical book with coloured section titles, every heading would import.
A luminance-only mask cannot fix this — the mask has to know that saturated
*strokes* are type. Eric's call how, and it changes F2.

**Two scenes added to the shoot (rows 16 and 17), both false-positive hunts.**
16 is a page of coloured headings with no highlighter, which must score zero;
it is the scene that would have caught the above. 17 is a printed form marked
in a non-yellow highlighter, because the shot list had assumed worksheets are
unmarked and marked pages are books, and a form somebody highlighted at work
is neither. **The acceptance table above is Eric's and still has 15 rows** —
these two want rows 16 and 17 added to it, which is his to sign, not mine to
edit.

**Strikethrough is undefined in v1.** D-H6 excludes pen underlines, circles,
brackets and margin notes; it does not mention a line drawn THROUGH a word.
The red one measured above is cleanly separable by colour, but `frac_sat` is
the fraction of a box's background that reads marked and a thin stroke
plausibly lands under D-H1's 0.50 — untested. The semantic question matters
more than the geometry: a struck-through word usually means *done*, which is
the opposite of *teach me this*.

**F2's text mask is fixed, 2026-10-06, and F2's wording no longer matches
the code.** The absolute `v_min` is replaced by a paper-relative
`max_v_drop` of 0.45. A highlighter is a FILL and keeps most of the page's
brightness; coloured type, pen ink and shadow are strokes and are far darker
than the page they sit on. Measured V drop below paper, across six real
pages:

| | V drop |
|---|---|
| Apple Books highlight | 0.00 |
| Kindle highlight | 0.05 |
| yellow highlighter on cream book paper (row 18) | 0.08 |
| cyan marker on a photographed form | 0.35 |
| **Kindle's blue section heading — type** | **0.53** |
| red pen strikethrough — ink | 0.59 |
| gutter shadow of an open book | 0.77 |

Marks 0.00–0.35, everything else 0.53–0.77. Row 18 is the load-bearing
measurement: it is the only physical highlighter on physical paper, and
without it the rule rested entirely on pixel-perfect screenshots.

The shim reports one number per box (`mark_v_drop`); the threshold stays in
the core so the fixture can retune it without an app build. A box that trips
it reports `Rejected::TooDarkForAMark` rather than scoring zero in silence.
It also keeps pen strikethrough out with no special case, which D-H6 wanted.

**This changes F2, which is Eric's text.** The spec above still says "mask
out text pixels (luminance below Otsu)" and `V >= V_min`; the code no longer
does either. The wording needs his edit, not mine.

**Row 18 filed**: an open book with the facing page in frame. The facing
page is the row's point — it is readable, so OCR returns words from a page
nobody meant to import, and `must_not_import` is the interesting half of its
ground truth.

**Row 19 filed, and it changes how the two rules should be read.** A
handwritten list in biro on ruled paper, shot in poor light: paper V 0.580,
ink 0.38 below it — **under** `max_v_drop`, so the fill rule does not reject
it. The page is still safe because the ink is 0.03% of the page and no box
comes near D-H1's 0.50 coverage.

So the two filters divide the work, and neither is redundant:

- **Coverage** catches THIN marks. Pen strokes and handwriting never fill a
  word box, whatever colour they are.
- **The fill rule** catches DARK things that DO fill one — a coloured
  heading, a gutter shadow — which coverage alone would wave through.

**Do not tighten `max_v_drop` to catch row 19's ink.** Real marks top out at
0.34 (a cyan marker on a form) and that ink is 0.38; a threshold in the
0.04 between them would start rejecting markers in dim light. Normalising
the drop by the page's paper-to-text range was tested as the alternative and
is worse — it puts the cyan marker at 1.13 above the blue heading at 0.60,
so nothing separates them. Recorded so it is not re-tried.

Row 19 also carries a question for CC-SNAP-CLEAN rather than this file: all
three words are genuinely hyphenated, `mother-in-law` twice over, and the
hyphens are mid-line rather than line-break ones — F3's rejoin must not
touch them and CLEAN must not split on them.

**The F2 fix is NOT sufficient, and row 16 proves it. 2026-10-06.**
`max_v_drop` was set from one heading sample at 0.53. A second real page —
fixture row 16, a Kindle screenshot of a Python book — has blue headings at
**0.29**, under the threshold. Its section titles would import, and coverage
will not save it: they are 4.3% of the page in large type, not thin ink.

**No threshold fixes this.** Coloured headings now span 0.29–0.53; real
marks span 0.00–0.35, the dimmest being a cyan marker at 0.34. They overlap.
Lowering `max_v_drop` to 0.29 would reject that marker.

The difference is structural rather than chromatic, which is what F2's mask
was always for. A highlight is a FILL: its saturated pixels sit in the gaps
BETWEEN glyphs and the strokes stay dark inside it. Coloured type is the
inverse — the saturated pixels ARE the strokes, and the gaps are paper. A
mask that asked "is this pixel part of a stroke" rather than "is this pixel
dark" removes the heading entirely and leaves the highlight untouched.

So the standing position: `max_v_drop` stays, because it does reject pen ink
(0.55–0.59) and gutter shadow (0.70–0.77) that coverage alone would pass. It
is necessary and not sufficient. The sufficient fix is a stroke-aware text
mask, which is platform image work and a deeper change to F2 than the one
already made. Eric's call, and it is now evidenced rather than argued.

`known_gap_a_light_coloured_heading_still_reads_as_a_highlight` pins the
wrong answer on purpose so it cannot be forgotten.

**Four more rows filed**: 1 (six marks on a dense page, one of them only
partly covered), 8 (Bud-/dhist across a line break, with `fifty-two`
mid-line on the same page as F3's own negative), 15 (pen underlines,
documenting D-H6), 16 (the above). Eight of twenty-five.

**Rows 5 and 6 filed from one frame, 2026-10-06.** A close crop carrying
both the positive and the negative: `retur` of "returning" (5 of 9 letters,
~56%, COUNTS) and `se` of "second" (2 of 6, ~33%, does NOT). Same paper,
same light, same marker — better evidence than two pages shot apart, which
is why the same pixels are filed under both row names with different
sidecars. The difference between those sidecars is the test.

Coverage is recorded by LETTER COUNT. An automated width measurement was
tried and deliberately not recorded: at this crop the kerning gaps inside a
word are close to the word spaces, and show-through from the reverse side
fills some of them, so every denominator came out as two or three words.
Real percentages need OCR boxes the core does not have. It does not matter
here — what these rows score against is WHICH WORD COUNTS, not a number.

It does, though, sharpen the open D-H1 question. D-H1 thresholds `frac_sat`,
the fraction of background pixels after text masking; rows 5 and 6 describe
box width. These photos are described in width because that is what a person
can see. For a dense word the two differ, and a detector could pass the row
text while failing D-H1 or the reverse. Still Eric's to resolve.

**C1 HALTs, and C2 is why it stays halted.** `docs/CC-SNAP-BOXES.md` now
delivers word boxes to the core, so half of C1 is satisfied. The other half —
pixels — is deliberately not built. CC-SNAP-BOXES F3 specifies the shim but
leaves it unwritten, because `MIN_PAPER_PX` and D-H2's ΔS can only be
calibrated against the C2 fixture, and writing the shim first would mean
inventing the constants it exists to carry. **The 20-photo folder is the
single thing blocking Level 1.3.**

**C1's "either way" is already decided, and not the way F1 assumes.** F1
offers "the core or a thin platform shim". The bridge is Capacitor JSON, and a
12-megapixel page as base64 is tens of megabytes per capture. So the
statistics are computed in Swift where the pixels already are, and only
numbers cross — which also satisfies I-H6 by construction rather than by
audit. That is CC-SNAP-BOXES D-B5.

**Four findings from the review, each needing a decision or a wording fix:**

- **F1's paper estimate can have almost nothing to measure.** `paper_S` samples
  pixels "not inside any word box's dilated region". On a 300-word book page
  with 15% dilation that is margins only, and on a cropped e-reader screenshot
  it can be empty. CC-SNAP-BOXES carries `paper_sample_px` so a sample below
  `MIN_PAPER_PX` scores nothing highlighted rather than scoring wrongly — but
  the threshold itself is unset until the fixture exists.
- **I-H1 contradicts F3.** "The All words list is byte-identical to v1" is
  false on any page where `remem-`+`ber` joins, because the join happens before
  cleanup. It passes only because it is golden'd on the 10 no-highlight
  worksheets. Scope the invariant to `N = 0`, or state HyphenJoin as a
  deliberate exception.
- **D-H1 and rows 5/6 are in different units.** D-H1 thresholds `frac_sat`,
  the fraction of *background pixels after masking text*; rows 5 and 6 describe
  a word "60% covered" and "30% covered", which is box area. For a bold word
  with many text pixels these are not the same number.
- **D-H8 collides with the test-seam gate.** "DEV_PREVIEW and TestFlight only
  … compiled out of production" — TestFlight ships the release binary, and
  `scripts/seam-absence-check.mjs` greps the shipped bundle to prove seams are
  absent. A long-press debug overlay present in TestFlight is either a runtime
  flag in the production binary, which that gate exists to forbid, or it needs
  a third build configuration. Eric's call which.
