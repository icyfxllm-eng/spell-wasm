# CC-SPELLPIC-AUDITPASS — F0 step-0 diagnosis

Status: **FIXED.** Diagnosed against build 154 (ship 147); Eric ruled the
renderer in scope ("fix the ids, its the renderer") and then took the three
side findings too. See Resolution at the bottom.
Date: 2026-08-08.

## The finding: SVG IDs are document-scoped and unnamespaced

`src/wordpic_screen.rs` emits picture geometry as `<path>` elements whose IDs
are generated from a bare loop index:

- line 1346 — `<path id="wps{si}" …>`   (scan-stack strokes)
- line 1545 — `<path id="sl{i}" …>`     (scanlock baselines)

and then references them by fragment:

- line 1368 — `<use href="#wps{si}" …>`
- line 1429 — `<textPath href="#wps{si}" …>`
- line 1621 — `<textPath href="#sl{i}" …>`

Nothing in those IDs is derived from the picture. Every picture that has at
least one baseline emits `sl0`. Every picture with at least one stroke emits
`wps0`.

A fragment reference is resolved against **the whole document**, not the
enclosing `<svg>`. When two picture SVGs are live in the DOM at the same time,
every `href="#sl3"` in the second one resolves to the **first** one's `sl3`.
The second picture's words are then laid along the first picture's geometry.

That is the fusion. It is not a state bug and not a binding bug — the correct
picture ID reaches the renderer. The renderer emits correct markup. The
*browser* then wires the second copy to the first copy's skeleton.

## Where two picture SVGs coexist

1. **Stage + reveal — always.** `#wpStage` (index.html:1669) and
   `#wpRevealStage` (index.html:1695) are both children of `#wpPlay` and are
   both permanently in the DOM. The stage comes first in document order, so it
   always wins. A reveal opened over a live stage renders the finished piece's
   words on the live picture's baselines.

2. **The gallery — one collision per extra trophy.** `render_gallery`
   (~line 1655) loops finished pictures, calls `scanlock_svg` per picture, and
   concatenates them into a single `set_html("wpGallery", …)`. With N finished
   pieces, tiles 2..N all render against tile 1's geometry. Two finished
   pictures is enough to see it.

## Proposed fix (NOT APPLIED — see below)

Namespace the IDs per render instance. A monotonic counter bumped once per
`scanlock_svg` / stroke-render call is enough; the picture ID alone is not,
because the same picture legitimately appears twice (a gallery tile and the
reveal of that same tile). Touches the two `format!` sites that mint IDs and
the three that reference them. No layout, geometry, or data change — the
rendered result is what the current code already intends.

## Why this stopped here

The pass's own clause: *"If the guilty layer turns out to be the renderer, stop
and ask before proceeding — that scope may belong to CC-RENDER-FIXPASS
instead."* The guilty layer is the renderer. Awaiting Eric.

## Side findings (separate from F0, found while tracing)

- **`wpShare` is wired twice** — `wordpic_screen.rs:163` and `:184`.
  `dom::on_click` calls `add_event_listener_with_callback`, which *adds*; it
  never replaces. One tap therefore fires both: the text-only
  `share_wordpic` fallback *and* the `export_and(ShareCard)` image export.
- **Viewing a trophy reseeds the next play of that picture.**
  `open_gallery_piece` (:1674) writes `PIC` but not `SEED_BUMP`. `open_play`
  only clears `SEED_BUMP` when `PIC != pic_id`, so after viewing fish in the
  gallery, playing fish keeps the previous session's bump — which feeds
  `layout_feed_opt(…, run.seed + bump, …, bump >= 5)` and changes both the word
  feed and the relax flag. `open_gallery_piece` also leaves `FEED`, `LADDER`
  and `MILESTONE` on the previous picture's values; no path currently reads
  them before `open_play` overwrites them, so this is latent, not live.
- **`#wpGallery` is duplicated in the shell** — index.html:1631 and :1635 are
  an identical comment+div pair. `getElementById` returns the first, so the
  second is dead markup with a colliding ID. Introduced during the F5 /
  sticky-head rework.

## Resolution

**F0.** `next_ns()` mints a monotonic namespace, bumped once per render call.
All five sites carry it: the two `<path>` mints (`sl{ns}_{i}`,
`wps{ns}_{si}`), the `<use>`, and the two `<textPath>`s. Once per CALL and not
once per picture, because the same picture legitimately renders twice at the
same time — a gallery tile and the reveal of that tile — and keying on the
picture ID would have left exactly that pair colliding.

Pinned by `wordpic_screen::export_tests::two_renders_never_share_element_ids`,
which renders the same plan twice and requires the two ID sets to be disjoint
AND every `href="#…"` to resolve inside its own `<svg>`. Verified to bite:
reverting `sl{ns}_{i}` to `sl{i}` fails it with
`two renders share element IDs: ["sl0", "sl1"]`.

**Side 1 (double share).** The earlier of the two `#wpShare` handlers is
deleted; the image path with its text fallback is the survivor. A comment at
the old site records why there is exactly one, since `dom::on_click` adds.

**Side 2 (stale reroll).** New `bind_pic()` is now the only writer of `PIC`,
and both `open_play` and `open_gallery_piece` go through it. On a picture
CHANGE it clears `SEED_BUMP`, `MILESTONE`, `FEED` and `LADDER`; on a re-open
of the same picture it deliberately keeps the reroll position. The bug was two
writers with different discipline, so the fix is one writer.

**Side 3 (duplicate `#wpGallery`).** The second comment+div pair is deleted,
and the class of defect is now a gate law: `scripts/dom-id-check.mjs` (wired
into `gate.sh` beside the scroll law) fails on any duplicate element ID in the
shell. It ignores comments, `<script>` and `<style>` so a CSS selector or a JS
string cannot trip it, and it fails if it finds no IDs at all rather than
passing vacuously. Verified to bite by re-adding the duplicate: `FAIL
index.html: id="wpGallery" declared 2x`. Clean run: 393 IDs, all unique.
