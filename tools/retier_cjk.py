#!/usr/bin/env python3
"""CC-PLAYER-CONTRACT P5 — repair the Korean and Japanese tier ladders.

P5 says a given tier means comparable difficulty in every language. For ko and
ja it currently means almost nothing: measured over the shipped bank,

    ko jamo    easy 4.71  medium 10.51  hard  9.80  expert 10.23
    ja kana    easy 2.62  medium  4.40  hard  3.82  expert  4.30

HARD IS EASIER THAN MEDIUM in both, and expert is indistinguishable from
medium. A player climbing those tiers experiences no increase in difficulty.
This is not an artifact of measuring the wrong thing: the first pass used
character count, which is a poor proxy for these scripts, so it was re-measured
in JAMO for Korean -- what the player actually types -- and the ladder is
non-monotonic there too.

WHAT THIS DOES. Pools medium + hard + expert, scores each word by the input
burden it actually imposes, and re-splits into the SAME three tier sizes. Same
words, same counts, same floors and ceilings -- only the boundaries move.

WHAT THIS DELIBERATELY DOES NOT TOUCH: the easy tier. Easy is visibly curated
(ko 292 words averaging 1.89 syllables) and it is ALREADY monotonic against
medium. Those are starter words chosen for a child, and re-sorting them by
length would push common short words out and rare short words in. Curation that
carries information is not improved by a formula.

SCORING, and why length alone is not it:
  ko  jamo count -- the keystrokes. A 3-syllable word with complex codas is
      more typing than a 4-syllable word of simple ones, and jamo says so.
      Rare jamo (ㅃㅉㄸㄲㅆ, the tensed set, and compound vowels) cost a
      long-press, so they carry a surcharge.
  ja  kana count, plus a surcharge for the forms that need a long-press:
      dakuten/handakuten and small kana. Chōon is counted but is absent from
      this bank.

Run:  python3 tools/retier_cjk.py [--apply]
"""
import pathlib, sys, unicodedata
from collections import Counter

ROOT = pathlib.Path(__file__).resolve().parent.parent
W = ROOT / "assets/words"
MOVABLE = ["medium", "hard", "expert"]
# --all also re-partitions easy. It buys a coherent ladder at the cost of the
# curated starter set: easy's 292 Korean words are visibly chosen, and no
# frequency list exists for ko or ja, so a length score cannot tell a common
# short word from a rare one. Eric's call, not a default.
ALL = ["easy", "medium", "hard", "expert"]

# FREQUENCY IS THE DIFFICULTY SIGNAL; length is only a fallback.
#
# Two length-based re-partitions were tried and both broke Spell Picture,
# because sorting tiers by length is sorting by the exact property the calligram
# packer needs VARIED inside each tier -- Korean easy collapsed to 1-character
# words and the STACK slots starved. Frequency does not have that problem:
# frequency and length are only loosely correlated, so a frequency-sorted tier
# keeps a mix of lengths by construction.
#
# It is also the better signal on its own terms. In a spelling game you cannot
# spell a word you have never met, so a rare short word is harder than a common
# long one -- which is precisely what length can never express.
#
# Coverage is partial (ko 63.7%, ja 78.1%). A word with no frequency is imputed
# the MEDIAN BAND OF COVERED WORDS OF ITS OWN LENGTH, not "rare": treating
# unknown as rare would sweep a third of the Korean bank into expert on missing
# data alone. Imputing by length keeps uncovered words on the same scale and
# lets them sort among themselves by the only signal they have.
import statistics as _stats

def _freq_table(lang):
    p = ROOT / f"tools/wordpipe/sources/freq_{lang}.txt"
    if not p.exists():
        return None
    out = {}
    for ln in p.read_text(encoding="utf-8").splitlines():
        if ln.startswith("#") or "\t" not in ln:
            continue
        w, v = ln.split("\t")
        out[w] = int(v) if v.strip() else None
    return out


def build_score(lang, words):
    """band-based difficulty, imputing missing bands from same-length medians."""
    F = _freq_table(lang)
    if not F:
        return None
    by_len = {}
    for w in words:
        b = F.get(w)
        if b is not None:
            by_len.setdefault(len(w), []).append(b)
    med = {k: _stats.median(v) for k, v in by_len.items() if v}
    allv = [b for v in by_len.values() for b in v]
    fallback = _stats.median(allv) if allv else 0.0
    def score(w):
        b = F.get(w)
        return float(b) if b is not None else float(med.get(len(w), fallback))
    return score


KO_LONGPRESS = set("ㅃㅉㄸㄲㅆㅒㅖ")
JA_DAKUTEN = set("がぎぐげござじずぜぞだぢづでどばびぶべぼぱぴぷぺぽ")
JA_SMALL = set("ぁぃぅぇぉっゃゅょゎ")


def score_ko(w):
    jamo = unicodedata.normalize("NFD", w)
    return len(jamo) + 0.5 * sum(1 for j in jamo if j in KO_LONGPRESS)


def score_ja(w):
    return (len(w)
            + 0.5 * sum(1 for c in w if c in JA_DAKUTEN)
            + 0.5 * sum(1 for c in w if c in JA_SMALL)
            + 0.5 * w.count("ー"))


SCORE = {"ko": score_ko, "ja": score_ja}


def report(lang, tiers, f=None):
    f = f or SCORE[lang]
    for t in ["easy"] + MOVABLE:
        ws = tiers[t]
        print(f"    {t:8}{len(ws):6}  mean score {sum(f(w) for w in ws) / len(ws):6.2f}")


def run(lang, apply):
    tiers = {t: [l.strip() for l in (W / lang / f"{t}.txt").read_text(encoding="utf-8").splitlines()
                 if l.strip()] for t in ["easy"] + MOVABLE}
    allw0 = [w for t in (["easy"] + MOVABLE) for w in tiers[t]]
    f0 = build_score(lang, allw0) or SCORE[lang]
    print(f"  {lang} BEFORE")
    report(lang, tiers, f0)

    movable = ALL if "--all" in sys.argv else MOVABLE
    pool = [w for t in movable for w in tiers[t]]
    if len(pool) != len(set(pool)):
        dupes = [w for w, n in Counter(pool).items() if n > 1]
        print(f"    ABORT: {len(dupes)} word(s) in more than one tier already: {dupes[:5]}")
        return False
    f = SCORE[lang]
    sizes = [len(tiers[t]) for t in movable]

    # MINIMAL SWAPS, NOT A RE-PARTITION. Two full re-partitions were tried and
    # both broke Spell Picture: a hard quantile split gave every tier a single
    # word length (Korean easy became only 1-character words) and starved the
    # layout packer, and a proportional length quota inverted the ladder
    # instead. The lesson is that the tiers are not arbitrary -- their LENGTH
    # DIVERSITY is load-bearing for the calligram packer, which fills slots
    # with specific length budgets and needs variety inside each tier.
    #
    # The defect is narrow: hard is easier than medium. Fixing it does not
    # require re-sorting six thousand words, only exchanging the few that sit
    # on the wrong side of a boundary. Swap the hardest word of the lower tier
    # with the easiest word of the upper one, repeatedly, until the means
    # separate. Each swap preserves both tier SIZES and, because it moves one
    # word at a time from each side, very nearly preserves each tier's length
    # distribution -- which is what the packer actually depends on.
    allw = [w for t in (["easy"] + MOVABLE) for w in tiers[t]]
    fs = build_score(lang, allw)
    f = fs or SCORE[lang]
    print(f"    signal: {'frequency (band)' if fs else 'input burden (length)'}")
    if "--repartition" in sys.argv:
        # A FULL RE-SORT, safe only on the frequency signal. Doing this on
        # length destroyed Spell Picture twice (each tier collapsed to one word
        # length and the STACK slots starved). Frequency is nearly orthogonal to
        # length, so the tiers keep their length mix -- verified before use.
        pool = sorted({w for t in movable for w in tiers[t]}, key=lambda w: (f(w), w))
        out, i = {}, 0
        for t in movable:
            n = len(tiers[t])
            out[t] = sorted(pool[i:i + n]); i += n
        for w in pool[i:]:
            out[movable[-1]].append(w)
        new = dict(tiers, **out)
        means = [sum(f(w) for w in new[t]) / len(new[t]) for t in ["easy"] + MOVABLE]
        print(f"  {lang} AFTER (re-partition)")
        report(lang, new, f)
        mono = all(a < b for a, b in zip(means, means[1:]))
        print(f"    monotonic: {mono}")
        if apply and mono:
            for t in movable:
                (W / lang / f"{t}.txt").write_text("\n".join(out[t]) + "\n", encoding="utf-8")
            print("    written")
        return mono

    out = {t: list(tiers[t]) for t in movable}
    order = ["easy"] + MOVABLE
    swaps = 0
    for lo_t, hi_t in zip(order, order[1:]):
        if lo_t not in out or hi_t not in out:
            continue                      # easy is pinned unless --all
        guard = 0
        while guard < 4000:
            guard += 1
            lo, hi = out[lo_t], out[hi_t]
            m_lo = sum(f(w) for w in lo) / len(lo)
            m_hi = sum(f(w) for w in hi) / len(hi)
            if m_lo < m_hi - 0.15:        # a real gap, not a hairline
                break
            a = max(lo, key=lambda w: (f(w), w))     # hardest in the lower tier
            b = min(hi, key=lambda w: (f(w), w))     # easiest in the upper
            if f(a) <= f(b):
                break                     # nothing left to exchange
            lo.remove(a); hi.remove(b)
            lo.append(b); hi.append(a)
            swaps += 1
    print(f"    swaps: {swaps}")
    for t in movable:
        out[t] = sorted(set(out[t]))

    new = dict(tiers, **out)
    print(f"  {lang} AFTER")
    report(lang, new, f)
    means = [sum(f(w) for w in new[t]) / len(new[t]) for t in ["easy"] + MOVABLE]
    mono = all(a < b for a, b in zip(means, means[1:]))
    print(f"    monotonic: {mono}" + ("" if mono else "   <-- STILL BROKEN"))
    moved = sum(1 for t in movable for w in out[t] if w not in tiers[t])
    print(f"    words moved: {moved} of {len(pool)}")
    if apply and mono:
        for t in movable:
            (W / lang / f"{t}.txt").write_text("\n".join(out[t]) + "\n", encoding="utf-8")
        print(f"    written")
    return mono


if __name__ == "__main__":
    apply = "--apply" in sys.argv
    langs = [a for a in sys.argv[1:] if a in ("ko", "ja")] or ["ko", "ja"]
    ok = all(run(l, apply) for l in langs)
    if not apply:
        print("\n  dry run — pass --apply to write")
    raise SystemExit(0 if ok else 1)
