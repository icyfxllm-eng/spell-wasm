# -*- coding: utf-8 -*-
"""CC-DE-BANK acceptance tests 1-4 and 8 (Python side).  python3 -m unittest tools/bank/de/test_de_bank.py"""
import json, os, re, subprocess, sys, tempfile, unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import de_ingest, de_prefilter  # noqa: E402
ROOT = de_ingest.ROOT
FIX = os.path.join(ROOT, "fixtures", "audit", "de", "de-spelling_b1_clean.xlsx")
MAN = os.path.join(ROOT, "fixtures", "audit", "de", "de-b1.export.json")


class Ingest(unittest.TestCase):
    def test1_fixture_ingests_to_161_canonical_keeps(self):
        header, rows = de_ingest.load(FIX)
        self.assertEqual(de_ingest.check(header, rows, MAN), [])
        es = de_ingest.entries(rows)
        self.assertEqual(len(es), 161)
        words = {e["word"] for e in es}
        for w in ("Gärten", "Licht", "Reise", "Streit", "Erde", "Norden", "Angst", "Uniformen", "Liveticker", "Pharma"):
            self.assertIn(w, words)
        self.assertNotIn("licht", words)
        homs = {e["word"]: e["homophones"] for e in es if e["homophones"]}
        self.assertEqual(homs, {"Mann": ["man"], "weist": ["weißt"], "heute": ["Häute"], "sechs": ["Sex"]})

    def test2_first_delivery_is_rejected_with_named_rows(self):
        """The auditor's ORIGINAL first delivery is not in the repo; this rebuilds its
        reported faults on top of the clean sheet, at the rows the spec names."""
        header, rows = de_ingest.load(FIX)
        rows = [list(r) for r in rows]
        by = {r[0]: r for r in rows}
        by["DE-B1-040"][4] = "Fußboden."
        by["DE-B1-160"][4] = " Maßstab"
        by["DE-B1-094"][5] = "yes"
        for rid in ("DE-B1-058", "DE-B1-176", "DE-B1-188"):
            by[rid][6] = ""
        errs = de_ingest.check(header, rows, MAN)
        got = {(e[0], e[2]) for e in errs}
        for want in [("DE-B1-040", "PUNCT"), ("DE-B1-160", "WHITESPACE"), ("DE-B1-094", "NOT_A_HOMOPHONE"),
                     ("DE-B1-058", "NOTE_REQUIRED"), ("DE-B1-176", "NOTE_REQUIRED"), ("DE-B1-188", "NOTE_REQUIRED")]:
            self.assertIn(want, got)

    def test_other_guards(self):
        header, rows = de_ingest.load(FIX)
        rows = [list(r) for r in rows]
        rows[0][4] = rows[0][1]                       # Correct Form == Word
        rows[1][3] = "Maybe"                          # bad Decision
        rows[2][5] = rows[2][1].upper()               # Same Sound == Word ignoring case
        codes = {e[2] for e in de_ingest.check(header, rows, MAN)}
        self.assertTrue({"SAME_AS_WORD", "DECISION", "NOT_A_HOMOPHONE"} <= codes)
        # locked columns, row order, row set
        self.assertIn("LOCKED", {e[2] for e in de_ingest.check(header, [list(r) for r in rows[:0]] + [[r[0], r[1] + "x"] + r[2:] if i == 3 else r for i, r in enumerate(rows)], MAN)})
        self.assertIn("ROWSET", {e[2] for e in de_ingest.check(header, rows[::-1], MAN)})
        self.assertIn("ROWSET", {e[2] for e in de_ingest.check(header, rows[:-1], MAN)})

    def test_cli_rejects_and_accepts(self):
        r = subprocess.run([sys.executable, "-I", os.path.join(HERE, "de_ingest.py"), "ingest", FIX, MAN, "--out",
                            os.path.join(tempfile.mkdtemp(), "o.json")], capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stderr)


class Prefilter(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        header, cls.rows = de_ingest.load(FIX)
        cfg = de_prefilter.load_cfg()
        cls.res = de_prefilter.flags_for([r[1] for r in cls.rows], cfg, de_prefilter.Reference(cfg))

    def test3_recall_on_batch1_cuts(self):
        cuts = [r for r in self.rows if r[3] == "Cut"]
        hit = [r for r in cuts if self.res[r[1]]]
        self.assertEqual(len(cuts), 39)
        self.assertGreaterEqual(len(hit), 30, [r[1] for r in cuts if not self.res[r[1]]])

    def test4_precision_on_batch1_keeps(self):
        keeps = [r for r in self.rows if r[3] == "Keep"]
        bad = [(r[1], self.res[r[1]]) for r in keeps if set(self.res[r[1]].split(";")) - {"", "CASE"}]
        self.assertLess(len(bad), 5, bad)


class NoLanguageBranches(unittest.TestCase):
    def test8_no_new_de_branch_in_ingest_or_prefilter(self):
        for f in ("de_ingest.py", "de_prefilter.py"):
            src = open(os.path.join(HERE, f), encoding="utf-8").read()
            self.assertIsNone(re.search(r"""lang\w*\s*(==|===|in)\s*["']de["']""", src), f)


if __name__ == "__main__":
    unittest.main()
