# Ship build 56 to TestFlight — runbook

Everything below runs **on your Mac** (needs Xcode + your Apple Developer signing).
The iOS project already exists and is patched (mic/audio-session strings, audio
plays on silent), so the one-time `cap add ios` / `ios-setup.sh` steps are done.

App: `net.spellgame.app` · Team `WCH6H5NAWH` · marketing version `1.1`.
The `beta` lane auto-derives the build number from App Store Connect (latest + 1),
so you never bump it by hand.

---

## 1. One-time toolchain (skip anything already installed)

```bash
xcode-select --install                                   # Xcode command-line tools (also install Xcode from the App Store)
# Rust + the wasm target
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown
# wasm-bindgen CLI — MUST match the crate version EXACTLY or the build errors out
cargo install wasm-bindgen-cli --version 0.2.126
# Node deps (Capacitor CLI + plugins) and Ruby gems (fastlane + cocoapods)
npm install
bundle install
# optional: smaller wasm (build-web.sh uses them if present)
brew install brotli
```

Gotcha: `wasm-bindgen-cli` **must be 0.2.126** (matches `wasm-bindgen` in
`Cargo.lock`). A mismatch fails with a "schema version" error. If you later bump
the crate, reinstall the CLI to match.

## 2. One-time signing (fastlane match + API key)

Signing certs/profiles live in a **separate private repo**,
`git@github.com:icyfxllm-eng/spell-certs.git` (see `fastlane/Matchfile`).

- If that repo already has your certs: nothing to do — the lane pulls them.
- If it's empty/new (first machine ever): create the repo, then once:
  ```bash
  bundle exec fastlane match appstore      # creates + uploads the distribution cert + profile
  ```

App Store Connect API key (for upload auth) — these files must be present locally
(they are **gitignored**, never committed): `fastlane/AuthKey_P37FSPAUWV.p8` and
`fastlane/api_key.json`. If you're on a fresh Mac, copy them over from your
password manager / other machine.

Secrets the lanes read from the environment:
```bash
export MATCH_PASSWORD='...'                               # decrypts the spell-certs repo
export APP_STORE_CONNECT_API_KEY_PATH="$PWD/fastlane/api_key.json"
```

## 3. Before every ship — the 3 checks that actually bite

```bash
cargo test --lib                          # the whole session's Rust changes are UNCOMPILED — this is the real gate
cargo test --lib --features audit_preview # the audit-preview registry variant
npm run words:check                       # word-bank gates (charset/exclusions/size) — should already be green
```

Also, for **Swahili audio** to work in the beta, the *backend* (not the app) needs
Azure configured — set `AZURE_SPEECH_KEY` + `AZURE_SPEECH_REGION` in the backend's
`.env` and redeploy it, then confirm:
```bash
curl -s "https://spellgame.net/api/speak?lang=sw&word=paka" -o /tmp/paka.mp3 && open /tmp/paka.mp3
```
(Without it, Swahili words 502 — the app plays no audio for them.)

## 4. Build + ship (every time)

From the **repo root**:

```bash
# 1. Build the WASM + web bundle and sync it into the iOS project
npm run sync:ios          # == npm run build (cargo -> wasm-bindgen -> dist/) && cap sync ios

# 2. Sign, archive, upload to TestFlight
bundle exec fastlane beta            # INTERNAL testers (you + your team) — fastest, no review
#   or, for EXTERNAL testers (goes through Apple Beta App Review, ~a day):
bundle exec fastlane beta_external
```

`fastlane beta` runs `match` (pull certs) → sets the next build number → forces
manual signing → `gym` (archive) → `pilot` (upload). When it finishes, the build
shows up in App Store Connect → TestFlight after Apple finishes processing
(usually a few minutes), then it's installable on your device via the TestFlight app.

## 5. What ships in build 56 (current code state)

Active/playable: **English + the 10 LTR/CJK languages + Russian + Swahili** (sw-TZ
voice via Azure). **Arabic and Hindi are gated / unavailable** by design. Nothing
else about monetization or entitlements changed.

## Troubleshooting

- **`wasm-bindgen` schema mismatch** → CLI ≠ crate version; `cargo install
  wasm-bindgen-cli --version 0.2.126`.
- **`match` can't decrypt / auth** → `MATCH_PASSWORD` unset, or no SSH access to
  `spell-certs`.
- **`ITMS-90189 Redundant Binary Upload`** → shouldn't happen; the lane derives the
  build number from App Store Connect. If it does, run `bundle exec fastlane bump`
  then retry.
- **CocoaPods errors after `cap sync`** → `cd ios/App && bundle exec pod install`.
- **Upload auth fails** → check `APP_STORE_CONNECT_API_KEY_PATH` points at
  `fastlane/api_key.json` and the `.p8` it names is present.
