#!/usr/bin/env python3
"""CC-HUMAN-AUDIO Phase B acceptance tests (the tool-side ones).

  2  license lint     a CC BY-NC clip is excluded by the harvester, with a reason
  3  homograph        a title spelled замок never matches the bank's замок (D5)
  4  inflection       a lemma recording (рука) never serves an inflected entry (руку)
  6  auto-QC          a clip of the wrong word is rejected by the loopback, logged
  7  decoy gate       a sheet with two accepted decoys is rejected at ingest
  9  D3 gate          a tier under 80% verified coverage ships nothing (whole
                      tier stays on TTS); at 80% it ships, with credits for BY

Test 6 needs whisper-cli, a model and macOS `say`; it skips, naming what is
missing, rather than passing. Run with the Phase B venv (numpy, soundfile,
pyloudnorm):  <venv>/bin/python -m unittest tools/human-audio/test_phase_b.py
Set HUMAN_AUDIO_WHISPER_MODEL to point at a ggml model for test 6.
"""
import csv
import hashlib
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "human-audio-census"))
import bundle  # noqa: E402
import harvest  # noqa: E402
import ingest  # noqa: E402
import sheet  # noqa: E402

MODEL = os.environ.get("HUMAN_AUDIO_WHISPER_MODEL",
                       os.path.expanduser("~/repos/ha-census-cache/phase-b/models/ggml-base.en.bin"))


def fixture_cache(root):
    """A miniature census cache: two English clips (one CC BY-NC), and two
    Russian clips (the homograph замок, the lemma рука against a bank that holds
    only the inflected руку)."""
    root = pathlib.Path(root)
    (root / "bank.tsv").write_text("en\teasy\tapple\nen\teasy\tbanana\n"
                                   "ru\tmedium\tзамок\nru\teasy\tруку\n")
    (root / "titles-eng.json").write_text(json.dumps([
        "File:LL-Q1860 (eng)-Spk-apple.wav", "File:LL-Q1860 (eng)-Spk-banana.wav"]))
    (root / "titles-rus.json").write_text(json.dumps([
        "File:LL-Q7737 (rus)-Rsp-замок.wav", "File:LL-Q7737 (rus)-Rsp-рука.wav"], ensure_ascii=False))

    def meta(lic, spk, upl, sha):
        return {"license": lic, "license_raw": lic or "cc-by-nc-4.0", "license_short": None,
                "speaker": spk, "uploader": upl, "sha1": sha, "duration": 0.8}
    (root / "meta.json").write_text(json.dumps({
        "File:LL-Q1860 (eng)-Spk-apple.wav": meta("cc0", "Q1", "Spk", "a" * 40),
        "File:LL-Q1860 (eng)-Spk-banana.wav": meta(None, "Q1", "Spk", "b" * 40),
        "File:LL-Q7737 (rus)-Rsp-замок.wav": meta("cc-by-sa", "Q2", "Rsp", "c" * 40),
        "File:LL-Q7737 (rus)-Rsp-рука.wav": meta("cc-by-sa", "Q2", "Rsp", "d" * 40),
    }, ensure_ascii=False))
    (root / "speakers.json").write_text(json.dumps({
        "Q1": {"langs": {"Q22": {"level": ["Q15"], "learned": ["P1"]}}, "residence": []},
        "Q2": {"langs": {"Q129": {"level": ["Q15"], "learned": ["P2"]}}, "residence": []},
    }))
    (root / "places.json").write_text(json.dumps({
        "P1": {"label": "Ohio", "country": ["Q30"]}, "P2": {"label": "Kazan", "country": ["Q159"]}}))
    (root / "homographs.json").write_text(json.dumps({"ru": ["замок"]}, ensure_ascii=False))
    return root


class Harvest(unittest.TestCase):
    def setUp(self):
        self.td = tempfile.TemporaryDirectory()
        self.cache = fixture_cache(self.td.name)

    def tearDown(self):
        self.td.cleanup()

    def test_2_license_nc_excluded_with_reason(self):
        accepted, excluded = harvest.select("en", self.cache)
        self.assertEqual([c["key"] for c in accepted], ["apple"])
        why = {k: r for k, _, r in excluded}
        self.assertIn("banana", why)
        self.assertTrue(why["banana"].startswith("license"), why["banana"])

    def test_3_homograph_never_matched(self):
        accepted, excluded = harvest.select("ru", self.cache)
        self.assertNotIn("замок", [c["key"] for c in accepted])
        self.assertIn(("замок", "File:LL-Q7737 (rus)-Rsp-замок.wav", "homograph/polyphone (D5)"), excluded)

    def test_4_lemma_never_serves_inflected_form(self):
        accepted, excluded = harvest.select("ru", self.cache)
        titles = [c["title"] for c in accepted] + [t for _, t, _ in excluded]
        self.assertNotIn("File:LL-Q7737 (rus)-Rsp-рука.wav", titles)
        self.assertNotIn("руку", [c["key"] for c in accepted])


def fixture_sheet(root, decoy_verdicts):
    """A three-real, three-decoy sheet with fake clip bytes. Real rows accept."""
    root = pathlib.Path(root)
    folder = root / "audit" / "en-easy-test"
    (folder / "clips").mkdir(parents=True)
    rows, pub = [], []
    for i in range(1, 7):
        name = f"r{i:04d}.m4a"
        (folder / "clips" / name).write_bytes(f"clip {i}".encode())
        digest = hashlib.sha256(f"clip {i}".encode()).hexdigest()
        decoy = i > 3
        rows.append({"row": i, "entry": f"word{i}", "clip": name, "clip_sha256": digest,
                     "decoy": decoy, "commons_sha1": f"{i:040d}", "speaker": "Spk", "license": "CC0"})
        pub.append({"row": i, "entry": f"word{i}", "clip": name, "clip_sha256": digest})
    st = sheet.stamp("en-easy-test", pub)
    key = root / "audit" / "en-easy-test-DECOY-KEY.json"
    key.write_text(json.dumps({"sheet_id": "en-easy-test", "lang": "en", "tier": "easy",
                               "stamp": st, "verdicts": sheet.VERDICTS, "rows": rows}))
    csv_path = root / "returned.csv"
    with open(csv_path, "w", newline="") as fh:
        fh.write(f"# sheet en-easy-test stamp {st}\n")
        w = csv.writer(fh)
        w.writerow(["row", "entry", "clip", "verdict", "note"])
        for r in rows:
            v = decoy_verdicts.pop(0) if r["decoy"] else "accept"
            w.writerow([r["row"], r["entry"], r["clip"], v, ""])
    return csv_path, folder, key


class DecoyGate(unittest.TestCase):
    def run_sheet(self, decoys):
        with tempfile.TemporaryDirectory() as td:
            return ingest.check(*fixture_sheet(td, list(decoys)))

    def test_7_two_missed_decoys_reject(self):
        out, problems = self.run_sheet(["accept", "accept", "wrong word"])
        self.assertTrue(any(p.startswith("DECOY") for p in problems), problems)

    def test_7_one_missed_decoy_tolerated(self):
        out, problems = self.run_sheet(["accept", "wrong word", "wrong word"])
        self.assertEqual(problems, [])
        self.assertEqual(len(out), 3)  # decoy rows are never written

    def test_7_all_caught(self):
        out, problems = self.run_sheet(["wrong word"] * 3)
        self.assertEqual(problems, [])


class BundleGate(unittest.TestCase):
    """D3: five easy entries; 4 accepted is 80% and ships, 3 is 60% and doesn't."""

    def build(self, accepted_n):
        td = tempfile.TemporaryDirectory()
        self.addCleanup(td.cleanup)
        root = pathlib.Path(td.name)
        work, assets = root / "work", root / "assets"
        (work / "clips" / "norm").mkdir(parents=True)
        (assets / "en").mkdir(parents=True)
        words = ["cat", "dog", "hat", "sun", "map"]
        (root / "bank.tsv").write_text("".join(f"en\teasy\t{w}\n" for w in words))
        clips, qc, entries = [], {}, {}
        target = json.loads((HERE.parents[1] / "config" / "audio-loudness.json").read_text())["target_lufs"]
        for i, w in enumerate(words):
            sha1 = f"{i:040d}"
            data = f"clip {w}".encode()
            (work / "clips" / "norm" / f"{sha1}.m4a").write_bytes(data)
            lic = "CC BY-SA" if w == "cat" else "CC0"
            clips.append({"key": w, "entry": w, "commons_sha1": sha1, "license": lic,
                          "speaker": "Spk", "page_url": f"https://example/{w}"})
            qc[sha1] = {"norm_path": f"clips/norm/{sha1}.m4a", "final_lufs": target, "status": "to_audit"}
            if i < accepted_n:
                entries[w] = [{"verdict": "accept", "clip_sha256": hashlib.sha256(data).hexdigest(),
                               "commons_sha1": sha1, "sheet": "t", "auditor": "eric"}]
        (work / "manifest.json").write_text(json.dumps({"lang": "en", "clips": clips}))
        (work / "qc.json").write_text(json.dumps({"clips": qc}))
        (assets / "en" / "verdicts.json").write_text(json.dumps({"lang": "en", "entries": entries}))
        (assets / "runtime.json").write_text(json.dumps({"version": 1, "langs": {}}))
        (assets / "credits.json").write_text(json.dumps({"version": 1, "clips": []}))
        res = bundle.main(["--lang", "en", "--work", str(work), "--bank", str(root / "bank.tsv"),
                           "--assets", str(assets), "--write"])
        return res, assets

    def test_9_under_80_percent_ships_nothing(self):
        res, assets = self.build(3)
        self.assertEqual(res["enabled"], [])
        self.assertEqual(list((assets / "en").glob("*.m4a")), [])
        self.assertNotIn("en", json.loads((assets / "runtime.json").read_text())["langs"])

    def test_9_at_80_percent_the_tier_ships_with_credits(self):
        res, assets = self.build(4)
        self.assertEqual(res["enabled"], ["easy"])
        self.assertEqual(sorted(res["ship"]), ["cat", "dog", "hat", "sun"])
        rt = json.loads((assets / "runtime.json").read_text())
        self.assertEqual(sorted(rt["langs"]["en"]["clips"]), ["cat", "dog", "hat", "sun"])
        credits = json.loads((assets / "credits.json").read_text())["clips"]
        self.assertEqual([c["entry"] for c in credits], ["cat"], "only the BY-SA clip needs a credit")


class AutoQC(unittest.TestCase):
    def test_6_wrong_word_rejected_by_loopback(self):
        missing = [n for n, ok in (("whisper-cli", pathlib.Path("/opt/homebrew/bin/whisper-cli").exists()),
                                   ("model " + MODEL, pathlib.Path(MODEL).exists()),
                                   ("say", shutil.which("say")), ("afconvert", shutil.which("afconvert")))
                   if not ok]
        if missing:
            self.skipTest("needs " + ", ".join(missing))
        with tempfile.TemporaryDirectory() as td:
            d = pathlib.Path(td)
            (d / "clips" / "raw").mkdir(parents=True)
            clips = []
            # Two clips that SAY "apple"; one is filed as "apple", one as "banana".
            for key, sha in (("apple", "1" * 40), ("banana", "2" * 40)):
                aiff = d / f"{sha}.aiff"
                subprocess.run(["say", "-o", str(aiff), "apple"], check=True)
                subprocess.run(["afconvert", "-f", "WAVE", "-d", "LEI16@48000", "-c", "1", str(aiff),
                                str(d / "clips" / "raw" / f"{sha}.wav")], check=True)
                clips.append({"key": key, "entry": key, "commons_sha1": sha,
                              "raw_path": f"clips/raw/{sha}.wav"})
            (d / "manifest.json").write_text(json.dumps({"lang": "en", "clips": clips}))
            subprocess.run([sys.executable, str(HERE / "qc.py"), "--dir", str(d), "--model", MODEL],
                           check=True, capture_output=True)
            qc = json.loads((d / "qc.json").read_text())["clips"]
            wrong = qc["2" * 40]
            self.assertEqual(wrong["status"], "rejected")
            self.assertTrue(any(r.startswith("loopback heard") for r in wrong["reasons"]), wrong)
            right = qc["1" * 40]
            self.assertFalse(any(r.startswith("loopback") for r in right["reasons"]), right)


if __name__ == "__main__":
    unittest.main()
