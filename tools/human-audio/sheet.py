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
  keys/<sheet-id>-DECOY-KEY.json   outside the audit folder (which may be served); never send it

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
    """The page's rows. Clips are fetched by URL from the local audit server
    (one shared player), not embedded: hundreds of embedded media elements
    were more than DuckDuckGo/Safari (WebKit) would run."""
    return json.dumps([{"row": r["row"], "entry": r["entry"], "clip": r["clip"]} for r in public],
                      ensure_ascii=False)


LISTEN = """<!doctype html><html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Audio audit __ID__</title>
<style>
:root{--bg:#fff;--fg:#1b1d22;--mut:#646b78;--line:#e3e6ec;--acc:#2f5bd3;--row:#f6f7f9;--bad:#b3261e}
@media (prefers-color-scheme:dark){:root{--bg:#15171b;--fg:#e8eaee;--mut:#9aa1ad;--line:#2b2f37;--acc:#7da2ff;--row:#1c1f24;--bad:#ff8a80}}
body{margin:0;background:var(--bg);color:var(--fg);font:15px/1.4 -apple-system,system-ui,sans-serif}
main{max-width:760px;margin:0 auto;padding:16px}
h1{font-size:20px;margin:8px 0}p{color:var(--mut);margin:4px 0 12px}
.bar{position:sticky;top:0;background:var(--bg);padding:10px 0;border-bottom:1px solid var(--line);display:flex;gap:10px;align-items:center;flex-wrap:wrap;z-index:1}
button{font:inherit;padding:8px 14px;border-radius:8px;border:1px solid var(--line);background:var(--acc);color:#fff;cursor:pointer}
.r{display:grid;grid-template-columns:44px 52px 1fr auto;gap:10px;align-items:center;padding:10px;border-bottom:1px solid var(--line)}
.r.cur{background:var(--row)}.n{color:var(--mut);font-variant-numeric:tabular-nums}
.pl{width:44px;height:44px;padding:0;border-radius:50%;font-size:16px}
.w{font-size:20px;font-weight:600}select{font:inherit;padding:6px;border-radius:6px;max-width:210px}
.done{color:var(--mut)}#msg{color:var(--bad);min-height:1.2em}
</style></head><body><main>
<h1>Audio audit: __ID__</h1>
<p>Play each clip. Is it <b>exactly</b> the word shown, in US English, clean, with nothing unsuitable for kids in the background? Some rows are planted with a different, similar word: listen for the word, not just for a nice voice.</p>
<p>Keys: <b>Space</b> play, <b>A</b> accept, <b>1–5</b> the other verdicts in list order, <b>J/K</b> next/previous. Or tap ▶ and pick from the list.</p>
<div class="bar"><button id="exp">Export sheet.csv</button><button id="cpy" style="display:none">Copy sheet</button><span id="cnt" class="done"></span></div>
<div id="msg"></div>
<textarea id="out" readonly style="display:none;width:100%;height:9em;font:12px ui-monospace,monospace;margin:8px 0;background:var(--row);color:var(--fg);border:1px solid var(--line);border-radius:8px"></textarea>
<div id="rows"></div></main>
<script>
const ID="__ID__",ROWS=__ROWS__,VERDICTS=__VERDICTS__,KEY="audit-v2-"+ID;
// ONE shared player, loading each clip from the local server only when asked.
// A media element per row (hundreds) is more than WebKit will run on one page.
const player=new Audio();player.preload="auto";
player.onerror=()=>say("That clip would not play ("+(player.error?player.error.code:"?")+"). Reload the page; if it keeps happening, tell Claude.");
let saved={};try{saved=JSON.parse(localStorage.getItem(KEY)||"{}")}catch(e){}
const box=document.getElementById("rows");let cur=0;
function say(t){document.getElementById("msg").textContent=t}
ROWS.forEach((r,i)=>{const d=document.createElement("div");d.className="r";d.id="row"+i;
const b=document.createElement("button");b.className="pl";b.textContent="▶";b.onclick=(e)=>{e.stopPropagation();focusRow(i,false);play()};
const n=document.createElement("span");n.className="n";n.textContent=r.row;
const w=document.createElement("span");w.className="w";w.textContent=r.entry;
const s=document.createElement("select");s.innerHTML='<option value="">verdict…</option>'+VERDICTS.map(v=>'<option>'+v+'</option>').join("");
s.value=saved[r.row]||"";s.onchange=()=>{saved[r.row]=s.value;store()};s.onclick=(e)=>e.stopPropagation();
d.append(b,n,w,s);d.onclick=()=>focusRow(i,false);box.appendChild(d)});
function store(){try{localStorage.setItem(KEY,JSON.stringify(saved))}catch(e){}count()}
function count(){const n=ROWS.filter(r=>saved[r.row]).length;document.getElementById("cnt").textContent=n+" of "+ROWS.length+" done"}
function focusRow(i,scroll=true){document.querySelectorAll(".r.cur").forEach(e=>e.classList.remove("cur"));cur=Math.max(0,Math.min(ROWS.length-1,i));const e=document.getElementById("row"+cur);e.classList.add("cur");if(scroll)e.scrollIntoView({block:"center"})}
function setV(v){const r=ROWS[cur];saved[r.row]=v;document.querySelector("#row"+cur+" select").value=v;store();focusRow(cur+1);play()}
function play(){say("");try{player.pause()}catch(e){}player.src="clips/"+ROWS[cur].clip;
const p=player.play();if(p&&p.catch)p.catch(err=>{if(err&&err.name!=="AbortError")say("Could not play: "+err.name+". Tap ▶ on the row to try again.")})}
document.addEventListener("keydown",e=>{if(e.target.tagName==="SELECT"||e.metaKey||e.ctrlKey)return;const k=e.key.toLowerCase();
if(k===" "){e.preventDefault();play()}else if(k==="a")setV("accept");else if(["1","2","3","4","5"].includes(k))setV(VERDICTS[+k]);
else if(k==="j")focusRow(cur+1);else if(k==="k")focusRow(cur-1)});
function csvCell(s){return /[",\\n]/.test(s)?'"'+s.replace(/"/g,'""')+'"':s}
function sheetText(){const lines=["# sheet "+ID+" stamp __STAMP__","row,entry,clip,verdict,note"];
ROWS.forEach(r=>lines.push([r.row,r.entry,r.clip,saved[r.row]||"",""].map(csvCell).join(",")));return lines.join("\\n")+"\\n"}
// Some browsers silently block downloads from a local page, so the sheet is
// ALSO shown as text with a Copy button: an export can't be lost.
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
    ap.add_argument("--entries", help="with one --tiers value: only these entries (a file, one per line)")
    ap.add_argument("--label", help="sheet-id label instead of the tier name, e.g. easy-part2")
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
        only = None
        if a.entries:
            # A partial re-audit (Eric, 2026-09-19): a sheet whose decoys failed
            # is redone as a FRESH sheet of the rows in question, with new
            # decoys, so nothing learned from the failed sheet carries over.
            only = {l.strip() for l in pathlib.Path(a.entries).expanduser().read_text().splitlines() if l.strip()}
        for tier in a.tiers:
            keys = sorted(k for k in passed if min(tiers_of[k], key=TIERS.index) == tier
                          and (only is None or passed[k][0][0]["entry"] in only))
            real = [real_row(*sorted(passed[k], key=rank)[0]) for k in keys]
            jobs.append((f"{lang}-{a.label or tier}-{today}", tier, real, [tier], set()))

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
        (d / "keys").mkdir(exist_ok=True)
        (d / "keys" / f"{sheet_id}-DECOY-KEY.json").write_text(json.dumps({
            "sheet_id": sheet_id, "lang": lang, "tier": tier, "stamp": st,
            "verdicts": VERDICTS, "rows": key_rows}, ensure_ascii=False, indent=1))
        print(f"{sheet_id}: {len(real)} real rows + {len(decoys)} decoys -> {folder}")

if __name__ == "__main__":
    main()
