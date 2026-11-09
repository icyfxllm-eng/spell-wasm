"""Gig A sheet + ingest round trip (A15.2, A15.3, A15.4, A15.6). Stdlib only.

    python3 -m unittest tools/defs/test_gen_sheet.py
"""
import csv
import json
import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import gen_ingest  # noqa: E402
import gen_sheet  # noqa: E402

TIERS = ["easy", "medium", "hard", "expert"]


class RoundTrip(unittest.TestCase):
    def setUp(self):
        self.tmp = pathlib.Path(tempfile.mkdtemp())
        self.run = self.tmp / "run"
        self.run.mkdir()
        excluded = gen_sheet.lines(gen_sheet.ROOT / "assets" / "words" / "kid-exclude" / "en.txt")[0]
        words = [excluded] + [f"w{i:04d}" for i in range(1499)]
        with open(self.run / "rows.csv", "w", newline="", encoding="utf-8") as f:
            w = csv.writer(f)
            w.writerow(["word", "tier", "pos", "synset", "swapped", "meaning_used", "field", "text", "status",
                        "reasons", "source", "source_license"])
            for i, word in enumerate(words):
                t = TIERS[i % 4]
                dark = i == 7
                w.writerow([word, t, "noun", "", "", "m", "definition", "" if dark else f"Def {i}.",
                            "dark" if dark else "ok", "", "spell-gen-gen-v2", "original"])
                w.writerow([word, t, "noun", "", "", "m", "sentence", f"Sentence {i} here.", "ok", "",
                            "spell-gen-gen-v2", "original"])
        self.keys = self.tmp / "keys"
        self.out, self.meta = gen_sheet.build(self.run, self.tmp / "out", self.keys, sheet_id="en-gen-test")
        self.key = json.loads((self.keys / "en-gen-test-DECOY-KEY.json").read_text())["decoys"]

    def fill(self, name="rated.csv", rate=lambda r, f: "0", kid_only=False):
        src = self.out / "sheet.csv"
        with open(src, encoding="utf-8") as f:
            head = f.readline()
            rows = list(csv.DictReader(f))
        dec = {d["row"]: d for d in self.key}
        for r in rows:
            for f in ("definition", "sentence"):
                if not r[f] or (kid_only and r["kid_visible"] != "yes"):
                    continue
                d = dec.get(int(r["row"]))
                r[f + "_severity"] = ("2" if d["severe"] else "1") if d and d["field"] == f else rate(r, f)
        p = self.out / name
        with open(p, "w", newline="", encoding="utf-8") as f:
            f.write(head)
            w = csv.DictWriter(f, fieldnames=gen_sheet.HEADER)
            w.writeheader()
            w.writerows(rows)
        return p

    def ingest(self, p, eric=None, apply=False):
        return gen_ingest.ingest(p, eric, apply, keys_dir=self.keys)

    def test_sheet_shape(self):
        self.assertEqual(self.meta["sourceRows"], 1500)
        self.assertGreaterEqual(self.meta["severeDecoys"], gen_sheet.MIN_SEVERE)
        chunks = {d["chunk"] for d in self.key}
        self.assertEqual(len(chunks), len(self.key), "one decoy per chunk")
        self.assertEqual(self.meta["darkFields"], 1)

    def test_clean_pass_accepts_and_marks_reviewed(self):
        res = self.ingest(self.fill(), self.fill("eric.csv", kid_only=True), apply=True)
        self.assertEqual(res["verdict"], "ACCEPT", res)
        out = json.loads((self.out / "reviewed.en.json").read_text())
        self.assertEqual(len(out), 1500)
        dark = [o for o in out if not o["definition"]]
        self.assertEqual(len(dark), 1)
        self.assertFalse(dark[0]["reviewed_definition"])
        self.assertTrue(all(o["reviewed_sentence"] for o in out))

    def test_kid_row_needs_eric(self):
        res = self.ingest(self.fill(), apply=True)
        self.assertEqual(res["verdict"], "ACCEPT")
        out = json.loads((self.out / "reviewed.en.json").read_text())
        self.assertFalse(any(o["reviewed_definition"] for o in out if o["kid_visible"]))
        self.assertTrue(all(o["reviewed_definition"] for o in out if not o["kid_visible"]))

    def test_eric_flag_blocks_kid_row_A15_4(self):
        target = next(r for r in self._rows() if r["kid_visible"] == "yes" and r["word"].startswith("w"))
        eric = self.fill("eric.csv", rate=lambda r, f: "1" if r["row"] == target["row"] and f == "sentence" else "0",
                         kid_only=True)
        self.ingest(self.fill(), eric, apply=True)
        out = {o["word"]: o for o in json.loads((self.out / "reviewed.en.json").read_text())}
        self.assertFalse(out[target["word"]]["reviewed_sentence"])
        self.assertTrue(out[target["word"]]["reviewed_definition"])

    def _rows(self):
        with open(self.out / "sheet.csv", encoding="utf-8") as f:
            f.readline()
            return list(csv.DictReader(f))

    def _miss(self, decoys):
        rows = {d["row"] for d in decoys}
        p = self.fill()
        text = p.read_text().splitlines(True)
        head, rows_ = text[0], list(csv.DictReader(text[1:]))
        for r in rows_:
            if int(r["row"]) in rows:
                for f in ("definition", "sentence"):
                    if r[f]:
                        r[f + "_severity"] = "0"
        with open(p, "w", newline="", encoding="utf-8") as f:
            f.write(head)
            w = csv.DictWriter(f, fieldnames=gen_sheet.HEADER)
            w.writeheader()
            w.writerows(rows_)
        return self.ingest(p)

    def test_severe_decoy_missed_rejects(self):
        res = self._miss([d for d in self.key if d["severe"]][:1])
        self.assertEqual(res["verdict"], "REJECT")
        self.assertIn("severe", res["errors"][0])

    def test_one_mild_miss_requeues_its_chunk(self):
        d = [d for d in self.key if not d["severe"]][0]
        res = self._miss([d])
        self.assertEqual(res["verdict"], "ACCEPT", res)
        self.assertEqual(res["requeuedChunks"], [d["chunk"]])

    def test_three_mild_misses_reject(self):
        res = self._miss([d for d in self.key if not d["severe"]][:3])
        self.assertEqual(res["verdict"], "REJECT")

    def test_empty_severity_rejects_A15_2(self):
        res = self.ingest(self.fill(rate=lambda r, f: "" if r["row"] == "3" else "0"))
        self.assertEqual(res["verdict"], "REJECT")
        self.assertTrue(any("COMPLETE" in e for e in res["errors"]))

    def test_edited_text_rejects(self):
        p = self.fill()
        p.write_text(p.read_text().replace("Sentence 12 here.", "Sentence 12 changed."))
        res = self.ingest(p)
        self.assertTrue(any("STAMP" in e for e in res["errors"]))

    def test_source_row_added_after_export_rejects_A15_3(self):
        with open(self.run / "rows.csv", "a", newline="", encoding="utf-8") as f:
            csv.writer(f).writerow(["zzz", "easy", "noun", "", "", "m", "definition", "Late.", "ok", "",
                                    "spell-gen-gen-v2", "original"])
        res = self.ingest(self.fill())
        self.assertTrue(any("SOURCE" in e for e in res["errors"]))

    def test_spotcheck_over_5pct_voids_A15_6(self):
        eric = self.fill("eric.csv", rate=lambda r, f: "1" if int(r["row"]) % 10 == 0 else "0", kid_only=True)
        res = self.ingest(self.fill(), eric)
        self.assertEqual(res["verdict"], "VOID")


if __name__ == "__main__":
    unittest.main()
