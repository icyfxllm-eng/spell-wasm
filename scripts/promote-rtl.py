#!/usr/bin/env python3
"""Promote the audited RTL/ru drafts into the PRODUCTION word pipeline.

This is the mechanical half of the CC-RTL ship (docs/rtl-ship-checklist.md). Run it
AFTER the native audit is complete and its flags are ingested — it copies the
reviewed draft banks into the production seed dirs and wires the languages into the
build so `word_data.rs` carries real ar/fa/ur/ru banks. It does NOT flip any gate;
un-gating (RTL_SUPPORTED + the ComingSoon→Active statuses) is a deliberate, separate
code change that a human applies and signs off — see the checklist.

Idempotent: re-running copies the same files and leaves LANGS unchanged.

    python3 scripts/ingest-audit-flags.py audit-flags.txt   # 1. fold in the audit cuts
    python3 scripts/build-draft-banks.py                     # 2. regenerate the reviewed drafts
    python3 scripts/promote-rtl.py                            # 3. THIS: draft -> production pipeline
    #                                                            4. apply the flips (checklist) + test
"""
import os, re, shutil, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LANGS = ["ar", "fa", "ur", "ru"]


def promote():
    for lang in LANGS:
        src = os.path.join(ROOT, "assets", "words-draft", lang)
        dst = os.path.join(ROOT, "assets", "words", lang)
        if not os.path.isdir(src):
            sys.exit(f"  missing draft dir {src} — run build-draft-banks.py first")
        os.makedirs(dst, exist_ok=True)
        n = 0
        for tier in ("easy", "medium", "hard", "expert"):
            shutil.copyfile(os.path.join(src, f"{tier}.txt"), os.path.join(dst, f"{tier}.txt"))
            n += sum(1 for ln in open(os.path.join(dst, f"{tier}.txt"), encoding="utf-8") if ln.strip())
        print(f"  {lang}: promoted {n} words -> assets/words/{lang}/")

    # Wire the four into the production pipeline's LANGS (idempotent).
    p = os.path.join(ROOT, "scripts", "build-wordlists.py")
    s = open(p, encoding="utf-8").read()
    m = re.search(r"^LANGS = (\[[^\]]*\])", s, re.M)
    cur = eval(m.group(1))
    add = [l for l in LANGS if l not in cur]
    if add:
        s = s[: m.start(1)] + repr(cur + add) + s[m.end(1):]
        open(p, "w", encoding="utf-8").write(s)
        print(f"  build-wordlists LANGS += {add}")
    else:
        print("  build-wordlists LANGS already includes ar/fa/ur/ru")

    subprocess.run([sys.executable, os.path.join(ROOT, "scripts", "build-wordlists.py")], check=True)
    print("\n  Content promoted. STILL GATED — now apply the flips in")
    print("  docs/rtl-ship-checklist.md (RTL_SUPPORTED, statuses, tier_for arms, tests),")
    print("  then: cargo test --lib   (expect green, RTL languages active).")


if __name__ == "__main__":
    promote()
