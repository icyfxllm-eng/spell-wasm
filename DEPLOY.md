# Deploying Spell at spellgame.net

Runs Spell — frontend and word-server backend both — from this machine,
publicly reachable at `spellgame.net` over a Cloudflare Tunnel, installable
as a PWA ("Add to Home Screen").

## What's in here

```
Dockerfile             # Builds the Rust/WASM frontend, serves it with Caddy
backend/               # Flask word-server (TTS + answer-checking)
  app.py
  requirements.txt
  Dockerfile
docker-compose.yml      # Runs frontend + backend + cloudflared together
Caddyfile               # Proxies /api/* to the backend; tunnel-mode :8080
cloudflared/config.yml  # Tunnel ingress rules (hostname -> localhost:8080)
manifest.json, sw.js, icons/   # PWA bits
.env.example            # Copy to .env and fill in your TTS key + tunnel creds path
```

Both services share one origin: Caddy serves the static frontend and
reverse-proxies `/api/*` to the backend container, so the browser only
ever talks to `spellgame.net` — no CORS to fight with. `spellgame.net` is
already registered and its nameservers already point at Cloudflare, so this
is all set up for the Cloudflare Tunnel path (no port-forwarding needed).

## Step 1 — Get a Google Cloud Text-to-Speech API key

The backend needs this to generate word audio.

1. Go to https://console.cloud.google.com/ and create a project (or use
   an existing one).
2. Enable billing on the project (TTS has a free monthly quota, but
   billing must be turned on to use the API past it).
3. Enable the **Cloud Text-to-Speech API** for that project.
4. Go to **APIs & Services → Credentials → Create Credentials → API key**.
5. Copy the key.

In this repo:

```bash
cp .env.example .env
# edit .env, set GOOGLE_TTS_API_KEY=<the key you just copied>
```

`.env` is gitignored — never commit it.

## Step 2 — Build and verify locally

```bash
docker compose up -d --build
curl http://localhost:8080/api/health
curl "http://localhost:8080/api/speak?word=hello" -o /tmp/hello.mp3 && file /tmp/hello.mp3
```

## Step 3 — Install cloudflared and create the tunnel

```bash
curl -L https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb -o cloudflared.deb
sudo dpkg -i cloudflared.deb
cloudflared tunnel login       # opens a browser link — authorize spellgame.net
cloudflared tunnel create spell
```

This writes credentials to `~/.cloudflared/<tunnel-id>.json` and prints the
tunnel ID — put both into [cloudflared/config.yml](cloudflared/config.yml)
(the `tunnel:` field) and `.env`'s `CLOUDFLARE_CREDENTIALS_FILE`.

Route the domain and define ingress from the CLI rather than the dashboard's
Public Hostname panel — that panel is built for tunnels created *via* the
dashboard wizard and 404'd for this CLI-created one:

```bash
cloudflared tunnel route dns spell spellgame.net
```

[cloudflared/config.yml](cloudflared/config.yml) already has the ingress
rule (`spellgame.net` → `http://localhost:8080`, catch-all 404). The
`cloudflared` service in [docker-compose.yml](docker-compose.yml) runs with
`network_mode: host` so that `localhost:8080` inside the container reaches
the `spell` container's published port on the host, and mounts both the
config and the credentials file (read-only) from `.env`'s
`CLOUDFLARE_CREDENTIALS_FILE`.

The credentials JSON cloudflared writes is `chmod 400`, but the official
`cloudflare/cloudflared` image runs as a non-root UID (`65532`) that can't
read a different user's `400` file when bind-mounted — loosen it once:

```bash
chmod 444 ~/.cloudflared/<tunnel-id>.json
```

`docker compose up -d --build` then brings up all three services —
`spell`, `backend`, `cloudflared` — together. Since Docker is enabled to
start on boot and every service has `restart: unless-stopped`, the whole
stack (including the tunnel) survives reboots with no separate systemd unit
needed.

## Step 4 — Verify publicly

```bash
curl -I https://spellgame.net/
curl https://spellgame.net/api/health
```

Then open `https://spellgame.net` in a real browser, press the orb, and
confirm you hear a word and a full round (hear → type → check) works. On a
phone: browser menu → "Add to Home Screen" → Spell launches like a native app.

## Day-2 operations

```bash
docker compose logs -f spell backend cloudflared   # logs for all three services
docker compose up -d --build                       # deploy a new version
docker compose restart                             # restart everything
docker system prune                                # clean old images occasionally
```

Bump `CACHE_VERSION` in [sw.js](sw.js) whenever you change any static
file (index.html, ocr-shim.js, the pkg/ build, icons) so returning
players get the fresh assets instead of a stale cached copy. The
backend's synthesized audio is cached permanently on a Docker volume
(`audio_cache`), so the same word is only ever billed to Google once.

## Sanity checklist before going public

- [ ] `.env` has a real `GOOGLE_TTS_API_KEY` and is **not** committed
      (check `git status` / `docker history` doesn't show it)
- [ ] `/api/health` responds over `https://spellgame.net`
- [ ] A full round — press orb, hear word, type answer, check — works
      over `https://spellgame.net`, not just `localhost`
- [ ] Lighthouse PWA audit passes (Chrome DevTools → Lighthouse)
- [ ] Tested on an actual phone over cellular, not just wifi
- [ ] Tested "Add to Home Screen" and that the installed app opens to
      the game, not a browser chrome

## Direct exposure instead of a tunnel (not used here)

If you ever move off a home connection to a VPS with a public IP, you can
skip the tunnel: point an A record at the server's IP, switch
[Caddyfile](Caddyfile) back to its commented `spellgame.net { ... }` domain
block, and switch [docker-compose.yml](docker-compose.yml)'s port mapping
back to `"80:80"` / `"443:443"` — Caddy then handles HTTPS automatically.

## Android App Links (deep linking)

`https://spellgame.net/app` and `/challenge/...` open the native app directly
(intent-filter in `android/app/src/main/AndroidManifest.xml`, `autoVerify`).
Verification needs `/.well-known/assetlinks.json` served over HTTPS at the
domain — it's in this repo (`.well-known/assetlinks.json`), copied to `/srv`
by the [Dockerfile](Dockerfile) and served by [Caddyfile](Caddyfile).

**IMPORTANT — signing fingerprints:** `assetlinks.json` lists the local
**debug** and **upload** keystore SHA-256s (both good for locally-built
testing). Before production you MUST also add:
- the **Google Play App Signing** SHA-256 (Play Console → Setup → App signing),
  since Play re-signs the app.

Get a keystore's fingerprint with:
`keytool -list -v -keystore <file> -alias <alias> | grep SHA256`

Add each as another string in the `sha256_cert_fingerprints` array, redeploy
the site, then confirm with:
`adb shell pm verify-app-links --re-verify net.spellgame.app` and
`adb shell pm get-app-links net.spellgame.app` (state should be `verified`).

**iOS Universal Links** use the sibling file
`.well-known/apple-app-site-association` (no extension, served as JSON — the
[Caddyfile](Caddyfile) sets that). Its `appID` is set to the Apple Team ID
`WCH6H5NAWH.net.spellgame.app`. Before iOS ships, add the "Associated Domains"
capability (`applinks:spellgame.net`) to the Xcode project on the Mac, then
deploy this file to prod so Universal Links verify.

## Android release build (Play / Amazon AAB)

Release signing reads `keystore.properties` (repo root, **gitignored**) which
points at the upload keystore in `keys/` (also gitignored). Neither is in git.

**Back these up off-machine NOW:** `keys/spell-upload.jks` + its password. With
Google Play App Signing the upload key is recoverable if lost, but keep it safe.

Build a signed release bundle (for Play/Amazon) and a signed APK (for local
install tests):

```bash
source ~/.android_sdk_env
npm run build && npx cap sync android
(cd android && ./gradlew :app:bundleRelease)   # -> app/build/outputs/bundle/release/app-release.aab
(cd android && ./gradlew :app:assembleRelease) # -> app/build/outputs/apk/release/app-release.apk
```

Verify the AAB is signed: `jarsigner -verify android/app/build/outputs/bundle/release/app-release.aab`.

Bump `versionCode` (integer, must increase every upload) and `versionName` in
`android/app/build.gradle` before each release.

On a fresh checkout without `keystore.properties`, release builds are produced
**unsigned** (build still succeeds) — add the file to sign. See `store/play-listing.md`
for the Play listing copy, data-safety answers, and rollout steps.

## iOS build (Phase 4 — on the Mac / MacStadium)

The iOS project is generated on the Mac (CocoaPods can't run on Linux). One-time:

```bash
# Toolchain: Xcode (App Store), then:
brew install node cocoapods fastlane
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown && cargo install wasm-bindgen-cli --version 0.2.126

npm ci
npm run build            # wasm + dist/
npx cap add ios          # generates ios/ + pod install
bash scripts/ios-setup.sh   # Info.plist strings + AVAudioSession(.playback) for silent-switch
npx capacitor-assets generate --ios   # icons + splash from assets/
npx cap open ios         # Xcode: set Team, add Associated Domains (applinks:spellgame.net)
```

The AASA file `.well-known/apple-app-site-association` is already set to
`WCH6H5NAWH.net.spellgame.app`; just redeploy the site so Universal Links verify.

**TestFlight** — either Xcode (Product → Archive → Distribute → TestFlight) or
fastlane (preferred, repeatable):

```bash
bundle install                       # installs fastlane + cocoapods (Gemfile)
bundle exec fastlane match appstore  # first run: creates certs in the private certs repo
npm run sync:ios
bundle exec fastlane ios beta        # gym + pilot -> TestFlight
```

`fastlane/` (Appfile, Matchfile, Fastfile) is templated — set the Apple Team ID
in `Appfile` and the private certs repo URL in `Matchfile` first.

**Automation (steady state):** register the Mac as a GitHub self-hosted runner,
then enable the `ios` job in `.github/workflows/release.yml` (remove `if: false`)
and add the Apple secrets (match repo deploy key, `MATCH_PASSWORD`,
App Store Connect API key). A `git tag vX.Y.Z` then ships both platforms.

See `store/app-review-notes.md` for the App Review submission text.
