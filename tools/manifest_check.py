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


def check_attribution() -> list[str]:
    """Every master credits its artist.

    The `attribution` on a scan is what the app SHOWS beside the piece. Dürer
    shipped uncredited: rhino.json carried an empty string while Hokusai,
    Munch and Leonardo were all named, and nothing noticed because provenance
    (source.title / source.license, above) lives in the MANIFEST and is a
    different field from the attribution in the SCAN. Two records of where a
    picture came from, one checked.

    Scoped to the `masters` category: a spiderweb has no artist to credit, and
    demanding one would be noise. A master with no scan yet is not a failure —
    it simply has nothing to attribute.
    """
    bad: list[str] = []
    pics = json.loads((ROOT / "config/wordpic/pictures.json").read_text())["pictures"]
    for e in sorted(pics, key=lambda x: x["id"]):
        if "masters" not in e.get("categories", []):
            continue
        scan = SCANS / f"{e['id']}.json"
        if not scan.exists():
            continue
        if not json.loads(scan.read_text()).get("attribution", "").strip():
            bad.append(
                f"{e['id']} is in `masters` but its scan carries no attribution — "
                f"the artist must be credited where the app displays it")
    return bad


def check_audits(mans: set[str]) -> list[str]:
    """CC-PICTURE-BANK Done #7 — per-language culture-pack audit gate.

    A subject under a PENDING audit is staged: it may be traced and
    manifested, but it must NOT be playable (present in pictures.json).
    A SIGNED audit must carry the recorded sign-off — date and the
    auditor's actual words; an unrecorded sign-off is not a sign-off.
    There is no override flag, same doctrine as D5.
    """
    bad: list[str] = []
    audit_path = ROOT / "content-pipeline/wordpic/audits.json"
    if not audit_path.exists():
        return ["audits.json is missing — Done #7's gate has nothing to enforce"]
    audits = json.loads(audit_path.read_text())["audits"]
    playable = {
        e["id"]
        for e in json.loads((ROOT / "config/wordpic/pictures.json").read_text())["pictures"]
    }
    for lang, a in sorted(audits.items()):
        subs = a.get("subjects", [])
        for sub in subs:
            if sub not in mans:
                bad.append(f"audit[{lang}] lists {sub!r}, which has no manifest — typo or lost work")
        if a["status"] == "pending":
            leaked = sorted(set(subs) & playable)
            if leaked:
                bad.append(
                    f"audit[{lang}] is PENDING ({a['auditor']}) but {leaked} are "
                    f"playable in pictures.json — the release is blocked until the "
                    f"sign-off is recorded in audits.json")
        elif a["status"] == "signed":
            if not a.get("signed_date") or not a.get("quote"):
                bad.append(
                    f"audit[{lang}] claims signed but the record is incomplete: "
                    f"signed_date and the auditor's quote are both required")
        else:
            bad.append(f"audit[{lang}] has unknown status {a['status']!r}")
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

    for b in check_audits(mans):
        failures += 1
        print(f"AUDIT      FAIL\n           - {b}")

    for b in check_attribution():
        failures += 1
        print(f"ATTRIB     FAIL\n           - {b}")

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
          f"schema + provenance + path coverage + D5 monotonicity + Done #7 audits")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
