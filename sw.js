// Service worker for the Spell PWA.
//
// Only the static shell (this repo's own files) is cached. Backend API
// calls (/api/speak, /api/meaning, /api/check, ...) are same-origin in
// production (Caddy reverse-proxies /api/* to the backend on the same
// domain), so they'd otherwise get swept into this same handler too —
// excluded below on purpose. Word/sentence audio already carries its own
// long-lived Cache-Control header from the backend, and /api/meaning's
// masked-vs-unmasked variants shouldn't pile up indefinitely in the
// service worker's cache.
//
// Network-first, not cache-first: this app is still under active
// development, and a cache-first strategy means every future fix silently
// doesn't reach anyone with an already-installed service worker until this
// version number is bumped again — a real instance of that already
// happened (a phone kept getting a stale pre-fix build for several
// deploys). Falling back to cache only when the network request itself
// fails means an actual offline visit still works, but anyone with a live
// connection always gets the current deployed version.
const CACHE_VERSION = "v13";
const CACHE_NAME = `spell-shell-${CACHE_VERSION}`;

const STATIC_ASSETS = [
  "./",
  "./index.html",
  "./manifest.json",
  "./ocr-shim.js",
  "./audio-native.js",
  "./pkg/spell_wasm.js?v=DEV",
  "./pkg/spell_wasm_bg.wasm?v=DEV",
  "./icons/icon-192.png",
  "./icons/icon-512.png",
  "./icons/icon-512-maskable.png",
];

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => cache.addAll(STATIC_ASSETS)).then(() => self.skipWaiting())
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k !== CACHE_NAME).map((k) => caches.delete(k))))
      .then(() => self.clients.claim())
  );
});

self.addEventListener("fetch", (event) => {
  const req = event.request;
  if (req.method !== "GET") return;

  const url = new URL(req.url);
  if (url.origin !== self.location.origin || url.pathname.startsWith("/api/")) return; // let backend/API calls pass straight through

  event.respondWith(
    fetch(req)
      .then((res) => {
        if (res.ok) {
          const copy = res.clone();
          caches.open(CACHE_NAME).then((cache) => cache.put(req, copy));
        }
        return res;
      })
      .catch(() => caches.match(req))
  );
});
