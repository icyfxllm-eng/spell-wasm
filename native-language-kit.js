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
  };
})();
