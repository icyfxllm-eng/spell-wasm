#!/usr/bin/env python3
"""Grow the Mandarin (zh) word bank — the ONE language the generic pipeline can't.

Every other language is a string of letters, so build-bigbank.py tiers it by
length and build-wordlists.py charset-gates the whole word. Mandarin is different
on both counts: a character is a whole word (length-tiering is meaningless), and
each entry is a "pinyin|hanzi" PAIR — the pinyin (before '|') is what the player
types, the hanzi (after '|') is what TTS speaks and what's revealed. So zh needs
its own toolchain and its own home; it lives in assets/words/zh/*.txt and is
emitted into src/words.rs by THIS script, not by build-wordlists.py.

What it does:
  1. Bootstraps assets/words/zh/{tier}.txt from the existing curated consts in
     words.rs (one-time) so the hand-curated, native-review-flagged core becomes
     zh's source of truth — exactly the seed-dir every other language already has.
  2. AUGMENTS each tier with common two-character words from the Leipzig cmn
     corpus (CC BY 4.0). The corpus mixes Traditional and Simplified (it's drawn
     from zh Wikipedia), so every candidate is first normalized to Simplified with
     OpenCC (t2s) — this bank is Simplified, its seed is 我们 not 我們 — and then
     converted to the game's tone-number pinyin via pypinyin (Style.TONE3,
     neutral_tone_with_five; v→ü to match "nü3ren2"). The curated seed is kept
     verbatim and always sorts first — corpus words only add volume.
  3. Gates every candidate: the pinyin must be typeable on the zh keyboard
     (reachable_chars — qwerty + tone digits 1-5 + ü), and pinyin must be UNIQUE
     across the bank (two entries sharing a pinyin would be indistinguishable when
     typed), and the hanzi must not already be in the seed.
  4. Tiers corpus additions by FREQUENCY rank (a stand-in for HSK level, since a
     licensed HSK list isn't wired in): most frequent → easy, rarest → expert.
  5. Regenerates the four ZH_* consts in src/words.rs from the augmented files.

This bank stays GATED (zh is ComingSoon) and native audit precedes activation —
that audit is the quality gate for the un-reviewed corpus additions, the same
footing as the ar/fa/ur audit drafts.

Usage:  python3 scripts/build-zh-bank.py [--target N]   (default 300 per tier)
"""
import os, re, sys, tarfile, unicodedata, urllib.request, importlib.util

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CACHE = os.path.join(ROOT, ".corpus-cache")
WORDS_RS = os.path.join(ROOT, "src", "words.rs")
ZH_DIR = os.path.join(ROOT, "assets", "words", "zh")
TIERS = ["easy", "medium", "hard", "expert"]
SOURCE = "cmn_wikipedia_2021_100K"
HAN = re.compile(r"^[一-鿿]+$")

# Reuse the production charset gate so the pinyin is validated exactly as the
# keyboard test validates it (qwerty + "12345" + ü via long-press v).
_spec = importlib.util.spec_from_file_location("bw", os.path.join(ROOT, "scripts", "build-wordlists.py"))
_bw = importlib.util.module_from_spec(_spec)
try:
    _spec.loader.exec_module(_bw)
except SystemExit:
    pass
reach_zh = _bw.reachable_chars("zh")


def to_pinyin(hanzi):
    from pypinyin import pinyin, Style
    syl = [s[0] for s in pinyin(hanzi, style=Style.TONE3, neutral_tone_with_five=True)]
    return "".join(syl).replace("v", "ü")


def bootstrap_seed():
    """One-time: lift the curated ZH_* consts out of words.rs into the seed files."""
    if all(os.path.exists(os.path.join(ZH_DIR, f"{t}.txt")) for t in TIERS):
        return
    src = open(WORDS_RS, encoding="utf-8").read()
    os.makedirs(ZH_DIR, exist_ok=True)
    for tier in TIERS:
        block = re.search(rf"pub const ZH_{tier.upper()}: &\[&str\] = &\[(.*?)\];", src, re.S).group(1)
        pairs = re.findall(r'"([^"]+)"', block)
        open(os.path.join(ZH_DIR, f"{tier}.txt"), "w", encoding="utf-8").write("\n".join(pairs) + "\n")
    print("  bootstrapped assets/words/zh/*.txt from the curated consts")


def corpus_rows():
    os.makedirs(CACHE, exist_ok=True)
    tgz = os.path.join(CACHE, SOURCE + ".tar.gz")
    if not os.path.exists(tgz):
        print(f"    fetching {SOURCE} …", flush=True)
        urllib.request.urlretrieve(f"https://downloads.wortschatz-leipzig.de/corpora/{SOURCE}.tar.gz", tgz)
    with tarfile.open(tgz) as t:
        member = next(m for m in t.getmembers() if m.name.endswith("-words.txt"))
        rows = []
        for line in t.extractfile(member).read().decode("utf-8").splitlines():
            p = line.split("\t")
            if len(p) >= 3 and HAN.match(p[1]) and len(p[1]) == 2:
                try:
                    rows.append((p[1], int(p[2])))
                except ValueError:
                    pass
    rows.sort(key=lambda x: -x[1])
    return rows


def build(target):
    bootstrap_seed()
    seed = {t: [ln.strip() for ln in open(os.path.join(ZH_DIR, f"{t}.txt"), encoding="utf-8") if ln.strip()] for t in TIERS}
    seed_hanzi = {p.split("|", 1)[1] for t in TIERS for p in seed[t]}
    seen_pinyin = {p.split("|", 1)[0] for t in TIERS for p in seed[t]}

    import opencc
    t2s = opencc.OpenCC("t2s")

    # New corpus pairs in frequency order: normalize to Simplified, then gate +
    # dedup. Two Traditional words can converge to one Simplified word, so `hz`
    # (post-conversion) is deduped too, not just the pinyin.
    additions = []
    for hz_raw, _ in corpus_rows():
        hz = t2s.convert(hz_raw)
        if len(hz) != 2 or not HAN.match(hz) or hz in seed_hanzi:
            continue
        seed_hanzi.add(hz)
        pin = to_pinyin(hz)
        if not pin or any(c not in reach_zh for c in pin) or pin in seen_pinyin:
            continue
        seen_pinyin.add(pin)
        additions.append(f"{pin}|{hz}")

    # Fill each tier to `target`: keep the whole curated seed, then take corpus
    # words. Frequency tiers the additions — the most frequent fill easy first.
    banks, i = {}, 0
    for tier in TIERS:
        want = max(0, target - len(seed[tier]))
        banks[tier] = list(seed[tier]) + additions[i:i + want]
        i += want

    for tier in TIERS:
        open(os.path.join(ZH_DIR, f"{tier}.txt"), "w", encoding="utf-8").write("\n".join(banks[tier]) + "\n")
    regen_words_rs(banks)
    return {t: (len(seed[t]), len(banks[t])) for t in TIERS}


def regen_words_rs(banks):
    src = open(WORDS_RS, encoding="utf-8").read()
    for tier in TIERS:
        body = "".join(f'    "{p}",\n' for p in banks[tier])
        block = f"pub const ZH_{tier.upper()}: &[&str] = &[\n{body}];"
        src = re.sub(rf"pub const ZH_{tier.upper()}: &\[&str\] = &\[.*?\];", lambda _m: block, src, count=1, flags=re.S)
    open(WORDS_RS, "w", encoding="utf-8").write(src)


def main():
    target = 300
    if "--target" in sys.argv:
        target = int(sys.argv[sys.argv.index("--target") + 1])
    print(f"  growing zh to {target}/tier (curated seed + Leipzig cmn, CC BY):")
    counts = build(target)
    for tier in TIERS:
        s, tot = counts[tier]
        print(f"    {tier:7} {tot:4}  (seed {s} + corpus {tot - s})")


if __name__ == "__main__":
    main()
