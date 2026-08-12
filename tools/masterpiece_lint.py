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
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SCANS = ROOT / "content-pipeline/wordpic/scans"
MANIFESTS = ROOT / "content-pipeline/wordpic/manifests"
PICTURES = ROOT / "config/wordpic/pictures.json"

SILHOUETTE = {"outline"}
ANCHORS = {"features"}
TEXTURE = {"texture", "shading"}
KNOWN = SILHOUETTE | ANCHORS | TEXTURE

# Masters with no F4 anchor table yet. Held so the gate stays green while Eric
# authors them in the tool; the list SHRINKS ONLY — a piece that gains a table
# must leave, or the next master to ship without one hides behind a stale pass.
# Adding a name here is a deliberate act to argue for in the commit.
#
# redfuji, starrynight and sunflowers cannot gain one before they are re-traced:
# their whole layout is a single path labelled "skeleton". greatwave, scream and
# rhino could be authored today.
ANCHOR_TABLES_PENDING = frozenset({
    "greatwave", "scream", "rhino", "redfuji", "starrynight", "sunflowers",
})


def masters() -> list[str]:
    pics = json.loads(PICTURES.read_text())["pictures"]
    return sorted(p["id"] for p in pics if "masters" in p.get("categories", []))


def layers_of(pid: str) -> dict[str, list[int]]:
    mf = MANIFESTS / f"{pid}.json"
    if not mf.exists():
        return {}
    return {L["name"]: list(L["path_ids"]) for L in json.loads(mf.read_text())["layers"]}


# A principal contour must carry at least this share of the silhouette's
# total arc. Calibrated against the reference success: Mona's figure contour
# and frame split 52/33, and Red Fuji and Great Wave are single paths at 100%.
# A silhouette shattered into twenty even fragments tops out near 5% and fails,
# which is the mode this rule exists to catch.
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
    """
    if not paths:
        return False, 0.0
    arcs = [p.get("arc", 0.0) for p in paths]
    total = sum(arcs)
    if total <= 0:
        return False, 0.0
    return (max(arcs) / total) >= PRINCIPAL_SHARE, max(arcs) / total


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


def _p(arc: float, thin: bool = False) -> dict:
    return {"points": [[0, 0], [1, 1]], "arc": arc, "decorative_thin": thin}


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
    return 0


if __name__ == "__main__":
    sys.exit(main())
