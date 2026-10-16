#!/usr/bin/env python3
"""F2 -- English difficulty score en_difficulty in [0,1].

  en_difficulty = w_irr * irregularity + w_rar * rarity + w_len * length

irregularity: how unpredictable the word's spelling is given its sound. Letters are
aligned to CMUdict phonemes by hard-EM (each phoneme emits 0..3 letters; a NULL
unit absorbs silent letters). The model is P(grapheme | phoneme) over a general
English training set (CMUdict words with Zipf >= train_zipf_min), i.e. the
conditional entropy of spelling given sound. A word's raw irregularity is the mean
surprisal of its aligned units plus a share of its worst unit; the reported term is
that raw value's percentile among all scored words.
rarity: linear in Zipf over 2.5..6.0 (lower Zipf = harder).
length: min(len, cap) / cap.

Deterministic: no randomness, sorted iteration. Run with .corpus-cache/wf/v/bin/python.
"""
import json, math, os, sys
from collections import defaultdict
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from common import *

CFG = json.load(open(os.path.join(ROOT, "config/en_difficulty.json")))
AL = CFG["aligner"]
NULL = "<NULL>"


def align(word, phs, cost_fn, max_g, max_n):
    """Min-cost alignment. Returns list of (unit_phoneme, grapheme)."""
    n, m = len(word), len(phs)
    INF = float("inf")
    best = [[INF] * (m + 1) for _ in range(n + 1)]
    back = [[None] * (m + 1) for _ in range(n + 1)]
    best[0][0] = 0.0
    for i in range(n + 1):
        for j in range(m + 1):
            c0 = best[i][j]
            if c0 == INF:
                continue
            if j < m:
                for k in range(0, max_g + 1):
                    if i + k > n:
                        break
                    c = c0 + cost_fn(phs[j], word[i:i + k])
                    if c < best[i + k][j + 1]:
                        best[i + k][j + 1] = c
                        back[i + k][j + 1] = (i, j, phs[j], word[i:i + k])
            for k in range(1, max_n + 1):
                if i + k > n:
                    break
                c = c0 + cost_fn(NULL, word[i:i + k])
                if c < best[i + k][j]:
                    best[i + k][j] = c
                    back[i + k][j] = (i, j, NULL, word[i:i + k])
    if best[n][m] == INF:
        return None
    out, i, j = [], n, m
    while (i, j) != (0, 0):
        pi, pj, ph, g = back[i][j]
        out.append((ph, g))
        i, j = pi, pj
    return out[::-1]


class Model:
    def __init__(self):
        self.counts = defaultdict(lambda: defaultdict(float))
        self.tot = defaultdict(float)

    def entropy(self, ph):
        """H(G | ph) in bits: how ambiguous the spelling of this sound is."""
        c = self.counts.get(ph)
        tot = self.tot.get(ph, 0.0)
        if not c or tot == 0:
            return 0.0
        return -sum((v / tot) * math.log2(v / tot) for v in c.values())

    def cost(self, ph, g):
        c = self.counts.get(ph)
        tot = self.tot.get(ph, 0.0)
        p = ((c.get(g, 0.0) if c else 0.0) + 0.1) / (tot + 20.0)
        bits = -math.log2(p)
        if ph == NULL:
            bits += AL["null_penalty_bits"]
        return bits


def train(words_phs):
    model = Model()
    prior = lambda ph, g: (1.2 * len(g) + (AL["null_penalty_bits"] + 2.0 if ph == NULL else 0.0) + 1.0)
    cost_fn = prior
    for it in range(AL["iterations"]):
        new = Model()
        for w, phs in words_phs:
            a = align(w, phs, cost_fn, AL["max_grapheme"], AL["max_null"])
            if not a:
                continue
            for ph, g in a:
                new.counts[ph][g] += 1.0
                new.tot[ph] += 1.0
        model = new
        cost_fn = model.cost
    return model


def raw_irregularity(model, w, phs):
    a = align(w, phs, model.cost, AL["max_grapheme"], AL["max_null"])
    if not a:
        return None, None
    cap = AL["unit_cost_cap"]
    costs = [min(cap, model.cost(ph, g)) for ph, g in a]
    ents = [model.entropy(ph) for ph, g in a if ph != NULL]
    ent = sum(ents) / len(ents) if ents else 0.0
    VOWELS = set("aeiouy")
    # vowel letters swallowed by a consonant/NULL unit: the vowel slot the speller must guess
    hidden = sum(1 for ph, g in a if (ph == NULL or ph[0] not in "AEIOU") and any(ch in VOWELS for ch in g))
    # reduced-vowel ambiguity: H(G | AH0/IH0/ER0), summed (schwa is where English spelling is least predictable)
    red = sum(model.entropy(ph) for ph, g in a if ph in ("AH0", "IH0", "ER0"))
    return (sum(costs) / len(costs) + AL["max_unit_weight"] * max(costs) + AL["entropy_weight"] * ent
            + AL["hidden_vowel_weight"] * hidden + AL["reduced_vowel_weight"] * red), a


SUFFIXES = [("ness", ""), ("ly", ""), ("ily", "y"), ("ers", ""), ("er", ""), ("ing", ""), ("ing", "e"), ("ed", ""), ("ed", "e"),
            ("ment", ""), ("ish", ""), ("ful", ""), ("less", ""), ("able", ""), ("ably", ""), ("ity", ""), ("ist", ""), ("ism", ""), ("ize", "")]


def transparent(w, common):
    """True for a transparent compound (two common words) or a common stem plus a productive suffix."""
    for i in range(3, len(w) - 2):
        if w[:i] in common and w[i:] in common:
            return True
    for suf, rep in SUFFIXES:
        if w.endswith(suf) and len(w) - len(suf) >= 4 and (w[: -len(suf)] + rep) in common:
            return True
    return False


def main():
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("--extra", help="extra words file")
    ap.add_argument("--out", default=os.path.join(ROOT, "assets/words/en/en_difficulty.json"))
    a_ = ap.parse_args()
    zipf = zipf_fn()
    cmu = load_cmu_stress()
    train_words = sorted((w for w in cmu if len(w) >= 3 and zipf(w) >= AL["train_zipf_min"]),
                         key=lambda w: (-zipf(w), w))[: AL["train_max_words"]]
    model = train([(w, cmu[w][0]) for w in train_words])
    b = bank_rows()
    words = [w for t in TIERS for w in b[t]]
    if a_.extra:
        words += [l.strip() for l in open(a_.extra, encoding="utf-8") if l.strip() and not l.startswith("#")]
    words = sorted(set(words))
    raw, skipped = {}, []
    for w in words:
        if w not in cmu:
            skipped.append(w)
            continue
        best = None
        for phs in cmu[w]:
            r, _ = raw_irregularity(model, w, list(phs))
            if r is not None and (best is None or r < best):
                best = r
        if best is None:
            skipped.append(w)
        else:
            raw[w] = best
    # Irregularity percentile is taken against a FIXED reference (the training set), not against whatever words
    # happen to be scored this run, so a word's score never depends on which other words are in the batch.
    import bisect
    ref = []
    for w in train_words:
        best_r = None
        for phs in cmu[w]:
            r, _ = raw_irregularity(model, w, list(phs))
            if r is not None and (best_r is None or r < best_r):
                best_r = r
        if best_r is not None:
            ref.append(best_r)
    ref.sort()
    order = sorted(raw, key=lambda w: (raw[w], w))
    pct = {w: bisect.bisect_left(ref, raw[w]) / len(ref) for w in order}
    W = CFG["weights"]; lo, hi = CFG["rarity_band"]["zipf_hard"], CFG["rarity_band"]["zipf_easy"]
    common = {w for w in cmu if len(w) >= 3 and zipf(w) >= 4.0}
    scores, comps = {}, {}
    for w in order:
        rar = min(1.0, max(0.0, (hi - zipf(w)) / (hi - lo)))
        ln = min(len(w), CFG["length_cap"]) / CFG["length_cap"]
        tr = 1.0 if transparent(w, common) else 0.0
        comps[w] = [round(pct[w], 6), round(rar, 6), round(ln, 6), tr]
        scores[w] = round(W["irregularity"] * pct[w] + W["rarity"] * rar + W["length"] * ln - W.get("transparency_penalty", 0.0) * tr, 6)
    doc = {"$doc": "CC-EN-EXPERT-GROWTH F2 output; generated by tools/en_growth/score.py.",
           "config_sha256": sha256_file(os.path.join(ROOT, "config/en_difficulty.json")),
           "pron_source_sha256": sha256_file(CMU), "wordfreq": WORDFREQ_VERSION,
           "bank_hash": bank_hash(), "components": comps, "scores": scores, "null_key": skipped}
    json.dump(doc, open(a_.out, "w", encoding="utf-8"), ensure_ascii=False, sort_keys=True, indent=0)
    open(a_.out, "a").write("\n")
    print(f"scored {len(scores)}, unscored {len(skipped)}")


if __name__ == "__main__":
    main()
