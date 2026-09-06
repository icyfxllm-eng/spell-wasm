# CC-PICTURE-INTEGRITY v2 — rewritten against the pipeline that exists

**Status:** REVIEW-GATED. v1 is not executable; this replaces it. Nothing here
executes until Eric signs §0's findings and the decisions.
**Target build:** 222 (v1 said 220; 221 shipped 2026-09-03).
**Owns:** geometry and render integrity. Subject decisions stay with
CC-BUILD219-FIXES; F7 approval pinning is the same gap as this file's F6 and
they should be built once, not twice.

---

## §0 — What v1 assumed, and what is actually there

v1 is written against an SVG asset pipeline. **There isn't one.**

```
content-pipeline/wordpic/ref/*.png      403 PNG + 18 JPG references
  -> target/release/trace-pgm           the tracer
  -> content-pipeline/wordpic/scans/*.json   point lists, per subject
  -> tools/bundle_scans.py
  -> config/wordpic/scans.json          include_str!'d into the wasm
```

The 147 SVGs in the tree are draft previews, overlays and frozen-corpus
outputs. No asset is authored as SVG, so these v1 items have no referent:

| v1 item | why it cannot run |
|---|---|
| §0.1 marker census | no SVG assets, therefore no `marker-*` attributes |
| F1 lint on `fill`, `viewBox`, aspect ratio | the asset is a JSON point list |
| F2 `<g id="part-name">` grouping | groups do not exist in the format |
| D1 "restore the authored spine strokes" | cactus is `hand-traced` = derived from `cactus.png` by the tracer; there were never authored strokes to restore |

**D1's own clause fires.** It says stop if the source SVG contradicts the
premise. There is no source SVG for the cactus, so the triangles are what the
tracer made of the reference — not an import artifact. Restoring spines is not
an available action; the choices are re-trace with different parameters, or
accept.

### What the real defect turned out to be

v1's instinct — "import adds and drops geometry" — is a symptom of something
one layer down, measured 2026-09-03:

```
343 of 391 subjects were authored suggested-then-approved,
    meaning their outlines were never derived from their references at all
 86 had ZERO traced ink (100% of reference ink untraced)
342 of 343 fail the 3% continuity bar
```

v1's seven assets are all in that population:

| asset | authoring | paths | reading |
|---|---|---|---|
| dragon | hand-traced | **1** | "fragmented" is 1 path cut into 65 word slots |
| sunflower | hand-traced | 2 | |
| balloon | suggested-then-approved | **1** | one path was never going to hold envelope + knot + string |
| castle | suggested-then-approved | 3 | |
| sailboat | suggested-then-approved | 3 | |
| cactus | hand-traced | 7 | |
| submarine | suggested-then-approved | 4 | archived |

So F5's "two assets are badly drawn" is closer to eighty, and the balloon's
missing string is not a dropped part — it is a subject drawn with one stroke.

**77 of the 86 zero-ink subjects have since been retraced** (gate green,
awaiting a ship number). That work is the real F5, already done for the worst
tier of the problem.

---

## §0 census — rewritten, executable

Per active subject, from the scan docs and the bundle:

1. **Provenance.** `authoring` value, and whether a suggestion file still
   shadows the reference (the side door that produced this whole class).
2. **Geometry.** path count, node count, arc length; subpaths under 1% of bbox
   diagonal; segments collinear (within 0.5 deg) with and within 2% of a bbox
   edge. *These three v1 checks survive verbatim — they are format-independent.*
3. **Continuity.** worst untraced run per reference ink component
   (`tools/ink_continuity.py`, built 2026-09-03).
4. **Trace quality.** ink recall and path precision from `target/release/ink-eval`.
5. **Chunking.** pack word count `N`, total arc, arc per chunk, and whether
   chunk boundaries fall on arc length or on subpath ends.
6. **Parts.** the manifest already carries `parts` and each scan path carries a
   `feature` name — this is F2's hook, and it exists.

**Stop and ask if:** any retraced subject regresses on continuity; more than a
quarter of subjects fail the subpath census (it will — expect ~85%, which is
why F5's queue is the whole bank, not two assets).

---

## F1 — Asset lint, on the format we have

A `lint:pictures` job over `content-pipeline/wordpic/scans/*.json`. Fails the
build on: an orphan subpath under 1% of bbox diagonal; a segment collinear with
a bbox edge; geometry outside a 2% inner margin; **and two the real pipeline
adds** — a word path within the packer's corridor floor of another (this
refused the Dallah's plan and would have shipped a blank card), and a
continuity regression against the recorded value.

**I1:** no asset reaches a build without passing. No override flag.

---

## F2 — Parts, using the fields that already exist

Do not invent SVG groups. Each scan path already carries `feature`; each
manifest already carries `parts`. Declare required parts per subject and assert
each resolves to at least one path of nonzero arc.

**Caveat that decides the scope:** a 1-path balloon cannot satisfy
`[envelope, knot, string]`. F2 does not restore the string — it makes the
absence *fail a test*, and the fix is a re-trace or a re-author. Ordering
matters: retrace first, then declare parts against what the trace produced.

---

## F3 — Geometry constraints

Unchanged from v1. `mirror`, `level`, `centered-on` operate on point lists
directly and need no SVG. Declared per part, never inferred. **Adopt as
written.**

---

## F4 — Chunking, not dash geometry

**v1's premise is factually wrong and the mandatory change is a no-op.** The
unrevealed dash is a fixed CSS constant — `.wp-outline.next {stroke-dasharray:
6 8}` in index.html — and it never reads `N`. Only the *next* stroke is dashed;
the rest render solid at .22 opacity.

What is coupled to `N` is **chunk count**. Dragon and balloon each have ONE
path in their scan doc; dragon's is divided into 65 word slots and balloon's
into 21. That is the 3x fragmentation, and v1's second bullet is the fix:
boundaries at equal arc-length intervals over one ordered polyline, silhouette
before interior detail.

**I4 restated:** the rendered geometry at 0% progress is already a pure
function of the asset. Keep it that way; the acceptance test (dragon at N=65
and N=12 rasterize identically at 0%) should pass **today** and is worth
landing as a regression test even though no change is required.

---

## F5 — The queue is the bank

Budget (<=12 subpaths, <=200 nodes, parts connected) is sound. The disposition
table is not: it lists 7 assets against a population of ~342 failing subjects.
Replace it with the continuity ranking, worst first — 77 done, 265 remaining,
and Eric's judgement needed on how far down to go, since below the zero-ink
tier his original approvals may be the better picture.

---

## F6 — Contact sheet and approval pinning

**Adopt as written, and build it once.** This is the same mechanism as
CC-BUILD219-FIXES F7, and the same gap the forensics found: no approval record
exists for any of the 394 pictures. Rasterizing 0/50/100% per subject works
directly from `scans.json` — no SVG needed.

This is also the only item that addresses the actual root cause. Approval by
playthrough is what let 343 untraced subjects and 15 masterpieces ship.

---

## Decisions

- **D1 — cactus.** v1's premise is void (no source SVG). New question: re-trace
  with different parameters, or accept the current 7-path trace? **Needs Eric.**
- **D2 — chunk cap.** Unchanged from v1; still recommend deferring.
- **D3 — constraints check-only.** Agree with v1.
- **D4 — sheet-level re-approval.** Agree, and it is the only tractable option
  at 77+ retraced subjects.
- **D5 — submarine's pack.** Unchanged, still open.
- **D6 (new) — bundle size.** The 77-subject retrace took `scans.json` from
  975 KB to 1958 KB, and it is `include_str!`'d into the wasm. Accept, decimate
  harder in `bundle_scans.py`, or narrow the retrace set? **Needs Eric.**
- **D7 (new) — continuity gate shape.** Ratchet (fail only on regression) or
  hard 3% bar (red on 265 subjects today)? Recommend ratchet, matching the
  existing density-ratchet precedent.
