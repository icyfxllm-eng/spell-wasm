"""
Spell Game Backend — Flask + Google Cloud Text-to-Speech
Auth: API key (via environment variable GOOGLE_TTS_API_KEY)

Local (PyCharm): set GOOGLE_TTS_API_KEY in your Run Configuration
Replit: add GOOGLE_TTS_API_KEY in Tools -> Secrets
"""

import os
import re
import json
import hashlib
import urllib.parse
import urllib.request
from flask import Flask, request, jsonify, send_file
from flask_cors import CORS
from google.cloud import texttospeech

# ---------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------

API_KEY = os.environ.get("GOOGLE_TTS_API_KEY")
if not API_KEY:
    raise RuntimeError(
        "GOOGLE_TTS_API_KEY is not set. "
        "PyCharm: Run > Edit Configurations > Environment variables. "
        "Replit: Tools > Secrets."
    )

CACHE_DIR = os.environ.get("AUDIO_CACHE_DIR", "audio_cache")
os.makedirs(CACHE_DIR, exist_ok=True)

VOICE_NAME = "en-US-Neural2-D"
LANGUAGE_CODE = "en-US"

# Backend Google-TTS voice per built-in language. To add a language: add an
# entry here + its word bank on the client (words.rs) — audio + spelling then
# work end-to-end. (`en-US-Neural2-D` above stays the default / sentence voice.)
LANG_VOICES = {
    "en": ("en-US", "en-US-Neural2-D"),
    "es": ("es-ES", "es-ES-Neural2-B"),
    "fr": ("fr-FR", "fr-FR-Neural2-A"),
    "de": ("de-DE", "de-DE-Neural2-B"),
    "pt": ("pt-BR", "pt-BR-Neural2-B"),
    "it": ("it-IT", "it-IT-Neural2-A"),
    "nl": ("nl-NL", "nl-NL-Wavenet-B"),
    "pl": ("pl-PL", "pl-PL-Wavenet-B"),
    "sv": ("sv-SE", "sv-SE-Wavenet-C"),
    "nb": ("nb-NO", "nb-NO-Wavenet-B"),
    "tr": ("tr-TR", "tr-TR-Wavenet-B"),
    "vi": ("vi-VN", "vi-VN-Wavenet-A"),
    "ko": ("ko-KR", "ko-KR-Wavenet-A"),
    "ja": ("ja-JP", "ja-JP-Wavenet-B"),
}
DEFAULT_LANG = "en"

SPEAKING_RATE_NORMAL = 0.85  # slower, clearer enunciation
SPEAKING_RATE_SLOW = 0.6
VOLUME_GAIN_DB = 4.0  # louder baseline; stay well under the 16 max to avoid clipping
MAX_WORD_LENGTH = 45  # longest word in major dictionaries

# Bumped whenever synthesis settings change, so old cached clips (made with
# the previous rate/volume/SSML) are simply orphaned rather than reused —
# no need to delete anything on disk.
CACHE_VERSION = "v3"

DICTIONARY_API = "https://api.dictionaryapi.dev/api/v2/entries/en/{}"

app = Flask(__name__)

# Lock CORS to your frontend origin(s). Same-origin requests (frontend and
# backend served from the same domain via a reverse proxy) *usually* never
# hit CORS at all — except our <audio> elements set crossOrigin="anonymous"
# (needed for the volume-boost feature's Web Audio graph), which makes some
# browsers enforce a real CORS check even for a same-origin URL. Some
# privacy-hardened browsers' stricter isolation behavior is a plausible
# way to hit this in practice, so the production domain itself needs to be
# in this list too, not just treated as implicitly fine.
# ALLOWED_ORIGINS is a comma-separated list, e.g. "https://spell.example.com".
_extra_origins = [o.strip() for o in os.environ.get("ALLOWED_ORIGINS", "").split(",") if o.strip()]
CORS(app, origins=[
    "http://localhost:3000",
    "http://localhost:5173",
    "http://127.0.0.1:5000",
    "https://spellgame.net",
    # Capacitor native app (Android serves the bundled webview from
    # https://localhost, iOS from capacitor://localhost). The native audio
    # cache downloads clips via fetch(), which is CORS-enforced, so these
    # must be allowed for the on-device audio pack to work.
    "https://localhost",
    "capacitor://localhost",
    *_extra_origins,
])

# TTS client authenticated with the API key (no JSON file needed)
tts_client = texttospeech.TextToSpeechClient(
    client_options={"api_key": API_KEY}
)

# ---------- The Climb (accounts + leaderboard) ----------
# Same origin as the word API (Caddy proxies /api/* here), so no extra CORS.
import db  # noqa: E402
import climb  # noqa: E402

db.init()
app.register_blueprint(climb.bp)

# ---------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------

def validate_word(word: str):
    """Normalize and validate a word. Returns cleaned word or None."""
    word = word.lower().strip()
    if not word or len(word) > MAX_WORD_LENGTH:
        return None
    # Letters only — blocks injection of arbitrary text into TTS,
    # which protects both your quota and your cache directory.
    if not word.isalpha():
        return None
    return word


def cache_path_for(word: str, variant: str, lang: str = "en") -> str:
    # `lang` is part of the key so e.g. Spanish "casa" and English "casa" don't
    # share a clip. Existing English clips (no lang prefix) stay valid via "en".
    key = word if lang == "en" else f"{lang}:{word}"
    digest = hashlib.md5(key.encode()).hexdigest()
    return os.path.join(CACHE_DIR, f"{CACHE_VERSION}_{digest}_{variant}.mp3")


def _audio_config(speaking_rate: float) -> texttospeech.AudioConfig:
    return texttospeech.AudioConfig(
        audio_encoding=texttospeech.AudioEncoding.MP3,
        speaking_rate=speaking_rate,
        volume_gain_db=VOLUME_GAIN_DB,
        sample_rate_hertz=24000,
        effects_profile_id=["headphone-class-device"],
    )


def synthesize_to_cache(word: str, variant: str, path: str, lang: str = "en") -> None:
    """Call Google TTS once and store the MP3 permanently.

    Each variant is spoken once — the player already has a dedicated Repeat
    button for hearing a word again, so a built-in double-speak just means
    two automatic hearings before they've even asked for a repeat.
    """
    synthesis_input = texttospeech.SynthesisInput(text=word)
    rate = SPEAKING_RATE_SLOW if variant == "slow" else SPEAKING_RATE_NORMAL
    language_code, voice_name = LANG_VOICES.get(lang, LANG_VOICES[DEFAULT_LANG])

    response = tts_client.synthesize_speech(
        input=synthesis_input,
        voice=texttospeech.VoiceSelectionParams(
            language_code=language_code,
            name=voice_name,
        ),
        audio_config=_audio_config(rate),
    )
    with open(path, "wb") as f:
        f.write(response.audio_content)


def mask_word(text: str, word: str) -> str:
    """Replaces `word` and its common inflections with blanks, so a
    definition/sentence can be shown as a spelling hint without giving away
    the spelling itself."""
    if not text or not word:
        return text
    pattern = re.compile(rf"\b{re.escape(word)}(s|es|ed|d|ing|er|est)?\b", re.IGNORECASE)
    return pattern.sub("_____", text)


def meaning_cache_path(word: str) -> str:
    digest = hashlib.md5(word.encode()).hexdigest()
    return os.path.join(CACHE_DIR, f"meaning_v1_{digest}.json")


def fetch_meaning(word: str):
    """Looks up (and permanently caches, unmasked) {pos, definition,
    example} for `word` from dictionaryapi.dev. Returns None if the word
    isn't found or the lookup fails. Cached at rest without masking —
    masking is applied per-request in the route, so mask=0 (the post-answer
    reveal) and mask=1 (pre-answer hint) share one cache entry.
    """
    path = meaning_cache_path(word)
    if os.path.exists(path):
        with open(path, "r", encoding="utf-8") as f:
            return json.load(f)

    url = DICTIONARY_API.format(urllib.parse.quote(word))
    # dictionaryapi.dev 403s the default urllib User-Agent (bot-blocking) —
    # a normal-looking one is all it takes.
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0 (compatible; SpellGame/1.0)"})
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            data = json.loads(resp.read().decode("utf-8"))
        meaning = data[0]["meanings"][0]
        definition_obj = meaning["definitions"][0]
        result = {
            "pos": meaning.get("partOfSpeech", ""),
            "definition": definition_obj.get("definition", ""),
            "example": definition_obj.get("example", ""),
        }
    except Exception as e:
        app.logger.info(f"dictionary lookup failed for '{word}': {e}")
        return None

    with open(path, "w", encoding="utf-8") as f:
        json.dump(result, f)
    return result

# ---------------------------------------------------------------
# Routes
# ---------------------------------------------------------------

@app.route("/api/health")
def health():
    """Quick check that the server is up (doesn't call Google)."""
    return jsonify({"status": "ok"})


@app.route("/api/speak")
def speak():
    """Return MP3 audio for a word. Cached after first synthesis.

    `variant=normal` (default) speaks the word twice with a pause;
    `variant=slow` is a single slower utterance for careful listening.
    """
    word = validate_word(request.args.get("word", ""))
    if word is None:
        return jsonify({"error": "invalid word"}), 400
    variant = "slow" if request.args.get("variant") == "slow" else "normal"
    lang = request.args.get("lang", DEFAULT_LANG)
    if lang not in LANG_VOICES:
        lang = DEFAULT_LANG

    path = cache_path_for(word, variant, lang)

    if not os.path.exists(path):
        try:
            synthesize_to_cache(word, variant, path, lang)
        except Exception as e:
            app.logger.error(f"TTS failed for '{word}' ({variant}): {e}")
            return jsonify({"error": "speech synthesis failed"}), 502

    resp = send_file(path, mimetype="audio/mpeg")
    # Audio for a given word+variant never changes, so the browser (and any
    # CDN/tunnel edge in front of us) can cache it indefinitely — this is
    # also what makes next-word preloading actually pay off.
    resp.headers["Cache-Control"] = "public, max-age=31536000, immutable"
    return resp


@app.route("/api/check", methods=["POST"])
def check():
    """Compare the player's answer to the target word."""
    data = request.get_json(silent=True)
    if not data or "answer" not in data or "word" not in data:
        return jsonify({"error": "missing fields"}), 400

    answer = str(data["answer"]).lower().strip()
    word = str(data["word"]).lower().strip()

    return jsonify({"correct": answer == word})


@app.route("/api/meaning")
def meaning():
    """Definition + example sentence for a word, from dictionaryapi.dev.

    `mask=1` (default) blanks out the target word/inflections in both
    fields — used for the pre-answer Definition/Sentence hint buttons, so
    they can't be used to just read off the spelling. `mask=0` returns the
    real text, used for the post-answer reveal (the round is already over).
    """
    word = validate_word(request.args.get("word", ""))
    if word is None:
        return jsonify({"error": "invalid word"}), 400

    data = fetch_meaning(word)
    if data is None:
        return jsonify({"error": "not found"}), 404

    mask = request.args.get("mask", "1") != "0"
    pos, definition, example = data["pos"], data["definition"], data["example"]
    if mask:
        definition = mask_word(definition, word)
        example = mask_word(example, word)
    return jsonify({"pos": pos, "definition": definition, "example": example})


@app.route("/api/sentence-audio")
def sentence_audio():
    """Speaks the word's real (unmasked) example sentence — hearing a word
    used in a sentence doesn't give away its spelling, so unlike the on-
    screen text, the audio is never masked. The example itself comes from
    this word's own cached /api/meaning lookup, not client-supplied text,
    so there's no way to feed arbitrary text into TTS through this route.
    """
    word = validate_word(request.args.get("word", ""))
    if word is None:
        return jsonify({"error": "invalid word"}), 400

    data = fetch_meaning(word)
    example = data.get("example") if data else ""
    if not example:
        return jsonify({"error": "no example sentence"}), 404

    path = cache_path_for(word, "sentence")
    if not os.path.exists(path):
        try:
            response = tts_client.synthesize_speech(
                input=texttospeech.SynthesisInput(text=example),
                voice=texttospeech.VoiceSelectionParams(language_code=LANGUAGE_CODE, name=VOICE_NAME),
                audio_config=_audio_config(SPEAKING_RATE_NORMAL),
            )
            with open(path, "wb") as f:
                f.write(response.audio_content)
        except Exception as e:
            app.logger.error(f"TTS failed for sentence of '{word}': {e}")
            return jsonify({"error": "speech synthesis failed"}), 502

    resp = send_file(path, mimetype="audio/mpeg")
    resp.headers["Cache-Control"] = "public, max-age=31536000, immutable"
    return resp

# ---------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------

if __name__ == "__main__":
    # host="0.0.0.0" is required on Replit; fine locally too
    app.run(host="0.0.0.0", port=5000, debug=True)
