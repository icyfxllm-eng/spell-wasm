#!/usr/bin/env python3
"""CC-SENSE-CUE v1.1 F6 -- the auditor's "same sound?" sheet and its locked ingest.

There is no Gig B sheet artifact in this repo, so this follows the one locked-sheet
scheme that exists, CC-HUMAN-AUDIO F3 (tools/human-audio/sheet.py + ingest.py):
a stamped header, unmarked decoy rows, a decoy key kept OUTSIDE the audit folder,
and four gates where any failure rejects the whole sheet and writes nothing.

  build    python3 tools/soundalike_audit.py build --lang en [--out DIR]
  ingest   python3 tools/soundalike_audit.py ingest --sheet RETURNED.csv --key KEY.json --auditor NAME [--write]
  selftest python3 tools/soundalike_audit.py selftest

One row per candidate set (every kind "true" set, plus any set carrying an F5
loopbackFlag). The auditor answers: do all these spellings sound the same in
<voice>? yes / no / unsure.

DECOYS are pairs of bank words that look alike (edit distance 1-2) but have
different spoken keys, so the honest answer is "no". Nothing on the sheet marks
them. Answering "yes" to more than one decoy rejects the sheet.

VERDICTS live in assets/words/<lang>/soundalike-verdicts.json, keyed by setId
(S1). tools/build_collisions.py applies them every time it regenerates a table, so
an ingested verdict survives regeneration while its set's members are unchanged
(I2). This ingest is the only writer of that file (I5):
  yes     -> confirmed: true, loopback flag cleared
  no      -> kind: accent, mergerRules: [auditor-distinguishes], flagged for Eric
  unsure  -> no change (recorded, so the set is not asked again by accident)
"""
import argparse
import csv
import hashlib
import json
import pathlib
import random
import sys
import tempfile
import time

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from soundalikes import ROOT, soundalikes  # noqa: E402

VERDICTS = ["yes", "no", "unsure"]
DECOY_TOLERANCE = 1
AUDIT_HOME = pathlib.Path.home() / "repos" / "sense-cue-audit"


def verdict_path(lang, root=ROOT):
    return pathlib.Path(root) / "assets" / "words" / lang / "soundalike-verdicts.json"


def load_verdicts(lang, root=ROOT):
    p = verdict_path(lang, root)
    return json.loads(p.read_text(encoding="utf-8")) if p.exists() else {"lang": lang, "sets": {}}


def stamp(sheet_id, rows):
    """Hash over everything the auditor must not change: row, setId and members."""
    h = hashlib.sha256(sheet_id.encode())
    for r in rows:
        h.update(f"\n{r['row']}\t{r['set']}\t{r['members']}".encode())
    return h.hexdigest()


def edit_distance(a, b):
    prev = list(range(len(b) + 1))
    for i, ca in enumerate(a, 1):
        cur = [i]
        for j, cb in enumerate(b, 1):
            cur.append(min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (ca != cb)))
        prev = cur
    return prev[-1]


def decoy_pairs(lang, count, rng):
    """Look-alike bank pairs whose spoken keys differ: the true answer is 'no'."""
    import build_collisions as bc
    key = bc.key_en() if lang == "en" else bc.KEYS.get(lang)
    words = [w for w in bc.bank(lang) if 4 <= len(w) <= 9]
    in_sets = {m["memberId"] for s in soundalikes(lang)["sets"] for m in s["members"]}
    rng.shuffle(words)
    out, used = [], set()
    pool = words[:3000]
    for a in pool:
        if len(out) >= count:
            break
        if a in used or a in in_sets:
            continue
        ka = key(a)
        if not ka:
            continue
        for b in pool:
            if b == a or b in used or b in in_sets or abs(len(a) - len(b)) > 1:
                continue
            if not 1 <= edit_distance(a, b) <= 2:
                continue
            kb = key(b)
            if not kb:
                continue
            ska = set(ka) if isinstance(ka, list) else {ka}
            skb = set(kb) if isinstance(kb, list) else {kb}
            if ska & skb:
                continue
            out.append(sorted([a, b]))
            used.update((a, b))
            break
    return out


def build(lang, out_dir, seed=None, flagged_only=False):
    t = soundalikes(lang)
    if flagged_only:
        # Only the sets the F5 loopback flagged: the ones worth a careful listen.
        cands = [s for s in t["sets"] if s.get("loopbackFlag")]
    else:
        cands = [s for s in t["sets"] if s["kind"] == "true" or s.get("loopbackFlag")]
    already = load_verdicts(lang)["sets"]
    cands = [s for s in cands if s["setId"] not in already]
    if not cands:
        raise SystemExit(f"{lang}: no unjudged candidate sets")
    sheet_id = f"{lang}-soundalike-{'flagged-' if flagged_only else ''}{time.strftime('%Y%m%d')}"
    rng = random.Random(seed if seed is not None else sheet_id)
    n_decoy = 8 if len(cands) >= 32 else (2 if len(cands) < 6 else 4)
    decoys = decoy_pairs(lang, n_decoy, rng)
    items = [{"set": s["setId"], "members": " / ".join(m["memberId"] for m in s["members"]), "decoy": False}
             for s in cands]
    # Same shape as a real setId (lang + 12 hex), so the id cannot mark a decoy.
    items += [{"set": f"{lang}:{hashlib.sha256(('decoy ' + ' '.join(p)).encode()).hexdigest()[:12]}",
               "members": " / ".join(p), "decoy": True} for p in decoys]
    rng.shuffle(items)
    rows = [{"row": i + 1, **it} for i, it in enumerate(items)]
    st = stamp(sheet_id, rows)
    folder = pathlib.Path(out_dir) / sheet_id
    folder.mkdir(parents=True, exist_ok=True)
    keys = pathlib.Path(out_dir) / "keys"
    keys.mkdir(parents=True, exist_ok=True)
    with open(folder / "sheet.csv", "w", encoding="utf-8", newline="") as f:
        f.write(f"# sheet {sheet_id} stamp {st}\n")
        w = csv.writer(f)
        w.writerow(["row", "set", "members", "verdict", "note"])
        for r in rows:
            w.writerow([r["row"], r["set"], r["members"], "", ""])
    (folder / "README.txt").write_text(
        f"Same sound? -- {lang}, voice {t.get('voice')}\n\n"
        "Each row lists spellings. Do ALL of them sound exactly the same when spoken in this voice's accent?\n"
        "Write yes, no or unsure in the verdict column. Do not edit any other column.\n", encoding="utf-8")
    write_listen_page(folder, lang, rows, t.get("voice"))
    key = {"sheet_id": sheet_id, "stamp": st, "lang": lang, "voice": t.get("voice"), "rows": rows}
    (keys / f"{sheet_id}-DECOY-KEY.json").write_text(json.dumps(key, ensure_ascii=False, indent=1) + "\n")
    print(f"{sheet_id}: {len(cands)} sets + {len(decoys)} decoys -> {folder / 'sheet.csv'}")
    print(f"  key (never send it): {keys / (sheet_id + '-DECOY-KEY.json')}")
    return folder / "sheet.csv", keys / f"{sheet_id}-DECOY-KEY.json"


LISTEN = """<!doctype html><html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1"><title>Same sound? __LANG__</title>
<style>
:root{--bg:#fff;--fg:#1b1d22;--mut:#646b78;--line:#e3e6ec;--acc:#2f5bd3}
@media (prefers-color-scheme:dark){:root{--bg:#15171b;--fg:#e8eaee;--mut:#9aa1ad;--line:#2b2f37;--acc:#7da2ff}}
body{margin:0;background:var(--bg);color:var(--fg);font:16px/1.4 -apple-system,system-ui,sans-serif}
main{max-width:720px;margin:0 auto;padding:16px}p{color:var(--mut)}
.row{display:flex;flex-wrap:wrap;gap:8px;align-items:center;padding:10px 0;border-bottom:1px solid var(--line)}
.n{width:3em;color:var(--mut)}button{font:inherit;padding:6px 12px;border:1px solid var(--line);border-radius:8px;background:none;color:var(--acc);cursor:pointer}
</style></head><body><main><h1>Same sound? (__LANG__, __VOICE__)</h1>
<p>Each row plays the game's own clips. Do all the spellings on a row sound the same? Write yes, no or unsure in the sheet's verdict column, by row number.</p>
__ROWS__</main><script>let a=new Audio();function p(s){a.pause();a=new Audio(s);a.play();}</script></body></html>"""


def write_listen_page(folder, lang, rows, voice):
    """A local page that plays each member with the game's clip (fetched through the
    same /api/speak the app uses, via the loopback's cache). Clips are named by row
    and position, so nothing marks a check row."""
    import html
    import shutil
    import soundalike_loopback as lb
    clips = folder / "clips"
    clips.mkdir(exist_ok=True)
    out = []
    for r in rows:
        btns = []
        for i, w in enumerate(m.strip() for m in r["members"].split("/")):
            lb.audio(w, lang)                       # makes sure the mp3 is cached
            src = lb.CACHE / lang / (__import__("urllib.parse").parse.quote(w, safe="") + ".mp3")
            name = f"r{r['row']:04d}-{i}.mp3"
            if src.exists():
                shutil.copyfile(src, clips / name)
            btns.append(f'<button onclick="p(\'clips/{name}\')">{html.escape(w)}</button>')
        out.append(f'<div class="row"><span class="n">{r["row"]}</span>{"".join(btns)}</div>')
    page = LISTEN.replace("__LANG__", lang).replace("__VOICE__", html.escape(voice or "")).replace("__ROWS__", "\n".join(out))
    (folder / "listen.html").write_text(page, encoding="utf-8")


def read_sheet(path):
    lines = pathlib.Path(path).read_text(encoding="utf-8-sig").splitlines()
    if not lines or not lines[0].startswith("# sheet "):
        return None, None, "missing the '# sheet <id> stamp <hash>' header line"
    parts = lines[0].split()
    if len(parts) != 5 or parts[3] != "stamp":
        return None, None, "malformed header line"
    return (parts[2], parts[4]), list(csv.DictReader(lines[1:])), None


def check(sheet_path, key_path, current_sets):
    """-> (verdicts to write, problems). Any problem rejects the whole sheet."""
    key = json.loads(pathlib.Path(key_path).read_text())
    head, rows, err = read_sheet(sheet_path)
    if err:
        return None, [err]
    problems = []
    if head[0] != key["sheet_id"]:
        problems.append(f"STAMP: sheet id {head[0]} is not the key's {key['sheet_id']}")
    if head[1] != key["stamp"] or stamp(key["sheet_id"], key["rows"]) != key["stamp"]:
        problems.append("STAMP: the sheet's stamp does not match the key's")
    by_row = {str(k["row"]): k for k in key["rows"]}
    if len(rows) != len(by_row):
        problems.append(f"AGREEMENT: sheet has {len(rows)} rows, key has {len(by_row)}")
    missed, out = [], []
    for r in rows:
        k = by_row.get((r.get("row") or "").strip())
        if k is None:
            problems.append(f"AGREEMENT: unknown row {r.get('row')!r}")
            continue
        if r.get("set") != k["set"] or r.get("members") != k["members"]:
            problems.append(f"AGREEMENT: row {k['row']} set/members edited")
            continue
        v = (r.get("verdict") or "").strip().lower()
        if v not in VERDICTS:
            problems.append(f"COMPLETE: row {k['row']} verdict {v!r} is not one of {VERDICTS}")
            continue
        if k["decoy"]:
            if v == "yes":
                missed.append(k["row"])
            continue
        if k["set"] not in current_sets:
            problems.append(f"AGREEMENT: row {k['row']} set {k['set']} is no longer in the table (regenerated); rebuild the sheet")
            continue
        out.append({"setId": k["set"], "verdict": v, "members": k["members"], "note": (r.get("note") or "").strip()})
    if len(missed) > DECOY_TOLERANCE:
        problems.append(f"DECOY: {len(missed)} decoys answered yes (tolerance {DECOY_TOLERANCE}): rows {missed}")
    return out, problems


def ingest(sheet, key_path, auditor, write, root=ROOT):
    key = json.loads(pathlib.Path(key_path).read_text())
    lang = key["lang"]
    current = {s["setId"] for s in soundalikes(lang, root=root)["sets"]}
    out, problems = check(sheet, key_path, current)
    if problems:
        print(f"REJECTED {key['sheet_id']}: nothing written")
        for p in problems[:40]:
            print("  " + p)
        return 1
    counts = {}
    for r in out:
        counts[r["verdict"]] = counts.get(r["verdict"], 0) + 1
    print(f"OK {key['sheet_id']}: {len(out)} verdicts {counts}")
    if not write:
        print("dry run: pass --write to record them, then regenerate with tools/build_collisions.py")
        return 0
    store = load_verdicts(lang, root)
    when = time.strftime("%Y-%m-%d")
    for r in out:
        store["sets"][r["setId"]] = {"verdict": r["verdict"], "members": r["members"], "note": r["note"],
                                     "sheet": key["sheet_id"], "auditor": auditor, "ingested": when}
    store["sets"] = dict(sorted(store["sets"].items()))
    verdict_path(lang, root).write_text(json.dumps(store, ensure_ascii=False, indent=1, sort_keys=True) + "\n")
    print(f"wrote {verdict_path(lang, root)}; now run tools/build_collisions.py {lang}")
    return 0


OVERRIDE_ACTIONS = ("drop_member", "target_only")


def overrides_path(lang, root=ROOT):
    return pathlib.Path(root) / "assets" / "words" / lang / "soundalike-overrides.json"


def ingest_overrides(path, key_path, auditor, write, root=ROOT):
    """Member-level fixes returned with a sheet. Same trust model as the sheet: the
    header must carry the sheet's id and stamp, every row must name a real row/set of
    that sheet, and every member must be in that set. Any problem writes nothing.
      drop_member  remove one member from that set only (e.g. 'for' from fir/fur)
      target_only  accept the set only when THIS member is the prompt (the voice says
                   a lone 'read' as 'reed', so red<-read is fair but read<-red is not)"""
    key = json.loads(pathlib.Path(key_path).read_text())
    lines = pathlib.Path(path).read_text(encoding="utf-8-sig").splitlines()
    problems = []
    head = lines[0].split() if lines else []
    if len(head) != 7 or head[:4] != ["#", "overrides", "for", "sheet"] or head[5] != "stamp":
        return print("REJECTED: missing '# overrides for sheet <id> stamp <hash>' header") or 1
    if head[4] != key["sheet_id"] or head[6] != key["stamp"]:
        problems.append("STAMP: the overrides name a different sheet or stamp than the key")
    by_row = {str(r["row"]): r for r in key["rows"]}
    out = {}
    for r in csv.DictReader(lines[1:]):
        k = by_row.get((r.get("row") or "").strip())
        if k is None or k["set"] != r.get("set"):
            problems.append(f"AGREEMENT: row {r.get('row')} / set {r.get('set')} is not on the sheet")
            continue
        if k["decoy"]:
            problems.append(f"AGREEMENT: row {k['row']} is not a set that can be overridden")
            continue
        act, mem = (r.get("action") or "").strip(), (r.get("member") or "").strip()
        members = [m.strip() for m in k["members"].split("/")]
        if act not in OVERRIDE_ACTIONS:
            problems.append(f"COMPLETE: row {k['row']} action {act!r} is not one of {OVERRIDE_ACTIONS}")
            continue
        if mem not in members:
            problems.append(f"AGREEMENT: row {k['row']} member {mem!r} is not in {members}")
            continue
        o = out.setdefault(k["set"], {"drop": [], "targetOnly": [], "reasons": []})
        o["drop" if act == "drop_member" else "targetOnly"].append(mem)
        if r.get("reason") and r["reason"] not in o["reasons"]:
            o["reasons"].append(r["reason"])
    for sid, o in out.items():
        members = [m.strip() for m in next(r["members"] for r in key["rows"] if r["set"] == sid).split("/")]
        if len(set(members) - set(o["drop"])) < 2:
            problems.append(f"COMPLETE: {sid}: dropping {o['drop']} leaves fewer than two members")
    if problems:
        print(f"REJECTED overrides for {key['sheet_id']}: nothing written")
        for p in problems:
            print("  " + p)
        return 1
    print(f"OK overrides for {key['sheet_id']}: {len(out)} sets "
          f"({sum(len(o['drop']) for o in out.values())} drops, {sum(len(o['targetOnly']) for o in out.values())} target-only)")
    if not write:
        print("dry run: pass --write to record them")
        return 0
    lang = key["lang"]
    store = json.loads(overrides_path(lang, root).read_text()) if overrides_path(lang, root).exists() else {"lang": lang, "sets": {}}
    when = time.strftime("%Y-%m-%d")
    for sid, o in out.items():
        store["sets"][sid] = {"drop": sorted(set(o["drop"])), "targetOnly": sorted(set(o["targetOnly"])),
                              "reason": " | ".join(o["reasons"]), "sheet": key["sheet_id"], "auditor": auditor, "ingested": when}
    store["sets"] = dict(sorted(store["sets"].items()))
    overrides_path(lang, root).write_text(json.dumps(store, ensure_ascii=False, indent=1, sort_keys=True) + "\n")
    print(f"wrote {overrides_path(lang, root)}; now run tools/build_collisions.py {lang}")
    return 0


def selftest():
    """Each gate must reject what it exists to reject, and a clean sheet must pass."""
    with tempfile.TemporaryDirectory() as d:
        rows = [{"row": 1, "set": "en:aaa", "members": "sink / sync", "decoy": False},
                {"row": 2, "set": "en:dbbb", "members": "cart / card", "decoy": True},
                {"row": 3, "set": "en:dccc", "members": "plan / plant", "decoy": True},
                {"row": 4, "set": "en:ddd", "members": "pair / pear", "decoy": False}]
        sid = "en-soundalike-test"
        st = stamp(sid, rows)
        kp = pathlib.Path(d) / "key.json"
        kp.write_text(json.dumps({"sheet_id": sid, "stamp": st, "lang": "en", "rows": rows}))

        def sheet(verdicts, members=None, header=None):
            p = pathlib.Path(d) / f"s{len(list(pathlib.Path(d).iterdir()))}.csv"
            with open(p, "w", newline="") as f:
                f.write(header or f"# sheet {sid} stamp {st}\n")
                w = csv.writer(f)
                w.writerow(["row", "set", "members", "verdict", "note"])
                for r, v in zip(rows, verdicts):
                    w.writerow([r["row"], r["set"], (members or {}).get(r["row"], r["members"]), v, ""])
            return p
        cur = {"en:aaa", "en:ddd"}
        cases = [
            ("clean", sheet(["yes", "no", "no", "unsure"]), None),
            ("STAMP", sheet(["yes", "no", "no", "yes"], header=f"# sheet {sid} stamp {'0' * 64}\n"), "STAMP"),
            ("AGREEMENT", sheet(["yes", "no", "no", "yes"], members={1: "sink / zinc"}), "AGREEMENT"),
            ("COMPLETE", sheet(["yes", "", "no", "yes"]), "COMPLETE"),
            ("DECOY", sheet(["yes", "yes", "yes", "yes"]), "DECOY"),
        ]
        for name, p, want in cases:
            out, problems = check(p, kp, cur)
            got = problems[0].split(":")[0] if problems else None
            if got != want:
                print(f"soundalike_audit selftest FAILED: {name} expected {want}, got {problems}")
                return 1
    print("soundalike_audit selftest: OK (clean sheet passes; STAMP, AGREEMENT, COMPLETE, DECOY each reject)")
    return 0


def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    b = sub.add_parser("build")
    b.add_argument("--lang", required=True)
    b.add_argument("--out", default=str(AUDIT_HOME))
    b.add_argument("--flagged-only", action="store_true", help="only sets the voice loopback flagged")
    i = sub.add_parser("ingest")
    i.add_argument("--sheet", required=True)
    i.add_argument("--key", required=True)
    i.add_argument("--auditor", required=True)
    i.add_argument("--write", action="store_true")
    o = sub.add_parser("ingest-overrides")
    o.add_argument("--file", required=True)
    o.add_argument("--key", required=True)
    o.add_argument("--auditor", required=True)
    o.add_argument("--write", action="store_true")
    sub.add_parser("selftest")
    a = ap.parse_args()
    if a.cmd == "build":
        build(a.lang, a.out, flagged_only=a.flagged_only)
        return 0
    if a.cmd == "ingest":
        return ingest(a.sheet, a.key, a.auditor, a.write)
    if a.cmd == "ingest-overrides":
        return ingest_overrides(a.file, a.key, a.auditor, a.write)
    return selftest()


if __name__ == "__main__":
    raise SystemExit(main())
