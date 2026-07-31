#!/usr/bin/env python3
"""CC-PICTURE-BANK manifest gate (Done #1 and D5).

Checks every picture manifest against the schema, against the traced paths it
claims, and against the monotonicity law:

  Done #1  schema-valid, provenance present, every traced path assigned to
           exactly ONE layer, every layer's word count met in every language.
  D5       within any picture x language, band difficulty strictly increases
           per layer -- mean AND max. A violating manifest fails the build,
           and there is no override flag.

Written in Python, not JS, on purpose: it recomputes difficulty from the same
`extractors` module the generator uses. A validator with its own private
notion of "hard" would drift from the bank it is meant to police, which is
the exact failure CC-PICTURE-BANK D2 warns about.

Note what this does NOT do: it never trusts the generator's recorded `band`
to prove monotonicity. It re-derives the scores from the pools and checks the
words themselves climb. A generator and its own checker agreeing proves
nothing.
"""
from __future__ import annotations

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools/difficulty-score"))
sys.path.insert(0, str(ROOT / "tools"))
from build_manifests import bands_for, difficulty  # noqa: E402

SCANS = ROOT / "content-pipeline/wordpic/scans"
MANIFESTS = ROOT / "content-pipeline/wordpic/manifests"
REQUIRED = {"id", "tier", "source", "parts", "layers", "audio_gate", "completion"}
AUDIO_TIERS = {"starter", "intermediate", "advanced", "expert", "masterpiece"}


def check(sub: str, m: dict, scan: dict) -> list[str]:
    bad: list[str] = []
    missing = REQUIRED - set(m)
    if missing:
        bad.append(f"missing required field(s): {sorted(missing)}")
        return bad  # nothing below is meaningful without the shape

    if m["id"] != sub:
        bad.append(f"id {m['id']!r} does not match its filename")
    if m["audio_gate"] not in AUDIO_TIERS:
        bad.append(f"audio_gate {m['audio_gate']!r} is not a known tier")

    src = m["source"]
    if not src.get("title") or not src.get("license"):
        bad.append("provenance incomplete: every picture needs a title and a licence")
    # A sourced picture must say where it came from; an original must not
    # pretend to. Both directions matter -- one is a licence risk, the other
    # is a false claim of authorship.
    original = src["license"].startswith("Original artwork")
    if not original and not src.get("url"):
        bad.append(f"licensed as {src['license']!r} but carries no source URL")
    if original and src.get("url"):
        bad.append("claims original authorship yet cites an external source URL")

    # Every traced path lands in exactly one layer.
    n_paths = len(scan["paths"])
    seen: dict[int, int] = {}
    for li, layer in enumerate(m["layers"]):
        if not layer["path_ids"]:
            bad.append(f"layer {li} ({layer['name']}) is empty")
        for pid in layer["path_ids"]:
            if not 0 <= pid < n_paths:
                bad.append(f"layer {li} claims path {pid}, which does not exist")
            elif pid in seen:
                bad.append(f"path {pid} is in layer {seen[pid]} AND layer {li}")
            else:
                seen[pid] = li
    orphans = sorted(set(range(n_paths)) - set(seen))
    if orphans:
        bad.append(f"paths {orphans} belong to no layer — they would never be spellable")

    # Per language: the band must actually hold enough words, and the climb
    # must be real. Recomputed from the pools, never read off the manifest.
    langs = sorted(m["layers"][0]["per_language"])
    for lang in langs:
        bands = bands_for(lang, m["tier"])
        prev_mean = prev_max = None
        for li, layer in enumerate(m["layers"]):
            spec = layer["per_language"].get(lang)
            if spec is None:
                bad.append(f"layer {li} has no plan for {lang}")
                continue
            words = bands[spec["band"]]
            if len(words) < spec["words"]:
                bad.append(
                    f"{lang} layer {li}: needs {spec['words']} words, "
                    f"band {spec['band']} holds {len(words)}")
            if not words:
                continue
            scores = [difficulty(w, lang) for w in words]
            mean, mx = sum(scores) / len(scores), max(scores)
            if prev_mean is not None:
                # D5, both halves. Mean alone would let a layer of uniformly
                # slightly-harder words count as a climb while its ceiling
                # stayed flat; max alone would let one outlier carry a layer
                # that is otherwise easier than the one before it.
                if mean <= prev_mean:
                    bad.append(
                        f"{lang} layer {li}: mean difficulty {mean:.2f} does not "
                        f"exceed layer {li-1}'s {prev_mean:.2f} (D5)")
                if mx <= prev_max:
                    bad.append(
                        f"{lang} layer {li}: max difficulty {mx:.2f} does not "
                        f"exceed layer {li-1}'s {prev_max:.2f} (D5)")
            prev_mean, prev_max = mean, mx
    return bad


def main() -> int:
    scans = {p.stem for p in SCANS.glob("*.json")}
    mans = {p.stem for p in MANIFESTS.glob("*.json")}
    failures = 0
    if scans - mans:
        print(f"FAIL: traced but unmanifested: {sorted(scans - mans)}")
        failures += 1
    if mans - scans:
        print(f"FAIL: manifest with no traced scan: {sorted(mans - scans)}")
        failures += 1

    for sub in sorted(mans & scans):
        m = json.loads((MANIFESTS / f"{sub}.json").read_text())
        scan = json.loads((SCANS / f"{sub}.json").read_text())
        bad = check(sub, m, scan)
        depth = len(m["layers"])
        if bad:
            failures += 1
            print(f"{sub:10} FAIL")
            for b in bad:
                print(f"           - {b}")
        else:
            print(f"{sub:10} ok   layers={depth} gate={m['audio_gate']}")

    print()
    if failures:
        print(f"manifest-check: {failures} FAILED")
        return 1
    print(f"manifest-check: OK — {len(mans & scans)} manifests, "
          f"schema + provenance + path coverage + D5 monotonicity")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
