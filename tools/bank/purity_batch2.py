#!/usr/bin/env python3
"""CC-BANK-PURITY F3 second batch — the C7 proper-noun rows, applied only from a signed review.

    python3 tools/bank/purity_batch2.py export [--out audit] [lang ...]
    python3 tools/bank/purity_batch2.py apply <review.csv> --signed-by "Name" [--date YYYY-MM-DD] [--dry-run]

The Wiktionary part-of-speech filter (tools/bank/census_c7.py -> reports/bank-purity-c7.csv) flags real common
words (hi सूरज, ja たこ, ar الأرض) as well as names, so nothing here is mechanical: every row needs a person's
`cut`, `keep` or `unsure`.

calendar  the policy step: months and weekdays on a fixed per-language list are recorded as `keep`.
export  writes <out>/<lang>/c7-review.csv: one row per C7 candidate still in the bank and not already ruled
        `keep`, with a blank `decision` column.  It changes nothing in the repo.
apply   takes one language's returned review:
          cut    -> the row moves to assets/words-quarantine/<lang>/<tier>.txt (manifest class
                    `proper_noun_pos`, evidence names the signer), its gloss and definition-pool rows are pruned,
                    its ledger line goes.  Same machinery as F8's `cut` (purity_sheet.apply_cuts), so the
                    quarantine `restore` round-trip holds.
          keep   -> recorded in assets/words/purity/c7-kept/<lang>.txt so a re-export skips it
          unsure -> nothing changes; it comes back on the next export
        A blank decision is an error.  A row that is not a C7 candidate is an error: this tool cannot be used to
        cut arbitrary rows (that is F8).  Against config/bank_floors.json poolFloors: a cut that takes a tier of a
        `launched` language below its floor is a HALT and nothing is written.  For a language that is not launched
        the floors are report-only (most banks are below them already), so the before/after is printed with the gap
        and the pool gates in scripts/gate.sh are the check.  No threshold is ever edited.
--dry-run  prints the per-tier before/after and writes nothing.
"""
import argparse, csv, os, sys, json, unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)
import purity_gate as g
import purity_sheet as ps

nfc = lambda s: unicodedata.normalize("NFC", s)
CLASS = "proper_noun_pos"
COLS = "lang,tier,word,current_class,decision,note".split(",")
DECISIONS = ("cut", "keep", "unsure")


# C7 policy (Eric, 2026-10-12): months and weekdays are ordinary words and stay in the banks.  A fixed list, so
# the policy is a lookup and not a judgement; anything not on it still needs a person's decision.
CALENDAR = {
    "ko": "월요일 화요일 수요일 목요일 금요일 토요일 일요일 일월 이월 삼월 사월 오월 유월 칠월 팔월 구월 시월 십일월 십이월",
    "ja": "げつようび かようび すいようび もくようび きんようび どようび にちようび いちがつ にがつ さんがつ しがつ ごがつ ろくがつ しちがつ はちがつ くがつ じゅうがつ じゅういちがつ じゅうにがつ",
    "ar": "يناير فبراير مارس أبريل إبريل ابريل مايو يونيو يوليو أغسطس سبتمبر أكتوبر نوفمبر ديسمبر الاثنين الإثنين الثلاثاء الأربعاء الخميس الجمعة السبت الأحد",
    "hi": "जनवरी फ़रवरी फरवरी मार्च अप्रैल मई जून जुलाई अगस्त सितंबर सितम्बर अक्टूबर नवंबर नवम्बर दिसंबर दिसम्बर सोमवार मंगलवार बुधवार गुरुवार बृहस्पतिवार शुक्रवार शनिवार रविवार इतवार",
    "sw": "januari februari machi aprili mei juni julai agosti septemba oktoba novemba desemba jumatatu jumanne jumatano alhamisi ijumaa jumamosi jumapili",
    "ru": "январь февраль март апрель май июнь июль август сентябрь октябрь ноябрь декабрь понедельник вторник среда четверг пятница суббота воскресенье",
}


def calendar(signer, date, dry):
    """Record `keep` for every open C7 candidate that is a month or weekday on CALENDAR."""
    for lang in sorted(CALENDAR):
        cal = {nfc(w) for w in CALENDAR[lang].split()}
        hit = sorted(w for w in candidates(lang) if w in cal)
        if not hit:
            continue
        print(f"{lang}: {len(hit)} month/weekday rows stay in: {' '.join(hit)}")
        if dry:
            continue
        p = kept_path(lang)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        have = sorted(kept(lang) | set(hit))
        open(p, "w", encoding="utf-8").write("# C7 rows ruled to stay in the bank (a word of the language, not a name)\n"
                                              + "\n".join(have) + "\n")


def kept_path(lang):
    return f"{REPO}/assets/words/purity/c7-kept/{lang}.txt"


def kept(lang):
    p = kept_path(lang)
    if not os.path.exists(p):
        return set()
    return {nfc(l.strip()) for l in open(p, encoding="utf-8") if l.strip() and l[0] != "#"}


def candidates(lang):
    """(tier, word) -> current_class for C7 rows of `lang` that are still bank rows and not ruled `keep`."""
    inbank = {nfc(w): t for t, w in g.bank(REPO, lang)}
    k = kept(lang)
    out = {}
    for r in csv.DictReader(open(f"{REPO}/reports/bank-purity-c7.csv", encoding="utf-8")):
        w = nfc(r["word"])
        if r["lang"] == lang and w in inbank and w not in k:
            out[w] = (inbank[w], r["current_class"])
    return out


def export(out, langs):
    for lang in langs:
        c = candidates(lang)
        if not c:
            continue
        d = f"{out}/{lang}"
        os.makedirs(d, exist_ok=True)
        with open(f"{d}/c7-review.csv", "w", newline="", encoding="utf-8") as f:
            w = csv.writer(f, lineterminator="\n"); w.writerow(COLS)
            for word, (tier, cur) in c.items():
                w.writerow((lang, tier, word, cur, "", ""))
        print(f"{lang}: {len(c)} candidates -> {d}/c7-review.csv")


def floors(lang):
    """({tier: poolFloor}, launched) from config/bank_floors.json."""
    try:
        row = json.load(open(f"{REPO}/config/bank_floors.json", encoding="utf-8"))["languages"][lang]
    except (KeyError, FileNotFoundError):
        return {}, False
    j = row["poolFloors"]
    return {t: j[i] for i, t in enumerate(g.TIERS) if i < len(j)}, bool(row.get("launched"))


def apply(path, signer, date, dry):
    rows = list(csv.DictReader(open(path, encoding="utf-8")))
    if not rows:
        sys.exit("empty review")
    langs = {r["lang"] for r in rows}
    if len(langs) != 1:
        sys.exit(f"one language per review, got {sorted(langs)}")
    lang = langs.pop()
    blank = [r["word"] for r in rows if r["decision"].strip().lower() not in DECISIONS]
    if blank:
        sys.exit(f"{len(blank)} row(s) have no cut/keep/unsure decision (first: {blank[:5]}); refusing a half-answered review")
    cand = candidates(lang)
    decision = {nfc(r["word"]): r["decision"].strip().lower() for r in rows}
    stray = [w for w in decision if w not in cand]
    if stray:
        sys.exit(f"{len(stray)} row(s) are not C7 candidates still in the bank (first: {stray[:5]}); "
                 f"use F8's sheet for anything else")
    cuts = {w for w, d in decision.items() if d == "cut"}
    keeps = {w for w, d in decision.items() if d == "keep"}
    # pool floors: HALT, never edit a threshold
    before = {}
    for t, w in g.bank(REPO, lang):
        before[t] = before.get(t, 0) + 1
    removed = {}
    for w in cuts:
        removed[cand[w][0]] = removed.get(cand[w][0], 0) + 1
    fl, launched = floors(lang)
    halt = []
    print(f"{lang}: {len(cuts)} cut, {len(keeps)} keep, {sum(1 for d in decision.values() if d == 'unsure')} unsure")
    for t in g.TIERS:
        if t in before or t in removed:
            b, a = before.get(t, 0), before.get(t, 0) - removed.get(t, 0)
            fl_t = fl.get(t)
            note = ""
            if fl_t is not None and removed.get(t) and a < fl_t:
                if launched:
                    halt.append(f"{lang}/{t}: {b} -> {a}, below poolFloor {fl_t}")
                    note = f"  HALT (poolFloor {fl_t})"
                else:
                    note = f"  below poolFloor {fl_t} (report-only, {lang} not launched)"
            print(f"  {t:7s} {b:6d} -> {a:6d}{note}")
    if halt:
        sys.exit("HALT, nothing written: " + "; ".join(halt))
    if dry:
        print("dry run: nothing written")
        return
    moved = ps.apply_cuts(lang, cuts, CLASS, f"wiktionary name-only; {signer} {date}", date)
    if keeps:
        p = kept_path(lang)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        have = sorted(kept(lang) | keeps)
        open(p, "w", encoding="utf-8").write("# C7 rows a native reader ruled are words of the language (not names)\n"
                                              + "\n".join(have) + "\n")
    ps.rebuild_and_rehash()
    print(f"{lang}: {len(moved)} moved to quarantine, {len(keeps)} recorded as kept")
    print("next: python3 tools/bank/purity_gate.py && python3 tools/bank/ledger_check.py && bash scripts/gate.sh")


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    e = sub.add_parser("export"); e.add_argument("--out", default="audit"); e.add_argument("langs", nargs="*")
    c = sub.add_parser("calendar"); c.add_argument("--signed-by", default="Eric"); c.add_argument("--date", default="2026-10-12")
    c.add_argument("--dry-run", action="store_true")
    a = sub.add_parser("apply"); a.add_argument("review"); a.add_argument("--signed-by", required=True)
    a.add_argument("--date", default="2026-10-11"); a.add_argument("--dry-run", action="store_true")
    x = ap.parse_args()
    if x.cmd == "export":
        export(x.out, x.langs or "ko ja ar hi sw ru".split())
    elif x.cmd == "calendar":
        calendar(x.signed_by, x.date, x.dry_run)
    else:
        apply(x.review, x.signed_by, x.date, x.dry_run)
