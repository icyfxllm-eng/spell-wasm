// NativeLanguageKit — the SINGLE web-layer interface to the iOS native language
// capabilities (offline TTS, on-device word validation, language detection).
//
// This is the anti-silent-fallback boundary: it loads on EVERY platform and
// exposes `window.SpellNativeLang`. On iOS (Capacitor + the NativeLanguageKit
// plugin present) it routes to native; on web/PWA/Tauri/Android every capability
// resolves to an explicit `available:false` from this same interface — no shims,
// no platform conditionals leaking into game logic. The Rust core reflects on
// `window.SpellNativeLang` exactly like it does `window.SpellAudio`.
//
// Loaded via a classic <script> in index.html BEFORE the WASM module, and copied
// into dist/ by scripts/build-web.sh. There is no TypeScript build in this repo,
// so this JS file with full JSDoc IS the interface contract.
//
// @typedef {{ id:string, name:string, quality:('default'|'enhanced'|'premium') }} VoiceInfo
// @typedef {{ tts:{available:boolean, voices:VoiceInfo[]},
//             spellcheck:{available:boolean},
//             langDetect:{available:boolean} }} CapabilityReport
// @typedef {{ supported:boolean, isWord:boolean }} WordCheckResult
// @typedef {{ supported:boolean, lang:string, confidence:number }} LanguageGuess
// @typedef {{ available:boolean, supportsOnDevice:boolean, locale:string }} SpeechCapability
// @typedef {{ transcription:string }} SpeechResult
(function () {
  'use strict';

  var _proxy;
  /**
   * The native plugin proxy. This plugin ships as native-only Swift with NO npm
   * JS package, so `Capacitor.Plugins.NativeLanguageKit` is never auto-populated
   * — we must create the proxy ourselves via `Capacitor.registerPlugin` (the
   * documented Capacitor way for a custom plugin). Cached after first call.
   * @returns {any} the proxy, or undefined when Capacitor isn't present (web).
   */
  function plugin() {
    var cap = window.Capacitor;
    if (!cap) return undefined;
    if (!_proxy && typeof cap.registerPlugin === 'function') {
      _proxy = cap.registerPlugin('NativeLanguageKit');
    }
    return _proxy || (cap.Plugins && cap.Plugins.NativeLanguageKit);
  }

  /**
   * Is the native capability layer usable right now?
   * True only on a Capacitor native platform with the plugin registered.
   * @returns {boolean}
   */
  function available() {
    var cap = window.Capacitor;
    return !!(cap && cap.isNativePlatform && cap.isNativePlatform() && plugin());
  }

  /** The all-false report handed back on every non-iOS platform. */
  function unavailableReport() {
    return {
      tts: { available: false, voices: [] },
      spellcheck: { available: false },
      langDetect: { available: false },
    };
  }

  window.SpellNativeLang = {
    /** @returns {boolean} */
    available: available,

    /**
     * Query what this platform can do for `lang`, per capability. Call this and
     * branch on the result — never assume a capability exists.
     * @param {string} lang bare app language code, e.g. "en" | "es"
     * @returns {Promise<CapabilityReport>} all-false off iOS; never rejects.
     */
    capabilities: function (lang) {
      if (!available()) return Promise.resolve(unavailableReport());
      return plugin().capabilities({ lang: lang }).catch(unavailableReport);
    },

    /**
     * Speak `text` offline via AVSpeech. `voiceId` is REQUIRED — the caller
     * selects it from capabilities().tts.voices; the plugin never picks a voice.
     * A new speak cancels the previous utterance.
     * @param {string} text
     * @param {string} voiceId AVSpeech voice identifier from the catalog
     * @param {number} [rate] game rate (0.9 normal, 0.7 slow); defaults to normal
     * @returns {Promise<void>} resolves on natural completion; REJECTS with code
     *   "SPEAK_INCOMPLETE" if cancelled/superseded or the voice is unknown, and
     *   "BAD_ARGS" if text/voiceId missing. Off iOS: rejects "UNAVAILABLE".
     */
    speak: function (text, voiceId, rate) {
      if (!available()) return Promise.reject(new Error('UNAVAILABLE'));
      return plugin().speak({ text: text, voiceId: voiceId, rate: rate });
    },

    /**
     * Speak `syllables` (in order) as ONE offline utterance, reporting each
     * syllable boundary as AVSpeech reaches it so the caller can highlight the
     * revealed spelling in sync (Feature F7). `onIndex(i)` fires with the
     * 0-based syllable index each time a new syllable begins; the plugin emits
     * these via the `syllableBoundary` event, wired to a per-call listener that
     * is torn down when the promise settles. `voiceId` is REQUIRED (from
     * capabilities().tts.voices) — the plugin never picks a voice.
     * @param {string[]} syllables ordered syllable tokens, e.g. ["ca","sa"]
     * @param {string} voiceId AVSpeech voice identifier from the catalog
     * @param {number} [rate] game rate (0.9 normal, 0.7 slow); defaults to normal
     * @param {(index:number)=>void} [onIndex] per-syllable highlight callback
     * @returns {Promise<void>} resolves on natural completion; REJECTS with
     *   "SPEAK_INCOMPLETE" if cancelled/superseded or the voice is unknown, and
     *   "BAD_ARGS" if syllables/voiceId missing. Off iOS: rejects "UNAVAILABLE".
     */
    speakSyllables: function (syllables, voiceId, rate, onIndex) {
      if (!available()) return Promise.reject(new Error('UNAVAILABLE'));
      var p = plugin();
      // addListener may return a handle or a Promise<handle> depending on the
      // Capacitor version — normalize both so cleanup always works.
      var listening = (typeof onIndex === 'function')
        ? p.addListener('syllableBoundary', function (ev) {
            onIndex(ev && typeof ev.index === 'number' ? ev.index : 0);
          })
        : null;
      function cleanup() {
        if (!listening) return;
        if (typeof listening.then === 'function') {
          listening.then(function (h) { if (h && h.remove) h.remove(); });
        } else if (listening.remove) {
          listening.remove();
        }
      }
      return p.speakSyllables({ syllables: syllables, voiceId: voiceId, rate: rate })
        .then(function (r) { cleanup(); return r; },
              function (e) { cleanup(); throw e; });
    },

    /**
     * Stop any in-flight utterance. No-op (resolves) off iOS.
     * @returns {Promise<void>}
     */
    stop: function () {
      if (!available()) return Promise.resolve();
      return plugin().stop();
    },

    /**
     * On-device real-word check via UITextChecker. An ADDITIONAL gate — charset
     * and profanity still run around it. `supported:false` means iOS has no
     * dictionary for `lang`; the caller then skips this gate (no silent verdict).
     * @param {string} word
     * @param {string} lang
     * @returns {Promise<WordCheckResult>} {supported:false,isWord:false} off iOS.
     */
    checkWord: function (word, lang) {
      if (!available()) return Promise.resolve({ supported: false, isWord: false });
      return plugin().checkWord({ word: word, lang: lang })
        .catch(function () { return { supported: false, isWord: false }; });
    },

    /**
     * Detect the language of `text` (NLLanguageRecognizer). Single words give a
     * weak signal — the caller uses a high confidence bar and only ever shows a
     * non-blocking hint. Never blocks entry.
     * @param {string} text
     * @returns {Promise<LanguageGuess>} {supported:false,lang:'',confidence:0} off iOS.
     */
    detectLanguage: function (text) {
      if (!available()) return Promise.resolve({ supported: false, lang: '', confidence: 0 });
      return plugin().detectLanguage({ text: text })
        .catch(function () { return { supported: false, lang: '', confidence: 0 }; });
    },

    // ---- Say It (Feature F2): ON-DEVICE speech recognition ONLY ----

    /**
     * Can `lang` be recognized entirely ON-DEVICE on this platform? The privacy
     * contract lives in the shape: `available` is NEVER true unless on-device
     * recognition is supported. Treat `available:false` as "the Say-It mode is
     * UNAVAILABLE for this language" — it must NEVER be read as permission to use
     * server-based recognition (a child's voice never leaves the phone).
     * @param {string} lang bare app language code, e.g. "en"
     * @returns {Promise<SpeechCapability>} all-false off iOS; never rejects.
     */
    speechCapabilities: function (lang) {
      var off = { available: false, supportsOnDevice: false, locale: '' };
      if (!available()) return Promise.resolve(off);
      return plugin().speechCapabilities({ lang: lang }).catch(function () { return off; });
    },

    /**
     * Start listening and return the ON-DEVICE transcription. On iOS this sets
     * SFSpeechRecognizer `requiresOnDeviceRecognition = true`; the mic audio is
     * streamed only to the on-device recognizer and never persisted or uploaded.
     * The OS mic + speech permission prompts appear on the FIRST call (the caller
     * shows a plain-language pre-prompt first).
     * @param {{ lang:string }} opts
     * @returns {Promise<SpeechResult>} resolves `{ transcription }`. REJECTS with
     *   an Error whose message is one of: "UNAVAILABLE" (no on-device path — do
     *   NOT fall back to a server), "PERMISSION_DENIED" (→ needs-mic state),
     *   "BUSY", "AUDIO_ERROR", "NO_SPEECH". Off iOS: rejects "UNAVAILABLE".
     */
    startListening: function (opts) {
      if (!available()) return Promise.reject(new Error('UNAVAILABLE'));
      return plugin().startListening({ lang: (opts && opts.lang) || '' });
    },

    /**
     * Stop listening; the in-flight startListening resolves with whatever
     * on-device transcription was captured. No-op (resolves) off iOS.
     * @returns {Promise<void>}
     */
    stopListening: function () {
      if (!available()) return Promise.resolve();
      return plugin().stopListening();
    },

    /**
     * True only where the on-device VisionKit text recognizer is available
     * (iOS + the NativeLanguageKit plugin). The photo-list camera affordance is
     * shown off this capability check (Feature F1). @returns {boolean}
     */
    supported: function () {
      // Gate on native platform, not just proxy presence — registerPlugin returns
      // a (non-null) proxy even in a Capacitor web context, so the camera button
      // must not appear off a real device.
      return available();
    },

    /**
     * Capture a photo and recognize a word list entirely on-device (Feature F1).
     * The image goes straight from the native picker to Vision — never uploaded,
     * cached, or exposed to JS as bytes; only recognized text lines return.
     * @param {{ lang?: string, source?: ("camera"|"library"|"auto") }} [opts]
     * @returns {Promise<{ supported: boolean, lines: string[] }>} on non-iOS
     *   resolves { supported:false, lines:[] }.
     */
    recognizeWordList: function (opts) {
      var P = plugin();
      if (!P) return Promise.resolve({ supported: false, lines: [] });
      var args = {
        lang: (opts && opts.lang) || 'en-US',
        source: (opts && opts.source) || 'auto',
      };
      return P.recognizeWordList(args).then(function (res) {
        return { supported: true, lines: (res && Array.isArray(res.lines)) ? res.lines : [] };
      });
    },
  };
})();
