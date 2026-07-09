# App Store Release — Resource Checklist (Spell / net.spellgame.app)

Everything gathered for the App Store release, plus the steps that still need a
human in App Store Connect (ASC).

## ✅ Gathered in this repo (ready to upload)

| Resource | Where | Notes |
|---|---|---|
| Listing text | `fastlane/metadata/en-US/` | name, subtitle, description, keywords, promotional_text, release_notes, support/marketing/privacy URLs — all within Apple's character limits |
| Category / copyright | `fastlane/metadata/` | primary_category = EDUCATION; copyright = 2026 Eric Hillman |
| App Review Information | `fastlane/metadata/review_information/` | Eric Hillman · 2604584592 · icyfxllm@gmail.com · **no demo account**; test-notes included |
| Screenshots | `fastlane/screenshots/en-US/` | iPhone 6.9" (1290×2796) + iPad 13" (2064×2752), gameplay + settings. Regenerate with `node scripts/screenshots.mjs` |
| App icon (1024²) | `ios/App/App/Assets.xcassets/AppIcon.appiconset/AppIcon-512@2x.png` | present |
| Export compliance | `ios/App/App/Info.plist` | `ITSAppUsesNonExemptEncryption = NO` (exempt/standard only) |
| Age rating | — | 4+ / Education (answer the ASC questionnaire accordingly) |
| Price | — | Free |

## Upload the metadata + screenshots to ASC

```bash
export APP_STORE_CONNECT_API_KEY_PATH=$PWD/fastlane/api_key.json
bundle exec fastlane deliver --skip_binary_upload true --force
```
This pushes the text metadata + screenshots to the "Prepare for Submission"
version. It does **not** submit for review (add `--submit_for_review true` when
you're ready).

## Still needs a human in App Store Connect

- [ ] **App Privacy ("nutrition labels")** — declare data collection. For this
      build: **No data collected** (progress + custom words stay on-device; word
      audio/definitions come from the developer's server and aren't tied to an
      identity or used for tracking). Set "Data Not Collected."
- [ ] **Age rating questionnaire** — answer for a 4+ rating.
- [ ] **Pricing & Availability** — Free; pick territories.
- [ ] **Select the build** — build `1.1 (14)` (export-compliant).
- [ ] **Submit for Review** — after the above are green.

## Notes
- The submitted build (`main`, 1.1 build 14) has **no account/login** — The
  Climb accounts are unmerged on `feature-the-climb`, so no in-app
  account-deletion requirement is triggered for this release.
- TestFlight **external** Beta App Review for build 14 is already submitted
  (separate from this App Store submission).
