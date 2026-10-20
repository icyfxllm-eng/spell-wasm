#!/usr/bin/env python3
"""R1 styling inventory. Read-only. Usage: style_inventory.py <repo> <out_csv>"""
import re, sys, subprocess, csv, pathlib, collections

repo = pathlib.Path(sys.argv[1]); out = sys.argv[2]
files = subprocess.run(["git", "-C", str(repo), "ls-files"], capture_output=True, text=True).stdout.split("\n")
EXCL = ("node_modules/", "archive/", "tests/", "scripts/", "audit/", "reports/", "tools/", "examples/",
        "dist-test-web/", "android/", "ios/", "fixtures/", "census/", "docs/", "store/", "desktop/",
        "backend/", "workers/", "content-pipeline/", "fastlane/", "proptest", "tonal/", "scanlock/", "tracer-core/")
sel = [f for f in files if re.search(r"\.(css|html|js|mjs|rs)$", f) and not f.startswith(EXCL)]
# rust test files are not shipped
sel = [f for f in sel if not re.search(r"(^|/)tests?\.rs$|_tests\.rs$", f)]

HEX = re.compile(r"(?<![\w&])#(?:[0-9a-fA-F]{8}|[0-9a-fA-F]{6}|[0-9a-fA-F]{4}|[0-9a-fA-F]{3})(?![0-9a-zA-Z_-])")
FUNC = re.compile(r"\b(?:rgba?|hsla?|oklch|oklab|lab|lch|color-mix)\(", re.I)
NAMED = "white|black|red|green|blue|gray|grey|orange|yellow|purple|pink|gold|silver|navy|teal|crimson|tomato|maroon|olive|lime|aqua|cyan|magenta|indigo|violet|brown|beige|ivory|coral|salmon|khaki|lavender|darkgr[ae]y|lightgr[ae]y|whitesmoke|gainsboro|dimgr[ae]y|slategr[ae]y"
NAMEDRE = re.compile(r"(?:^|[\s:,(])(" + NAMED + r")(?=[\s;,)!\"'}]|$)", re.I)
COLORPROP = re.compile(r"(?:color|background|border[\w-]*|fill|stroke|outline[\w-]*|box-shadow|text-shadow|caret-color|accent-color|fillStyle|strokeStyle|shadowColor)\s*[:=]\s*([^;}\n]*)", re.I)
DIM = re.compile(r"(?<![\w#.-])-?(?:\d+\.?\d*|\.\d+)(px|rem|em|vh|vw|vmin|vmax|dvh|svh|lvh|dvw|ch)\b")
DYNDIM = re.compile(r"\}(px|rem|em|vh|vw|dvh)\b")  # rust format!("{x}px")
DURPROP = re.compile(r"(?:transition|animation)[\w-]*\s*:\s*([^;}\"\n]*)", re.I)
DUR = re.compile(r"(?<![\w.])(?:\d+\.?\d*|\.\d+)(ms|s)\b")
DEF = re.compile(r"(--[\w-]+)\s*:\s*([^;}\n]*)")
VARUSE = re.compile(r"var\(\s*--[\w-]+")
KEYF = re.compile(r"@keyframes\s+[\w-]+")
STYLEATTR = re.compile(r"style\s*=\s*\\?[\"']")
STRLIT = re.compile(r'r#+"(?:.|\n)*?"#+|"(?:[^"\\\n]|\\.)*"')
TIMEOUT = re.compile(r"\b(setTimeout|after_ms|set_timeout_with_callback_and_timeout_and_arguments_0)\b")

def strip_rust_comments(s):
    return re.sub(r"(?m)^\s*//.*$", "", s)

def segments(f, text):
    """yield (label, text) chunks."""
    if f.endswith(".rs"):
        lits = STRLIT.findall(strip_rust_comments(text))
        yield f, "\n".join(lits)
    elif f.endswith(".html"):
        styles = re.findall(r"<style[^>]*>(.*?)</style>", text, re.S)
        scripts = re.findall(r"<script[^>]*>(.*?)</script>", text, re.S)
        markup = re.sub(r"<style[^>]*>.*?</style>|<script[^>]*>.*?</script>", "", text, flags=re.S)
        if styles: yield f + " [<style>]", "\n".join(styles)
        if scripts: yield f + " [<script>]", "\n".join(scripts)
        yield f + " [markup]", markup
    else:
        yield f, text

def count(label, t):
    r = collections.Counter()
    # colors
    hexes = HEX.findall(t); r["hex"] = len(hexes)
    r["colorfn"] = len(FUNC.findall(t))
    named = 0
    for m in COLORPROP.finditer(t):
        named += len(NAMEDRE.findall(m.group(1)))
    r["named"] = named
    r["dims"] = len(DIM.findall(t)) + len(DYNDIM.findall(t))
    r["dims_dynamic"] = len(DYNDIM.findall(t))
    durs = 0
    for m in DURPROP.finditer(t):
        durs += len(DUR.findall(m.group(1)))
    r["durations"] = durs
    # custom property defs
    dc = dd = dn = 0
    for m in DEF.finditer(t):
        v = m.group(2)
        if label.endswith("[markup]") and "style" not in t: pass
        r["var_defs"] += 1
        dc += len(HEX.findall(v)) + len(FUNC.findall(v))
        dd += len(DIM.findall(v)); dn += len(DUR.findall(v))
    r["colors_in_defs"] = dc; r["dims_in_defs"] = dd; r["durs_in_defs"] = dn
    r["var_uses"] = len(VARUSE.findall(t))
    r["keyframes"] = len(KEYF.findall(t))
    r["style_attrs"] = len(STYLEATTR.findall(t))
    r["timeouts"] = len(TIMEOUT.findall(t))
    r["colors"] = r["hex"] + r["colorfn"] + r["named"]
    r["total"] = r["colors"] + r["dims"] + r["durations"]
    return r

rows = []
for f in sel:
    p = repo / f
    try: text = p.read_text(errors="replace")
    except Exception: continue
    if f.endswith(".rs"):
        # timeouts are code, count on full text
        pass
    for label, t in segments(f, text):
        c = count(label, t)
        if f.endswith(".rs"):
            c["timeouts"] = len(TIMEOUT.findall(text))
        if c["total"] or c["var_uses"] or c["var_defs"]:
            rows.append((label, c))

rows.sort(key=lambda x: -x[1]["total"])
cols = ["total", "colors", "hex", "colorfn", "named", "dims", "dims_dynamic", "durations", "var_defs",
        "colors_in_defs", "dims_in_defs", "durs_in_defs", "var_uses", "keyframes", "style_attrs", "timeouts"]
with open(out, "w", newline="") as fh:
    w = csv.writer(fh); w.writerow(["file"] + cols)
    tot = collections.Counter()
    for label, c in rows:
        w.writerow([label] + [c[k] for k in cols]); tot.update(c)
    w.writerow(["TOTAL"] + [tot[k] for k in cols])
print("files scanned:", len(sel), "rows with hits:", len(rows))
print("| file | total | colors | dims | durations | --defs | var() uses | style= attrs |")
print("|---|---|---|---|---|---|---|---|")
for label, c in rows[:30]:
    print(f"| {label} | {c['total']} | {c['colors']} | {c['dims']} | {c['durations']} | {c['var_defs']} | {c['var_uses']} | {c['style_attrs']} |")
print(f"| TOTAL ({len(rows)} rows) | {tot['total']} | {tot['colors']} | {tot['dims']} | {tot['durations']} | {tot['var_defs']} | {tot['var_uses']} | {tot['style_attrs']} |")
print({k: tot[k] for k in cols})
