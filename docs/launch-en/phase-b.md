# CC-LAUNCH-EN Phase B — language lock, placeholder purge, privacy table

Branch `launch-en/phase-b`, cut from origin/main 7d2b7baa. Independent of Phase A (`launch-en/phase-a`).

## 1. Play-language registry: `active | gated | hidden`

- `LangStatus` is now `Active | Gated | Hidden` (src/consts.rs, enum near the top). `ComingSoon` was renamed to `Gated`. `consts::LangStatus` is a different enum from `modes::ComingSoon`, which is unchanged.
- `RELEASE_LANG_PROFILE` (src/consts.rs) is true unless the build has `all_langs`, `audit_preview` or `web`. It is build-time only. Nothing reads flags, localStorage or the kill file. The registry table uses `launch(..)` for English and `later(..)` for the other 15, and `later` gives `Hidden` in the release profile. There is one table and no per-language `if`.
- New accessors: `is_listed_lang`, `listed_langs`, `is_hidden_builtin`. `is_active_lang` matches on the status, and `Hidden` is never playable. An unknown code is now `Hidden`, not coming-soon.
- New cargo feature `all_langs` (Cargo.toml). `scripts/build-web-dev.sh` and the default `scripts/build-web-test.sh` add it. `SPELL_RELEASE_LANGS=1 bash scripts/build-web-test.sh` builds the release profile into `dist-test-release/`. `scripts/build-web.sh` (TestFlight/App Store) and `build-web-edu.sh` do not add it, so they ship English only.
- The web build keeps its existing rule (`is_active_lang`'s `web` clause, other languages as coming-soon tiles). D7 web gating is a separate later item, so it is not built here.
- Consumers changed:
  - main picker `game::build_source_options` (listed rows only)
  - startup language restore in `lib.rs` (a saved or device language must be listed, otherwise English; `i18n::init` and uiLang are untouched, I3.3)
  - `render_coming_soon` (backstop: never for a Hidden language)
  - both "Speak in" pickers via `words::speak_in_languages` (lib.rs import sheet, photo_list.rs)
  - Translate pickers (`translate_screen::picker_languages`)
  - the import language hint (`lib.rs` `lang_hint`)
  - Misses and the tone drill (`misses::due_misses`, `tone_drill::due`): saved entries in a Hidden language are kept but not served
- Consumers already safe because they filter by `is_active_lang`: Online `olLang`, Daily, Impossible/Mix, packs, racing, reports share, the widget.
- Tests:
  - I3.1: `consts::registry_tests::i3_1_the_release_profile_plays_english_only`
  - kill file: `killflags::tests::the_file_cannot_unhide_a_language`
  - the all-langs build: `every_registered_language_is_active_in_the_all_langs_build` (`cargo test --lib --features all_langs`)
  - A3.2: `tests/e2e/specs/release-langs.mjs`, release-only in `run.mjs`. It writes `TEST-REPORT-release-langs.md`.

Known leftovers, not changed:
- The entitlement map still carries rows for Hidden languages, and a purchase raises them to Full. Phase A owns the entitlement code. No surface can reach them.
- A My Words list whose stored speak language is non-English still plays in that voice. That is user content; the picker offers English only.
- The `photoLang` picker is iOS-only and is not reached in e2e. It uses the same accessor as `importLang`.

## 2. Placeholder purge

Deleted from all 15 locales:
- `coming.badge`
- `coming.notice`
- `racing.soon`
- `sd.avail`
- `tools.defmatch.avail`
- `tools.practice.avail` (the 8th "soon" line in the recon)
- `ws.avail`
- `xw.avail`

What replaced them:
- The per-language mode reasons in `play_hub::unavailable_reason` and the Spell Search/WordCross toasts now use one new key, `mode.unavailable`. Its value in each locale is copied from that locale's translated `chains.unavailable`.
- The drawer hides an unavailable mode, so that key is never shown as a tile.
- index.html: the badge `<div>` and its CSS are gone. The gated panel now shows only the language name and the vote button, and it exists only in builds carrying every language.
- `racing.soon` had no call site, only a comment.
- `tools/audit/audit_lang.py` no longer lists the two deleted keys.
- `privacy.html`: the "Voting for coming-soon languages" section is now "Language votes (website only)".

`impossible.lock.no_drawn_entry` cannot render in an English-only build:
- Only the zh and ja pools carry `reason: "drawn-entry"`.
- `lang_availability` returns `Hidden` for any non-active language before it reads the pool (src/impossible.rs:182-184 vs :192-193).
- The Mix path collects only `Available` languages (src/impossible.rs:255-256).

Related, for Phase A's flag table:
- If `impossible` and `impossible_mix` are ON in release, the level picker shows a disabled Mix row. It reads "Comes with SpellGame Complete", or "Needs two Impossible languages" after a purchase (src/impossible/engine.rs:158). The second one is a dead greyed row in an English-only app. Both flags are OFF on main.

Not purged, because they don't match the placeholder pattern: `bg.soon`, `rs.soon` and `*.unavailable` ("...in this language yet"). They are unreachable for English.

## 3. privacy.html data-flow table

There is a new "At a glance: what leaves your device" table. The effective date moved to 7 November 2026. The table was built from the code:

| Item | Evidence |
|---|---|
| Typed English answer + word to /api/check, every mode incl. Jr | src/game.rs:2281-2283, src/api.rs:692-697 |
| Word to /api/speak, /api/meaning, /api/sentence-audio, /api/defpool | src/api.rs:166-183, 704-747; src/defmatch_screen.rs:249-256 |
| Telemetry on by default; route policy; remote flag can only turn off | src/telemetry/mod.rs:66-72, 630-642; endpoint mod.rs:495-497 |
| Climb front door at first 13+ launch (email, username, password) | src/lib.rs:665-679; src/climb.rs:557-649 |
| Say-It on device; server rung only with consent, never Kid Mode/education | src/spell_aloud.rs:841-858; src/consts.rs:345-352 |
| Photo OCR on device (Vision), no network | ios/App/App/NativeLanguageKitPlugin+PhotoList.swift; src/photo_list.rs:3-4 |
| Guardian/progress report: no egress (test-enforced) | src/reports_share.rs:593-603 |
| Language vote: language + persistent random install id to /api/notify | src/notify.rs:12, 24-63 |
| Kill-flags GET every launch | src/killflags.rs:126-133; src/lib.rs:290 |

## 4. Store text vs the code: contradictions (not edited; for Eric)

1. `store/app-review-notes.md:17`, `review_information/notes.txt`: "No account, login, or personal information is required". The Climb front door offers an account (email, username, password) at the first 13+ launch (src/lib.rs:665-679). It is optional, so "required" is true, but "no account/login" is not.
2. `store/app-review-notes.md:34`, `notes.txt`: "No personal data is collected." The Climb account collects email and username. Telemetry (crash/perf, with study language, mode and app version) is on by default.
3. `store/app-review-notes.md:35-37`: the only server traffic named is "dictionary-word audio/definitions". It omits that every English answer the player types is sent to /api/check (src/game.rs:2281), plus telemetry, the kill-flags fetch and the optional STT audio.
4. `store/app-review-notes.md:41-43`: "the microphone feature ... is inert on iOS". iOS has a native on-device Say-It path (src/spell_aloud.rs:829-858, NativeLanguageKitPlugin.swift). The mic purpose strings in Info.plist describe it.
5. `store/app-review-notes.md:44`, `RELEASE_CHECKLIST.md:16,35`: age rating 4+. The app has optional accounts, an online leaderboard with public usernames, and (flag-gated) online play. Eric should re-answer the questionnaire.
6. `store/RELEASE_CHECKLIST.md:31-34`: App Privacy "Data Not Collected". The code collects:
   - Contact Info: email (Climb)
   - Identifiers: username, Climb user id
   - User Content: typed answers (not stored, but transmitted, and Apple counts collection on transmission unless it is ephemeral)
   - Diagnostics: crash/perf data
   - Audio Data: optional STT clips
   - Usage Data: telemetry mode/lang
   - Device ID: the random install id sent with a language vote (web only now)

   `store/IOS_RUNBOOK.md:104-113` already says the label must declare Climb data.
7. `store/RELEASE_CHECKLIST.md:41-43`: "no account/login ... Climb accounts are unmerged". Climb accounts are on main and reachable. The in-app account deletion requirement applies, and `climb.rs` has `delete-account`.
8. `fastlane/metadata/en-US/promotional_text.txt` and `description.txt:12` say "Free, offline" / "Works offline after the first load". English answers fall back to local checking offline, but uncached word audio needs the server, and there is no offline screen (recon R3). With Phase A's paywall, "Free" is no longer accurate either.
9. `notes.txt` / review notes: "fully playable offline" and "All content ... ship in the app bundle". No English audio is bundled (recon R3: `runtime.json` `"langs": {}`).

Policy vs code, worth a look:
- privacy.html lists an optional phone number for SMS reset. The subagent found no phone field in the client (unverified against the backend).
- The "Can't tell" clip report sends a hash of lang|word (src/telemetry/mod.rs:341-343), which contradicts "a report never contains the words you practice". It is behind `cant_tell`, which is OFF on main.
- There is no `PrivacyInfo.xcprivacy` in the repo.
