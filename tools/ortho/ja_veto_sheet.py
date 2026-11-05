#!/usr/bin/env python3
"""Builds reports/ja-purity-veto-sheet.csv: every row the 2026-11-04 ja PURITY batch quarantined (to veto) plus the proposed second batch
(removal candidates, fragment siblings, character/brand names). Blank `verdict` column: keep_cut / restore."""
import csv, gzip, os, sys, xml.etree.ElementTree as ET
REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
rebs = {}
with gzip.open(os.path.expanduser("~/repos/spell-wasm/.corpus-cache/ja/JMdict_e.gz"), "rb") as f:
    for _, el in ET.iterparse(f):
        if el.tag == "entry":
            g = next((x.text for x in el.iter("gloss")), "")
            for r in el.iter("reb"): rebs.setdefault(r.text, g)
            el.clear()
done = [r for r in csv.DictReader(open(f"{REPO}/assets/words-quarantine/manifest.csv", encoding="utf-8")) if r["lang"] == "ja" and "CC-ORTHO-SWEEP-DE-JA v1.1" in r["evidence"] and r["class"] != "accepted_variant"]
SIB = "ぎかわさき ぎけんどう ぎじぞうで ぎたゆうそうし ぎはふりと ぎじもんだい ぎゅうといい ぎたかひろ ざいしりつ ざいじょうほう ざつぎょ ざとちょう ざじゅうおり ざなみのみこと ざんじんじゃ ざんてつどう づがわしりつ づみみずはら づのおかといい づらく ぺきのりせい びさひもまさみ".split()
CHAR = "どらえもん うるとらまん さざえさん ぷくちゃん ざぐるまん とんねるず にほんてれび てれびあさひ にってれ びるらじお ぱみゅぱみゅの ぱれっとの ぷよはすべて びのにゃんこ ぷせとねでぶ のぎざか".split()
REM = "いっつも すっごい でっかい いやぁ てぇ".split()
bank = {}
for t in "easy medium hard expert".split():
    for w in open(f"{REPO}/assets/words/ja/{t}.txt", encoding="utf-8").read().split(): bank[w] = t
rows = []
for r in done: rows.append(("batch1 (already cut)", r["word"], r["tier"], r["class"], rebs.get(r["word"], "")))
for grp, ws, k in (("batch2: removal candidate", REM, "misspelled_nonstandard"), ("batch2: fragment sibling", SIB, "fragment"), ("batch2: character/brand name", CHAR, "proper_noun")):
    for w in ws:
        if w in bank: rows.append((grp, w, bank[w], k, rebs.get(w, "")))
        else: rows.append((grp, w, "(not in bank)", k, rebs.get(w, "")))
with open(f"{REPO}/reports/ja-purity-veto-sheet.csv", "w", newline="", encoding="utf-8") as f:
    w = csv.writer(f, lineterminator="\n"); w.writerow("group,word,tier,proposed_class,jmdict_gloss,verdict".split(","))
    for r in rows: w.writerow([*r, ""])
import collections; print(collections.Counter(r[0] for r in rows)); print([r[1] for r in rows if r[2] == "(not in bank)"])
