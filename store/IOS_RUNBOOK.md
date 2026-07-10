# Spell — iOS Runbook (do this on the Mac)

All commands run on the **Mac**, in **Terminal**, from the repo root
(`spell-wasm/`). iOS builds need macOS + Xcode — they cannot run on Linux.

Facts baked into this project (already configured — don't re-enter):
- Bundle ID: `net.spellgame.app`  ·  Apple Team ID: `WCH6H5NAWH`
- Xcode project: `ios/App/App.xcodeproj`  ·  scheme: `App`
- Signing certs live in the private repo `spell-certs` (via `fastlane match`)
- Current: version **1.1**, build **19** (TestFlight)

---

## 0. First time on a fresh Mac (skip if already set up)

```bash
# Tools
brew install node cocoapods fastlane
xcode-select --install                      # or install Xcode from the App Store

# Get the code + build the web bundle
git clone git@github.com:icyfxllm-eng/spell-wasm.git
cd spell-wasm
npm ci
npm run build

# iOS project is already committed, so just sync web assets + pods:
npx cap sync ios
bundle install                              # installs fastlane from the Gemfile

# One-time: create/download signing certs into the spell-certs repo
bundle exec fastlane match appstore         # first run creates them; set a
                                            # passphrase and SAVE it (MATCH_PASSWORD)
```

If Xcode ever needs a manual open (signing UI, capabilities):
```bash
npx cap open ios     # Xcode: Team = WCH6H5NAWH, Associated Domains = applinks:spellgame.net
```

---

## 1. Ship a new TestFlight build (the routine loop)

```bash
# 1. Rebuild the web app and sync it into iOS
npm run build
npx cap sync ios

# 2. Bump the build number to the next TestFlight number (auto-queries Apple)
bundle exec fastlane bump

# 3. Build, sign, upload. Pick ONE:
bundle exec fastlane beta            # -> internal TestFlight testers (fast, no review)
bundle exec fastlane beta_external   # -> external TestFlight group "Spell" (needs Beta App Review)
```

- `beta` is the quick one for yourself / internal testers — available in minutes.
- `beta_external` submits to Beta App Review (usually < 24h) then notifies the
  external "Spell" group. The reviewer notes/description are baked into the lane.

After upload, the build shows in **App Store Connect → TestFlight** after
processing (a few minutes).

---

## 2. Submit to the App Store (public release)

Metadata/screenshots are managed via `fastlane deliver`
(`fastlane/metadata/`, `fastlane/screenshots/`).

```bash
# Upload metadata + screenshots only (no binary) if you changed them:
bundle exec fastlane deliver --skip_binary_upload true --skip_screenshots false

# Submit a build already on TestFlight for App Store review:
bundle exec fastlane submit_review
```

> ⚠️ **Before running `submit_review`:** open `fastlane/Fastfile` and change
> `build_number: "14"` in the `submit_review` lane to the build you actually
> want to submit (e.g. `"19"`). It is currently hardcoded to an old build.

You can also submit from the **App Store Connect** website instead of the lane.

---

## 3. Version vs build number

- **Build number** (`CURRENT_PROJECT_VERSION`, e.g. 19) — must increase for
  every TestFlight upload. `fastlane bump` handles it.
- **Marketing version** (`MARKETING_VERSION`, e.g. 1.1) — the public version.
  Bump it for a real release: `npm run version:set 1.2.0` (updates iOS + Android
  + package.json together), or edit it in Xcode.

---

## 4. ⚠️ Compliance — MUST fix before App Store submission

The app now includes **The Climb** (optional accounts + leaderboard collecting
username/email/phone/scores). Two artifacts still say the old "no accounts" story
and are now **inaccurate** — Apple can reject on this:

1. **App Store Connect → App Privacy** — the data-collection questionnaire must
   declare the collected data (Name, Email, Phone (optional), User ID, App
   activity), used for App Functionality + Account Management, not shared, not
   for tracking. (Same change already made for Google Play data-safety.)
2. **Review notes** (`store/app-review-notes.md`) and the `beta_external`
   description in `fastlane/Fastfile` say "No account or login required / no
   personal data collected." Update them to: "An optional online leaderboard
   ('The Climb') lets users create an account (username + email); the rest of the
   game needs no account. Account creation is gated behind a 13+ age screen."

Ask Claude to update `app-review-notes.md` + the Fastfile strings + point you at
the exact App Privacy answers — it's the iOS twin of the Play data-safety fix.

---

## Quick reference

| Goal | Command (on Mac) |
|---|---|
| Rebuild + sync web into iOS | `npm run build && npx cap sync ios` |
| Bump build number | `bundle exec fastlane bump` |
| Internal TestFlight | `bundle exec fastlane beta` |
| External TestFlight (+review) | `bundle exec fastlane beta_external` |
| Refresh certs (read-only) | `bundle exec fastlane certs` |
| Upload metadata/screenshots | `bundle exec fastlane deliver ...` |
| Submit to App Store | edit build_number, then `bundle exec fastlane submit_review` |
| Open in Xcode | `npx cap open ios` |
