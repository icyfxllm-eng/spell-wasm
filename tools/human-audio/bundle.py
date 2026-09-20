#!/usr/bin/env python3
"""CC-HUMAN-AUDIO D3/D13/F7: turn ingested verdicts into what the app ships.

Reads the verdicts that tools/human-audio/ingest.py wrote (the only write path,
I2) and decides, per (language, tier), whether that tier switches on:

  D3   verified coverage of the tier's pool must reach 80%. Below that the
       whole tier stays on TTS, so the voice never flips word to word.

For every entry in an enabled tier it copies the normalized clip the auditor
HEARD, checked against the sha256 the verdict is bound to, into
assets/human-audio/<lang>/<sha256>.m4a (D13: bundled, not served), and writes:

  assets/human-audio/runtime.json     entry -> file, compiled into the app
  assets/human-audio/<lang>/bundle.json  provenance of every shipped clip
  assets/human-audio/credits.json     every shipped clip that needs attribution (F7)

scripts/human-audio-check.mjs re-verifies all of it at build time.

Run: python3 tools/human-audio/bundle.py --lang en \\
       --work ~/repos/ha-census-cache/phase-b/en --bank ~/repos/ha-census-cache/bank.tsv [--write]
"""
import argparse
import collections
import hashlib
import json
import math
import pathlib
import shutil
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
ASSETS = ROOT / "assets" / "human-audio"
GATE = 0.80                       # D3 (default; --gate overrides for a trial)
ATTRIBUTION = {"CC BY", "CC BY-SA"}
TIERS = ["easy", "medium", "hard", "expert"]


def accepted(verdicts):
    """entry -> the verdict record in force. The LATEST ingestion wins, so a
    later rejection withdraws an earlier accept."""
    out = {}
    for entry, recs in verdicts["entries"].items():
        last = recs[-1]
        if last["verdict"] == "accept":
            out[entry] = last
    return out


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument("--lang", required=True)
    ap.add_argument("--work", required=True, help="harvest/qc folder with manifest.json and qc.json")
    ap.add_argument("--bank", required=True, help="bank.tsv from the census dump")
    ap.add_argument("--write", action="store_true")
    ap.add_argument("--assets", default=str(ASSETS), help="output root (tests point this elsewhere)")
    ap.add_argument("--gate", type=float, default=GATE,
                    help="coverage a tier needs to switch on (D3 is 0.80; lowering it is Eric's call)")
    a = ap.parse_args(argv)
    lang, work = a.lang, pathlib.Path(a.work).expanduser()
    assets = pathlib.Path(a.assets)

    vpath = assets / lang / "verdicts.json"
    ok = accepted(json.loads(vpath.read_text())) if vpath.exists() else {}
    man = {c["commons_sha1"]: c for c in json.loads((work / "manifest.json").read_text())["clips"]}
    qc = json.loads((work / "qc.json").read_text())["clips"]
    target = json.loads((ROOT / "config" / "audio-loudness.json").read_text())

    pools = collections.defaultdict(list)
    for line in pathlib.Path(a.bank).expanduser().read_text().splitlines():
        lg, tier, entry = line.split("\t")
        if lg == lang:
            pools[tier].append(entry)

    gate = a.gate
    enabled, report = [], []
    for tier in TIERS:
        n = len(pools[tier])
        v = sum(1 for e in pools[tier] if e in ok)
        on = n > 0 and v / n >= gate
        report.append((tier, n, v, on))
        if on:
            enabled.append(tier)
    for tier, n, v, on in report:
        print(f"{lang} {tier:6} verified {v:4}/{n:4} = {100 * v / max(n, 1):5.1f}%  "
              f"{'ENABLED' if on else f'stays on TTS (needs {max(0, math.ceil(gate * n) - v)} more)'}")

    ship = []
    problems = []
    for tier in enabled:
        for entry in pools[tier]:
            rec = ok.get(entry)
            if rec is None:
                continue  # unverified entries in an enabled tier stay on TTS
            q = qc.get(rec["commons_sha1"])
            c = man.get(rec["commons_sha1"])
            if not q or not c:
                problems.append(f"{entry}: verdict names clip {rec['commons_sha1']} not in this work folder")
                continue
            src = work / q["norm_path"]
            digest = hashlib.sha256(src.read_bytes()).hexdigest()
            if digest != rec["clip_sha256"]:
                problems.append(f"{entry}: the clip on disk is not the clip the auditor heard")
                continue
            if abs(q["final_lufs"] - target["target_lufs"]) > target["tolerance_lu"]:
                problems.append(f"{entry}: {q['final_lufs']} LUFS is outside tolerance")
                continue
            ship.append({"entry": entry, "tier": tier, "file": f"{digest}.m4a", "sha256": digest,
                         "src": src, "license": c["license"], "speaker": c["speaker"],
                         "source": c["page_url"], "commons_sha1": c["commons_sha1"],
                         "final_lufs": q["final_lufs"], "sheet": rec["sheet"], "auditor": rec["auditor"]})
    if problems:
        print("REFUSED: nothing written")
        for p in problems[:30]:
            print("  " + p)
        sys.exit(1)

    result = {"enabled": enabled, "ship": [s["entry"] for s in ship]}
    size = sum(s["src"].stat().st_size for s in ship)
    print(f"{lang}: {len(ship)} clips to ship, {size / 1e6:.1f} MB, tiers {enabled or 'none'}")
    if not a.write:
        print("dry run: pass --write to update assets/human-audio")
        return result

    d = assets / lang
    d.mkdir(parents=True, exist_ok=True)
    keep = {s["file"] for s in ship}
    for old in d.glob("*.m4a"):  # a withdrawn clip must stop shipping
        if old.name not in keep:
            old.unlink()
    for s in ship:
        dst = d / s["file"]
        if not dst.exists():
            shutil.copyfile(s["src"], dst)
    (d / "bundle.json").write_text(json.dumps({
        "lang": lang, "gate": gate, "enabled_tiers": enabled,
        "coverage": {t: {"pool": n, "verified": v} for t, n, v, _ in report},
        "clips": [{k: v for k, v in s.items() if k != "src"} for s in ship],
    }, ensure_ascii=False, indent=1) + "\n")

    rt_path = assets / "runtime.json"
    rt = json.loads(rt_path.read_text())
    if ship:
        rt["langs"][lang] = {"base": f"human-audio/{lang}/",
                             "clips": {s["entry"]: s["file"] for s in sorted(ship, key=lambda s: s["entry"])}}
    else:
        rt["langs"].pop(lang, None)
    rt_path.write_text(json.dumps(rt, ensure_ascii=False, indent=1) + "\n")

    cr_path = assets / "credits.json"
    cr = json.loads(cr_path.read_text())
    cr["clips"] = [c for c in cr["clips"] if c["lang"] != lang] + [
        {"lang": lang, "entry": s["entry"], "speaker": s["speaker"], "license": s["license"],
         "source": s["source"], "sha256": s["sha256"]}
        for s in ship if s["license"] in ATTRIBUTION]
    cr["clips"].sort(key=lambda c: (c["lang"], c["entry"]))
    cr_path.write_text(json.dumps(cr, ensure_ascii=False, indent=1) + "\n")
    print(f"wrote {rt_path}, {d / 'bundle.json'}, {cr_path}")
    return result


if __name__ == "__main__":
    main()
