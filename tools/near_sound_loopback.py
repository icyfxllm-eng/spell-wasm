#!/usr/bin/env python3
"""CC-LAUNCH-EN F8 / A8.2 -- the near-sound loopback for English.

    python3 tools/near_sound_loopback.py            (needs the local server on :8080)

"were" typed for "worm" is a real miss, so the voice has to keep near-sounds
apart. For every pair in tests/fixtures/en-near-sounds.json both words are
synthesized by the Layer-1 voice through /api/speak (the clip the app plays)
and compared with the SAME measurement and the SAME per-run calibration as the
SENSE-CUE Phase B loopback (tools/soundalike_loopback.py): a same-sound control
sets the floor, a different-word control the ceiling, and the bar sits a fifth
of the way up. Lower DTW means more alike.

GATE: every pair's distance must be AT OR ABOVE the bar, i.e. the voice
separates the pair at least as well as Phase B requires before it would even
flag a true homophone set as "voice may distinguish". Exit 1 if any pair is
under the bar or has no audio.

Supporting evidence, not gated: whisper.cpp transcribes each clip and the
report records whether it heard the intended word (isolated single words are
hard for any recognizer; see the audio measurement limits note).

Writes reports/launch-en-near-sound-loopback.json.
"""
import importlib.util
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = pathlib.Path(__file__).resolve().parent.parent
_spec = importlib.util.spec_from_file_location("sl", ROOT / "tools" / "soundalike_loopback.py")
sl = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(sl)
vc = sl.vc

FIXTURE = ROOT / "tests" / "fixtures" / "en-near-sounds.json"
OUT = ROOT / "reports" / "launch-en-near-sound-loopback.json"
WHISPER = shutil.which("whisper-cli")
MODEL = pathlib.Path.home() / ".cache" / "whisper" / "ggml-small.bin"
MODEL_LARGE = pathlib.Path.home() / ".cache" / "whisper" / "ggml-large-v3.bin"
LEAD_IN = "the word is "   # VOICE-CLARITY: 124 English words are rendered with this lead-in


def true_homophones():
    """word -> its true-homophone set members (bored hears right for board)."""
    sa = json.loads((ROOT / "assets" / "words" / "en" / "soundalikes.json").read_text())
    out = {}
    for s in sa["sets"]:
        if s["kind"] == "true":
            ms = {m["memberId"] for m in s["members"]}
            for m in ms:
                out.setdefault(m, set()).update(ms)
    return out


HOMO = true_homophones()


def right(word, said):
    return said is not None and (said == word or said in HOMO.get(word, ()))


def heard(word, model=MODEL):
    """whisper.cpp's transcript of the cached clip for `word`, lowercased, lead-in stripped, or None."""
    wav = sl.CACHE / "en" / f"{word}.wav"
    if not (WHISPER and model.exists() and wav.exists()):
        return None
    with tempfile.TemporaryDirectory() as d:
        base = pathlib.Path(d) / "t"
        subprocess.run([WHISPER, "-m", str(model), "-l", "en", "-np", "-otxt", "-of", str(base), str(wav)],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        txt = base.with_suffix(".txt")
        if not txt.exists():
            return None
        raw = txt.read_text().lower()
        # whisper writes small numbers as digits ("3." for the word three)
        digits = "zero one two three four five six seven eight nine ten".split()
        raw = " ".join(digits[int(w)] if w.isdigit() and int(w) <= 10 else w
                       for w in "".join(c if c.isalnum() or c == " " else " " for c in raw).split())
        t = " ".join("".join(c for c in raw if c.isalpha() or c == " ").split())
        return t[len(LEAD_IN):] if t.startswith(LEAD_IN) else t


def hear(word):
    """(transcript, model) from the small model, falling back to large-v3 when small is not right."""
    t = heard(word)
    if right(word, t):
        return t, MODEL.name
    t2 = heard(word, MODEL_LARGE)
    return (t2, MODEL_LARGE.name) if t2 is not None else (t, MODEL.name)


def main():
    pairs = [(p["target"], p["near"]) for p in json.loads(FIXTURE.read_text())["pairs"]]
    feats = {}

    def F(w):
        if w not in feats:
            s = sl.audio(w, "en")
            feats[w] = vc.features(s) if s else None
        return feats[w]

    ctl = sl.CONTROLS["en"]
    same = [vc.dtw(F(a), F(b)) for a, b in ctl["same"] if F(a) and F(b)]
    diff = [vc.dtw(F(a), F(b)) for a, b in ctl["different"] if F(a) and F(b)]
    if not same or not diff:
        sys.exit("calibration failed: could not synthesize the control words (is the server on :8080?)")
    floor, ceiling = max(same), min(diff)
    bar = floor + (ceiling - floor) * 0.20

    rows, failed = [], []
    for a, b in pairs:
        fa, fb = F(a), F(b)
        if not fa or not fb:
            rows.append({"target": a, "near": b, "result": "no-audio"})
            failed.append(f"{a}/{b}: no audio")
            continue
        d = round(vc.dtw(fa, fb), 4)
        dtw_ok = d >= bar
        (ha, ma), (hb, mb) = hear(a), hear(b)
        # The whisper gate: each clip is heard as itself (or a true homophone of
        # itself) and never as its near-sound partner.
        w_ok = right(a, ha) and right(b, hb) and not right(b, ha) and not right(a, hb)
        rows.append({"target": a, "near": b, "dtw": d, "dtwPass": dtw_ok, "whisperPass": w_ok,
                     "whisperTarget": ha, "whisperTargetModel": ma, "whisperNear": hb, "whisperNearModel": mb})
        if not dtw_ok:
            failed.append(f"{a}/{b}: DTW {d} under bar {bar:.4f}")
        if not w_ok:
            failed.append(f"{a}/{b}: whisper heard {ha!r} / {hb!r}")
    store = {
        "$doc": "CC-LAUNCH-EN A8.2 near-sound loopback. Written by tools/near_sound_loopback.py. "
                "Gate: dtw >= bar for every pair. Whisper columns are evidence only.",
        "voice": sl.layer1_voices().get("en"), "measured": time.strftime("%Y-%m-%d"),
        "calibration": {"sameSoundFloor": round(floor, 4), "differentCeiling": round(ceiling, 4), "bar": round(bar, 4)},
        "pairs": len(pairs),
        "dtwPassed": sum(1 for r in rows if r.get("dtwPass")),
        "whisperPassed": sum(1 for r in rows if r.get("whisperPass")),
        "rows": rows,
    }
    OUT.write_text(json.dumps(store, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(f"en near-sounds: whisper {store['whisperPassed']}/{len(pairs)} pairs heard apart; "
          f"DTW {store['dtwPassed']}/{len(pairs)} at or above bar {bar:.4f} (floor {floor:.4f}, ceiling {ceiling:.4f})")
    for f in failed:
        print("  FAIL", f)
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
