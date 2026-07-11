// Digital-Ink bridge for the Capacitor build — wraps the native ML Kit Digital
// Ink Recognition plugin (`DigitalInk`) and exposes a small promise API on
// `window.SpellInk`. Mirrors `audio-native.js` (no bundler → reached through
// `window.Capacitor.Plugins.DigitalInk`, registered natively by `cap sync`).
//
// Boundary (drawing-input design): the webview calls `recognize` here; the
// Rust/WASM core NEVER calls the plugin. JS forwards the ranked candidates to
// the core's scorer (`digital_ink::score_drawn`) and applies the verdict.
//
// Why drawing: for 中文/日本語-kanji the answer IS a handwritten character
// (听写/書き取り) — pinyin/kana typing can't test character recall. ML Kit
// Digital Ink is on-device, offline after a one-time per-language model
// download, and stroke-based (far better than image OCR for handwriting).
//
// Dormant until the native plugin ships AND the zh/ja drawing relaunch lands.
// On the web / a native build without the plugin, `available()` is false and
// the UI hides draw mode (identical to model-not-downloaded).
(function () {
  "use strict";

  function warn() { try { console.warn.apply(console, ["[SpellInk]"].concat([].slice.call(arguments))); } catch (e) {} }

  // ML Kit needs FULL BCP-47 tags with script+region; bare "zh"/"ja" fail
  // identifier lookup. Map the app's language code → the recognizer tag.
  var LANG_TAG = {
    zh: "zh-Hani-CN", // simplified
    ja: "ja-JP",
    ko: "ko-KR",
    th: "th-TH",
  };
  function tagFor(lang) { return LANG_TAG[lang] || null; }

  // Lazy — Capacitor injects `window.Capacitor` after this classic <script>
  // runs during HTML parsing, so never capture it at load time.
  function plugin() {
    var C = window.Capacitor;
    if (!(C && typeof C.isNativePlatform === "function" && C.isNativePlatform()
      && C.Plugins && C.Plugins.DigitalInk)) {
      return null;
    }
    return C.Plugins.DigitalInk;
  }

  window.SpellInk = {
    available: function () { return !!plugin(); },
    tagFor: tagFor,

    // Download the on-device model for `lang`'s script (call on first selection
    // of a gated language, never at launch). Resolves { status } on success.
    downloadModel: function (lang) {
      var P = plugin(); var tag = tagFor(lang);
      if (!P || !tag) return Promise.reject(new Error("digital ink unavailable"));
      return P.downloadModel({ languageTag: tag });
    },

    isModelDownloaded: function (lang) {
      var P = plugin(); var tag = tagFor(lang);
      if (!P || !tag) return Promise.resolve(false);
      return P.isModelDownloaded({ languageTag: tag }).then(
        function (r) { return !!(r && r.downloaded); }, function () { return false; });
    },

    deleteModel: function (lang) {
      var P = plugin(); var tag = tagFor(lang);
      if (!P || !tag) return Promise.reject(new Error("digital ink unavailable"));
      return P.deleteModel({ languageTag: tag });
    },

    // Recognize one drawn character. `strokes` is [{points:[{x,y,t}]}] in CSS-
    // pixel space (NOT DPR-scaled). `writingArea` is {width,height} in that same
    // space. `preContext` = already-accepted characters of the word (improves
    // accuracy). Resolves { candidates: [{text, score}] } ranked best-first.
    recognize: function (opts) {
      var P = plugin(); var tag = tagFor(opts.lang);
      if (!P || !tag) return Promise.reject(new Error("digital ink unavailable"));
      return P.recognize({
        strokes: opts.strokes,
        writingArea: opts.writingArea,
        expected: opts.expected || "",
        preContext: opts.preContext || "",
        languageTag: tag,
      }).catch(function (err) { warn("recognize failed:", String(err)); throw err; });
    },
  };
})();
