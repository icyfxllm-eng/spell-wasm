# Word Picture authoring tool (D10) — for Eric

Turns a public-domain scan into a DRAFT stroke map for an expert picture.
Build-time only; nothing here ships or runs in the app.

## One-time setup
    pip3 install pillow

## Make a draft
    cd content-pipeline/wordpic
    python3 trace.py ref/mona-lisa.jpg --bands 4 --out mona-draft.py

You get `mona-draft.py` (flow(...) lines in the generator's vocabulary,
band = tonal band → text size/weight) and `mona-draft-preview.svg`
(open it — squint: does it read as the subject?).

## Curate → commit → sweep
1. Open the draft; delete noise lines, keep the ~50-90 that carry the image
   (silhouette + masses first, features last — reorder so the best stroke
   is the FINAL word).
2. Paste the keepers into `scripts/gen-wordpic-strokemaps.py` (mona section
   or a new picture), run `python3 scripts/gen-wordpic-strokemaps.py`.
3. `cargo test --lib wordpic_layout` — the L9 sweep names anything that
   overlaps or can't host words in any of the 15 languages. Fix by moving
   or lengthening strokes, never by hand-placing words.
4. Provenance block in the generator (PD source, URL, basis, date) — the
   reference image itself stays in ref/ and NEVER ships.

Denser maps (more, shorter strokes) are what unlock Mona in Korean and
Japanese — their word pools run long-compound and need more hosting room.
