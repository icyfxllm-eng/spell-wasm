"""CC-ZH-TONE Done 6 — the polyphone loopback.

Synthesize each word twice, with the pinyin phoneme forced and without, then
transcribe both and compare the READING back to what was intended. The
without-phoneme run is not a control for its own sake: it is the evidence that
the lever does work, and without it "30/30" only proves the words were
pronounceable.

The API key is read from the environment and never printed.
"""
import base64, html, json, os, pathlib, re, subprocess, sys, urllib.request

SP = pathlib.Path(__file__).resolve().parent
KEY = os.environ["GOOGLE_TTS_API_KEY"]
VOICE, LANG = "cmn-CN-Wavenet-A", "cmn-CN"
WORDS = json.loads((SP / "poly30.json").read_text())
OUT = SP / "clips"; OUT.mkdir(exist_ok=True)


def synth(ssml_or_text, is_ssml, path):
    body = {
        "input": ({"ssml": ssml_or_text} if is_ssml else {"text": ssml_or_text}),
        "voice": {"languageCode": LANG, "name": VOICE},
        # LINEAR16 at 16k is exactly what whisper wants, so no ffmpeg hop.
        "audioConfig": {"audioEncoding": "LINEAR16", "sampleRateHertz": 16000},
    }
    req = urllib.request.Request(
        f"https://texttospeech.googleapis.com/v1/text:synthesize?key={KEY}",
        data=json.dumps(body).encode(), headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=60) as r:
        path.write_bytes(base64.b64decode(json.load(r)["audioContent"]))


def transcribe(path):
    r = subprocess.run(
        ["/opt/homebrew/bin/whisper-cli", "-m", str(SP / "whisper/ggml-small.bin"),
         "-l", "zh", "-nt", "-np", "-f", str(path)],
        capture_output=True, text=True)
    return "".join(r.stdout.split())


def to_pinyin(hanzi):
    code = ("import sys,json\nfrom pypinyin import lazy_pinyin, Style\n"
            "print(json.dumps(lazy_pinyin(sys.argv[1], style=Style.TONE3, neutral_tone_with_five=True)))")
    r = subprocess.run([str(SP / "zhvenv/bin/python"), "-c", code, hanzi],
                       capture_output=True, text=True)
    return json.loads(r.stdout) if r.returncode == 0 else []


rows = []
for i, w in enumerate(WORDS, 1):
    ph = " ".join(re.findall(r"[a-zü]+[0-5]", w["surface"])).replace("v", "ü")
    ssml = f'<speak><phoneme alphabet="pinyin" ph="{html.escape(ph, quote=True)}">{html.escape(w["hanzi"])}</phoneme></speak>'
    a, b = OUT / f"{i:02d}-forced.wav", OUT / f"{i:02d}-bare.wav"
    synth(ssml, True, a)
    synth(w["hanzi"], False, b)
    want = [s for s in re.findall(r"[a-zü]+[0-5]", w["surface"])]
    want = [s.replace("v", "ü") for s in want]
    res = {}
    for tag, f in (("forced", a), ("bare", b)):
        txt = transcribe(f)
        got = [s for s in to_pinyin(txt)]
        res[tag] = {"text": txt, "pinyin": got, "match": got[:len(want)] == want}
    rows.append({**w, "want": want, **res})
    print(f'  {i:2}/{len(WORDS)} {w["hanzi"]:6} want {" ".join(want):18} '
          f'forced {"OK " if res["forced"]["match"] else "MISS"} ({" ".join(res["forced"]["pinyin"])[:22]})  '
          f'bare {"OK " if res["bare"]["match"] else "MISS"} ({" ".join(res["bare"]["pinyin"])[:22]})',
          flush=True)

(SP / "loopback-result.json").write_text(json.dumps(rows, ensure_ascii=False, indent=1))
f_ok = sum(r["forced"]["match"] for r in rows)
b_ok = sum(r["bare"]["match"] for r in rows)
print(f"\n  FORCED {f_ok}/{len(rows)}   BARE {b_ok}/{len(rows)}")
