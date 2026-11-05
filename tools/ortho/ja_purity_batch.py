#!/usr/bin/env python3
"""CC-ORTHO-SWEEP-DE-JA v1.1 follow-up (Eric 2026-11-04): PURITY on the ja bank before the near-sound / devoicing pair files.

Quarantines (never deletes) fragments, places and personal names, and the slur list, then moves two spelling variants onto
data/ja/accepted-variants.tsv. Run from the repo root; then rebuild_and_rehash() runs here, and the caller regenerates the
sound-alike tables."""
import csv, os, sys
HERE = os.path.dirname(os.path.abspath(__file__)); REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, f"{REPO}/tools/bank")
import purity_sheet as ps

DATE = "2026-11-04"; WHO = "Eric 2026-11-04"; TAG = "CC-ORTHO-SWEEP-DE-JA v1.1: "
STEMS = "おいし くるし むずかし やさし すばらし かわいらし いとし ひとし うらやまし うっとうし".split()
FRAG = ("っぱなし くっつい すすま すくなから ぐつたくもあり ざきけんりつ ぎふしりつ ぎふけんどう ざわいして").split()
SLUR = "きちがい ばばあ しこしこ ちつ".split()
PLACES = ("あきはばら あさくさ あさひかわ あまくさ いまばり おだわら くまがや こうりやま さがみはら しながわ しんじゅく たからづか たまがわ "
          "つくば ひたちなか ひがしあじあ ねりま みなとみらい みなみあふりか よこすか まつど ぬまづ にしのみや えどがわ きりしま しもきたざわ ふしみ "
          "ひだか とよしま きたかみ ふくやま ふくま みうら みしま おばま おぐら いつくしま あきば").split()
NAMES = ("あべ あきやま ありむら ありよし いえやす うえすぎ うえはら うたがわ おおくぼ おおにし おがさわら かどかわ きたむら きのした きむ くろさき "
         "こいずみ さかもと さくらい すがわら たかし たきざわ たけしま たけなか だいすけ つかもと なかはら ながおか にしむら はとやま はまさき はんざわ "
         "ひできち ふじさわ ふじわら まえかわ まつした まつしま むらかみ やまざき よしかわ よしもと あきもと あきら あさの あそう いけだ いしだ いわさき "
         "うえだ うちだ うの うめだ おかだ おざわ かざま かすが かねこ きよし きょうごく くろだ こいけ ささき さとみ さかた さがわ しのだ しらい "
         "しんいちろう しんたろう しんのすけ じゅんいち すぎた すずか ぜるだ たかだ たぐち だいき つとむ つよし てぃむ なおと なかの にしだ にしの はせがわ "
         "はねだ はるき まえだ まつい まりお まるこ みずの みほ みやた むらた もにか やすし やすだ よしの りさ れな るいす ゆい さの ひの "
         "あんとにお えりざべす かるろす きゃさりん じぇしか じょんそん ふぃりっぷ ふらんしす らいあん うぃるそん みしぇる ぱとりっく そろもん あんでぃ あれっくす くりす じぇふ えゔぁ").split()
VARIANTS = [("めんどくさい", "めんどうくさい"), ("かっこいい", "かっこういい")]   # stored, dropped from the bank

def main():
    moved = set()
    for ws, k, why in ((STEMS, "fragment", "adjective stem missing its final い"), (FRAG, "fragment", "fragment of a longer string"),
                       (SLUR, "profanity", "slur / vulgar, not for a children's bank"),
                       (PLACES, "proper_noun", "place name"), (NAMES, "proper_noun", "personal name")):
        moved |= ps.apply_cuts("ja", set(ws), k, TAG + why + "; " + WHO, DATE)
    drop = {b for _, b in VARIANTS}
    moved |= ps.apply_cuts("ja", drop, "accepted_variant", TAG + "now an accepted variant of the stored form; " + WHO, DATE)
    with open(f"{REPO}/tools/bank/purity_overrides.csv", "a", newline="", encoding="utf-8") as f:
        w = csv.writer(f, lineterminator="\n")
        for ws, k, why in ((STEMS, "fragment", "adjective stem missing its final い"), (FRAG, "fragment", "fragment of a longer string"),
                           (SLUR, "profanity", "slur / vulgar"), (PLACES, "proper_noun", "place name"), (NAMES, "proper_noun", "personal name")):
            for x in ws:
                if x in moved: w.writerow(["ja", x, k, WHO, TAG + why])
    ps.rebuild_and_rehash()
    print(len(moved), "ja rows quarantined")
    print("not in bank (skipped):", sorted((set(STEMS + FRAG + SLUR + PLACES + NAMES) | drop) - moved))

if __name__ == "__main__": main()
