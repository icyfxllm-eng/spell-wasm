#!/usr/bin/env python3
"""CC-BANK-PURITY F6 — the purity gate.  Offline: needs no dictionary, only the committed ledger.

    python3 tools/bank/purity_gate.py              # check this checkout
    python3 tools/bank/purity_gate.py --selftest   # plant four bad rows per language in a temp copy; each must be rejected

A bank row is admitted only if
  1. assets/words/purity/<lang>.tsv has exactly one line for it (and none without a row), with the tier hashes
     in step with src/word_data.rs;
  2. the line's verdict is `dictionary`, or `exception` carrying a signer and a reason, or `pending` AND the
     word is in assets/words/purity/grandfathered/<lang>.txt (the pending set frozen at Phase A: no new row may
     ever enter as pending; the file only shrinks);
  3. it is not on the language's profanity list, unless Eric's C6 mark cleared it
     (assets/words/purity/profanity-cleared/<lang>.txt) or it is still awaiting that mark
     (assets/words/purity/awaiting-c6/<lang>.txt, a ratchet: a listed row that left the bank, or was
     cleared, or no longer matches, fails until its line is deleted; a NEW match fails outright).
Plus two meta-checks: scripts/gate.sh must run this file's --selftest, and every script that writes a bank must
call tools/bank/purity_admit.py.

The ledger's `dictionary` verdicts are written only by tools/bank/gen_ledger.py from an F1 run, and the bank
writers below call F1 (purity_admit) before they write, so a row cannot get a verdict without being checked.
"""
import os, re, shutil, sys, tempfile, unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)
from census_c6 import fold_lenient, load   # same fold as src/norm.rs

LANGS = "en es fr de pt pl ko ja fil zh ru ar hi sw".split()
TIERS = "easy medium hard expert".split()
VERDICTS = ("dictionary", "exception", "pending")
# every script that writes assets/words/<lang>/<tier>.txt in order to GROW a bank must call purity_admit
BANK_WRITERS = ["scripts/build-bigbank.py", "scripts/build-hi-bank.py", "scripts/build-zh-bank.py",
                "scripts/grow-basic-nouns.py", "scripts/build-draft-banks.py"]

def lines(path):
    if not os.path.exists(path):
        return []
    return [l.rstrip("\n") for l in open(path, encoding="utf-8") if l.strip() and not l.startswith("#")]

def bank(root, lang):
    rows = []
    for t in TIERS:
        for w in lines(f"{root}/assets/words/{lang}/{t}.txt"):
            rows.append((t, w.strip()))
    return rows

def tier_hashes(root):
    s = open(f"{root}/src/word_data.rs", encoding="utf-8").read()
    return {(a, b): c for a, b, c in re.findall(r'\("(\w+)", "(\w+)", (0x[0-9A-F]+)\)', s)}

def check(root=REPO, langs=LANGS):
    """Return a list of problems; empty means the bank is admitted."""
    bad = []
    th = tier_hashes(root)
    for l in langs:
        rows = bank(root, l)
        words = [w for _, w in rows]
        led, hashes = [], {}
        p = f"{root}/assets/words/purity/{l}.tsv"
        if not os.path.exists(p):
            bad.append(f"{l}: no ledger assets/words/purity/{l}.tsv"); continue
        for line in open(p, encoding="utf-8").read().split("\n"):
            if not line:
                continue
            if line.startswith("#tier"):
                _, t, h = line.split("\t"); hashes[t] = h
            elif line[0] != "#":
                led.append(line.split("\t"))
        by = {}
        for f in led:
            by.setdefault(f[0], []).append(f)
        for w in dict.fromkeys(words):
            if w not in by:
                bad.append(f"{l}: bank row {w!r} has no purity verdict (not checked by F1, or a ledger edit was skipped)")
        for w, fs in by.items():
            if len(fs) > 1:
                bad.append(f"{l}: {w!r} has {len(fs)} ledger lines")
            if w not in set(words):
                bad.append(f"{l}: ledger line {w!r} has no bank row")
        for t in TIERS:
            if (l, t) in th and hashes.get(t) != th[(l, t)]:
                bad.append(f"{l}/{t}: ledger tier hash is stale against src/word_data.rs")
        grand = set(lines(f"{root}/assets/words/purity/grandfathered/{l}.txt"))
        for f in led:
            v = f[1] if len(f) > 1 else ""
            if v not in VERDICTS:
                bad.append(f"{l}: {f[0]!r} has verdict {v!r}, not one of {VERDICTS}")
            elif v == "exception" and (len(f) < 5 or not f[3].strip() or not f[4].strip()):
                bad.append(f"{l}: exception for {f[0]!r} has no signer or reason")
            elif v == "pending" and f[0] not in grand:
                bad.append(f"{l}: {f[0]!r} is pending but was not pending at Phase A: no new row may enter as pending")
        # profanity
        prof = load(f"{root}/assets/words/profanity/{l}.txt") or set()
        fold = lambda w: fold_lenient(w.rsplit("|", 1)[-1])
        cleared = {fold(w) for w in lines(f"{root}/assets/words/purity/profanity-cleared/{l}.txt")}
        awaiting = set(lines(f"{root}/assets/words/purity/awaiting-c6/{l}.txt"))
        hits = {w for w in words if fold(w) in prof}
        for w in sorted(hits):
            if fold(w) not in cleared and w not in awaiting:
                bad.append(f"{l}: bank row {w!r} is on the profanity list and has no C6 clearance")
        for w in sorted(awaiting):
            if w not in hits:
                bad.append(f"{l}: awaiting-c6 lists {w!r}, which is no longer a bank row on the profanity list: delete its line")
            elif fold(w) in cleared:
                bad.append(f"{l}: awaiting-c6 lists {w!r}, which is cleared: delete its line")
    return bad

def meta(root=REPO):
    bad = []
    gate = f"{root}/scripts/gate.sh"
    if not os.path.exists(gate) or "purity_gate.py --selftest" not in open(gate, encoding="utf-8").read():
        bad.append("scripts/gate.sh does not run `tools/bank/purity_gate.py --selftest`: the purity gate no longer bites")
    for w in BANK_WRITERS:
        p = f"{root}/{w}"
        if os.path.exists(p) and "purity_admit" not in open(p, encoding="utf-8").read():
            bad.append(f"{w} writes a bank without calling tools/bank/purity_admit.py (F1 must run before any row is written)")
    return bad

# --------------------------------------------------------------------------- selftest
def _plants(lang):
    eng, noun, frag = "website", "tottenham", "qwrtp"
    return {"english": {"ko": "웹사이트"}.get(lang, eng), "proper_noun": noun, "fragment": frag}

def selftest():
    import subprocess
    fails, ok = [], 0
    base = tempfile.mkdtemp(prefix="purity-selftest-")
    try:
        for l in LANGS:
            root = os.path.join(base, l)
            for rel in (f"assets/words/{l}", "assets/words/purity", "assets/words/profanity", "src"):
                src = f"{REPO}/{rel}"
                if os.path.isdir(src):
                    shutil.copytree(src, f"{root}/{rel}", ignore=shutil.ignore_patterns("*.rs") if rel == "src" else None)
            shutil.copy(f"{REPO}/src/word_data.rs", f"{root}/src/word_data.rs")
            clean = check(root, [l])
            if clean:
                fails.append(f"{l}: selftest cannot start, the real bank is not clean: {clean[0]}"); continue
            prof = sorted(load(f"{REPO}/assets/words/profanity/{l}.txt") or [])
            plants = dict(_plants(l))
            # the profanity plant carries a (forged) dictionary verdict so ONLY the profanity rule can reject it
            inbank = {fold_lenient(w.rsplit("|", 1)[-1]) for _, w in bank(root, l)}
            fresh = [w for w in prof if w not in inbank and w.isalpha()]
            if fresh:
                plants["profanity"] = fresh[0]
            for kind, word in plants.items():
                r2 = os.path.join(base, f"{l}-{kind}")
                shutil.copytree(root, r2)
                with open(f"{r2}/assets/words/{l}/easy.txt", "a", encoding="utf-8") as f:
                    f.write(word + "\n")
                if kind == "profanity":
                    with open(f"{r2}/assets/words/purity/{l}.tsv", "a", encoding="utf-8") as f:
                        f.write(f"{word}\tdictionary\tselftest\n")
                got = check(r2, [l])
                if not got:
                    fails.append(f"{l}: planted {kind} row {word!r} was ADMITTED")
                elif kind == "profanity" and not any("profanity" in g for g in got):
                    fails.append(f"{l}: planted profanity row was rejected, but not by the profanity rule: {got[0]}")
                else:
                    ok += 1
                shutil.rmtree(r2)
            if "profanity" not in plants:
                print(f"  note: {l} has no usable profanity list, so the profanity plant is skipped for it")
        # a tampered ledger is caught too: a forged `pending` for a new row, an exception with no signer
        r3 = os.path.join(base, "tamper"); shutil.copytree(os.path.join(base, "en"), r3)
        with open(f"{r3}/assets/words/en/easy.txt", "a", encoding="utf-8") as f:
            f.write("wxyzq\n")
        with open(f"{r3}/assets/words/purity/en.tsv", "a", encoding="utf-8") as f:
            f.write("wxyzq\tpending\tselftest\n")
        if not any("no new row may enter as pending" in g for g in check(r3, ["en"])):
            fails.append("en: a new row forged as `pending` was admitted")
        else:
            ok += 1
        with open(f"{r3}/assets/words/purity/en.tsv", "a", encoding="utf-8") as f:
            f.write("wxyzr\texception\tselftest\n")
        with open(f"{r3}/assets/words/en/easy.txt", "a", encoding="utf-8") as f:
            f.write("wxyzr\n")
        if not any("no signer or reason" in g for g in check(r3, ["en"])):
            fails.append("en: an unsigned exception was admitted")
        else:
            ok += 1
        # the meta-check bites: gate.sh without the selftest line, a writer without purity_admit
        m = os.path.join(base, "meta"); os.makedirs(f"{m}/scripts")
        open(f"{m}/scripts/gate.sh", "w").write("echo nothing\n")
        open(f"{m}/scripts/build-bigbank.py", "w").write("open('x','w')\n")
        got = meta(m)
        if len(got) < 2:
            fails.append("meta-check did not reject a gate.sh without the selftest and a writer without purity_admit")
        else:
            ok += 1
    finally:
        shutil.rmtree(base, ignore_errors=True)
    for f in fails:
        print("  SELFTEST FAIL:", f)
    print(f"purity-gate selftest: {'FAILED' if fails else 'OK'} — {ok} plants rejected, {len(fails)} survived")
    return 1 if fails else 0

def main(argv):
    if "--selftest" in argv:
        sys.exit(selftest())
    bad = check() + meta()
    for b in bad[:60]:
        print("  ", b)
    if len(bad) > 60:
        print(f"   ... and {len(bad) - 60} more")
    print(f"purity-gate: {'FAILED' if bad else 'OK'} — {len(bad)} problem(s)")
    sys.exit(1 if bad else 0)

if __name__ == "__main__":
    main(sys.argv[1:])
