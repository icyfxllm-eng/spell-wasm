# SpellGame Desktop (Tauri)

Native Windows / macOS / Linux builds that wrap the **same web build** as iOS
and the PWA (`../dist`). No forked frontend — `beforeBuildCommand` runs the
existing `npm run build`, and the bundled assets ship inside the binary so the
app is offline-capable. The service worker does **not** run inside Tauri
(`index.html` guards registration with `window.isWrappedPlatform()`, which
detects `__TAURI_INTERNALS__`).

- **Identifier:** `net.spellgame.desktop` (kept distinct from the iOS bundle ID
  `net.spellgame.app` to avoid capability collisions).
- **Window:** 1200×800 default, 800×600 min, resizable; size/position remembered
  across launches (`tauri-plugin-window-state`).

## One-time setup

```bash
cargo install tauri-cli --version '^2' --locked
# Generate the platform icon set (.ico/.icns/PNGs) from the 1024px source:
cd desktop/src-tauri && cargo tauri icon icons/icon.png
```

## Build

```bash
cd desktop/src-tauri
cargo tauri build          # current OS -> installer(s) in target/release/bundle/
cargo tauri dev            # run locally
```

Outputs per OS:

| OS | Artifacts |
|----|-----------|
| Windows | `.msi` (WiX) + `.exe` (NSIS) |
| macOS | `.app` + `.dmg` (Developer ID signing / notarization for direct download) |
| Linux | `.AppImage` + `.deb` |

## macOS signing / notarization (direct download)

Reuse the existing Apple Developer account + ASC API key. Set before building:

```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: … (WCH6H5NAWH)"
export APPLE_API_KEY_PATH=/path/to/AuthKey.p8   # or notarytool creds
cargo tauri build --bundles app,dmg
```

## Mac App Store (MAS)

MAS needs sandbox entitlements + an App Store provisioning profile and a
distinct app record in App Store Connect. Entitlements are in
`entitlements.mas.plist` (sandbox + network client only). Build with a MAS
config overlay that sets `bundle.macOS.entitlements` and the App Store signing
identity, then submit with the existing `fastlane deliver` setup pointing at the
new macOS app record. Localized metadata reuses `fastlane/metadata/{locale}/`.

## CI

`.github/workflows/desktop.yml` builds the matrix (ubuntu → AppImage/deb,
windows → msi, macOS → dmg) on tag push and uploads artifacts. Signing and MAS
submission are gated on the secrets being present.

## Status

⚠️ This scaffold was authored to the Tauri v2 schema but **not compiled in the
authoring environment** (no Tauri toolchain / system webview there). Run
`cargo tauri build` once locally to validate before relying on CI.
