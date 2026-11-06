#!/usr/bin/env python3
"""CC-ORTHO-SWEEP-ALL F0.1: validate every data/<lang>/ortho-profile.json against schemas/ortho-profile.schema.json.

jsonschema is not installed here, so this checks the subset the schema uses: required keys, no extra keys, enum, type.
It also checks what a schema cannot: the variants file exists, the rule ids it names exist in the rules files, the
voice id is the one backend/app.py LANG_VOICES serves, and no stand-in has an empty pattern.
"""
import glob, json, os, re, sys
REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
TYPES = {"string": str, "array": list, "object": dict, "boolean": bool, "integer": int}

def validate(v, sch, path, errs):
    t = sch.get("type")
    if t and not isinstance(v, TYPES[t]): errs.append(f"{path}: expected {t}"); return
    if "enum" in sch and v not in sch["enum"]: errs.append(f"{path}: {v!r} not in enum")
    if t == "object":
        props = sch.get("properties", {})
        for k in sch.get("required", []):
            if k not in v: errs.append(f"{path}: missing {k}")
        if sch.get("additionalProperties") is False:
            for k in v:
                if k not in props: errs.append(f"{path}: unexpected {k}")
        for k, s in props.items():
            if k in v: validate(v[k], s, f"{path}.{k}", errs)
    if t == "array" and "items" in sch:
        for i, x in enumerate(v): validate(x, sch["items"], f"{path}[{i}]", errs)

def main():
    sch = json.load(open(f"{REPO}/schemas/ortho-profile.schema.json", encoding="utf-8"))
    app = open(f"{REPO}/backend/app.py", encoding="utf-8").read()
    errs, n = [], 0
    for p in sorted(glob.glob(f"{REPO}/data/*/ortho-profile.json")):
        n += 1; lang = os.path.basename(os.path.dirname(p)); d = json.load(open(p, encoding="utf-8"))
        validate(d, sch, lang, errs)
        if d.get("lang") != lang: errs.append(f"{lang}: lang field is {d.get('lang')!r}")
        if not os.path.exists(f"{REPO}/{d.get('variants_file', '')}"): errs.append(f"{lang}: variants_file missing")
        for si in d.get("stand_ins", []):
            if not si.get("pattern"): errs.append(f"{lang}: empty stand-in pattern")
        vid = d.get("voice", {}).get("voice_id", "")
        if not re.search(r'"%s"\s*:[^\n]*%s' % (lang, re.escape(vid)), app): errs.append(f"{lang}: voice {vid} is not what backend/app.py serves")
        ids = {r["id"] for r in json.load(open(f"{REPO}/data/{lang}/sound-alike-rules.json", encoding="utf-8"))["rules"]}
        for r in d.get("sound_alike", {}).get("rule_ids", []):
            if r not in ids: errs.append(f"{lang}: sound_alike rule {r} not in sound-alike-rules.json")
        nids = {r["id"] for r in json.load(open(f"{REPO}/data/{lang}/near-sound-rules.json", encoding="utf-8"))["rules"]}
        for r in d.get("near_sound_rule_ids", []):
            if r not in nids: errs.append(f"{lang}: near-sound rule {r} not in near-sound-rules.json")
    for e in errs: print("PROFILE FAIL:", e)
    print(f"profile_check: {n} profile(s), {len(errs)} problem(s)")
    sys.exit(1 if errs else 0)
main()
