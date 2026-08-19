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

### Eric's rulings, 2026-08-15

**D2 — SIGNED as recommended.** The Great Wave's LINE silhouette and anchor
authoring proceeds now; its *word render* stays behind the Mona Lisa
side-by-side gate. The two were only ever coupled by the proving-ground
sequence, and tracer-lane work does not touch the word-typesetting logic this
file is forbidden to touch. The Wave is the one D1 piece needing nothing
structural — 2076 points, rank 4 in the bank, read correctly on Aug 11 — so
signing D2 unblocks the cheapest anchor table of the five.

**D4 — SIGNED as written.** Red Fuji LINE, Great Wave LINE, Sunflowers TONAL,
The Scream TONAL.

**What D4 unblocks today: Great Wave only.** The other three assignments are
each blocked, and by different things, so the signature is worth less than it
looks:

- Red Fuji is LINE and its class is now settled, but at 214 points it sits
  under the 500 masters floor and is held in the density quarantine. LINE
  authoring cannot start until it is re-traced.
- Sunflowers is TONAL and has no scan file at all — not a thin one, none — and
  its only reference is a 1-bit pictogram. TONAL needs grey levels that do not
  exist in the file on disk.
- The Scream is TONAL and does have a healthy 1128-point trace, but its
  reference is also 1-bit. Its class assignment is therefore unexecutable for
  the same reason as Sunflowers, despite the trace being fine.

Of the seven masters in the bank, only Mona Lisa and the Dürer rhinoceros have
continuous-tone references. Every TONAL assignment D4 makes lands on a piece
that has none. This is not an objection to the ruling — the class calls are
right on the artwork — it is the statement that **two of D4's four assignments
convert directly into a request for source scans** rather than into authoring
work.

**Open sub-fork: Starry Night has no D4 class.** D4 was written against the
original four; D1 later added Starry Night as the other half of the confusion
pair, and no ruling has assigned it a class. Recommendation: **LINE.** Its
identity is the directional swirl — stroke direction, not tonal mass — and
LINE is the only class implementable from a 1-bit reference at all. Filed here
rather than assumed, per this file's no-unstated-decisions rule.

### The source pass, 2026-08-15 (Eric: "lets get all these right")

**Rhino's F4 anchor table is authored — and rhino was the piece that was
actually ready, not Great Wave.** The Aug 15 recommendation that the Wave's
table was cheap and unblocked was wrong. Its eight layout paths are labelled
"greatwave carrier 1" through "carrier 8", and Scream's ten are "scream
carrier 1..10": placeholders that satisfy the unlabeled-stroke check and carry
no semantics. An anchor is a claim about the claw crest or the boats; there is
no table to write over carrier 3. Rhino, looked at properly, already carried
25 real names across 39 paths, so its 13-name table (horn, dorsal hornlet,
forehead, jaw, gorget fold, shoulder/body/haunch plate, belly scales, shoulder
rivets, back, front leg, hind leg) was authorable from labels that existed.
Scene furniture — cloud, bird, ground, woodcut caption — is deliberately not
an anchor. Proven by breaking: renaming the dorsal-hornlet path fails
wordpic-check with "required feature 'dorsal hornlet' missing from trace".
Rhino has left ANCHOR_TABLES_PENDING; the list is now five.

**The original scans were not on disk.** Every manifest pinned a Commons
photograph and what sat beside it was a two-tone file of 9-17KB.

CORRECTED 2026-08-16: those files are not bad downloads, and calling this a
provenance mismatch was wrong. tools/prep_wave10.py AUTHORED them as subject
MASKS from those same scans, with its reasoning in the file -- Red Fuji's
mountain is taken as the region below its own two slopes because three
global-threshold attempts welded cloud lace into the summit, and Scream's dark
masses ARE the composition. A derived mask pointed at by its source's
provenance is the pipeline working. What was true is only that the originals
themselves were absent, which still mattered. All five originals have been fetched at 1920px from the URLs the provenance
already recorded, and sit alongside the masks as genuine continuous-tone
colour scans (150k-313k distinct colours each). Sunflowers is the National
Gallery London version, "Vase with Fifteen Sunflowers", August 1888, per
Eric's call. This unblocks re-tracing, TONAL, ridge refinement and colour
sampling together — they were all waiting on the same missing grey levels.

**Two provenance records disagreed, and one was false.** The manifests and
pictures.json cited different Commons files for Great Wave and for Scream.
pictures.json wins: it is the record wordpic-check enforces, so the manifests
were brought to it. Worse, rhino's manifest claimed "SpellGame original —
drawn rhino silhouette", licence "Original artwork (SpellGame)", while its
trace carries woodcut-caption and gorget-fold paths and pictures.json has
always recorded Duerer 1515. A wrong licence record is worse than a missing
one; it now reads PD-Art after Duerer.

**The I4 bundle scan was a grep for one filename.** It looked for
"mona-lisa.jpg" in dist and the iOS public folder, from when that was the only
continuous-tone artwork on disk. There are now six. Matching is by sha256 over
the whole reference directory instead of by name — names in ref/ are generic
enough (star, anchor, cloud) that basename matching would fire on legitimate
bundle assets, and content matching also catches a reference copied in under a
different name. Proven by planting sunflowers-source.jpg in the iOS bundle as
_i4probe.bin: the old grep passed it, the new scan names it.

**The density floor was guarding the authoring input, not the render.** It
measures content-pipeline/wordpic/scans; what ships is the "d" strings in
pictures.json, and nothing rendered on a device has ever read the scan. Great
Wave clears the 500-point floor on a scan of 2076 and ships eight paths. The
new SHIPPED_PATHS_FLOOR = 6 measures what renders. Vertices do not transfer as
the shipped measure — rhino reads correctly on 103 vertices across 39 short
carriers — but path count separates the record cleanly: rhino 39, mona 21,
scream 10, greatwave 8 all read correctly, against redfuji 1, starrynight 1
and sunflowers 3. The new floor fails exactly the three pieces already in
quarantine and nothing else, which is the calibration claim. Seven new
selftest cases, including Red Fuji's exact shape: a fat scan rendering as one
line, the case the scan-only floor could never see.

**Still open.** Starry Night has no D4 class (recommendation: LINE). The five
pending anchor tables now print WHY they are pending: all five carry
placeholder labels on every layout path, so naming the strokes comes before
any table. Re-tracing redfuji and starrynight and creating sunflowers is the
next block of work, and it is now unblocked for the first time.

### The 2026-08-15 pass: what landed and what did not

**Red Fuji did NOT land, and the reason is worth more than the trace was.**
It was re-traced off the real Commons scan into 38 named paths and 1638
points, Eric passed it on sight, and the layout solver could not fill it: 20
of 43 slots unfilled, and the rainbow's authored clouds 14 of 45.

CORNER_DEG is 35 degrees -- a turn past it ends a word run. All 38 paths
turned more than 60, most between 135 and 180. Great Wave's principal contour
turns 21.7.

The cause is inherent to the method, not to the tuning. These were **closed
region boundaries**: a contour walks out along a snow streak and comes back,
so there is a 180-degree reversal at every tip by construction. Great Wave's
paths are open ribbons. The medium wants strokes and it was given outlines --
it looks right as an SVG and cannot carry text. refine_trace with resample
auto and smooth 6 left the turns at 154-180 and cost 684 points: smoothing
cannot remove a reversal, because a reversal is a real feature of the path.

**So the re-trace has to be centerlines** -- ink-skeleton, or a medial axis per
region -- not boundary contours. Both Red Fuji and the rainbow's clouds are
reverted to their previous geometry pending that. Red Fuji is back in the
quarantine and back in ANCHOR_TABLES_PENDING; its anchor table described the
new trace and goes with it.

**What did land.**

- **The artist rule** (see below), enforced over every master.
- **F2's L1 measures the principal against its own feature group**, not the
  arc-sorted layer. The old calibration cited "Red Fuji ... 100%", a number
  taken from the 214-point single-skeleton trace -- the starvation this file
  exists to catch -- and it is not evidence of anything. Two new selftest
  cases: twenty even fragments sharing one feature label still score 5% and
  still fail, and a strong cone sharing its layer with longer texture passes.
- **The I4 bundle scan hashes the reference directory** instead of grepping
  for the single filename "mona-lisa.jpg". There are six continuous-tone
  artworks on disk now. Matching is by sha256, so generic names like star.png
  cannot false-positive and a reference copied in under a new name is still
  caught. Proven by planting sunflowers-source.jpg in the iOS bundle.
- **SHIPPED_PATHS_FLOOR = 6** in density_check. The floor measured
  content-pipeline scans, the authoring input; what renders is the "d" strings
  in pictures.json. Great Wave cleared a 500-point floor on a scan of 2076 and
  ships 8 paths. Path count separates the record where vertex count does not:
  rhino 39, mona 21, scream 10, greatwave 8 read correctly against redfuji 1,
  starrynight 1, sunflowers 3.
- **Five real source scans**, fetched from the URLs the provenance already
  pinned. Every one of them was a 9-17KB two-grey pictogram on disk while its
  manifest claimed a PD-Art photograph.
- **Four replacement candidates rejected**: Hiroshige, Rousseau, Bruegel,
  Klimt. Sources deleted. Between them and Van Gogh they bound the method:
  impasto fails (no region boundaries exist), filigree fails (Bruegel's masks
  are excellent and a bare tree's contour is a scribble), fine linework fails
  (the bridge truss contours into two long lines). Filigree and impasto defeat
  it from opposite directions.

**THE ARTIST RULE (Eric 2026-08-15).** "Every artist on masterpiece the artist
name should be recognized and never labeled as a spell game orginal if its
based off another artist painting."

The rhino was the case. Its provenance read licence "Original artwork
(SpellGame)", no artist, no URL, on the strength of a true note: the Duerer
plate's hatching floods every threshold, so the coordinate list was redrawn by
hand rather than machine-traced. That is a statement about method and it does
not transfer authorship -- the drawing is Duerer's rhinoceros down to the
dorsal hornlet and the gorget fold. It now reads PD-Art after Albrecht Duerer,
with the method note kept beside it, and the title says "SpellGame redraw"
because that is honest and is not an authorship claim.

Sunflowers had no provenance entry at all, so a Van Gogh was shipping
uncredited; it now credits him whatever becomes of its trace.

Enforced in masterpiece_lint over every master: an entry must exist, it must
name an artist, and its licence may not claim original authorship. Proven both
ways -- stripping the attribution fails, restoring the SpellGame licence fails.

Rhino's 13-name F4 anchor table stands: its trace is untouched and its strokes
were already named (horn, dorsal hornlet, gorget fold, shoulder/body/haunch
plate, belly scales, shoulder rivets, forehead, jaw, back, front leg, hind
leg). Scene furniture -- cloud, bird, ground, woodcut caption -- is not an
anchor. Proven by renaming the dorsal-hornlet path.

### WP_SLOT_DEBUG, and why Red Fuji stays quarantined (2026-08-15)

**The flag.** WP_SLOT_DEBUG=1 prints, for every slot the layout left empty,
which branch each of its ~790 candidates died in:

    SLOT s16 p3 band1 len164: cands=793 nosolve=24 collide=769 outframe=0 shrunk_collide=1538
    SLOT s15 p3 band2 len34:  cands=791 nosolve=791 collide=0 outframe=0 shrunk_collide=0

Pair it with WP_SWEEP_ONLY to scope the sweep to one subject: 19 minutes
becomes 33 seconds. Zero cost when the variable is unset -- one env read per
call, and every counter is behind the flag.

It exists because "unfilled" has several causes that look identical from
outside, and the re-trace cost three wrong diagnoses to that ambiguity. Each
fit the summary. None was the cause.

**The three wrong answers, kept because each is true of something.**

1. *Boundary contours cannot host words; it has to be centerlines.* True of a
   CLOSED thin region -- it reverses at both tips and no smoothing straightens
   it (155 degrees becomes 143 at step 17 x 52 passes). False of an open run:
   the skyline goes 114 -> 20 with plain resampling. The centerline pass built
   on this premise was gentle and unrecognisable as Fuji.
2. *It is the pixel-grid sampling; resampling fixes it.* True of open runs, and
   it lifted paths carrying a word-sized run from 2 of 38 to 12 of 38. Still
   failed, because run length was never the gate.
3. *Every path needs a run longer than its band's longest word.* Measured the
   wrong thing entirely. len in the failure report is the SLOT's length, not
   the word's, and 164px slots go unfilled beside 34px ones.

**The two real causes, which the flag separates in one run.**

- **nosolve** on short slots (34-47px): no word in any pool, down the whole
  borrow chain to easy, fits. A corner mints a slot boundary, so a short
  corner-split segment mints a runt slot nothing can fill.
- **collide** on long slots (95-164px): the word solves and lands on one
  already placed. outframe was zero throughout -- the frame was never it.

The collisions were not bad luck. v4's median clearance between paths was
3.96px with seven paths at 0-0.1, against a FLOOR of 13px per character:
eleven cloud ribbons stacked four pixels apart have room ALONG each stroke and
none ACROSS it. Great Wave fills 349 corner-split slots happily at a minimum
clearance of 23px, and Mona's word-hosting paths never come within the cap at
all.

**So the rule the bank had never written down: parallel strokes closer than
about 20px cannot both be spelled.** Curvature, length and count are all
downstream of it.

**Why Red Fuji is still quarantined.** Curating for separation (v5) took the
unfilled count from 23 of 55 to 8 of 26 -- and stalled. Tracing each region the
way its own shape allows (v6: open runs resampled, closed regions as spines)
collapsed to 3 paths, because a cloud's medial axis branches and comes back at
75-104 degrees, and the clean spines are 35-88px.

Red Fuji's cloud band is eleven small stacked parallel ribbons, and
small-parallel-many is precisely the shape this medium cannot host words on:
too short apart, too crowded together, and a closed loop each. What survives
every constraint is the cone, the treeline and perhaps two clouds -- five
paths, under the six-path shipped floor. The trace that FILLS is a thinner
picture than the one Eric passed on sight.

That is a statement about Red Fuji, not about the tracer. The high-water mark
is v5 at 7 paths and 594 points, in the scratchpad, not landed.

### The guide layer is the picture (2026-08-16)

Rendering all seven masters for the first time showed what the bank actually
looks like, and it was not what any of this file assumed.

**Red Fuji and Starry Night ship as the same octagon.** Not similar — the
identical generic polygon. That is the confusion pair: they were never two
pictures, and no amount of work on their word carriers was going to separate
them.

**Recognition lives in the guide layer, not the carriers.** Mona, Great Wave,
Scream and Sunflowers each carry ~200 guide paths; that is the artwork. The
word carriers are a handful of loose strokes on top. Red Fuji, Starry Night
and rhino have no guide at all.

And from guide_polys: the guide "never hosts words, never collides". So every
constraint the 2026-08-15 retrace fought — slot length, corner splits, the
20px separation rule, collision — applies ONLY to carriers. The whole retrace
was spent forcing the recognisable picture into the one layer that cannot hold
it.

**Three bugs on that layer, found by Eric asking why he could barely see it.**

1. The guide was painted rgba(232,236,245,.30) on EVERY ground. Ship 171 moved
   masterpieces to a cream canvas and added light overrides for pinned,
   feature and outline — and missed the guide, so near-white ink sat on a
   near-white ground. Six of seven masters had invisible artwork for three
   ships.
2. .30 is too faint for the layer that carries recognition even on dark.
   Now .62 dark / .58 light.
3. The inline export style block never declared .wp-guide at all. An <img>
   loads no stylesheet and SVG defaults an unstyled stroke to none, so
   exported pictures did not render the artwork faintly — they omitted it.

`guide` is now a token on Ground rather than a loose CSS rule, and
wordpic-export-parity checks 12 bindings AND that the export block paints the
same CLASS SET as the screen. A missing rule and a wrong value are the same
bug; the gate only knew about the second.

**How a good guide is actually made.** Not by a general tracer. Great Wave and
Scream look right because tools/prep_wave10.py hand-authored a per-piece MASK
first, with its judgement recorded — Fuji's mountain as the region below its
own two slopes, because three global-threshold attempts welded cloud lace into
the summit. Four general methods were tried on 2026-08-16 (boundary contours,
centerlines, a posterized level sweep, feature extraction) and only the last
produced anything shippable, because it is a per-piece mask in miniature.

Red Fuji's guide is landed: 40 paths, 1126 points, by feature extraction.
Starry Night has none and stays an octagon; it needs its own authored mask.

### Starry Night leaves the bank (Eric, 2026-08-16)

Not just the masters set — the bank. Eight extraction passes over two days
could not produce a guide that reads as the painting: boundary contours,
centerlines, an ink-edge chain, a posterized level sweep, feature extraction,
an authored mask, a hybrid, and a straight threshold of the cypress. The
subject is brush DIRECTION and every tool here finds regions.

What settled it was removing the two elements only authoring could supply.
With the swirl and the cypress crown taken out, what remained -- a moon, eight
stars, a hill line, a village with a spire -- is a generic night scene. The
parts that can be extracted honestly are exactly the parts that do not
identify the painting, and the parts that identify it were being drawn rather
than traced.

It was un-mastered rather than deleted at first, and that was worse: a picture
called Starry Night rendering as a bare octagon in the sky shelf, under a name
everyone knows. So it is out of config/wordpic/pictures.json entirely; the
bank is 385.

Everything needed to bring it back stays: the scan, ref/starrynight-source.jpg,
the manifest and the provenance entry. It returns by re-adding one registry
entry, once someone authors it the way Mona was authored. The density ratchet
does not fire, because the scan is still there — nothing was lost, only
unlisted.

Masters are six, and every one of them carries traced geometry.

### The first photographs, and what a rail may not do (2026-08-16)

Saguaro and Church, Taos Pueblo join as masters -- the bank's first
photographs. Ansel Adams, National Park Service Mural Project 1941-42, public
domain as US FEDERAL WORKS rather than by the artist's death, which is a new
pdBasis for this file. The artist is credited regardless: the rule is that a
masterpiece names its artist, and the licence basis is a separate fact.

**Guides by mkbitmap + potrace, not by threshold sweep.** The old generator
sliced at five luminance levels and contoured each region, which draws a line
wherever a smooth gradient crosses a threshold -- an Adams sky came out as five
stacked bands that are not in the photograph. potrace does curve fitting with
corner detection, and mkbitmap's highpass removes the gradient BEFORE
thresholding. Saguaro scores recall 0.856, precision 0.979 against a house
range of 0.80-0.89 and 0.58-0.87.

**But recall is not quality, and it is not a gate.** Red Fuji's 40-path guide
scores 0.174 and reads better than a potrace version at 0.877, because a
woodblock print's ink includes every decorative stripe and maximising coverage
buys clutter. Eric's ruling: keep the 40-path Fuji. Record recall per guide;
never gate on it. Precision is the number that tracks whether strokes are
honest. potrace for photographs, curated features for prints.

**RAILS MAY NOT CROSS.** The new law, and it cost four rounds to find. Word
carriers must clear each other by ~20px -- established earlier -- but the
saguaro showed the sharper form: two rails that INTERSECT put two words at the
intersection. Its trunk crossed both arm roots and the desert-floor line, and
1-2 slots went unfilled every seed. Widening the arm clearance did nothing,
because the arms were never the problem; ending the trunk above the floor line
fixed all fifteen languages at once.

Two corollaries worth keeping:

- Crossing is fine for GEOMETRY and fatal for PLACEMENT. An earlier filter
  rejected rails by minimum clearance, which scores 0 at every crossing and
  threw away every sky band the trunk passed through. The test that works is
  the FRACTION of a rail running alongside another; the rule that works is
  that rails must not intersect at all.
- Separation must hold for the WIDEST script, not the one being looked at.
  23px of clearance passed en and es and failed vi and ko, because Korean sets
  in blocks and Vietnamese stacks diacritics.

Rails are authored, not traced. They cannot come from the guide -- every guide
path is a closed contour that reverses at its tips, so none survives
corner_split. That is not a workaround: Great Wave has always shipped eight
paths named "greatwave carrier 1..8" under 220 paths of Hokusai.

### The quarantine empties (2026-08-16)

Red Fuji and Sunflowers get rails, and the density quarantine is empty for the
first time since it was created: all 8 masters clear the 500-point scan floor
and the 6-path shipped floor.

Both filled on the FIRST sweep, which is the point worth recording. The
saguaro cost four rounds to discover the rules; applied up front they simply
work:

  authored, never lifted from the guide -- guide paths are closed contours
    that reverse at their tips and do not survive corner_split;
  no rail may INTERSECT another, because a crossing puts two words in one
    place -- Red Fuji's cone is ONE path carrying both slopes so they cannot
    cross at the summit;
  34px clearance, sized for Korean blocks and Vietnamese diacritics rather
    than for English;
  arc over 130 so a rail mints a slot a word can use.

Red Fuji: 6 rails, 605 points -- cone profile, cloud bands above the summit
and clear to its right, treeline. Sunflowers: 7 rails, 561 points -- table
edge, vase rim, bloom rows, flower rows. Minimum gaps 43px and 35px, zero
crossings in either.

Three pictures entered the ratchet baseline (saguaro, sunflowers, taos) via
--accept, which names them rather than absorbing them.

**Still open on this file.** Six masters await F4 anchor tables. Great Wave and
Scream cannot have one at all until their rails are named -- every one is
"carrier N", a positional placeholder. Saguaro and Taos have named rails and
are ready whenever Eric rules. Rhino has no guide layer at all and reads
anyway, which is worth understanding rather than fixing. And Done #6, the
five-second naming pass on device, is untouched: it is the only check that
would catch a guide which passes every lint and still does not read.

## F9 -- the subject rule, and the eight that passed

Two masters left the bank in one session for the same reason. Starry Night
and Scream are soft paint: no tonal edge anywhere for potrace to find, so
every trace of them was an octagon or a smear. Red Fuji and Starry Night had
in fact shipped as the IDENTICAL octagon, which is what the confusion pair
was the whole time.

That failure is what the selection rule came out of. A subject passes when it
is a SINGLE CLEAR SUBJECT ON CLEAN GROUND -- engraving, woodblock, botanical
plate. Filigree, soft paint, ornament and multi-specimen sheets all fail, and
they fail before a single trace is run. Fifteen candidates were put to Eric at
full size; eight passed.

  hare      Duerer, A Young Hare, 1502          215 paths
  beetle    Duerer, Stag Beetle, 1505            55   frame + monogram stripped
  wing      Duerer, Wing of a Roller, c.1500    215   frame stripped
  banana    Merian, Metamorphosis, 1705         159
  flamingo  Audubon, Birds of America, c.1838    67
  carp      Hokusai, Two Carp in a Cascade       182
  rose      H. J. Redoute, Pompon Rose, 1817    166   caption + rule cut
  owl       Koson, Scops Owl, before 1945       112   frame REBUILT, not stripped

Two entries there record findings rather than settings.

The rose is by HENRY Joseph Redoute, not Pierre-Joseph -- the brother, not the
famous rose painter. The filename says so and the Cleveland accession says so;
the assumption would have put the wrong artist on a shipped picture. Cleveland
releases it CC0, a licence GRANT rather than a term expiry, so its pdBasis is
worded differently from the other seven on purpose.

The owl is the one picture here that keeps its frame. The beetle's frame came
off because the beetle is an isolated subject on blank ground; the owl's moon
and branch RUN INTO its border, so stripping it left the composition hanging.
The frame is rebuilt from the art's own bounding box at zero margin -- derived
from the extent rather than imposed on it, so it cannot clip. It is worth
knowing that the right-hand rule is positioned by a SINGLE stroke; a re-trace
that drops that stroke moves the frame.

Judging these at thumbnail size got three of the first candidates wrong. Every
review after that was at full size.

**Staged, not shipped.** All eight are in
content-pipeline/wordpic/staged-batch-f9.json with verified provenance and
their source scans in ref/. None is registered: each still needs authored
rails and captions in 15 locales, which is a Saguaro-sized job per picture.
The six F4 anchor tables remain drafted and unruled.
