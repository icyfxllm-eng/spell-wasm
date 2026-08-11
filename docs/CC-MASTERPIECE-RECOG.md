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
