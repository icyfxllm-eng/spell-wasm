"""
Spell Game Backend — Flask + Google Cloud Text-to-Speech (+ Azure Speech)

Auth:
  GOOGLE_TTS_API_KEY   — required; Google TTS for most languages.
  AZURE_SPEECH_KEY     — optional; Azure Speech for AZURE_VOICES langs (Swahili).
  AZURE_SPEECH_REGION  — optional; the Azure region, e.g. "eastus".

Local (PyCharm): set these in your Run Configuration.
Replit: add them in Tools -> Secrets.
"""

import os
import re
import json
import html
import threading
import time
import hashlib
import unicodedata
import urllib.error
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
# GOOGLE-synthesized languages. Keyed on the 2-letter study-language code the
# frontend sends. Reconciled with the active lineup (CC-MASTER-PARITY):
# it/nl/sv/nb/tr/th were cut and removed here; Russian added (ru-RU-Wavenet-D).
#
# Swahili is NOT here — it is synthesized via Azure instead (see AZURE_VOICES
# below): Azure has named, stable Swahili neural voices, which Google's TTS does
# not offer as confidently.
# ar (Arabic) — UNGATED (2026-07-25): Google's Arabic locale is ar-XA (MSA);
#   Wavenet-B is the male MSA voice with the clearest isolated-word enunciation.
# A lang in neither map falls back to the English voice (see the DEFAULT_LANG guard).
LANG_VOICES = {
    "en": ("en-US", "en-US-Neural2-D"),
    "es": ("es-ES", "es-ES-Neural2-B"),
    "fr": ("fr-FR", "fr-FR-Neural2-A"),
    "de": ("de-DE", "de-DE-Neural2-B"),
    "pt": ("pt-BR", "pt-BR-Neural2-B"),
    "pl": ("pl-PL", "pl-PL-Wavenet-B"),
    "ru": ("ru-RU", "ru-RU-Wavenet-D"),
    "vi": ("vi-VN", "vi-VN-Wavenet-A"),
    "ko": ("ko-KR", "ko-KR-Wavenet-A"),
    "ja": ("ja-JP", "ja-JP-Wavenet-B"),
    "fil": ("fil-PH", "fil-PH-Wavenet-A"),
    "zh": ("cmn-CN", "cmn-CN-Wavenet-A"),
    "ar": ("ar-XA", "ar-XA-Wavenet-B"),
    # hi (Hindi) — promoted 2026-07-25 (Eric's ruling); Google hi-IN neural voice.
    "hi": ("hi-IN", "hi-IN-Neural2-A"),
}

# AZURE-synthesized languages — for locales Google lacks a solid voice for.
# Azure Speech has named neural Swahili voices for both Kenya and Tanzania:
#   sw-KE (Kenya):    sw-KE-ZuriNeural (F), sw-KE-RafikiNeural (M)
#   sw-TZ (Tanzania): sw-TZ-RehemaNeural (F), sw-TZ-DaudiNeural (M)
# To switch variant/voice, change the entry below. Requires AZURE_SPEECH_KEY +
# AZURE_SPEECH_REGION (see below); if unset, a Swahili request 502s (caught,
# logged) rather than mispronouncing in English.
AZURE_VOICES = {
    "sw": ("sw-TZ", "sw-TZ-RehemaNeural"),
}
DEFAULT_LANG = "en"

SPEAKING_RATE_NORMAL = 0.85  # slower, clearer enunciation
SPEAKING_RATE_SLOW = 0.6
VOLUME_GAIN_DB = 4.0  # louder baseline; stay well under the 16 max to avoid clipping
# CC-AUDIO-CLARITY v1.1 F1 step 2 — digital silence around every clip, so a
# word's first and last sounds cannot be eaten by playback start-up or by a
# player's hardware. "half" is the reported case: /h/ and /f/ are the two
# lowest-energy sounds in English and there is no context to recover them from.
# Synthesized as SSML breaks rather than added afterwards, so the clip is still
# exactly what the provider produced -- F1 step 1 and I2 stay true.
# These two values are the only place padding is defined.
PAD_LEAD_MS = 200
PAD_TRAIL_MS = 150
MAX_WORD_LENGTH = 45  # longest word in major dictionaries

# Azure Speech (for AZURE_VOICES langs, e.g. Swahili). Optional: only needed if a
# request for an Azure-routed language comes in — a missing key/region raises at
# synth time (caught -> 502), it does not block startup like GOOGLE_TTS_API_KEY.
AZURE_SPEECH_KEY = os.environ.get("AZURE_SPEECH_KEY")
AZURE_SPEECH_REGION = os.environ.get("AZURE_SPEECH_REGION")  # e.g. "eastus"
# Google's numeric speaking_rate expressed as Azure SSML prosody rate (relative %).
AZURE_RATE = {"normal": "-15%", "slow": "-40%"}

# Bumped whenever synthesis settings change, so old cached clips (made with
# the previous rate/volume/SSML) are simply orphaned rather than reused —
# no need to delete anything on disk.
# v4: CC-AUDIO-CLARITY F1 -- every clip gains lead/trail silence and Swahili
# moves to 96 kbps, so no v3 clip may be served. Bumping this is what makes the
# re-cache total rather than gradual.
CACHE_VERSION = "v4"

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
import matches  # noqa: E402
import entitlements  # noqa: E402
import speak_metrics  # noqa: E402  (R8)

db.init()
app.register_blueprint(climb.bp)
app.register_blueprint(matches.bp)  # async 1v1 "Spell Off" (account-gated)
# Regional free-language grants, derived from the caller's Cloudflare edge
# country (no accounts, no tracking). The adapter — backend/entitlements.py — is
# the only place that reads the request header, and it resolves against the same
# country-language map the Rust core bundles. Ask it; never re-derive a country
# here (scripts/entitlement-core-purity-check.mjs enforces that).
app.register_blueprint(entitlements.bp)

# ---------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------

# Hyphen/apostrophe appear inside real words (e.g. Filipino "pang-uri",
# and apostrophes in some Latin scripts) — allowed alongside letters/marks.
_EXTRA_WORD_CHARS = set("-'’")


def _is_word_char(ch: str) -> bool:
    if ch in _EXTRA_WORD_CHARS:
        return True
    # Unicode Letters (L*) AND combining Marks (M*). The marks matter: Thai
    # vowel/tone signs (SARA I/II, MAI HAN-AKAT, …), Devanagari matras, etc. are
    # category Mn/Mc, so a plain str.isalpha() wrongly rejects any multi-syllable
    # Thai word — which is exactly why Thai "expert" words returned 400.
    return unicodedata.category(ch)[0] in ("L", "M")


def validate_word(word: str):
    """Normalize and validate a word. Returns cleaned word or None."""
    word = word.lower().strip()
    if not word or len(word) > MAX_WORD_LENGTH:
        return None
    # Letters + combining marks (+ hyphen/apostrophe) only — still blocks
    # whitespace, digits, and punctuation that could inject arbitrary text into
    # TTS (protecting both your quota and the cache), while accepting every
    # supported script.
    if not all(_is_word_char(ch) for ch in word):
        return None
    return word


MAX_PINYIN_LENGTH = 120
# Syllables of latin letters (v and ü both spell ü), each closed by a tone
# digit, single-space separated -- the shape Google's pinyin alphabet wants.
# Deliberately strict: this string is interpolated into SSML, so anything that
# is not pinyin has no business in it.
_PINYIN_RE = re.compile(r"^[a-zü]+[0-5](?: [a-zü]+[0-5])*$")


def validate_pinyin(py):
    """Normalize and validate a pinyin reading for <phoneme>. Returns it or None."""
    py = (py or "").strip().lower()
    if not py or len(py) > MAX_PINYIN_LENGTH:
        return None
    return py if _PINYIN_RE.match(py) else None


def zh_cache_key(py: str) -> str:
    """CC-ZH-TONE F6: keyed on the READING and the voice, never the hanzi.

    Pinyin keeps the cache stable across bank edits that do not change how a
    word sounds. The voice id fixes a latent bug older than this file: without
    it, changing a voice silently kept serving clips in the previous one.
    """
    _, voice_name = LANG_VOICES["zh"]
    return f"zh:{py}:{voice_name}"


def cache_path_for(word: str, variant: str, lang: str = "en", key_override: str = None) -> str:
    # `lang` is part of the key so e.g. Spanish "casa" and English "casa" don't
    # share a clip. Existing English clips (no lang prefix) stay valid via "en".
    key = key_override or (word if lang == "en" else f"{lang}:{word}")
    digest = hashlib.md5(key.encode()).hexdigest()
    return os.path.join(CACHE_DIR, f"{CACHE_VERSION}_{digest}_{variant}.mp3")


def _breaks() -> tuple:
    """The lead and trail silence, as SSML. One definition, three call sites."""
    return (
        f'<break time="{PAD_LEAD_MS}ms"/>',
        f'<break time="{PAD_TRAIL_MS}ms"/>',
    )


def _padded_ssml(body: str) -> str:
    """Wrap already-escaped SSML body in <speak> with F1's padding."""
    lead, trail = _breaks()
    return f"<speak>{lead}{body}{trail}</speak>"


def _audio_config(speaking_rate: float) -> texttospeech.AudioConfig:
    return texttospeech.AudioConfig(
        audio_encoding=texttospeech.AudioEncoding.MP3,
        speaking_rate=speaking_rate,
        volume_gain_db=VOLUME_GAIN_DB,
        sample_rate_hertz=24000,
        effects_profile_id=["headphone-class-device"],
    )


def _synthesize_azure(word: str, variant: str, path: str, lang: str) -> None:
    """Synthesize via Azure Speech REST and store the MP3. Used for AZURE_VOICES
    languages (Swahili) that Google TTS lacks a solid voice for. Output format and
    on-disk shape match the Google path exactly (24 kHz mono MP3), so caching,
    serving, and the volume-boost graph are all identical downstream."""
    if not (AZURE_SPEECH_KEY and AZURE_SPEECH_REGION):
        raise RuntimeError(
            f"Azure Speech not configured — set AZURE_SPEECH_KEY and "
            f"AZURE_SPEECH_REGION to synthesize '{lang}'."
        )
    language_code, voice_name = AZURE_VOICES[lang]
    rate = AZURE_RATE["slow" if variant == "slow" else "normal"]
    # Azure's prosody volume REJECTS dB values (HTTP 400 — bisected 2026-07-28);
    # it accepts percentages, so convert the shared gain: +4dB ≈ +59%.
    volume_pct = round((10 ** (VOLUME_GAIN_DB / 20) - 1) * 100)
    # word is pre-validated to letters/marks/hyphen/apostrophe, but escape anyway.
    ssml = (
        f"<speak version='1.0' xml:lang='{language_code}'>"
        f"<voice xml:lang='{language_code}' name='{voice_name}'>"
        f"<prosody rate='{rate}' volume='+{volume_pct}%'>"
        f"{_breaks()[0]}{html.escape(word, quote=False)}{_breaks()[1]}"
        f"</prosody></voice></speak>"
    )
    url = f"https://{AZURE_SPEECH_REGION}.tts.speech.microsoft.com/cognitiveservices/v1"
    req = urllib.request.Request(
        url,
        data=ssml.encode("utf-8"),
        method="POST",
        headers={
            "Ocp-Apim-Subscription-Key": AZURE_SPEECH_KEY,
            "Content-Type": "application/ssml+xml",
            "X-Microsoft-OutputFormat": "audio-24khz-96kbitrate-mono-mp3",
            "User-Agent": "SpellGame",  # Azure rejects requests with no User-Agent
        },
    )
    with urllib.request.urlopen(req, timeout=15) as resp:
        audio = resp.read()
    with open(path, "wb") as f:
        f.write(audio)


def _synthesize_zh(word: str, py: str, variant: str, path: str) -> None:
    """CC-ZH-TONE F6 — Mandarin, with the reading forced.

    An isolated word carries no context, so the TTS frontend guesses polyphone
    readings: 行 xíng/háng, 长 cháng/zhǎng, 重 zhòng/chóng, 了 le/liǎo. That is
    the real cause of most wrong-sounding Mandarin here, and it gets
    misdiagnosed as a tone problem. Naming the reading removes the guess.

    Google's pinyin alphabet takes numeric tones at the end of each syllable,
    whitespace between syllables -- their own example is /wo3 de5/.
    """
    language_code, voice_name = LANG_VOICES["zh"]
    rate = SPEAKING_RATE_SLOW if variant == "slow" else SPEAKING_RATE_NORMAL
    ssml = _padded_ssml(
        "<phoneme alphabet=\"pinyin\" ph=\"{}\">{}</phoneme>".format(
            html.escape(py, quote=True), html.escape(word, quote=False)
        )
    )
    response = tts_client.synthesize_speech(
        input=texttospeech.SynthesisInput(ssml=ssml),
        voice=texttospeech.VoiceSelectionParams(language_code=language_code, name=voice_name),
        audio_config=_audio_config(rate),
    )
    with open(path, "wb") as f:
        f.write(response.audio_content)


def synthesize_to_cache(word: str, variant: str, path: str, lang: str = "en", py: str = None) -> None:
    """Synthesize a word once and store the MP3 permanently. Routes to Azure for
    AZURE_VOICES languages, otherwise Google.

    Each variant is spoken once — the player already has a dedicated Repeat
    button for hearing a word again, so a built-in double-speak just means
    two automatic hearings before they've even asked for a repeat.
    """
    if lang == "zh":
        # Invariant 4: zh is NEVER synthesized from bare Hanzi. A missing or
        # malformed reading is an error, not a quiet fall back to guessing.
        if not py:
            raise ValueError("zh synthesis requires a pinyin reading (CC-ZH-TONE F6)")
        _synthesize_zh(word, py, variant, path)
        return

    if lang in AZURE_VOICES:
        _synthesize_azure(word, variant, path, lang)
        return

    synthesis_input = texttospeech.SynthesisInput(ssml=_padded_ssml(html.escape(word, quote=False)))
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


def meaning_cache_path(word: str, lang: str = "en") -> str:
    # `lang` in the key so es "casa" and pt "casa" don't share an entry; the
    # bare-word key keeps every existing English cache file valid.
    key = word if lang == "en" else f"{lang}:{word}"
    digest = hashlib.md5(key.encode()).hexdigest()
    return os.path.join(CACHE_DIR, f"meaning_v1_{digest}.json")


# Non-English definitions (2026-07-27, Eric's "build the definitions"):
# en.wiktionary's REST definition endpoint, PROXIED here — the same privacy
# posture as English (a child's word+IP only ever reaches our own server;
# our server talks to Wiktionary). Section keys are Wiktionary language codes;
# fil maps to tl (Tagalog). zh is ABSENT: the endpoint omits Chinese sections
# (verified 2026-07-27), so Chinese keeps no definition rather than a wrong one.
# Content is CC BY-SA — attribution in NOTICES.md and the app's meaning card.
MEANING_LANGS = {
    "es": "es", "fr": "fr", "de": "de", "pt": "pt", "pl": "pl", "vi": "vi",
    "ko": "ko", "ja": "ja", "ru": "ru", "ar": "ar", "hi": "hi", "sw": "sw",
    "fil": "tl",
}
WIKTIONARY_DEF = "https://en.wiktionary.org/api/rest_v1/page/definition/{}"

# Chinese: Wiktionary's endpoint omits zh sections, so glosses come from a
# BUNDLED CC-CEDICT extract (backend/zh_glosses.json, built by
# scripts/build-zh-glosses.py; CC BY-SA, attribution in NOTICES.md). Keyed by
# the simplified hanzi — the client sends the hanzi, not the typed pinyin.
try:
    with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "zh_glosses.json"), encoding="utf-8") as _f:
        ZH_GLOSSES = json.load(_f)
except OSError:
    ZH_GLOSSES = {}


def fetch_meaning_zh(word: str):
    g = ZH_GLOSSES.get(word)
    if not g:
        return None
    return {"pos": "", "definition": g["definition"], "example": ""}
_TAG_RE = re.compile(r"<[^>]+>")


def _strip_html(text: str) -> str:
    import html as _html
    return _html.unescape(_TAG_RE.sub("", text or "")).strip()


def fetch_meaning_wiktionary(word: str, lang: str):
    """{pos, definition, example} for a non-English word from en.wiktionary's
    definition endpoint (English glosses — short and kid-friendly). Permanently
    cached unmasked, like the English path. None when the word has no section
    for `lang` or the lookup fails."""
    path = meaning_cache_path(word, lang)
    if os.path.exists(path):
        with open(path, "r", encoding="utf-8") as f:
            return json.load(f)

    section = MEANING_LANGS[lang]
    url = WIKTIONARY_DEF.format(urllib.parse.quote(word, safe=""))
    req = urllib.request.Request(url, headers={"User-Agent": "SpellGame/1.0 (spellgame.net)"})
    result = None
    # Grammar cross-references ("verbal noun of X", "plural of X") aren't kid
    # definitions — skip them and take the first CONTENT sense; fall back to the
    # first form-of only when nothing else exists.
    form_of = re.compile(
        r"^(verbal noun|plural|inflection|alternative (?:form|spelling)|romanization|"
        r"feminine|masculine|diminutive|misspelling|obsolete (?:form|spelling)) of\b",
        re.IGNORECASE,
    )
    fallback = None
    try:
        with urllib.request.urlopen(req, timeout=6) as resp:
            data = json.loads(resp.read().decode("utf-8"))
        for entry in data.get(section, []):
            for d in entry.get("definitions", []):
                definition = _strip_html(d.get("definition", ""))
                if not definition:
                    continue
                if form_of.match(definition):
                    if fallback is None:
                        fallback = {
                            "pos": (entry.get("partOfSpeech") or "").lower(),
                            "definition": definition,
                            "example": "",
                        }
                    continue
                examples = d.get("parsedExamples") or []
                example = _strip_html(examples[0].get("example", "")) if examples else ""
                if not example:
                    raw = d.get("examples") or []
                    example = _strip_html(raw[0]) if raw else ""
                result = {
                    "pos": (entry.get("partOfSpeech") or "").lower(),
                    "definition": definition,
                    "example": example,
                }
                break
            if result:
                break
    except Exception as e:
        app.logger.info(f"wiktionary lookup failed for '{lang}:{word}': {e}")
        return None
    if result is None:
        result = fallback

    if result:
        with open(path, "w", encoding="utf-8") as f:
            json.dump(result, f, ensure_ascii=False)
    return result


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
    # CC-TELEMETRY-FOUNDATION R8: one metrics line per request (lang, variant,
    # cache hit/miss, status, ms). No IP and no word; see speak_metrics.py.
    t0 = time.perf_counter()
    meta = {"lang": "other", "variant": "normal", "cache": "miss"}
    resp, status = _speak(meta)
    speak_metrics.record(meta["lang"], meta["variant"], meta["cache"], status, (time.perf_counter() - t0) * 1000)
    return resp, status


def _speak(meta):
    word = validate_word(request.args.get("word", ""))
    if word is None:
        return jsonify({"error": "invalid word"}), 400
    variant = "slow" if request.args.get("variant") == "slow" else "normal"
    lang = request.args.get("lang", DEFAULT_LANG)
    if lang not in LANG_VOICES and lang not in AZURE_VOICES:
        lang = DEFAULT_LANG
    meta["lang"], meta["variant"] = lang, variant

    # F6: Mandarin must name its reading. No reading, no audio -- guessing is
    # what this feature exists to stop.
    py = validate_pinyin(request.args.get("py", "")) if lang == "zh" else None
    if lang == "zh" and py is None:
        return jsonify({"error": "zh requires a valid pinyin reading"}), 400

    path = cache_path_for(word, variant, lang, zh_cache_key(py) if py else None)

    if os.path.exists(path):
        meta["cache"] = "hit"
    else:
        try:
            synthesize_to_cache(word, variant, path, lang, py)
        except Exception as e:
            app.logger.error(f"TTS failed for '{word}' ({variant}): {e}")
            return jsonify({"error": "speech synthesis failed"}), 502

    resp = send_file(path, mimetype="audio/mpeg")
    # Audio for a given word+variant never changes, so the browser (and any
    # CDN/tunnel edge in front of us) can cache it indefinitely — this is
    # also what makes next-word preloading actually pay off.
    resp.headers["Cache-Control"] = "public, max-age=31536000, immutable"
    return resp, 200


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

    lang = (request.args.get("lang") or "en").split("-")[0].lower()
    if lang == "en":
        data = fetch_meaning(word)
    elif lang in ("zh", "cmn"):
        data = fetch_meaning_zh(word)
    elif lang in MEANING_LANGS:
        data = fetch_meaning_wiktionary(word, lang)
    else:
        data = None
    if data is None:
        return jsonify({"error": "not found"}), 404

    mask = request.args.get("mask", "1") != "0"
    pos, definition, example = data["pos"], data["definition"], data["example"]
    if mask:
        definition = mask_word(definition, word)
        example = mask_word(example, word)
    return jsonify({"pos": pos, "definition": definition, "example": example})


DEF_POOLS_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "def_pools")


@app.route("/api/defpool")
def defpool():
    """CC-DEF-MATCH round data: the prescreened definition pool + exclusion
    sets for one (lang, tier), built by scripts/build-def-pools.py. The Rust
    core consumes this verbatim and does ALL selection — this route never
    chooses content. Rows carry prompt_grade/kid_register flags; the core
    filters (D1/Invariant 6)."""
    lang = (request.args.get("lang") or "").split("-")[0].lower()
    tier = request.args.get("tier") or ""
    if tier not in ("easy", "medium", "hard", "expert"):
        return jsonify({"error": "bad tier"}), 400
    path = os.path.join(DEF_POOLS_DIR, f"{lang}.json")
    if not os.path.isfile(path):
        return jsonify({"error": "no pool"}), 404
    with open(path, encoding="utf-8") as f:
        data = json.load(f)
    rows = data.get("tiers", {}).get(tier, [])
    # Trim exclusions to the words actually present in this tier's rows.
    words = {r["word"] for r in rows}
    excl = {w: [x for x in xs if x in words] for w, xs in data.get("exclusions", {}).items() if w in words}
    resp = jsonify({"lang": lang, "tier": tier, "rows": rows, "exclusions": excl})
    resp.headers["Cache-Control"] = "public, max-age=3600"
    return resp


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

    lang = (request.args.get("lang") or "en").split("-")[0].lower()
    if lang == "en":
        data = fetch_meaning(word)
    elif lang in MEANING_LANGS:
        data = fetch_meaning_wiktionary(word, lang)
    else:
        data = None
    example = data.get("example") if data else ""
    if not example:
        return jsonify({"error": "no example sentence"}), 404

    # The example's own language's Google voice (Swahili is Azure-only for
    # words; its rare wiktionary examples fall back to the English voice
    # rather than growing an Azure sentence path here).
    v_lang, v_name = LANG_VOICES.get(lang, (LANGUAGE_CODE, VOICE_NAME))
    path = cache_path_for(word, "sentence", lang)
    if not os.path.exists(path):
        try:
            response = tts_client.synthesize_speech(
                input=texttospeech.SynthesisInput(text=example),
                voice=texttospeech.VoiceSelectionParams(language_code=v_lang, name=v_name),
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
# Notify Me — anonymous per-language interest for coming-soon languages.
# No email / account / PII: just a de-duplicated tap count (unique anonymous
# install ids) per language, so Eric can rank which languages return first.
# Client queues + retries these, so we may receive the same tap more than once.
# ---------------------------------------------------------------
NOTIFY_PATH = os.path.join(CACHE_DIR, "notify_interest_v1.json")
_notify_lock = threading.Lock()


def _load_notify() -> dict:
    try:
        with open(NOTIFY_PATH, encoding="utf-8") as f:
            return json.load(f)
    except (OSError, ValueError):
        return {}


# ---- Server speech-to-text (mic-everywhere, Eric 2026-07-28) -------------
# ONLY for languages with no on-device iOS model (today: sw, fil — the map
# below still covers all 15 defensively). The client shows an explicit
# internet-consent card first and NEVER offers this in Kid Mode. Audio is
# recognized and discarded — nothing is written to disk or logged.
STT_LANGS = {
    "en": "en-US", "es": "es-ES", "fr": "fr-FR", "de": "de-DE",
    "pt": "pt-BR", "pl": "pl-PL", "ru": "ru-RU", "vi": "vi-VN",
    "ko": "ko-KR", "ja": "ja-JP", "fil": "fil-PH", "zh": "cmn-Hans-CN",
    "ar": "ar-SA", "hi": "hi-IN", "sw": "sw-TZ",  # matches the sw-TZ voice (Eric, 2026-09-18)
}
STT_MAX_AUDIO_B64 = 1_400_000  # ~1MB PCM ≈ 30s @16k mono — letters are ~2s


@app.route("/api/stt", methods=["POST"])
def stt():
    j = request.get_json(force=True, silent=True) or {}
    lang = j.get("lang", "")
    code = STT_LANGS.get(lang)
    audio_b64 = j.get("audio", "")
    sample_rate = int(j.get("sampleRate", 16000))
    phrases = [p for p in (j.get("phrases") or []) if isinstance(p, str)][:100]
    if not code or not audio_b64:
        return jsonify({"error": "bad request"}), 400
    if len(audio_b64) > STT_MAX_AUDIO_B64:
        return jsonify({"error": "audio too long"}), 413
    body = {
        "config": {
            "encoding": "LINEAR16",
            "sampleRateHertz": sample_rate,
            "languageCode": code,
            "maxAlternatives": 2,
            "profanityFilter": True,
        },
        "audio": {"content": audio_b64},
    }
    if phrases:
        body["config"]["speechContexts"] = [{"phrases": phrases, "boost": 10}]
    req = urllib.request.Request(
        "https://speech.googleapis.com/v1/speech:recognize?key=" + API_KEY,
        data=json.dumps(body).encode("utf-8"),
        headers={"Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(req, timeout=12) as resp:
            data = json.loads(resp.read().decode("utf-8"))
    except urllib.error.HTTPError as e:
        detail = e.read().decode("utf-8", "replace")[:200]
        return jsonify({"error": "stt upstream", "detail": detail}), 502
    except Exception:
        return jsonify({"error": "stt unreachable"}), 502
    results = data.get("results") or []
    if not results:
        return jsonify({"transcript": "", "confidence": 0.0, "alt": ""})
    alts = results[0].get("alternatives") or [{}]
    return jsonify({
        "transcript": alts[0].get("transcript", ""),
        "confidence": float(alts[0].get("confidence", 0.0)),
        "alt": (alts[1].get("transcript", "") if len(alts) > 1 else ""),
    })


@app.route("/api/notify", methods=["POST"])
def notify_interest():
    data = request.get_json(silent=True) or {}
    lang = str(data.get("language", ""))
    install = str(data.get("id", ""))[:64]
    if not re.fullmatch(r"[a-z]{2,3}", lang):
        return jsonify({"error": "invalid language"}), 400
    with _notify_lock:
        store = _load_notify()
        ids = store.setdefault(lang, [])
        if install and install not in ids:  # de-dupe retries / repeat taps
            ids.append(install)
        with open(NOTIFY_PATH, "w", encoding="utf-8") as f:
            json.dump(store, f)
    return jsonify({"status": "ok"})


@app.route("/api/notify/totals")
def notify_totals():
    # Admin read: unique-install tap count per language, for demand ranking.
    store = _load_notify()
    return jsonify({lang: len(ids) for lang, ids in store.items()})


# ---------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------

if __name__ == "__main__":
    # host="0.0.0.0" is required on Replit; fine locally too
    app.run(host="0.0.0.0", port=5000, debug=True)
