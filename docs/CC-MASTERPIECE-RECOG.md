# CC-MASTERPIECE-RECOG — Masterpiece Outline Recognizability Pass

**Status:** REVIEW-GATED. Amendment atop CC-PICTURE-BANK, CC-MASTERPIECE-TONAL,
and CC-SPELLPIC-AUDITPASS F8 (scan queue). Tool-side (SCAN-STACK T0 lane).
Nothing here touches the app runtime.

Pasted by Eric 2026-08-11. Saved verbatim; the transcript is not a safe home
for law. Session notes at the bottom, separated from Eric's text.

## Intent

The Aug 11 outline review failed: Red Fuji read as Starry Night, Sunflowers
rendered as an over-extracted scribble, The Scream was marginal. Only the Great
Wave was recognizable. Root cause is not scan quality alone — the extractor
treats every edge equally, so a cloud fragment gets the same visual weight as
the mountain that names the piece. **A masterpiece outline in Spell Picture
must be instantly nameable with zero guessing.** Recognition comes from an
unbroken silhouette plus 2–3 signature anchors at correct relative position and
scale — never from edge density. Every rule below serves that sentence; where
this file is silent, decide in favor of it.

---

## Features

**F0 — Step-0: prove which extraction path produced the Aug 11 outlines.**
Before re-authoring anything, dump provenance for all four traces. Prime
suspect: these bypassed the v7.x tracer pipeline (pinned scans, containment
tree, required-features lint) and came from a raw edge pass — the Red Fuji
noise dots are the fingerprint of unfiltered edge extraction. If the pipeline
was bypassed, fix the routing first; re-authoring pieces through a broken path
wastes every hour spent. If provenance can't be determined: **stop and ask.**
Never tune two variables at once.

**F1 — Source scan quality bar.**
Each piece gets one pinned reference scan: PD/open-access museum source (Met
Open Access, Rijksmuseum, Art Institute of Chicago, Wikimedia Commons
highest-res file), minimum ~4000 px long edge, provenance URL + license
recorded in registry metadata. Existing PD-only license CI gate applies
unchanged. **References never ship** (standing invariant, restated here because
this file handles them constantly). A photo of a print or a compressed
thumbnail is never a legal source.

**F2 — Three-layer stroke hierarchy (the core mechanism).**
Every masterpiece outline is authored in exactly three layers:

- **Silhouette** (heaviest weight): the one continuous shape a human would draw
  first — Fuji's slope, the Wave's crest, the Scream figure's S-curve, the
  Sunflowers vase-and-bloom mass. Must be topologically unbroken; a fragmented
  silhouette is a build failure.
- **Anchors** (medium weight): the piece's registry `requiredAnchors` (F4).
  This generalizes the TONAL portraits' `requiredFeatures` lint to **all**
  masterpieces regardless of extractionClass. Missing anchor = export blocked.
- **Texture** (thin, optional): everything else, curated only.

Lints, all CI-enforced: silhouette-continuity; anchor-presence export block;
weight monotonicity — an anchor stroke is never thinner than a texture stroke,
silhouette never thinner than anchor.

**F3 — Noise Demotion Law (fixes the Red Fuji dots).**
Sibling of the FACEPASS Line/Density Law, applied outline-wide: **in a
masterpiece outline, the only strokes are silhouette, anchors, and curated
texture — every other extracted edge is deleted or merged, never rendered.**
Concretely: any edge fragment below a minimum arc-length threshold that lies
outside an anchor region is dropped. For TONAL pieces, demoted fragments may
become band fill (density, not lines) — same move that fixed the Mona Lisa's
shadow rims. The cloud chips inside Red Fuji's sky are the canonical
violation: the cloud bank is an *anchor* and must render as deliberate
horizontal bands, not confetti.

**F4 — Per-piece anchor tables (registry data, Eric-authored/approved in the tool).**

| Piece | Silhouette | requiredAnchors |
|---|---|---|
| Red Fuji | mountain slope, base to peak | snow-streak zigzag at summit; stratified horizontal cloud bands; slope-to-sky boundary |
| Great Wave | curling crest + trough | foam claws on crest; small Fuji triangle mid-right; ≥1 boat line |
| The Scream | figure's vertical S-curve | oval head + hands-to-face; diagonal bridge railing; ≥2 wavy sky bands |
| Sunflowers | vase + bloom mass as one closed silhouette | ≥3 distinct round flower heads at correct positions; vase contour; table line |

Anchors carry position/scale tolerance (reuse the existing 0.5% landmark
tolerance machinery). Wrong-place anchors fail the same lint as missing ones.

**F5 — extractionClass per piece (Eric sets; recommendations in Decisions).**
Ukiyo-e prints are literal line art → LINE is the honest medium for Red Fuji
and Great Wave. Impasto and expressionist paint have no clean edges → LINE is
the *wrong medium* for Sunflowers (the scribble proves it) and likely The
Scream; both should go TONAL (posterize bands + the F2 hierarchy for their
contour layer). Same diagnosis pattern as sfumato: don't fight the medium,
switch the class.

**F6 — Recognition eval, two probes, both report-only until calibrated (v7.5 trust pattern).**

- **Squint probe:** existing thumbnail-scale structural check, unchanged.
- **Blind-naming probe (new):** a fresh vision model is shown the trace only —
  no title, no context — and asked to name the artwork; run 5 seeds per piece.
  The number earns authority only by reproducing Eric's recorded verdicts
  across ≥2 review rounds; until then it reports, it never gates. **Eric's
  device pass is the gate, not the score** (Done, below).

**F7 — Deliberate-failure piece.**
One test trace with its anchors stripped (silhouette intact) must fail export
via the F2 anchor lint. If it exports, the lint is decoration and the build is
red. Same discipline as every prior lint in this project: proven by deliberate
failure before trusted.

---

## Decisions

- **D1 — Subject scope.** The four reviewed pieces: Red Fuji, Sunflowers, Great Wave, The Scream. Confirm Red Fuji enters the bank as a subject (it wasn't in the F8 scan queue, which named Mona Lisa/Great Wave/Scream/Sunflowers). Decide: add Red Fuji, or swap it for another queued piece.
- **D2 — Great Wave ordering.** The dense-pattern *word-typesetting* class stays gated behind the Mona Lisa side-by-side per the proving-ground sequence. Recommendation: this file's LINE silhouette/anchor authoring for the Wave may proceed now (it's tracer-lane work), while its word render stays behind the existing gate. Sign or amend.
- **D3 — Texture policy v1 for LINE pieces.** Recommendation: delete demoted fragments outright rather than thinning them. Fewer, deliberate strokes beat many faint ones at play scale.
- **D4 — Class assignments.** Recommendation: Red Fuji LINE, Great Wave LINE, Sunflowers TONAL, Scream TONAL. Sign or reassign per piece.
- **D5 — Blind-naming calibration.** Threshold stays report-only and freezes only after it reproduces Eric's grades. Sign the calibration protocol.

If any decision here is wrong or a new fork appears mid-execution: **stop and
ask.** No unstated decisions.

## Constraints & Non-Goals

- Do not touch the v6 layout solver, FACEPASS, PICTURE-COLOR, or any word-typesetting logic — this file is outlines only. Any layout-hash change is a stop-and-ask.
- No pieces beyond the D1 list. No runtime/on-device extraction changes; everything is curated tool-side (T0).
- Reference scans never ship; no new displayed strings; no new models (the blind-naming probe runs in the authoring tool only, never in any app target — extend the existing symbol scan).

## Done (all checkable)

1. F0 provenance dump reviewed; extraction path confirmed or stop-and-ask filed.
2. All D1 pieces have pinned reference scans meeting the F1 bar, license CI green.
3. Silhouette-continuity, anchor-presence, and weight-monotonicity lints green for all D1 pieces; F7 deliberate-failure piece **fails** export.
4. Zero rendered strokes outside silhouette/anchor/curated-texture layers (F3 audit dump is empty).
5. Both F6 probes reported per piece.
6. **The gate:** Eric, on device, names each piece in ≤5 seconds with no hints, no choices, no second guesses. Red Fuji must be named as Red Fuji — the Aug 11 Starry Night misread is the recorded failure this file exists to make impossible.

---

## Session notes, 2026-08-11 (NOT part of Eric's text)

**D1 is already answered by the registry: Red Fuji IS in the bank.** `redfuji`
is a live expert-tier subject with a pinned scan (4 paths, 214 points,
`after Hokusai`). So the decision is not "add it" but "re-author it" — and the
F8 queue named Mona/Great Wave/Scream/Sunflowers because it predates the Aug 11
review, not because Red Fuji was excluded.

**Two of the four have no scan to re-author from.** `sunflowers` is NOT in
scans.json at all — it runs on the legacy solver, so there is no trace to fix,
only one to create. `redfuji`, `greatwave` and `scream` are scan-locked.

**F1 cannot be satisfied from this repo.** Scan sources carry
`authoring: "hand-traced"` and reference images deliberately never ship, so
there is no 4000px master here to trace and no automated tracer. Pinning new
references is a workflow that lives outside this repository.

**F0's provenance question is answerable now, cheaply:** every scan source
carries an `authoring` field. If the Aug 11 traces say `hand-traced` they did
not come from a raw edge pass, and the prime suspect in F0 is wrong.

### F2 lints landed 2026-08-11 (ship 158) — and two rulings that bent F2

Written literally, two of F2's three lints **fail Mona** — the piece this
project treats as the reference success. Both were put to Eric; both readings
below are his ruling, recorded because the code now visibly diverges from the
prose above it.

**Silhouette continuity.** F2 says the silhouette "must be topologically
unbroken; a fragmented silhouette is a build failure." Read as *every outline
path chains into one component*, Mona fails: her outline is a closed frame
rectangle (5 points, arc 1346), the figure contour (145 points, arc 840) and
two shorter edges, and the nearest endpoint gap between them is **36.5px** —
not a tolerance problem, genuinely separate contours. A portrait is a frame
and a figure. The ruling: F2 requires the piece to *have* an unbroken
principal contour, not that all of them join. The lint asks whether the
longest single path carries at least 30% of the silhouette's arc. Mona's frame
and figure sit at 52/33, Great Wave and Red Fuji are single paths at 100%, and
a silhouette shattered into twenty even fragments lands near 5% — which is the
Sunflowers-scribble mode the rule exists to catch.

**Weight monotonicity.** F2 extends this "to **all** masterpieces regardless
of extractionClass." But on TONAL pieces a thin stroke is the medium, not a
weak stroke: Mona carries `decorative_thin` on a silhouette path and two
anchors because that is FACEPASS's thin-line-plus-band-fill treatment — the
same move F3 credits with fixing her shadow rims. Applied across classes the
rule outlaws the fix. The ruling: it is a LINE-class rule. Mona is the only
TONAL master; the other five are LINE and pass on their own merits, so the
check is not vacuous.

**A fourth check, not in F2.** The three-layer mapping is inferred from the
manifests, not stated in the spec, so a manifest inventing a fourth layer name
would silently fall outside all three lints. It fails loudly instead. Eric
kept it.

**Anchor presence is scaffolded, not enforced.** No piece carries
`required_anchors` yet — F4's tables are prose Eric authors in the tool, and
"snow-streak zigzag at summit" is not a geometric predicate. The lint reports
all six traced masters as unauthored and checks properly the moment a table
exists.

**F7 is discharged in CI, not by a one-off.** `masterpiece_lint.py --selftest`
runs eight deliberate fixtures on every gate — five that must fail naming the
exact rule, and Mona's frame-plus-figure-plus-thin-strokes shape, which must
stay clean. Disabling any rule turns the selftest red; that was verified by
neutering each of the five in turn. Same pattern as
`settings-truth-check --selftest`.

The first draft of that selftest was itself blind. It asserted only that a
fixture produced *some* failure, and the shattered-silhouette fixture carried
an empty features layer — so it tripped the anchor rule and kept passing with
the continuity rule disabled outright. Cases now assert on the message.

### Eric's rulings, 2026-08-11

**D1 — Starry Night joins the scope.** The list is now five: Red Fuji,
Sunflowers, Great Wave, The Scream, **Starry Night**. It was not in the
original four, but it is the other half of the confusion pair — Red Fuji was
misread *as* Starry Night — and it is the other suggester-derived master, a
50-point outline. Fixing Fuji alone would have left the pair half-repaired.
Of the five, Great Wave needs nothing (rank 4 in the bank, read correctly), so
the work list is: re-trace Red Fuji and Starry Night, create Sunflowers,
reassess The Scream.

**D3 — INVERTED.** As written, D3 chose to delete demoted fragments outright
rather than thin them, on the reasoning that fewer deliberate strokes beat
many faint ones. That is the right cure for over-extraction, and the bank says
these pieces suffer the opposite: Red Fuji is 214 points where hand-traced
masters run a median of 857. Deleting anything from a starved Hokusai makes it
less nameable, not more.

The inverted policy: **for a LINE piece below or near the masters floor,
fragments are retained or merged, never deleted.** Curation by subtraction is
a privilege of pieces that already have substance to spare; it is not a filter
to run over a thin trace. F3's Noise Demotion Law is scoped the same way — it
remains correct for a dense piece with genuine confetti, and must not run on a
piece that has not cleared the floor.

**This needs no new lint: the ratchet already enforces it.** Deleting
fragments lowers a trace's point count, and any drop beyond 5% fails
tools/density_check.py. An author who deletes their way through a sparse piece
hits the gate. The one deliberate exception — a re-authoring that genuinely
replaces a trace — goes through --accept, which prints every lowered piece by
name so the loss is stated rather than absorbed.

### F0 REVERSED, and the density work (ship 159)

**F0's hypothesis is backwards. The Aug 11 failures are starvation, not noise.**
Measured across all 382 traces: hand-traced pieces run a median of 857 points,
suggester output runs 60 — a 14x gap — and the two masterpieces that failed
review are exactly the two suggester-derived masterpieces.

| piece | points | authoring | Aug 11 verdict |
|---|---|---|---|
| Great Wave | 2076 | hand-traced | recognizable |
| The Scream | 1128 | hand-traced | marginal |
| Rhinoceros | 786 | hand-traced | not reviewed |
| Mona Lisa | 445 | hand-traced (TONAL) | the reference success |
| Red Fuji | 214 | suggested-then-approved | read as Starry Night |
| Starry Night | 138 | suggested-then-approved | the piece Fuji was mistaken for |
| Sunflowers | — | no scan; legacy solver | scribble |

Red Fuji's three anchors carry 8, 15 and 8 points. No weighting rule rescues
that — there is nothing there to weight. **The F2 hierarchy lints pass Red Fuji
green today**, because they check the shape of a trace and this is a question
of whether there is enough of one to shape. F2 is necessary and not sufficient.

This reverses two things above. **F3's Noise Demotion Law and D3's "delete
demoted fragments" both prescribe removal for pieces whose disease is
emptiness.** D3 should be inverted or made conditional on a piece first
clearing the floor; deleting anything from a 214-point Hokusai makes it worse.

**The floor (masters only).** 500 points for LINE masters, hand-traced
required, and a master with no trace at all is a failure rather than a silent
skip. 500 is calibrated to the recorded verdicts: the sparsest LINE master
that reads is Rhinoceros at 786, the densest that fails is Red Fuji at 214.
Above 786 it would condemn a working piece; below 215 it would admit a known
failure.

**Why masters only.** Category is not complexity — a 200-point floor would
fail 15 of 17 landmarks, because "landmark" covers a signpost and a Hokusai
alike. A per-anchor floor was tried and abandoned: it cannot separate the
failures from the successes, since Mona's thinnest legitimate anchor (13
points) is thinner than Red Fuji's (15).

**Why TONAL is exempt from the point floor.** Substance on a TONAL piece lives
in posterized bands, not vertices. Mona would happen to clear a 350-point
floor, but that is a coincidence of her trace, not a property of the medium.
TONAL keeps its own floor — requiredFeatures non-empty, asserted in
src/wordpic.rs — and is still bound by the hand-traced rule.

**The ratchet (all 382).** No trace may lose points: the gate fails if any
picture drops more than 5% below its recorded baseline, or if a baselined
trace disappears entirely. That is the guard the bank actually lacked. Red
Fuji is a hand-traced *subject* carrying suggester-grade density, which is
what re-running the suggester over an existing trace produces, and nothing
would have caught it. 5% slack absorbs re-bundling quantization; a real
regression is not subtle.

**The quarantine shrinks only.** Red Fuji, Starry Night and Sunflowers fail
the floor today and re-tracing them is Eric's authoring work, so they are held
in config/wordpic/density_quarantine.json with the measured reason each. The
gate fails if a held piece starts passing (stale entry, remove it) or names a
picture not in the bank. It cannot become the place failures go to be
forgotten.

**Open consequence for D1.** Starry Night is not in D1's four, but it is the
other half of the confusion pair and a 50-point outline. Fixing Red Fuji alone
leaves the pair half-repaired.
