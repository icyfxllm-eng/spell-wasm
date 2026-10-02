# CC-SNAP-BOXES v1 — word geometry and page pixels, from the camera to the core

**Status:** Phases A, B and C BUILT 2026-10-01. §0 answered against the live
code; three of its answers changed what the dependent files can ask for. D-B1
and D-B6 are signed.
**Layering:** sits under CC-SNAP-ROADMAP v1, beside CC-SNAP-LIST v1 rather
than above it. It is the file the roadmap's Level 1 is missing: CC-SNAP-CLEAN
v1.1 C1 and CC-SNAP-HIGHLIGHT C1 both HALT on geometry that no file owns, and
"CC-SNAP-LIST v1 (exists)" does not satisfy them because the plumbing is a
*change* to SNAP-LIST, not a property of it.
**Owns:** getting per-token geometry and page-pixel statistics from the
platform recognizer into the Rust core, and the one threshold that decides
what a gap is.
**Does not own:** what anyone does with that geometry. Splitting is
CC-SNAP-CLEAN v1.1; highlight scoring is CC-SNAP-HIGHLIGHT v1; capture, crop
and the review screen stay with CC-SNAP-LIST v1.

---

## Intent

Two Level 1 files are blocked on the same missing thing, and both describe it
the same way: "the per-word bounding boxes the OCR engine already produced."

That phrase is the problem. **Vision does not produce word boxes.** It
produces a line of text and will compute the box of any *range you name
inside it*. The boxes `healSplitWords` uses today are derived from the line
string's own whitespace tokens — so if the space is missing from the string,
there is no second box to find a gap between, and reading boxes can never
recover a space the recognizer did not already emit.

This file exists to say what geometry can actually be had, to carry it across
one bridge in one shape, and to put the gap threshold in one place — because
there is already a second consumer of it on the Swift side doing the opposite
operation, and two thresholds for one measurement is how they come to disagree.

When this spec is incomplete, prefer: geometry the recognizer vouches for over
geometry inferred from text; one shared threshold over a per-caller constant;
statistics computed where the pixels already are over moving pixels to where
the code is.

---

## §0 Census — answered 2026-10-01

Run against the live tree. Each answer names the evidence so it can be
re-checked rather than believed.

**C1. Does the platform hand the core per-word boxes today? NO — and it cannot
in the shape v1.1 assumes.**

`ios/App/App/NativeLanguageKitPlugin+PhotoList.swift` resolves
`["supported": true, "lines": [{text, confidence}]]` (line 239–240). No
geometry crosses the bridge.

The deeper answer is the one that matters. The only call to Vision geometry is
`candidate.boundingBox(for: t.range)` inside `healSplitWords` (line 164),
where `t.range` is a **whitespace-separated token of the candidate's own
string**. The boxes are therefore a function of the text, not an independent
observation of the page. A line that arrives as `bigreddog` has exactly one
token and exactly one box.

So CC-SNAP-CLEAN v1.1's F1 — "the page has gaps but OCR dropped them; fix it by
reading the word boxes the OCR engine already produced" — **has no mechanism on
iOS as written**. There is nothing to read.

What Vision *will* do is return the box for any `Range<String.Index>`,
including a single character. Intra-token gaps are therefore recoverable, at
one call per boundary. That is F1 below, and it is the only honest route to
v1.1's stated intent.

**C2. Is there an Android path? NO.** `android/` carries no text recognition
at all; `src/photo_import.rs` already records this ("the census found there is
only one platform to hold a number for"). CC-SNAP-CLEAN v1.1 C1 asks to
confirm boxes "on both iOS (Vision) and Android (ML Kit `Text.Element`)" — the
Android half is hypothetical. From a photo is an iOS feature today, and this
file plumbs iOS only. **Open for Eric:** whether Android OCR is on the roadmap
at all, since it changes whether the bridge shape should be Vision-shaped or
neutral.

**C3. Is per-word confidence available? NO.** Vision reports confidence per
`VNRecognizedText` candidate — per line — and `photo_import.rs` says so
("Vision reports per line, not per word"). `WordBox { … confidence }` in
v1.1's C1 cannot be filled with a per-word number. Either the field carries
the line's confidence, duplicated, or it goes. D-B4 chooses.

**C4. Is there already a consumer of the gap measurement? YES, and it runs the
other way.** `healSplitWords` merges adjacent tokens when the pixel gap is
small relative to the line's average character width — it exists because
Vision splits `software` into `soft ware`. CC-SNAP-CLEAN v1.1 F1 wants to
split when the gap is large by the same measure. Same quantity, opposite
verdicts, two constants. They must share one threshold and one definition of
"average character width" or a page will eventually be both merged and split.

**C5. Coordinate space.** Vision boxes are normalized (0…1) and origin
bottom-left, relative to the image passed to the request, after the
orientation fix at `cgOrientation(from:)`. The core's `WordBox` is declared
with `x0`/`x1` as plain `f32`. D-B2 fixes the space so that "0.35 of a glyph
width" means the same thing on both sides.

**C6. Can the core see pixels? NO, and it should not try.** The bridge is
Capacitor JSON. A 12-megapixel page as base64 is tens of megabytes per
capture, across a bridge that already carries only text. CC-SNAP-HIGHLIGHT's
F1/F2 offer "the core or a thin platform shim"; C6's answer makes that choice
for us — see F3 and D-B5.

**C7. Test command.** Same as CC-SNAP-CLEAN v1 C7.

---

## Features

### F1. Intra-line geometry, measured where it exists
On the Swift side, for each recognized line:

- Tokenize the candidate string on whitespace, as `healSplitWords` already
  does, and take each token's box via `candidate.boundingBox(for:)`.
- **Additionally**, for every token longer than `MIN_PROBE_LEN` characters,
  probe the boundaries *inside* it: for each interior character index `i`,
  take the box of `[i-1, i)` and `[i, i+1)` and record the horizontal gap
  between them.
- Emit one `WordBox` per token, plus the token's interior gap profile: the
  list of `(char_index, gap)` pairs, in the same normalized units.

Probing is bounded: at most `MAX_PROBES_PER_LINE` boundary queries per line,
spent on the longest tokens first. A line that exhausts the budget emits token
boxes with no interior profile, which is exactly the state everything is in
today — degraded, never wrong.

### F2. One bridge shape
`recognizeWordList` resolves, per line:

```json
{ "text": "...", "confidence": 0.93,
  "boxes": [{ "text": "bigreddog", "x0": 0.11, "x1": 0.48 }],
  "gaps":  [{ "box": 0, "at": 3, "w": 0.021 }, { "box": 0, "at": 6, "w": 0.019 }],
  "glyph": 0.013 }
```

`glyph` is the line's median glyph width in the same normalized units, computed
on the platform where the per-character boxes already are. Carrying it means
the core never re-derives a scale from text length, which is what
`split_on_box_gaps` does today as a stand-in.

Absent keys mean absent geometry. The core must behave exactly as it does now
when they are missing — that is I-B1.

### F3. Highlight statistics, computed at the pixels
For CC-SNAP-HIGHLIGHT, the same bridge carries per-box background statistics
rather than any image:

- `paper_s`, `paper_v`: the page's background estimate (HIGHLIGHT F1).
- per box: `frac_sat`, and `hue` quantized to 12 buckets.

All three are computed in Swift from the already-decoded `UIImage`. No bitmap
crosses the bridge and none is persisted, which satisfies HIGHLIGHT I-H6 by
construction rather than by audit.

This moves one open problem onto the platform rather than solving it: on a
dense page, "background pixels not inside any dilated word box" can be a very
small sample, or empty on a cropped e-reader screenshot. The shim therefore
reports `paper_sample_px`, and HIGHLIGHT's scoring treats a sample below
`MIN_PAPER_PX` as "no paper estimate" and scores nothing highlighted. Failing
to the v1 behaviour is HIGHLIGHT's own stated preference.

### F4. One threshold, one owner
`GAP_RATIO` moves out of `snap_clean.rs` and becomes the single constant this
file owns, consumed by:

- `healSplitWords`, to merge below it (C4's existing behaviour), and
- `split_on_box_gaps`, to split above it.

With one number and one definition of glyph width, "merge" and "split" become
two sides of one comparison instead of two rules that can both fire.

### F5. The core side is already there
`OcrLine` carries `boxes`, `highlighted` and `hue_bucket`, and
`split_on_box_gaps` is written and tested against them (shipped 2026-10-01,
commit 4ef812f2). This file fills them; it changes no signature in
`snap_clean.rs`. `extract_classified` grows a second entry point that accepts
geometry, and the existing text-only one stays, delegating with defaults.

---

## Invariants (each one is a test)

- **I-B1 Absent geometry is the identity.** A payload with no `boxes` produces
  byte-identical candidates to today's text-only path. Golden over the
  CC-SNAP-CLEAN v1 acceptance corpus.
- **I-B2 Geometry never adds or removes a line.** Boxes may change where a
  line is divided; the set of lines reaching `clean_ocr_lines` is unchanged.
  (Extends CLEAN I6.)
- **I-B3 Boxes are ordered and non-overlapping.** `x0 < x1` for each, and
  `x1[i] <= x0[i+1]` across a line. A payload violating this is dropped whole
  and the line falls back to text-only, rather than being half-trusted.
- **I-B4 Normalized and finite.** Every coordinate is in `0.0..=1.0` and not
  NaN. Hostile or garbled payloads cannot reach the splitter.
- **I-B5 Round trip.** For a line whose tokens are space-separated, the
  concatenation of box texts with single spaces equals the line text. This is
  what catches a tokenizer drifting from the geometry it describes.
- **I-B6 One threshold.** A symbol scan finds no second gap constant: exactly
  one definition of `GAP_RATIO` and one of median glyph width in the tree.
- **I-B7 No pixels cross the bridge.** Egress/symbol scan: the payload schema
  admits no image, data URI or base64 field, and nothing persists a bitmap
  past the capture call. (HIGHLIGHT I-H6, enforced here where it is true.)
- **I-B8 Probe budget respected.** A synthetic line of 400 characters issues
  no more than `MAX_PROBES_PER_LINE` geometry calls.

---

## Decisions

**Signed (Eric, 2026-10-01):**

- **D-B1 — SIGNED.** Per-character probing is the mechanism. Built.
- **D-B6 — SIGNED.** iOS only; Android dropped. C2's open question is closed:
  no Android OCR, so the payload is Vision-shaped without apology.

**Recommended, applied unless Eric reverses:**

- **D-B1 Per-character probing is the mechanism, not per-word boxes.** C1 shows
  word boxes cannot recover a dropped space. If Eric would rather not pay the
  probe cost, the honest alternative is to **drop CC-SNAP-CLEAN v1.1 F1
  entirely** and let unbunching rest on the dictionary path alone — which, per
  that file's own measurement, is currently switched off. Those are the two
  real options; "pass the boxes through" is not one.
- **D-B2 Normalized coordinates, origin top-left, orientation already applied.**
  Vision's bottom-left origin is flipped once, in Swift, at the boundary. One
  flip in one place beats a convention the core has to remember.
- **D-B3 Median glyph width is computed on the platform** and carried, because
  that is where per-character geometry exists. The core's current estimate
  (box width ÷ character count) stays only as the fallback when `glyph` is
  absent.
- **D-B4 `WordBox.confidence` carries the LINE's confidence**, documented as
  such, rather than being removed. Removing it would mean changing a published
  struct again when a future recognizer does report per word; carrying a
  duplicated value with a comment costs nothing and keeps the shape stable.
- **D-B5 Highlight statistics are computed in Swift, never in the core.** C6.
  This is a deviation from CC-SNAP-HIGHLIGHT F1/F2, which offer the core as an
  option; the bridge makes that option impractical rather than merely slower.
- **D-B6 iOS only.** SIGNED above. No Android shape is designed, and the
  payload may use Vision's conventions freely.
- **D-B7 `MIN_PROBE_LEN` = 8, `MAX_PROBES_PER_LINE` = 120, `MIN_PAPER_PX` set
  by the Phase A measurement.** A token under 8 characters is not a merged
  phrase worth probing; 120 boundaries is far past any worksheet line and
  bounds the worst case at well under a frame.
- **D-B8 This file does not re-litigate what geometry is for.** Whether
  auto-split is safe is CC-SNAP-CLEAN v1.1's measured question and Eric's
  call; whether highlight thresholds hold is CC-SNAP-HIGHLIGHT's fixture. This
  file is done when the numbers arrive intact.

**If Claude Code disagrees with any decision, stop and ask. Never infer a
reversal.**

---

## Non-goals

- No change to what Vision is asked to recognize, to capture, or to crop.
- No bitmap, data URI or image bytes across the bridge, ever.
- No splitting, merging, highlight scoring or bank matching in this file.
- No Android implementation, and no Android-shaped payload.
- No telemetry about geometry.
- No second gap constant, in either language.

---

## Acceptance table (`tests/snap_boxes_bridge.rs` + one Swift unit test)

| # | Input | Expected |
|---|---|---|
| 1 | Payload with no `boxes` key | identical candidates to the text-only path (I-B1) |
| 2 | `big red dog`, three boxes, real gaps | three candidates, no suggestion |
| 3 | `bigreddog`, one box, interior gaps at 3 and 6 | three candidates via F1 |
| 4 | `bigreddog`, one box, no interior profile (budget spent) | one candidate; U2 may offer |
| 5 | Boxes out of order | payload dropped, line falls back to text (I-B3) |
| 6 | `x1 < x0` on one box | payload dropped (I-B3) |
| 7 | Coordinate `1.4` | payload dropped (I-B4) |
| 8 | Coordinate `NaN` | payload dropped (I-B4) |
| 9 | Box texts rejoin to the line text | passes I-B5 |
| 10 | Box texts disagree with the line text | payload dropped, logged in DEV_PREVIEW only |
| 11 | 400-character line | ≤ `MAX_PROBES_PER_LINE` geometry calls (I-B8) |
| 12 | `soft ware` with a tight gap | merged once, by `healSplitWords`, and not re-split (F4) |
| 13 | Page with `paper_sample_px` below `MIN_PAPER_PX` | no highlights, review opens as v1 (F3) |
| 14 | zh line with a real box gap | split — box evidence is language-independent (v1.1 D-U1 governs only the dictionary path) |
| 15 | Payload containing an image field | rejected by schema; I-B7 scan fails the build |

---

## Phases

- **A — BUILT 2026-10-01.** Swift: F1 probing with the D-B7 budget, F2 payload,
  the `glyph` measurement, one shared tokenizer. JS: geometry passes through
  unjudged. Rust: `CharGap`, `OcrLine.glyph`, `geometry_ok`, interior-gap
  splitting, `extract_classified_geo`, and the bridge parser.
  `tests/snap_boxes_bridge.rs` — 18 tests, rows 1–10 and 14, I-B1 through
  I-B5 and I-B7.

  **Not built in A, and deliberately:**
  - **F3 (highlight statistics) is not written.** It needs the
    CC-SNAP-HIGHLIGHT C2 fixture to calibrate `MIN_PAPER_PX`, and that fixture
    is a 20-photo folder Eric assembles. Writing the shim first would mean
    inventing the constant it exists to carry. CC-SNAP-HIGHLIGHT C1 therefore
    still HALTs on pixels; v1.1's C1 does not, which is what unblocks
    unbunching.
  - **I-B8 and row 11 (probe budget) are asserted in Swift, not here.** They
    are properties of the probing loop. The loop is written with the budget;
    the test is not, and no Swift test was run on this machine.
  - **Row 12 and I-B6 are Phase B** — the threshold is not hoisted yet, so
    there are still two constants and the invariant would fail honestly.
- **B — BUILT 2026-10-01.** `config/snap-geometry.json` is the one source for
  every gap threshold. Rust bakes it in with `include_str!`, so the core
  cannot drift. Swift cannot read it — the file is not in the Xcode target and
  adding it would mean touching the pbxproj — so the agreement is enforced by
  `scripts/snap-geometry-check.mjs`, in `gate.sh` and the pre-push hook (29
  checks now), lesion-tested four ways. Rows 12 and 14 and the Rust half of
  I-B6 are tested; 20 tests in `snap_boxes_bridge.rs`.

  **What the hoist exposed.** There were FOUR thresholds for one measurement,
  not two: `0.35` in the core, and `1.1` / `0.45` / `0.25` inside
  `healSplitWords`. They are not complements — the plugin merged at or below
  `0.45` while the core split at or above `0.35` — so a gap between the two
  was both a space and not a space depending on which half of the bridge you
  asked. Phase B puts all four in one file and forbids a fifth; it does not
  reconcile them, because changing a merge rule with a device bug report
  attached ("five words written close together imported as ONE entry") is not
  something to do blind.

  **The consequence Phase C then fixed.** Phase A skipped a line's geometry
  whenever healing rewrote its text, which is what kept the two rules from
  ever firing on one line — and also meant a healed line lost probing for
  *its other tokens*.

- **C — BUILT 2026-10-01.** The merge moved into the core.

  `healSplitWords` is gone. In its place the plugin reports, per boundary, the
  three judgements UITextChecker can make and this file cannot delegate
  (`left`, `right`, `joined`), plus `avgChar` and whether the language has a
  dictionary at all. `snap_clean::merges_with_previous` makes the decision.

  **The rule did not change.** It is transcribed with its asymmetries intact:
  a moderate gap merges only when the join is a real word, a tight gap merges
  only when at least one side is not — the over-merge guard that keeps
  "cat dog" two words however cramped the handwriting — and a language with no
  dictionary gets geometry alone at the stricter ratio. The denominator is
  still `avgChar`, the mean the rule has always divided by, NOT the median
  glyph width the split side uses. Unifying those two is a measurable change
  with a device bug report attached and is not smuggled into a move.

  **The gain.** No heal means no rewritten text, so the line keeps its
  geometry: `soft ware bigreddog` now merges the first two tokens *and* probes
  the third. That is tested.

  **A consequence worth noting.** With the rule in the core, the Swift
  threshold mirror Phase B introduced became dead code and is removed — the
  cross-language contract is a single-language one now.
  `snap-geometry-check.mjs` stays, because a bare multiplier reappearing in
  either language is what would start the drift again, and because a mirror
  added back must still match.

  Six new tests hold the moved rule to the cases it was written for:
  `software` heals, `cat dog` never merges at any gap, `sof tware` heals only
  on a tight gap, a dictionary-less language falls to geometry, a healed line
  keeps probing, and the merge still divides by `avgChar`.

## Done when

1. C7's test command passes with every row and invariant. **Phases A, B and
   C: 26 tests green, full gate green.** Row 11 and I-B8 (the Swift probe budget)
   remain untested, and the Swift half of I-B6 is enforced by
   `snap-geometry-check.mjs` rather than by a Swift test.
2. CC-SNAP-CLEAN v1.1 C1 no longer HALTs — **done**, this file provides the
   geometry. CC-SNAP-HIGHLIGHT C1 still HALTs, on F3's pixel statistics.
3. On TestFlight, a worksheet whose words Vision returns merged imports as
   separate words with zero edits — the same photo and the same bar as v1.1's
   own done-check, now with a mechanism behind it. **Eric verifies by hand;
   nothing on this machine can.**

## What is still unverified

Everything on the Swift side. The probing loop, the budget, the payload keys
and the one-tokenizer sharing are written and read, and not one of them has
been compiled or run — there is no Xcode build or Swift test in this work. The
Rust side is fully exercised against synthetic payloads, which is precisely the
reason `geometry_ok` rejects a payload whole rather than trusting parts of it:
the core is the only half of this bridge that has been tested.
