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

WHAT THIS DOES NOT DO. F4's anchor tables are prose — "snow-streak zigzag at
summit" is not a geometric predicate — and Eric authors them in the tool with
position and scale tolerances. Until a piece carries `required_anchors`, the
anchor lint REPORTS it as unauthored instead of inventing coordinates for a
Hokusai. A piece that does carry them is checked properly.
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


CLASS = classes()


def check(pid: str) -> list[str]:
    sf = SCANS / f"{pid}.json"
    if not sf.exists():
        # Not a failure of THIS lint: a master with no trace has nothing to
        # check. It is reported by the scan-gap audit, which owns that.
        return []
    return evaluate(pid, json.loads(sf.read_text()), layers_of(pid),
                    CLASS.get(pid))


def evaluate(pid: str, doc: dict, lay: dict, cls: str | None) -> list[str]:
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

    # L3 — anchor presence. Scaffolded: checked when authored, reported when not.
    required = doc.get("required_anchors")
    if required is None:
        if not anc_ids:
            bad.append(f"{pid}: no anchors layer AND no required_anchors — nothing "
                       f"names this piece")
        # else: anchors exist but F4's table is unauthored; not a failure yet.
    else:
        have = {paths[i].get("anchor_id") for i in anc_ids if i < len(paths)}
        for a in required:
            if a not in have:
                bad.append(f"{pid}: required anchor {a!r} is missing — export blocked (F2)")
    return bad


def _p(arc: float, thin: bool = False, anchor: str | None = None) -> dict:
    return {"points": [[0, 0], [1, 1]], "arc": arc,
            "decorative_thin": thin, "anchor_id": anchor}


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
    cases = [
        ("shattered silhouette",
         {"paths": [_p(10) for _ in range(20)] + [_p(10, anchor="a")]},
         {"outline": list(range(20)), "features": [20]}, "LINE",
         "principal contour"),
        ("thin silhouette on a LINE piece",
         {"paths": [_p(100, thin=True), _p(10), _p(5)]}, one, "LINE",
         "silhouette path(s)"),
        ("thin anchor on a LINE piece",
         {"paths": [_p(100), _p(10, thin=True), _p(5)]}, one, "LINE",
         "anchor path(s)"),
        ("missing required anchor",
         {"paths": [_p(100), _p(10, anchor="nose")], "required_anchors": ["eyes"]},
         {"outline": [0], "features": [1]}, "LINE", "required anchor"),
        ("unknown layer name",
         {"paths": [_p(100), _p(10)]},
         {"outline": [0], "features": [1], "highlights": []}, "LINE",
         "unknown layer name"),
        ("no silhouette layer",
         {"paths": [_p(100)]}, {"features": [0]}, "LINE", "no silhouette layer"),
        ("no anchors and no table",
         {"paths": [_p(100)]}, {"outline": [0]}, "LINE", "nothing"),
        # The regression that forced Eric's two rulings: Mona's shape is legal.
        # Frame and figure 36.5px apart, thin strokes on outline and features.
        ("frame + figure + thin strokes on TONAL (Mona)",
         {"paths": [_p(1346), _p(840), _p(221, thin=True), _p(165),
                    _p(50, thin=True), _p(40, thin=True)]},
         {"outline": [0, 1, 2, 3], "features": [4, 5], "texture": []},
         "TONAL", None),
    ]
    bad = []
    for name, doc, lay, cls, want in cases:
        got = evaluate("selftest", doc, lay, cls)
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
    failures, checked, unauthored = [], 0, []
    for pid in ids:
        if not (SCANS / f"{pid}.json").exists():
            continue
        checked += 1
        if json.loads((SCANS / f"{pid}.json").read_text()).get("required_anchors") is None:
            unauthored.append(pid)
        failures += check(pid)
    for f in failures:
        print(f"FAIL {f}")
    if failures:
        return 1
    print(f"masterpiece-lint: OK — {checked} traced masters, silhouettes continuous, "
          f"weights monotonic")
    if unauthored:
        print(f"  note: F4 anchor tables unauthored for {unauthored} — anchor "
              f"presence is reported, not enforced, until Eric writes them")
    return 0


if __name__ == "__main__":
    sys.exit(main())
