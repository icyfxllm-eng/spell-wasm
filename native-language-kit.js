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
     * F3 — mirror the game's streak + daily-challenge snapshot into the iOS App
     * Group container (`group.net.spellgame.app`, key `widget_state_v1`) so the
     * home-screen widgets and F4 App Intents can read it. `state` is the
     * JSON-STRING encoding of the Rust core's WidgetState (see src/widgets.rs for
     * the key schema). No-op (resolves) off iOS. Never rejects — widget sync is
     * best-effort and must never disturb game flow.
     * @param {string} state JSON-encoded WidgetState
     * @returns {Promise<void>}
     */
    syncWidgetState: function (state) {
      if (!available()) return Promise.resolve();
      return plugin().syncWidgetState({ state: state }).catch(function () {});
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
  };
})();
