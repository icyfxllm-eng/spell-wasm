#!/usr/bin/env python3
"""CC-MASTERPIECE-RECOG F2 — the outline hierarchy lints.

A masterpiece outline must be instantly nameable. F2 says that comes from an
unbroken silhouette plus a few deliberate anchors, never from edge density,
and gives three CI-enforced rules. This is the tool-side half; it reads the
authoring sources, not the shipped bundle, because the bundle is generated
and checking it would test bundle_scans rather than the traces.

F2's three layers map onto the layer names the manifests already use:

    silhouette  <- outline
    anchors     <- features
    texture     <- texture, shading

That mapping is asserted, not assumed (unknown_layer_names), so a manifest
inventing a fourth category fails rather than being silently ignored.

THE ANCHOR RULE, corrected 2026-08-11. This lint first shipped reading
`required_anchors` off the scan and `anchor_id` off its paths. Neither exists
anywhere in the repo — they were invented from F2's prose. The real mechanism
predates this file: a picture declares `requiredFeatures` in pictures.json,
every layout path carries a `feature` label, and scripts/wordpic-check.mjs
fails the build when a declared feature has no path. Mona has carried ten
that way since the TONAL pass.

So the name matching is already enforced, on the correct vocabulary, and
duplicating it here would only add a second place to be wrong. What was NOT
enforced is F2's actual widening: src/wordpic.rs asserts requiredFeatures only
for TONAL subjects, while F2 extends the anchor rule to "all masterpieces
regardless of extractionClass". That gap is this lint's job — every master
must declare a table, and wordpic-check then holds it to it.

F4's tables are prose Eric authors in the tool ("snow-streak zigzag at
summit" is not a geometric predicate), so the six masters without one are
named in ANCHOR_TABLES_PENDING, which shrinks and cannot silently grow.
"""
from __future__ import annotations

import json
import re
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SCANS = ROOT / "content-pipeline/wordpic/scans"
MANIFESTS = ROOT / "content-pipeline/wordpic/manifests"
PICTURES = ROOT / "config/wordpic/pictures.json"

PROVENANCE = ROOT / "content-pipeline/wordpic/ref/provenance-f8.json"

# THE ARTIST RULE, Eric 2026-08-15: "every artist on masterpiece the artist
# name should be recognized and never labeled as a spell game orginal if its
# based off another artist painting."
#
# The rhino is why. Its coordinate list really is hand-authored -- the Duerer
# plate's hatching floods every threshold, so it was redrawn rather than
# traced -- and the provenance recorded that as licence "Original artwork
# (SpellGame)" with no artist and no URL. That is a true statement about
# method and a false one about authorship: the drawing is Duerer's rhinoceros
# down to the dorsal hornlet. How a stroke was produced never transfers
# authorship of what it depicts.
ORIGINAL_CLAIMS = ("spellgame", "spell game", "original artwork")

SILHOUETTE = {"outline"}
ANCHORS = {"features"}
TEXTURE = {"texture", "shading"}
KNOWN = SILHOUETTE | ANCHORS | TEXTURE

# Masters with no F4 anchor table yet. Held so the gate stays green while Eric
# authors them in the tool; the list SHRINKS ONLY — a piece that gains a table
# must leave, or the next master to ship without one hides behind a stale pass.
# Adding a name here is a deliberate act to argue for in the commit.
#
# redfuji, starrynight and sunflowers cannot gain one before they are
# re-traced: their whole layout is a single path labelled "skeleton".
# greatwave and scream have 8 and 10 paths but every label is a positional
# placeholder, so their strokes must be named first.
#
# Red Fuji briefly left this list on 2026-08-15 with a 38-path trace off the
# real Commons scan, and went back on: the trace was region BOUNDARIES, and a
# boundary reverses direction at every tip, so all 38 paths turned 60-180
# degrees against a CORNER_DEG of 35 and the layout could not fill 20 of 43
# slots. Smoothing cannot help -- a reversal is a real feature of the path.
# The re-trace has to be centerlines, and the table comes back with it.
# EMPTY as of 2026-08-20: every traced master now declares its anchors.
# Empty by ACHIEVEMENT, not by anyone forgetting to add to it -- the check below
# fails a master that has no table AND is not listed here, so a new master
# cannot slip in unanchored, and one that gains a table cannot stay listed.
ANCHOR_TABLES_PENDING: frozenset[str] = frozenset()


_PICS: dict | None = None


def pic_by_id() -> dict:
    global _PICS
    if _PICS is None:
        _PICS = {p["id"]: p for p in json.loads(PICTURES.read_text())["pictures"]}
    return _PICS


def masters() -> list[str]:
    pics = json.loads(PICTURES.read_text())["pictures"]
    return sorted(p["id"] for p in pics if "masters" in p.get("categories", []))


def layers_of(pid: str) -> dict[str, list[int]]:
    mf = MANIFESTS / f"{pid}.json"
    if not mf.exists():
        return {}
    return {L["name"]: list(L["path_ids"]) for L in json.loads(mf.read_text())["layers"]}


# A principal contour must carry at least this share of its own feature's arc
# (see principal_contour for why "its own feature" and not "the layer").
# Calibrated on: Mona's figure contour and frame split 52/33; Red Fuji's cone
# is 606 of 794 across three border-split runs, 76%; Great Wave is a single
# path at 100%. A silhouette shattered into twenty even fragments tops out
# near 5% and fails, which is the mode this rule exists to catch.
#
# The pre-2026-08-15 note cited "Red Fuji ... 100%" as a calibration point.
# That number came from the 214-point single-skeleton trace, i.e. from the
# starvation this whole file exists to catch, and it is not evidence of
# anything. Do not restore it.
PRINCIPAL_SHARE = 0.30


def principal_contour(paths: list[dict]) -> tuple[bool, float]:
    """Is there ONE unbroken shape carrying this silhouette?

    Eric 2026-08-11: F2's "topologically unbroken" means the piece must HAVE an
    unbroken principal contour, not that every outline path chains into one.
    Requiring the latter condemns the reference success — Mona's outline is a
    closed frame plus a figure contour plus two edges, endpoints 36.5px apart,
    and they were never meant to join. A portrait is a frame and a figure.

    So: find the longest single path and ask whether it dominates. One path is
    trivially unbroken; the question is whether the silhouette is carried by a
    contour or scattered across fragments.

    MEASURED AGAINST ITS OWN FEATURE, Eric 2026-08-15. The share used to be
    taken over the whole silhouette layer, and that layer is not curated: the
    manifest assigns layers mechanically by arc length, which is a deliberate
    design law, so "outline" simply means the longest strokes in the picture.
    That works while a piece is thin and breaks the moment it is not. Red Fuji
    re-traced off the real Commons scan puts a 894-arc forest fringe and a
    643-arc summit crown above the 606-arc cone: a stipple boundary has an
    enormous perimeter and almost no meaning, and the mountain came third in
    its own silhouette layer at 17%.

    The old calibration cannot arbitrate that, because it was taken from the
    broken piece — "Red Fuji and Great Wave are single paths at 100%" was true
    of the 214-point single-skeleton trace this lint's own file exists to
    replace. Fixing the starvation invalidated the data point.

    So the principal is compared against the paths sharing ITS feature label,
    which is the silhouette as authored rather than as sorted. Red Fuji's cone
    scores 606/794 = 76%. Twenty even fragments all labelled the same thing
    still score 5% and still fail, so the mode this rule exists to catch is
    caught unchanged. Traces with no feature labels are one unnamed group and
    behave exactly as before.
    """
    if not paths:
        return False, 0.0
    arcs = [p.get("arc", 0.0) for p in paths]
    total = sum(arcs)
    if total <= 0:
        return False, 0.0
    lead = max(range(len(paths)), key=lambda i: arcs[i])
    kin = paths[lead].get("feature")
    group = sum(a for p, a in zip(paths, arcs) if p.get("feature") == kin) or total
    return (arcs[lead] / group) >= PRINCIPAL_SHARE, arcs[lead] / group


def classes() -> dict[str, str]:
    pics = json.loads(PICTURES.read_text())["pictures"]
    return {p["id"]: p.get("extractionClass") for p in pics}


def required_features() -> dict[str, list]:
    pics = json.loads(PICTURES.read_text())["pictures"]
    return {p["id"]: p.get("requiredFeatures") or [] for p in pics}


CLASS = classes()
REQUIRED = required_features()


def check(pid: str) -> list[str]:
    sf = SCANS / f"{pid}.json"
    if not sf.exists():
        # Not a failure of THIS lint: a master with no trace has nothing to
        # check. It is reported by the scan-gap audit, which owns that.
        return []
    return evaluate(pid, json.loads(sf.read_text()), layers_of(pid),
                    CLASS.get(pid), REQUIRED.get(pid, []),
                    pid in ANCHOR_TABLES_PENDING)


def evaluate(pid: str, doc: dict, lay: dict, cls: str | None,
             required: list | None = None, pending: bool = False) -> list[str]:
    """The four rules, over data rather than files, so --selftest can feed
    them deliberately broken input without writing to the tree."""
    bad: list[str] = []
    paths = doc["paths"]
    if not lay:
        return [f"{pid}: no manifest layers — the hierarchy is unauthored"]

    unknown = sorted(set(lay) - KNOWN)
    if unknown:
        bad.append(f"{pid}: unknown layer name(s) {unknown} — F2 defines exactly "
                   f"silhouette/anchors/texture (outline/features/texture+shading)")

    sil_ids = [i for n in SILHOUETTE for i in lay.get(n, [])]
    anc_ids = [i for n in ANCHORS for i in lay.get(n, [])]

    # L1 — silhouette continuity: a principal unbroken contour must carry it.
    if not sil_ids:
        bad.append(f"{pid}: no silhouette layer — F2 requires one continuous shape")
    else:
        sil = [paths[i] for i in sil_ids if i < len(paths)]
        ok, share = principal_contour(sil)
        if not ok:
            bad.append(f"{pid}: silhouette has no principal contour — longest path is "
                       f"{share:.0%} of its arc, under {PRINCIPAL_SHARE:.0%}. A silhouette "
                       f"scattered across fragments is a build failure (F2)")

    # L2 — weight monotonicity, LINE pieces only.
    #
    # Eric 2026-08-11: on TONAL pieces thin strokes ARE the medium. Mona
    # carries decorative_thin on a silhouette path and two anchors because
    # that is FACEPASS's thin-line-plus-band-fill treatment — the move that
    # fixed her shadow rims. F2 says "regardless of extractionClass", but
    # applied literally that declares the one working masterpiece illegal.
    # The rule belongs to LINE, where a thin stroke really is a weak stroke.
    #
    # `decorative_thin` is the trace's own thin-stroke flag; the
    # word-placement tools already exclude it, so it stands in for weight.
    for label, ids in ((("silhouette", sil_ids), ("anchor", anc_ids)) if cls == "LINE" else ()):
        thin = [i for i in ids if i < len(paths) and paths[i].get("decorative_thin")]
        if thin:
            bad.append(f"{pid}: {label} path(s) {thin} are decorative_thin — an "
                       f"anchor is never thinner than texture (weight monotonicity)")

    # L3 — every master declares an anchor table.
    #
    # F2 generalizes the TONAL requiredFeatures lint to all masterpieces
    # regardless of extractionClass; src/wordpic.rs only asserts it for TONAL.
    # This closes that. The NAMES are matched by scripts/wordpic-check.mjs
    # against each layout path's `feature` label — the vocabulary that actually
    # exists — so this asks only that a table be declared, and never
    # re-implements the matching.
    if not required:
        if not pending:
            bad.append(f"{pid}: no requiredFeatures — F2 requires an anchor table "
                       f"for every master, not only TONAL ones. Declare it in "
                       f"config/wordpic/pictures.json; wordpic-check enforces the names")
    elif pending:
        bad.append(f"{pid}: has an anchor table but is still listed in "
                   f"ANCHOR_TABLES_PENDING — remove it from the list")

    # A declared table is meaningless if no path was ever assigned to anchors.
    if not anc_ids:
        bad.append(f"{pid}: no anchors layer in the manifest — nothing names this piece")
    return bad


def _p(arc: float, thin: bool = False, feature: str | None = None) -> dict:
    return {"points": [[0, 0], [1, 1]], "arc": arc, "decorative_thin": thin,
            "feature": feature}


def selftest() -> int:
    """F7: prove each rule still bites, by deliberate failure.

    Same discipline as settings-truth-check --selftest. A lint that has
    silently stopped firing is worse than no lint, because the green tick
    reads as a guarantee.

    Each case names the message it must provoke, not merely "something
    failed". The first draft of this asserted truthiness and was itself
    blind: the shattered-silhouette case carried an empty features layer, so
    it tripped the ANCHOR rule and kept passing with L1 disabled entirely.
    Every fixture is otherwise well-formed so exactly one rule can fire.
    """
    one = {"outline": [0], "features": [1], "texture": [2]}
    TABLE = ["nose"]          # a declared anchor table
    cases = [
        ("shattered silhouette",
         {"paths": [_p(10) for _ in range(21)]},
         {"outline": list(range(20)), "features": [20]}, "LINE", TABLE, False,
         "principal contour"),
        # Same shattering, every fragment carrying the SAME feature label, so
        # the feature-group measure cannot rescue it: 20 even pieces of one
        # silhouette is still 5%.
        ("shattered silhouette, one feature",
         {"paths": [_p(10, feature="cone profile") for _ in range(20)]
                   + [_p(10, feature="snow")]},
         {"outline": list(range(20)), "features": [20]}, "LINE", TABLE, False,
         "principal contour"),
        # Red Fuji's actual shape: a strong cone sharing its layer with longer
        # texture contours. Third by arc in its own layer, and legal.
        ("silhouette layer carrying longer texture (Red Fuji)",
         {"paths": [_p(894, feature="forest base"), _p(643, feature="summit crown"),
                    _p(606, feature="cone profile"), _p(162, feature="cone profile"),
                    _p(26, feature="cone profile"), _p(516, feature="cloud band"),
                    _p(30, feature="snow streak")]},
         {"outline": [0, 1, 2, 3, 4, 5], "features": [6]}, "LINE", TABLE, False,
         None),
        ("thin silhouette on a LINE piece",
         {"paths": [_p(100, thin=True), _p(10), _p(5)]}, one, "LINE", TABLE, False,
         "silhouette path(s)"),
        ("thin anchor on a LINE piece",
         {"paths": [_p(100), _p(10, thin=True), _p(5)]}, one, "LINE", TABLE, False,
         "anchor path(s)"),
        ("master with no anchor table",
         {"paths": [_p(100), _p(10)]},
         {"outline": [0], "features": [1]}, "LINE", [], False,
         "no requiredFeatures"),
        ("table exists but still listed pending",
         {"paths": [_p(100), _p(10)]},
         {"outline": [0], "features": [1]}, "LINE", TABLE, True,
         "still listed in ANCHOR_TABLES_PENDING"),
        ("no anchors layer at all",
         {"paths": [_p(100)]}, {"outline": [0]}, "LINE", TABLE, False,
         "no anchors layer"),
        ("unknown layer name",
         {"paths": [_p(100), _p(10)]},
         {"outline": [0], "features": [1], "highlights": []}, "LINE", TABLE, False,
         "unknown layer name"),
        ("no silhouette layer",
         {"paths": [_p(100)]}, {"features": [0]}, "LINE", TABLE, False,
         "no silhouette layer"),
        # A master awaiting its F4 table is held, not failed.
        ("pending master with no table",
         {"paths": [_p(100), _p(10)]},
         {"outline": [0], "features": [1]}, "LINE", [], True, None),
        # The regression that forced Eric's two rulings: Mona's shape is legal.
        # Frame and figure 36.5px apart, thin strokes on outline and features.
        ("frame + figure + thin strokes on TONAL (Mona)",
         {"paths": [_p(1346), _p(840), _p(221, thin=True), _p(165),
                    _p(50, thin=True), _p(40, thin=True)]},
         {"outline": [0, 1, 2, 3], "features": [4, 5], "texture": []},
         "TONAL", TABLE, False, None),
    ]
    bad = []
    for name, doc, lay, cls, req, pend, want in cases:
        got = evaluate("selftest", doc, lay, cls, req, pend)
        if want is None:
            if got:
                bad.append(f"  {name}: expected clean, got {got}")
        elif not any(want in g for g in got):
            bad.append(f"  {name}: expected a failure mentioning {want!r}, "
                       f"got {got or 'clean'}")
    if bad:
        print("FAIL masterpiece-lint selftest — the gate no longer bites:")
        print("\n".join(bad))
        return 1
    print(f"masterpiece-lint selftest: OK — {len(cases)} deliberate cases behave")
    return 0



def check_guide_anchors(pic) -> list[str]:
    """F4 as it was actually written: anchors are POSITIONAL, on the trace.

    The first implementation used `requiredFeatures`, which names RAILS -- the
    lines words ride on, not the picture. An anchor saying the Great Wave must
    show a boat had no mechanism behind it: the boats live in the guide, guide
    contours are anonymous, and no lint could see them.

    Two weaker versions of this check are worth recording, because each failed
    in a way the next had to answer.

    ASKING WHETHER THE REGION HAS INK is nearly always true -- a dense trace has
    ink everywhere. A box moved into the Wave's empty sky still found 48 points
    of foam and cloud edge, so it passed a boat drawn in the sky, which is the
    wrong-place anchor F4 says must fail.

    ASKING FOR A WHOLE CONTOUR INSIDE THE REGION is too strict, because only
    some features are standalone shapes. The Wave's boats are, so they passed;
    Fuji's triangle and Red Fuji's summit are SEGMENTS of longer lines and
    failed, though both are plainly present.

    What holds for both is a sustained RUN: the trace enters the region and
    stays there for a while, whether or not the shape closes on itself.

    Be clear about what this proves. Requiring the run to SPAN its box was
    tried and discarded -- a box in the empty sky spanned 70% of itself, more
    than several genuine anchors -- so run length is the only usable signal,
    and its margin is thin for small features: the sky box managed 10 points
    against the Wave's fuji triangle at 13. The threshold is therefore set per
    anchor from the feature's own measured run with headroom.

    So this is a RATCHET, not an identification. It reliably catches a feature
    that thins or disappears under a re-trace, which is the risk that actually
    materialises; it does not prove that the shape in the region is a boat
    rather than something else the same size. Naming guide contours would, and
    would cost a schema change.
    """
    out = []
    for a in pic.get("guideAnchors", []):
        x0, y0, x1, y1 = a["box"]
        need = a.get("minRun", 12)
        best = 0
        for d in pic.get("guide") or []:
            run = 0
            for xs, ys in re.findall(r"([-\d.]+) ([-\d.]+)", d):
                x, y = float(xs), float(ys)
                if x0 <= x <= x1 and y0 <= y <= y1:
                    run += 1
                    best = max(best, run)
                else:
                    run = 0
        if best < need:
            out.append(
                f"{pic['id']}: anchor '{a['name']}' — the trace never runs through "
                f"its region for more than {best} points, needs {need}. Missing, "
                f"or drawn somewhere else")
    return out


def main() -> int:
    if "--selftest" in sys.argv:
        return selftest()
    ids = masters()
    if not ids:
        print("FAIL: no pictures in the `masters` category — this lint is vacuous")
        return 1
    failures, checked, tabled = [], 0, []
    for pid in ids:
        if not (SCANS / f"{pid}.json").exists():
            continue
        checked += 1
        if REQUIRED.get(pid):
            tabled.append(pid)
        failures += check(pid)
        failures += check_guide_anchors(pic_by_id()[pid])

    # The artist rule, over every master in the bank.
    prov = {r["subject"]: r for r in json.loads(PROVENANCE.read_text())}
    for pid in ids:
        r = prov.get(pid)
        if r is None:
            failures.append(f"{pid}: a master with no provenance entry — every "
                            f"masterpiece names its artist")
            continue
        if not (r.get("attribution") or "").strip():
            failures.append(f"{pid}: no attribution — every masterpiece names its "
                            f"artist, whatever the trace was made from")
        # Only the LICENCE field carries the authorship claim. The title may
        # say "SpellGame redraw" and should -- that is an honest note about
        # method sitting next to a credited artist, which is exactly the shape
        # the rule wants.
        if any(c in (r.get("license") or "").lower() for c in ORIGINAL_CLAIMS):
            failures.append(f"{pid}: provenance claims original authorship "
                            f"({r.get('license','')!r}) — a redraw of another "
                            f"artist's work is still that artist's work")

    # The pending list may only shrink; a name that has left the bank is as
    # stale as one that has gained a table.
    for pid in sorted(ANCHOR_TABLES_PENDING - set(ids)):
        failures.append(f"{pid}: listed in ANCHOR_TABLES_PENDING but is not a "
                        f"master — remove the entry")

    for f in failures:
        print(f"FAIL {f}")
    if failures:
        return 1
    print(f"masterpiece-lint: OK — {checked} traced masters, silhouettes continuous, "
          f"weights monotonic, {len(tabled)}/{len(ids)} with an F4 anchor table")
    waiting = sorted(ANCHOR_TABLES_PENDING & set(ids))
    if waiting:
        print(f"  awaiting F4 anchor tables: {waiting}")
        print(f"  (names are enforced by scripts/wordpic-check.mjs once declared)")
        # WHY each one is waiting, because "pending" read as "someone just has
        # to sit down and type it" and that is only true for a piece whose
        # strokes are named. A layout labelled "greatwave carrier 1..8" has
        # nothing to build a table out of: an anchor is a claim about the claw
        # crest or the dorsal hornlet, not about carrier 3. Rhino left this
        # list the day it was looked at, because its 39 paths already carried
        # 25 real names.
        by_pid = {p["id"]: p for p in json.loads(PICTURES.read_text())["pictures"]}
        for pid in waiting:
            feats = [(q.get("feature") or "") for q in by_pid[pid].get("paths", [])]
            placeholder = [f for f in feats
                           if f == "skeleton" or f.startswith(f"{pid} carrier ")]
            if feats and len(placeholder) == len(feats):
                print(f"    {pid}: all {len(feats)} layout path(s) carry placeholder "
                      f"labels — the strokes must be named before a table can exist")
            elif placeholder:
                print(f"    {pid}: {len(placeholder)}/{len(feats)} layout paths still "
                      f"carry placeholder labels")
            else:
                print(f"    {pid}: strokes are named — the table can be authored now")
    return 0


if __name__ == "__main__":
    sys.exit(main())
