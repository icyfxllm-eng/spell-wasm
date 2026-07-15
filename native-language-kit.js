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

  /** @returns {any} the native plugin proxy, or undefined off-iOS. */
  function plugin() {
    var cap = window.Capacitor;
    return cap && cap.Plugins && cap.Plugins.NativeLanguageKit;
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

    // ---- Spell It Out Loud (voice spelling INPUT): letter-capture profile ----
    //
    // The SAME on-device recognizer as Say-It, a DIFFERENT profile: the recognizer
    // is biased with `contextualStrings` (the language's spoken letter names) and
    // streams RAW transcript tokens (partials included) live via callbacks, so the
    // Rust letter-parser can echo "C… CA… CAT" as the child speaks. The plugin does
    // ZERO parsing — it is a dumb mic. On-device only; nothing persisted or sent.

    /**
     * Start on-device LETTER capture. Subscribes to the plugin's raw-token events
     * and forwards them to the callbacks, then starts the capture. All plugin
     * event subscriptions are torn down automatically on the final/error callback.
     * @param {{ lang:string, contextualStrings?:string[] }} opts
     * @param {(rawTranscript:string)=>void} onToken partial transcript (streamed)
     * @param {(rawTranscript:string)=>void} onFinal final transcript, once
     * @param {(code:string)=>void} onError one of "UNAVAILABLE" | "PERMISSION_DENIED"
     *   | "BUSY" | "AUDIO_ERROR" | "NO_SPEECH". Off iOS: fires "UNAVAILABLE".
     * @returns {void}
     */
    startLetterCapture: function (opts, onToken, onFinal, onError) {
      if (!available()) { if (onError) onError('UNAVAILABLE'); return; }
      var p = plugin();
      var handles = [];
      function cleanup() {
        handles.forEach(function (h) { try { h && h.remove && h.remove(); } catch (e) {} });
        handles = [];
      }
      // addListener resolves to a handle; keep it so we can remove() on teardown.
      function sub(evt, fn) {
        var pr = p.addListener(evt, fn);
        if (pr && typeof pr.then === 'function') {
          pr.then(function (h) { handles.push(h); });
        } else {
          handles.push(pr);
        }
      }
      sub('letterToken', function (d) { if (onToken) onToken((d && d.token) || ''); });
      sub('letterFinal', function (d) { if (onFinal) onFinal((d && d.token) || ''); cleanup(); });
      sub('letterError', function (d) { if (onError) onError((d && d.code) || 'AUDIO_ERROR'); cleanup(); });
      p.startLetterCapture({
        lang: (opts && opts.lang) || '',
        contextualStrings: (opts && opts.contextualStrings) || [],
      }).catch(function (e) {
        if (onError) onError((e && e.message) || 'AUDIO_ERROR');
        cleanup();
      });
    },

    /**
     * Stop letter capture; the plugin finalizes and fires the letterFinal event.
     * No-op off iOS.
     * @returns {Promise<void>}
     */
    stopLetterCapture: function () {
      if (!available()) return Promise.resolve();
      return plugin().stopLetterCapture();
    },
  };
})();
