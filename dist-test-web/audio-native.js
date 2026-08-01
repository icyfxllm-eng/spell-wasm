// Native-audio bridge for the Capacitor build. This is the ONLY piece of
// platform JS the app carries — it wraps two native-backed Capacitor plugins
// (NativeAudio + Filesystem) and exposes a tiny promise API on
// `window.SpellAudio` that the Rust/WASM core calls into.
//
// Why this exists (see CLAUDE.md Phase 1): browser <audio>/TTS on mobile is
// fragile — autoplay policies, backgrounding, Bluetooth routing. For the
// server-voiced words the game downloads each clip once to on-device storage,
// then plays it through the OS audio path natively. No autoplay gate, no
// per-word network latency, and it works with the network off.
//
// Everything here is best-effort: every method rejects (never throws) on any
// failure so the Rust dispatcher can fall back to its existing HTML5 <audio>
// path and then Web Speech. On the web (spellgame.net PWA) `available()` is
// false and this module does nothing — web behaviour is unchanged.
//
// No bundler is used in this project, so the plugins are reached through
// `Capacitor.registerPlugin(name)`, which returns a proxy to the native
// implementation compiled in by `cap sync`. No plugin JS needs packaging.
(function () {
  "use strict";

  var CACHE_DIR = "CACHE"; // Capacitor Directory.Cache enum value

  function warn() { try { console.warn.apply(console, ["[SpellAudio]"].concat([].slice.call(arguments))); } catch (e) {} }

  // IMPORTANT: everything is resolved lazily, never at load. This file is a
  // classic <script> that runs during HTML parsing — the Capacitor runtime
  // injects `window.Capacitor` slightly later, so capturing it here would
  // always miss it and permanently disable the native path.
  //
  // With no bundler we don't have `@capacitor/core`'s `registerPlugin`; the
  // native bridge only exposes already-registered plugins under
  // `Capacitor.Plugins.<Name>`. That's what we read.
  function isNative() {
    var C = window.Capacitor;
    return !!(C && typeof C.isNativePlatform === "function" && C.isNativePlatform()
      && C.Plugins && C.Plugins.NativeAudio && C.Plugins.Filesystem);
  }

  var _configured = false;
  // Returns { NativeAudio, Filesystem } from the Capacitor bridge, or null on
  // the web / before the bridge is ready.
  function plugins() {
    if (!isNative()) return null;
    var P = window.Capacitor.Plugins;
    if (!_configured) {
      _configured = true;
      // focus:true → the plugin sets AVAudioSession category .playback, so word
      // audio is AUDIBLE EVEN WHEN THE RING/SILENT SWITCH IS ON. Audio *is* the
      // game ("hear it, spell it"), so it must never be muted by the hardware
      // switch. (focus:false maps to .ambient, which respects the mute switch and
      // silenced every word for anyone with their phone on silent — the P0 bug.)
      // Trade-off: .playback interrupts background music, which is the right call
      // for a spelling game where the word must be heard.
      if (typeof P.NativeAudio.configure === "function") {
        P.NativeAudio.configure({ focus: true, fade: false }).catch(function () {});
      }
    }
    return { NativeAudio: P.NativeAudio, Filesystem: P.Filesystem };
  }

  // Cap how many clips stay preloaded in native memory at once. The game
  // plays one word at a time (plus a warmed next word and a slow variant),
  // so a small LRU is plenty; older ones are unloaded, but their downloaded
  // file stays on disk for instant re-preload / offline replay.
  var MAX_PRELOADED = 8;

  var preloaded = new Map();        // assetId -> true (insertion order = LRU)
  var cachingInFlight = new Map();  // assetId -> Promise<fileUri>  (single-flight)
  var preloadInFlight = new Map();  // assetId -> Promise<void>     (single-flight)
  var current = null;               // assetId currently playing (to stop before next)

  // A stable, filesystem-safe, UNIQUE filename for an assetId. assetIds are like
  // "w:normal:apple" or "w:zh:normal:认识".
  //
  // The old version replaced every non-[a-zA-Z0-9._-] char with "_", which
  // collapsed non-ASCII text: "w:zh:normal:认识" and "w:zh:normal:直接" both
  // became "w_zh_normal___.mp3". So every 2-character Chinese (and Japanese /
  // Korean / Thai) word shared one cache file — the first clip cached replayed
  // for all of them, i.e. "the same word on every difficulty" on the native app.
  //
  // Hash the full assetId (djb2, unsigned 32-bit) so the name is unique per word
  // regardless of script; keep a short ASCII prefix for debuggability. Existing
  // (collided) cache files simply go unused — no migration needed.
  function fileName(assetId) {
    var h = 5381;
    for (var i = 0; i < assetId.length; i++) {
      h = (((h << 5) + h) + assetId.charCodeAt(i)) >>> 0;
    }
    var prefix = assetId.replace(/[^a-zA-Z0-9]/g, "").slice(0, 24);
    return "spell-audio/" + prefix + "-" + ("0000000" + h.toString(16)).slice(-8) + ".mp3";
  }

  function arrayBufferToBase64(buf) {
    var bytes = new Uint8Array(buf);
    var chunk = 0x8000;
    var out = "";
    for (var i = 0; i < bytes.length; i += chunk) {
      out += String.fromCharCode.apply(null, bytes.subarray(i, i + chunk));
    }
    return btoa(out);
  }

  // Ensure the clip for assetId is on disk; return its file:// URI. Downloads
  // from `url` (the backend /api/speak endpoint) only on a cache miss.
  function ensureCached(assetId, url) {
    if (cachingInFlight.has(assetId)) return cachingInFlight.get(assetId);
    var P = plugins();
    if (!P) return Promise.reject(new Error("native audio unavailable"));
    var Filesystem = P.Filesystem;
    var path = fileName(assetId);
    var p = Filesystem.stat({ path: path, directory: CACHE_DIR })
      .then(function () {
        // Already on disk.
        return Filesystem.getUri({ path: path, directory: CACHE_DIR });
      })
      .catch(function () {
        // Miss (stat rejects if absent): fetch and write it.
        return fetch(url)
          .then(function (res) {
            if (!res.ok) throw new Error("HTTP " + res.status + " fetching audio");
            return res.arrayBuffer();
          })
          .then(function (buf) {
            return Filesystem.writeFile({
              path: path,
              data: arrayBufferToBase64(buf),
              directory: CACHE_DIR,
              recursive: true,
            });
          })
          .then(function () {
            return Filesystem.getUri({ path: path, directory: CACHE_DIR });
          });
      })
      .then(function (r) { return r.uri; })
      .then(
        function (uri) { cachingInFlight.delete(assetId); return uri; },
        // Silent here (background prefetch is best-effort); the user-facing
        // play path logs its own warning before falling back.
        function (err) { cachingInFlight.delete(assetId); throw err; }
      );
    cachingInFlight.set(assetId, p);
    return p;
  }

  function touchLru(assetId) {
    // Move to most-recently-used, evicting the oldest beyond the cap. Eviction
    // only unloads from native memory; the on-disk file is kept.
    preloaded.delete(assetId);
    preloaded.set(assetId, true);
    while (preloaded.size > MAX_PRELOADED) {
      var oldest = preloaded.keys().next().value;
      preloaded.delete(oldest);
      if (oldest !== current) {
        var P = plugins();
        if (P) P.NativeAudio.unload({ assetId: oldest }).catch(function () {});
      }
    }
  }

  // Ensure the clip is preloaded into the native player from its on-disk file.
  function ensurePreloaded(assetId, url) {
    if (preloaded.has(assetId)) { touchLru(assetId); return Promise.resolve(); }
    if (preloadInFlight.has(assetId)) return preloadInFlight.get(assetId);
    var p = ensureCached(assetId, url)
      .then(function (fileUri) {
        var P = plugins();
        if (!P) throw new Error("native audio unavailable");
        return P.NativeAudio.preload({ assetId: assetId, assetPath: fileUri, isUrl: true });
      })
      .then(
        function () { preloadInFlight.delete(assetId); preloaded.set(assetId, true); touchLru(assetId); },
        function (err) { preloadInFlight.delete(assetId); throw err; }
      );
    preloadInFlight.set(assetId, p);
    return p;
  }

  window.SpellAudio = {
    available: function () { return isNative(); },

    // Play a word clip natively. Downloads+caches on first use, preloads, then
    // plays — stopping whatever was playing first. Rejects on any failure.
    playWord: function (assetId, url) {
      var P = plugins();
      if (!P) return Promise.reject(new Error("native audio unavailable"));
      var stopPrev = current && current !== assetId
        ? P.NativeAudio.stop({ assetId: current }).catch(function () {})
        : Promise.resolve();
      return stopPrev
        .then(function () { return ensurePreloaded(assetId, url); })
        .then(function () {
          current = assetId;
          return P.NativeAudio.play({ assetId: assetId, time: 0 });
        })
        .catch(function (err) { warn("playWord failed, falling back:", assetId, String(err)); throw err; });
    },

    // Download+cache a clip for later (offline pack / warming the next word).
    // Does not preload into native memory or play. This is fire-and-forget
    // from the Rust side, so it swallows its own failure (a warning is already
    // logged in ensureCached) rather than surfacing an unhandled rejection.
    prefetch: function (assetId, url) {
      return ensureCached(assetId, url).then(function () {}, function () {});
    },
  };
})();
