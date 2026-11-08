#!/usr/bin/env python3
"""CC-LAUNCH-EN Phase B item 3: English grading parity, local vs /api/check.

The App Store build grades English on the device only (src/game.rs
en_route + en_grade_local). Dev builds still ask the server's /api/check
(backend/app.py check()). This compares the two over the whole English bank,
legacy and re-tier tiers, plus both directions of every UK/US pair in
data/en/accepted-variants.tsv, and requires ZERO differences.

    python3 tools/parity/en_check_parity.py            # exit 0 = parity
    python3 tools/parity/en_check_parity.py --json OUT # also write every diff
    python3 tools/parity/en_check_parity.py --mutate   # self-test: a deliberately
        wrong server (accepts any prefix of the word) MUST produce diffs; exit 0
        only if it does, so a check that cannot fail never passes as parity

No network, no Flask: the body of backend/app.py's check() is lifted out of the
file with ast and run in-process against a stand-in request/jsonify, so the
server side is the code the backend actually serves. If check() ever calls
anything else in app.py, this fails loudly (NameError) rather than guessing.

What is compared is the VERDICT A PLAYER GETS, per build:
  - route other_sense / variant: correct in both builds (decided before any
    server call, by the same code);
  - route grade: release = en_grade_local, dev = server check().
"""
import ast
import json
import os
import subprocess
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "target" / "en-parity" / "local.jsonl"


def load_server_check():
    src = (ROOT / "backend" / "app.py").read_text(encoding="utf-8")
    tree = ast.parse(src)
    fn = next((n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == "check"), None)
    if fn is None:
        sys.exit("en_check_parity: backend/app.py has no check() -- the server grader moved; update this tool")
    fn.decorator_list = []
    mod = ast.Module(body=[fn], type_ignores=[])

    class Req:
        payload = None

        def get_json(self, silent=False):
            return self.payload

    req = Req()
    env = {"request": req, "jsonify": lambda d: d}
    exec(compile(mod, "backend/app.py:check", "exec"), env)
    check = env["check"]

    def server(word, typed):
        req.payload = {"word": word, "answer": typed}
        r = check()
        body = r[0] if isinstance(r, tuple) else r
        if "correct" not in body:
            raise RuntimeError(f"check() refused {word!r}/{typed!r}: {body}")
        return bool(body["correct"])

    return server


def main():
    want_json = "--json" in sys.argv
    OUT.parent.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, EN_PARITY_OUT=str(OUT))
    cmd = ["cargo", "test", "--lib", "-q", "en_check_parity_local_half", "--", "--ignored", "--nocapture"]
    print("==>", " ".join(cmd), file=sys.stderr)
    r = subprocess.run(cmd, cwd=ROOT, env=env)
    if r.returncode != 0:
        sys.exit("en_check_parity: the local half failed to run")
    server = load_server_check()
    mutate = "--mutate" in sys.argv
    if mutate:
        server = lambda word, typed: bool(typed) and word.lower().startswith(typed.lower().strip())

    cases = [json.loads(l) for l in OUT.read_text(encoding="utf-8").splitlines() if l.strip()]
    diffs, routes = [], Counter()
    words = {c["word"] for c in cases if c["source"] == "bank"}
    tiers = Counter(t for c in cases if c["source"] == "bank" and c["form"] == "exact" and not c["kid"] for t in c["tiers"])
    for c in cases:
        routes[c["route"]] += 1
        if c["route"] != "grade":
            continue  # decided locally in every build, before /api/check
        dev = server(c["word"], c["typed"])
        if dev != c["local"]:
            c["server"] = dev
            diffs.append(c)

    print(f"en_check_parity: {len(cases)} cases, {len(words)} bank words, "
          f"{sum(1 for c in cases if c['source'] == 'uk_variants') // 4} UK/US pairs")
    print("  words per tier: " + ", ".join(f"{t} {n}" for t, n in sorted(tiers.items())))
    print("  routes: " + ", ".join(f"{k} {v}" for k, v in sorted(routes.items())))
    if want_json:
        p = Path(sys.argv[sys.argv.index("--json") + 1])
        p.write_text(json.dumps(diffs, ensure_ascii=False, indent=1), encoding="utf-8")
        print(f"  diffs written to {p}")
    if mutate:
        ok = len(diffs) > 0
        print(f"en_check_parity --mutate: {'OK' if ok else 'FAILED'} -- a wrong server grader gives {len(diffs)} diffs")
        return 0 if ok else 1
    if not diffs:
        print("en_check_parity: OK -- 0 diffs (release local grade == dev /api/check)")
        return 0
    by = Counter((c["form"], "kid" if c["kid"] else "std", c["local"], c["server"]) for c in diffs)
    print(f"en_check_parity: FAILED -- {len(diffs)} diffs")
    print("  by class (form, grading, local, server): count")
    for k, n in sorted(by.items(), key=lambda kv: -kv[1]):
        print(f"    {k}: {n}")
    seen = set()
    print("  examples:")
    for c in diffs:
        k = (c["form"], c["kid"])
        if k in seen:
            continue
        seen.add(k)
        print(f"    word={c['word']!r} typed={c['typed']!r} kid={c['kid']} local={c['local']} server={c['server']}")
    return 1


if __name__ == "__main__":
    sys.exit(main())
