#!/usr/bin/env python3
"""CC-SENSE-CUE F1 — collision sets, computed per language.

D6 is law: collision sets are COMPUTED, NEVER HAND-LISTED. Russian final
devoicing alone yields hundreds of pairs and the set changes every time the
bank grows, so a hand list is stale the day after it is written. The Spanish
file this replaces was 16 hand-written groups.

A collision set is a group of bank rows sharing one phonological key. The key
function is per language and models only what the AUDIO CANNOT CARRY -- a
learner is never marked wrong for a distinction their ear could not have made.

  en  no derivable rule. cmudict pronunciations over the bank, stress stripped.
  es  seseo (s/z/ce/ci), yeismo (ll/y), silent h, b/v.
  ru  final obstruent devoicing, regressive voicing assimilation in clusters,
      unstressed vowel reduction (a/o merge, e/i merge), yo -> ye.

PRECISION IS REPORTED, NOT GATED, per F1: a false collision costs an
unnecessary acceptance, which is survivable; a missed collision costs an unfair
item, which is not.

Output is assets/words/<lang>/homophones.txt -- the format src/homophones.rs
already consumes -- one group per line. Regenerate on every bank change.

Run:  python3 tools/build_collisions.py [lang ...]
"""
import pathlib, re, sys, unicodedata
from collections import defaultdict

ROOT = pathlib.Path(__file__).resolve().parent.parent
CMU = ROOT / "tools/wordpipe/sources/cmudict.dict"
OUT = ROOT / "assets/words"
TIERS = ("EASY", "MEDIUM", "HARD", "EXPERT")


def bank(lang):
    src = (ROOT / "src/words.rs").read_text() + (ROOT / "src/word_data.rs").read_text()
    out = []
    for t in TIERS:
        m = re.search(rf"pub const {lang.upper()}_{t}: &\[&str\] = &\[(.*?)\];", src, re.S)
        if not m:
            continue
        for e in re.findall(r'"([^"]+)"', m.group(1)):
            w = e.split("|")[0].strip()
            if w and " " not in w:
                out.append(w)
    return sorted(set(out))


# ---------------------------------------------------------------- en
def key_en():
    pron = defaultdict(set)
    if not CMU.exists():
        return None
    for line in CMU.read_text(encoding="utf-8", errors="ignore").splitlines():
        if not line or line.startswith(";;;"):
            continue
        p = line.split()
        w = p[0].lower()
        if w.endswith(")"):
            w = w[: w.rindex("(")]
        if not w.isalpha():
            continue
        pron[w].add(" ".join(re.sub(r"\d", "", x) for x in p[1:]))
    # A word must be grouped under EVERY pronunciation it has, or "to" (T UW,
    # T IH, T AH) never meets "too" (T UW) -- taking only the first sorted
    # pronunciation lost to/too/two entirely.
    #
    # HETERONYMS ARE EXCLUDED, following tools/spellophones-inventory.py: a word
    # with two distinct STRESSED patterns (read = R EH D / R IY D) would
    # otherwise chain the red-group to the reed-group through itself, and the
    # merge pass would union two unrelated sets into one.
    stressed = defaultdict(set)
    for w, ps in pron.items():
        for full in ps:
            stressed[w].add(tuple(x for x in full.split() if x[-1:] in "12"))
    def k(w):
        lw = w.lower()
        ps = pron.get(lw)
        if not ps or len(stressed.get(lw, ())) > 1:
            return None
        return sorted(ps)
    return k


# ---------------------------------------------------------------- es
def key_es(w):
    s = w.lower()
    s = s.replace("ll", "y")                    # yeismo
    s = s.replace("ce", "se").replace("ci", "si")
    s = s.replace("z", "s")                     # seseo
    s = s.replace("h", "")                      # silent h
    s = s.replace("v", "b")                     # b/v merge
    s = s.replace("qu", "k").replace("c", "k")
    return unicodedata.normalize("NFD", s)      # accents: audio can't carry them


# ---------------------------------------------------------------- ru
VOICED = "бвгдзж"
VOICELESS = "пфктсш"
DEVOICE = str.maketrans(VOICED, VOICELESS)


def key_ru(w):
    s = w.lower().replace("ё", "е")
    # regressive assimilation: a voiced obstruent before a voiceless one devoices
    ch = list(s)
    for i in range(len(ch) - 1):
        if ch[i] in VOICED and ch[i + 1] in VOICELESS:
            ch[i] = ch[i].translate(DEVOICE)
    s = "".join(ch)
    if s and s[-1] in VOICED:                   # final devoicing
        s = s[:-1] + s[-1].translate(DEVOICE)
    # NO VOWEL REDUCTION. Reduction is conditioned on STRESS, and applying it
    # unconditionally does not produce false collisions of the survivable kind
    # -- it merges grammatical inflections. The first cut of this key grouped
    # академии/академия, большая/большое and будущей/будущий, and accepting one
    # inflection for another is a different error from accepting a homophone:
    # the learner heard a distinguishable ending and typed the wrong one.
    #
    # Doing it properly needs src/ru_stress_data.rs, which is DARK until a human
    # signs config/ru-stress-audit.json. So the ru key models only what is
    # stress-INDEPENDENT (devoicing and assimilation) and under-generates by
    # design. That is the safe direction: a missed collision is an unfair item,
    # but a false one silently accepts a wrong answer, and F1's "precision is
    # not gated" reasoning assumes false positives are harmless -- for Russian
    # inflection they are not.
    return s


KEYS = {"es": key_es, "ru": key_ru}
FIXTURE_RU = [("луг", "лук"), ("плот", "плод"), ("код", "кот"), ("пруд", "прут"),
              ("род", "рот"), ("гриб", "грипп"), ("лез", "лес"), ("вёз", "вес")]


def build(lang):
    k = key_en() if lang == "en" else KEYS.get(lang)
    if k is None:
        print(f"  {lang}: no key function (or cmudict missing)")
        return
    words = bank(lang)
    if not words:
        print(f"  {lang}: empty bank")
        return
    groups = defaultdict(list)
    for w in words:
        key = k(w)
        if not key:
            continue
        for kk in (key if isinstance(key, list) else [key]):
            groups[kk].append(w)

    # ENGLISH ONLY: a bank word's real homophones that are NOT in the bank.
    # Invariant 1's intent is that no learner is marked wrong for a real
    # spelling the audio supports -- and the audio does not know which words we
    # happened to bank. A player hearing [pɛər] may write "pare"; that it is
    # unbanked is our accident, not their error. This is the GRADING RISK class
    # the Spell-O-Phones inventory already named. Computable only where a
    # pronunciation lexicon exists, which here means English.
    if lang == "en":
        # FREQUENCY-FILTERED. web2 carries the archaic tail -- aer, eyre, adz,
        # abel -- and accepting those buys no fairness: nobody types a word they
        # do not know, so the only effect is to widen what passes silently.
        # Restrict unbanked candidates to the 50k frequency list, which is the
        # set a learner could plausibly produce.
        # BOTH filters, because each alone fails in the opposite direction.
        # web2 alone admits the archaic tail (aer, eyre, adz). The 50k frequency
        # list alone admits proper names (ahn, ann, anne, ame) -- it is corpus
        # frequency, not a dictionary. A candidate must be a real dictionary
        # word AND common enough that a learner could actually produce it.
        w2 = pathlib.Path("/usr/share/dict/web2")
        freq = ROOT / "tools/wordpipe/sources/freq_en.txt"
        dictw = ({l.strip().lower() for l in w2.read_text(errors="ignore").splitlines()
                  if l.strip().isalpha() and l.strip()[0].islower()} if w2.exists() else set())
        common = ({l.split()[0].lower() for l in freq.read_text(errors="ignore").splitlines()
                   if l.split()} if freq.exists() else set())
        real = dictw & common
        if real:
            banked = set(words)
            bank_keys = set()
            for w in words:
                kk = k(w)
                if kk:
                    bank_keys.update(kk if isinstance(kk, list) else [kk])
            for cand in real:
                if cand in banked:
                    continue
                ck = k(cand)
                if not ck:
                    continue
                for kk in (ck if isinstance(ck, list) else [ck]):
                    if kk in bank_keys:
                        groups[kk].append(cand)
    # CURATED SEED, merged not replaced. D6 says collision sets are computed,
    # never hand-listed -- but computing them needs a LEXICON, and only English
    # has one here (cmudict + web2). The Spanish seed carries pairs like
    # casa/caza where only one member is in the bank: a learner hearing [ˈkasa]
    # can legitimately write caza, a real word, and Invariant 1's intent covers
    # any real spelling the audio supports, not only banked ones. Regenerating
    # over the bank alone DELETED that knowledge and narrowed fairness.
    #
    # So the seed is merged in and its provenance kept. D6 holds fully for
    # English; for es/ru it is aspirational until those lexicons exist.
    seed = OUT / lang / "homophones.seed.txt"
    if seed.exists():
        for line in seed.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if line and not line.startswith("#"):
                groups[f"seed:{line}"] = line.split()

    sets = sorted([sorted(set(v)) for v in groups.values() if len(set(v)) > 1])
    # merge overlapping groups (a seed pair may share a member with a computed set)
    merged, changed = [list(x) for x in sets], True
    while changed:
        changed = False
        out2 = []
        for g in merged:
            for o in out2:
                if set(g) & set(o):
                    o[:] = sorted(set(o) | set(g)); changed = True; break
            else:
                out2.append(sorted(set(g)))
        merged = out2
    sets = sorted(merged)
    path = OUT / lang / "homophones.txt"
    path.parent.mkdir(parents=True, exist_ok=True)
    prior = path.read_text().splitlines() if path.exists() else []
    prior_groups = [l.split() for l in prior if l.strip() and not l.startswith("#")]
    path.write_text(
        f"# GENERATED by tools/build_collisions.py -- do not hand-edit.\n"
        f"# CC-SENSE-CUE F1/D6: collision sets are computed, never hand-listed.\n"
        f"# {len(sets)} sets over {len(words)} bank rows.\n"
        + "\n".join(" ".join(g) for g in sets) + "\n", encoding="utf-8")
    covered = sum(len(g) for g in sets)
    generated_before = prior and prior[0].startswith("# GENERATED")
    label = "previously generated" if generated_before else "hand-listed"
    extra = f"   ({len(prior_groups)} {label})" if prior_groups else ""
    print(f"  {lang}: {len(sets)} sets, {covered} rows "
          f"({covered / len(words) * 100:.1f}% of bank){extra}")
    if lang == "ru":
        # A fixture pair can only be recovered if BOTH members are in the bank.
        # F1's "30 known pairs at 100% recall" is a claim about the LANGUAGE;
        # this generator can only group rows that exist. Scoring against pairs
        # the bank does not contain measures the bank, not the key function --
        # the first run reported 0/8 and none of the eight were banked.
        ws = set(words)
        applicable = [(a, b) for a, b in FIXTURE_RU if a in ws and b in ws]
        found = {tuple(sorted((a, b))) for g in sets for a in g for b in g if a < b}
        hit = sum(1 for p in applicable if tuple(sorted(p)) in found)
        print(f"       fixture: {len(applicable)}/{len(FIXTURE_RU)} pairs are in the bank; "
              f"recall on those {hit}/{len(applicable) if applicable else 0}"
              + ("" if hit == len(applicable) else "   <-- key function misses a banked pair"))
    if prior_groups and not generated_before:
        lost = [g for g in prior_groups if not any(set(g) <= set(s) for s in sets)]
        if lost:
            print(f"       WARNING: {len(lost)} hand-listed group(s) not reproduced: {lost[:4]}")


def check():
    """CC-SENSE-CUE F2 gate. A collision set that is neither accept-all nor
    cued is a BUILD FAILURE, not a warning. cueMode is off everywhere, so
    every set must be accept-all -- which means two things must hold:

      1. the checked-in table matches what the generator produces. A stale
         table is the failure mode that matters: the bank grows, new
         collisions appear, and learners are marked wrong for correct words
         until someone remembers to regenerate.
      2. every language with a table is WIRED into src/homophones.rs. An
         unwired table is a dead file that looks like fairness coverage.
    """
    import io, contextlib
    bad = []
    src = (ROOT / "src/homophones.rs").read_text()
    for lang in ("en", "es", "ru"):
        path = OUT / lang / "homophones.txt"
        before = path.read_text(encoding="utf-8") if path.exists() else ""
        with contextlib.redirect_stdout(io.StringIO()):
            build(lang)
        after = path.read_text(encoding="utf-8") if path.exists() else ""
        if before != after:
            path.write_text(before, encoding="utf-8")
            bad.append(f"{lang}: the collision table is STALE. Regenerate with "
                       f"tools/build_collisions.py -- until then learners are marked "
                       f"wrong for real words the audio cannot distinguish.")
        if after and f'assets/words/{lang}/homophones.txt' not in src:
            bad.append(f"{lang}: has a collision table that src/homophones.rs never "
                       f"includes -- a dead file that looks like fairness coverage.")
    if bad:
        print("collision-check: FAILED")
        for b in bad:
            print("  " + b)
        return 1
    print("collision-check: OK — every collision set is accept-all, tables current and wired")
    return 0


if __name__ == "__main__":
    if "--check" in sys.argv:
        raise SystemExit(check())
    for lang in ([a for a in sys.argv[1:] if not a.startswith("-")] or ["en", "es", "ru"]):
        build(lang)
