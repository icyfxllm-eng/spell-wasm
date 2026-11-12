#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""CC-DE-BANK F5 -- does the German voice voice s before a vowel?

The auditor heard weißen = weisen and heise = heiße. If the voice does not keep
s (voiced) and ß (voiceless) apart, those pairs are real homophones IN OUR AUDIO
and must be grouped (decision D4: handled in the data, never by changing grading
or this test). Render each pair, transcribe each clip with whisper.cpp, and
assert the two transcripts differ.

    python3 tools/bank/de/de_voicing_loopback.py            # renders via GOOGLE_TTS_API_KEY
    python3 tools/bank/de/de_voicing_loopback.py --clips DIR  # DIR/<word>.wav already rendered
    ... --record     # on a FAILED pair, add a bank-homophone line for each bank word using it

Fail-closed: no whisper, no model, and neither a key nor clips is an error with the
setup command, never a pass. This test is never edited to make a pair pass.
The API key is read from the environment and never printed.
"""
import base64, json, os, subprocess, sys, urllib.request, unicodedata

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", ".."))
PAIRS = [("reisen", "reißen"), ("weisen", "weißen"), ("Rose", "Rosse"), ("heiser", "heißer")]
WHISPER = os.environ.get("WHISPER_CLI", "/opt/homebrew/bin/whisper-cli")
MODEL = os.environ.get("WHISPER_MODEL", os.path.expanduser("~/.cache/whisper/ggml-small.bin"))
VOICE, LANG = "de-DE-Neural2-B", "de-DE"      # the registry voice (audit/de report: LANG_VOICES)


def norm(t):
    t = unicodedata.normalize("NFC", t).lower()
    return "".join(c for c in t if c.isalpha())


def distinct(a, b):
    """Two transcripts count as distinct when they are not the same letters."""
    return bool(norm(a)) and bool(norm(b)) and norm(a) != norm(b)


def synth(word, path):
    key = os.environ["GOOGLE_TTS_API_KEY"]
    body = {"input": {"text": word}, "voice": {"languageCode": LANG, "name": VOICE},
            "audioConfig": {"audioEncoding": "LINEAR16", "sampleRateHertz": 16000}}
    req = urllib.request.Request("https://texttospeech.googleapis.com/v1/text:synthesize?key=" + key,
                                 data=json.dumps(body).encode(), headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=60) as r:
        open(path, "wb").write(base64.b64decode(json.load(r)["audioContent"]))


def transcribe(path):
    r = subprocess.run([WHISPER, "-m", MODEL, "-l", "de", "-nt", "-np", "-f", path], capture_output=True, text=True)
    return " ".join(r.stdout.split())


def bank_words():
    out = set()
    for t in ("easy", "medium", "hard", "expert"):
        out |= {l.strip() for l in open(os.path.join(ROOT, "assets", "words", "de", t + ".txt"), encoding="utf-8") if l.strip()}
    return out


def record(pair):
    """D4: group the pair as homophones for any bank entry that uses it; carrier left for a human."""
    path = os.path.join(ROOT, "assets", "words", "de", "homophone-groups.tsv")
    have = open(path, encoding="utf-8").read()
    bank, add = bank_words(), []
    for w, other in (pair, pair[::-1]):
        if w in bank and not any(l.split("\t")[0] == w for l in have.split("\n")):
            add.append("%s\t%s\t" % (w, other))
    if add:
        open(path, "a", encoding="utf-8").write("\n".join(add) + "\n")
    return add


def main(argv):
    clips = argv[argv.index("--clips") + 1] if "--clips" in argv else None
    if not os.path.exists(WHISPER) or not os.path.exists(MODEL):
        raise SystemExit("whisper.cpp not available (WHISPER_CLI=%s, WHISPER_MODEL=%s)." % (WHISPER, MODEL))
    if not clips and "GOOGLE_TTS_API_KEY" not in os.environ:
        raise SystemExit("cannot render: no GOOGLE_TTS_API_KEY and no --clips DIR. Not a pass.")
    work = clips or os.path.join(os.environ.get("TMPDIR", "/tmp"), "de-voicing-clips")
    os.makedirs(work, exist_ok=True)
    failed = 0
    for a, b in PAIRS:
        heard = []
        for w in (a, b):
            p = os.path.join(work, w + ".wav")
            if not os.path.exists(p):
                synth(w, p)
            heard.append(transcribe(p))
        ok = distinct(*heard)
        print("%-6s %-8s / %-8s -> %r / %r" % ("PASS" if ok else "FAIL", a, b, heard[0], heard[1]))
        if not ok:
            failed += 1
            if "--record" in argv:
                print("       recorded:", record((a, b)) or "no bank entry uses this pair")
    raise SystemExit(1 if failed else 0)


if __name__ == "__main__":
    main(sys.argv)
