#!/usr/bin/env python3
"""CC-HUMAN-AUDIO F3: build the auditor's verification sheets for one language.

One sheet per tier, one row per entry: the entry's best clip that passed F2.
"Best" is the dominant speaker first (one voice across the pilot, D7's intent),
then CC0 (no attribution to carry), then the cleanest signal. Other clips for
the same entry stay in reserve for a later round if the auditor rejects this one.

DECOYS ARE WRONG ROWS THAT LOOK LIKE REAL ONES (F3, after tools/ru_stress_annotate.py).
Each sheet gets DECOY_COUNT rows for entries that have NO usable clip, each
playing a real recording of a similar-looking different bank word (small edit
distance: affect for effect). Every entry appears once, clip files are named by
row number, so nothing on the sheet marks a decoy. The key that says which rows
are decoys goes to a separate file the auditor never sees.

Outputs, under <dir>/audit/<sheet-id>/:
  sheet.csv          what the auditor fills (verdict column), stamped
  clips/rNNNN.m4a    the normalized clips, named by row
  listen.html        a local page to play each row and fill the verdicts
  <sheet-id>-DECOY-KEY.json   next to the audit folder, NOT inside it; never send it

Run: python3 tools/human-audio/sheet.py --dir ~/repos/ha-census-cache/phase-b/en
"""
import argparse
import collections
import csv
import hashlib
import html
import json
import pathlib
import random
import shutil
import time

VERDICTS = ["accept", "wrong word", "wrong stress/tone", "wrong variety",
            "poor quality", "inappropriate background"]
DECOY_COUNT = 8           # F3: 5-10 per sheet
TIERS = ["easy", "medium", "hard", "expert"]


def sha256(p):
    return hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()


def edit_distance(a, b):
    prev = list(range(len(b) + 1))
    for i, ca in enumerate(a, 1):
        cur = [i]
        for j, cb in enumerate(b, 1):
            cur.append(min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (ca != cb)))
        prev = cur
    return prev[-1]


def stamp(sheet_id, rows):
    """Hash over everything the auditor must not change: row, entry, clip
    name and the clip's bytes. Ingest recomputes it and refuses a mismatch."""
    h = hashlib.sha256(sheet_id.encode())
    for r in rows:
        h.update(f"\n{r['row']}\t{r['entry']}\t{r['clip']}\t{r['clip_sha256']}".encode())
    return h.hexdigest()


def rows_json(folder, public):
    """The page's rows, each clip EMBEDDED as a data: URI. WebKit browsers
    (Safari, DuckDuckGo) refuse to load a sibling file from a page opened as a
    file, so a page that referenced clips/rNNNN.m4a showed "Error" on every row.
    Embedded, the page plays anywhere with no server. The clips folder stays:
    it is what the stamp and the ingest verify."""
    import base64
    out = []
    for r in public:
        data = base64.b64encode((pathlib.Path(folder) / "clips" / r["clip"]).read_bytes()).decode()
        out.append({"row": r["row"], "entry": r["entry"], "clip": r["clip"],
                    "data": "data:audio/mp4;base64," + data})
    return json.dumps(out, ensure_ascii=False)


LISTEN = """<!doctype html><html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Audio audit __ID__</title>
<style>
:root{--bg:#fff;--fg:#1b1d22;--mut:#646b78;--line:#e3e6ec;--acc:#2f5bd3;--row:#f6f7f9}
@media (prefers-color-scheme:dark){:root{--bg:#15171b;--fg:#e8eaee;--mut:#9aa1ad;--line:#2b2f37;--acc:#7da2ff;--row:#1c1f24}}
body{margin:0;background:var(--bg);color:var(--fg);font:15px/1.4 -apple-system,system-ui,sans-serif}
main{max-width:760px;margin:0 auto;padding:16px}
h1{font-size:20px;margin:8px 0}p{color:var(--mut);margin:4px 0 12px}
.bar{position:sticky;top:0;background:var(--bg);padding:10px 0;border-bottom:1px solid var(--line);display:flex;gap:10px;align-items:center;flex-wrap:wrap;z-index:1}
button{font:inherit;padding:8px 14px;border-radius:8px;border:1px solid var(--line);background:var(--acc);color:#fff;cursor:pointer}
.r{display:grid;grid-template-columns:52px 1fr auto;gap:10px;align-items:center;padding:10px;border-bottom:1px solid var(--line)}
.r.cur{background:var(--row)}.n{color:var(--mut);font-variant-numeric:tabular-nums}
.w{font-size:20px;font-weight:600}select{font:inherit;padding:6px;border-radius:6px;max-width:210px}
.done{color:var(--mut)}
</style></head><body><main>
<h1>Audio audit: __ID__</h1>
<p>Play each clip. Does it say the word shown, in US English, cleanly, with nothing unsuitable for kids in the background? Pick a verdict for every row, then export the sheet and send back <b>sheet.csv</b>.</p>
<p>Keys: <b>Space</b> play, <b>A</b> accept, <b>1–5</b> the other verdicts in list order, <b>J/K</b> next/previous.</p>
<div class="bar"><button id="exp">Export sheet.csv</button><button id="cpy" style="display:none">Copy sheet</button><span id="cnt" class="done"></span></div>
<textarea id="out" readonly style="display:none;width:100%;height:9em;font:12px ui-monospace,monospace;margin:8px 0;background:var(--row);color:var(--fg);border:1px solid var(--line);border-radius:8px"></textarea>
<div id="rows"></div></main>
<script>
const ID="__ID__",ROWS=__ROWS__,VERDICTS=__VERDICTS__,KEY="audit-v2-"+ID;
let saved={};try{saved=JSON.parse(localStorage.getItem(KEY)||"{}")}catch(e){}
const box=document.getElementById("rows");let cur=0;
ROWS.forEach((r,i)=>{const d=document.createElement("div");d.className="r";d.id="row"+i;
d.innerHTML=`<span class="n">${r.row}</span><span><span class="w"></span><br><audio preload="none" controls src="${r.data||("clips/"+r.clip)}"></audio></span>`;
d.querySelector(".w").textContent=r.entry;
const s=document.createElement("select");s.innerHTML='<option value="">verdict…</option>'+VERDICTS.map(v=>`<option>${v}</option>`).join("");
s.value=saved[r.row]||"";s.onchange=()=>{saved[r.row]=s.value;store()};d.appendChild(s);
d.onclick=()=>focusRow(i,false);box.appendChild(d)});
function store(){try{localStorage.setItem(KEY,JSON.stringify(saved))}catch(e){}count()}
function count(){const n=ROWS.filter(r=>saved[r.row]).length;document.getElementById("cnt").textContent=n+" of "+ROWS.length+" done"}
function focusRow(i,scroll=true){document.querySelectorAll(".r.cur").forEach(e=>e.classList.remove("cur"));cur=Math.max(0,Math.min(ROWS.length-1,i));const e=document.getElementById("row"+cur);e.classList.add("cur");if(scroll)e.scrollIntoView({block:"center"})}
function setV(v){const r=ROWS[cur];saved[r.row]=v;document.querySelector("#row"+cur+" select").value=v;store();focusRow(cur+1);play()}
function play(){const a=document.querySelector("#row"+cur+" audio");document.querySelectorAll("audio").forEach(x=>{if(x!==a)x.pause()});a.currentTime=0;a.play()}
document.addEventListener("keydown",e=>{if(e.target.tagName==="SELECT")return;const k=e.key.toLowerCase();
if(k===" "){e.preventDefault();play()}else if(k==="a")setV("accept");else if("12345".includes(k)&&k)setV(VERDICTS[+k]);
else if(k==="j")focusRow(cur+1);else if(k==="k")focusRow(cur-1)});
function csvCell(s){return /[",\\n]/.test(s)?'"'+s.replace(/"/g,'""')+'"':s}
function sheetText(){const lines=["# sheet "+ID+" stamp __STAMP__","row,entry,clip,verdict,note"];
ROWS.forEach(r=>lines.push([r.row,r.entry,r.clip,saved[r.row]||"",""].map(csvCell).join(",")));return lines.join("\\n")+"\\n"}
// Some browsers silently block downloads from a page opened as a file, so the
// sheet is ALSO shown as text with a Copy button: an export can't be lost.
document.getElementById("exp").onclick=()=>{const t=sheetText();
try{const b=new Blob([t],{type:"text/csv"});const a=document.createElement("a");a.href=URL.createObjectURL(b);a.download="sheet.csv";document.body.appendChild(a);a.click();a.remove()}catch(e){}
const o=document.getElementById("out");o.value=t;o.style.display="block";document.getElementById("cpy").style.display=""};
document.getElementById("cpy").onclick=async()=>{const o=document.getElementById("out");let ok=false;
try{await navigator.clipboard.writeText(o.value);ok=true}catch(e){o.select();try{ok=document.execCommand("copy")}catch(e2){}}
document.getElementById("cpy").textContent=ok?"Copied — paste it to Claude":"Select the text and copy it"};
focusRow(0,false);count();
</script></body></html>
"""


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dir", required=True)
    ap.add_argument("--tiers", nargs="*", default=TIERS)
    ap.add_argument("--seed", type=int, default=20260919)
    ap.add_argument("--bank", help="bank.tsv from the census dump (default: two levels above --dir)")
    ap.add_argument("--second-look", nargs="+", metavar="TIER",
                    help="instead: one sheet of these tiers' entries lost to the whisper check alone")
    a = ap.parse_args()
    d = pathlib.Path(a.dir).expanduser()
    man = json.loads((d / "manifest.json").read_text())
    qc = json.loads((d / "qc.json").read_text())["clips"]
    lang = man["lang"]
    rng = random.Random(a.seed)

    by_speaker = collections.Counter(c["speaker"] for c in man["clips"])
    passed = collections.defaultdict(list)
    tiers_of = {}
    for c in man["clips"]:
        tiers_of[c["key"]] = c["tiers"]
        q = qc.get(c["commons_sha1"])
        if q and q["status"] == "to_audit":
            passed[c["key"]].append((c, q))

    def rank(item):
        c, q = item
        return (-by_speaker[c["speaker"]], c["license"] != "CC0", -(q.get("snr_db") or 0))

    # Entries with no clip at all in this language (gaps) host the decoys.
    bank_path = pathlib.Path(a.bank).expanduser() if a.bank else d.parents[1] / "bank.tsv"
    bank = [l.split("\t") for l in bank_path.read_text().splitlines()]
    tier_entries = collections.defaultdict(list)
    for lg, tier, entry in bank:
        if lg == lang:
            tier_entries[tier].append(entry)
    have = set(passed)

    audit_root = d / "audit"
    audit_root.mkdir(exist_ok=True)
    today = time.strftime('%Y%m%d')

    def real_row(c, q):
        return {"entry": c["entry"], "src": d / q["norm_path"], "decoy": False,
                "commons_sha1": c["commons_sha1"], "speaker": c["speaker"], "license": c["license"]}

    jobs = []
    if a.second_look:
        # SECOND LOOK (Eric, 2026-09-19): entries whose EVERY clip failed F2
        # only because whisper heard another word. Whisper saves the auditor
        # time; it is not the judge. Loudness, clipping, noise and duration
        # rejects never come back here.
        whisper_only = collections.defaultdict(list)
        for c in man["clips"]:
            q = qc.get(c["commons_sha1"])
            if (q and q["status"] == "rejected" and c["key"] not in passed
                    and q["reasons"] and all(r.startswith("loopback heard") for r in q["reasons"])
                    and min(tiers_of[c["key"]], key=TIERS.index) in a.second_look):
                whisper_only[c["key"]].append((c, q))
        real = [real_row(*sorted(v, key=rank)[0]) for _, v in sorted(whisper_only.items())]
        jobs.append((f"{lang}-secondlook-{today}", "secondlook", real, a.second_look,
                     set(whisper_only)))
    else:
        for tier in a.tiers:
            real = [real_row(*sorted(passed[k], key=rank)[0])
                    for k in sorted(k for k in passed if min(tiers_of[k], key=TIERS.index) == tier)]
            jobs.append((f"{lang}-{tier}-{today}", tier, real, [tier], set()))

    for sheet_id, tier, real, gap_tiers, also_taken in jobs:
        folder = audit_root / sheet_id
        if folder.exists():
            shutil.rmtree(folder)
        (folder / "clips").mkdir(parents=True)
        # Decoys: a gap entry (no clip at all), played with a real clip of a close word.
        gaps = [e for t in gap_tiers for e in tier_entries[t] if e not in have and e not in also_taken]
        rng.shuffle(gaps)
        # A donor is heard once per sheet and never alongside its own real row:
        # the same recording twice on one sheet would give the decoy away.
        on_sheet = {c["entry"] for c in real}
        donors = [k for k in sorted(passed) if passed[k][0][0]["entry"] not in on_sheet]
        decoys = []
        for gap in gaps:
            if len(decoys) == DECOY_COUNT:
                break
            near = [k for k in donors if k != gap and abs(len(k) - len(gap)) <= 2
                    and 1 <= edit_distance(k, gap) <= 2]
            if not near:
                continue
            donor = rng.choice(near)
            donors.remove(donor)
            c, q = sorted(passed[donor], key=rank)[0]
            decoys.append({"entry": gap, "src": d / q["norm_path"], "decoy": True,
                           "actually_says": donor, "commons_sha1": c["commons_sha1"]})
        rows = real + decoys
        rng.shuffle(rows)
        public, key_rows = [], []
        for i, r in enumerate(rows, 1):
            name = f"r{i:04d}.m4a"
            shutil.copyfile(r["src"], folder / "clips" / name)
            digest = sha256(folder / "clips" / name)
            public.append({"row": i, "entry": r["entry"], "clip": name, "clip_sha256": digest})
            key_rows.append({**{k: v for k, v in r.items() if k != "src"},
                             "row": i, "clip": name, "clip_sha256": digest})
        st = stamp(sheet_id, public)
        with open(folder / "sheet.csv", "w", newline="", encoding="utf-8") as fh:
            fh.write(f"# sheet {sheet_id} stamp {st}\n")
            w = csv.writer(fh)
            w.writerow(["row", "entry", "clip", "verdict", "note"])
            for r in public:
                w.writerow([r["row"], r["entry"], r["clip"], "", ""])
        page = (LISTEN.replace("__ID__", html.escape(sheet_id)).replace("__STAMP__", st)
                .replace("__VERDICTS__", json.dumps(VERDICTS))
                .replace("__ROWS__", rows_json(folder, public)))
        (folder / "listen.html").write_text(page, encoding="utf-8")
        (audit_root / f"{sheet_id}-DECOY-KEY.json").write_text(json.dumps({
            "sheet_id": sheet_id, "lang": lang, "tier": tier, "stamp": st,
            "verdicts": VERDICTS, "rows": key_rows}, ensure_ascii=False, indent=1))
        print(f"{sheet_id}: {len(real)} real rows + {len(decoys)} decoys -> {folder}")

if __name__ == "__main__":
    main()
