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

---

# Phase B, round 2 (Eric's items 1-4, 2026-11-07)

## 1. Rebased onto origin/main cbd6f0a9

There were no conflicts; git merged en.json, the locales, index.html, privacy.html and the build scripts without stopping. Phase A's `store-native.js` is still in every build script's cp list. `scripts/placeholder-grep.sh` (Phase A's, on main) passes.

## 2. Four network paths compiled out of the App Store build

New Cargo features (Cargo.toml:56-78): `accounts_net`, `server_check`, `telemetry`, `server_stt`. `net_services` turns on all four.

Where each build stands:
- On: `build-web-dev.sh`, `build-web-test.sh` (default dist-test), `build-web-audit.sh`.
- Off: `build-web.sh` (App Store / TestFlight / Lab), `build-web-edu.sh`, and the release-profile e2e build (`SPELL_RELEASE_LANGS=1`).
- `web = ["net_services"]`: spellgame.net keeps exactly what it has today (see Decisions for Eric).

| Path | Off means | Code | Needle (seam-absence-check) |
|---|---|---|---|
| a. Climb accounts + leaderboard | `src/climb_off.rs` stands in for `climb.rs` (lib.rs:10-14). No front door, sign-in, account sheet, leaderboard view or run upload. `climbBtn`, `accountBtn` and `setAccountRow` stay hidden. The drawer has no account row (drawer.rs:614, `climb::ACCOUNTS`). Local Climb play, its local board and Spell Racing are untouched. | climb_off.rs; climb.rs:18; celebrate.rs:31,120,212 | `/api/auth/`, `/api/climb/` |
| b. /api/check | English is graded on the device only. `backend_verify` keeps its async hop but calls `en_grade_local` (game.rs:2480-2486). `api::check_answer` is not compiled (api.rs:692). The routing decision is now one pure function, `en_route`, used by `submit_guess` and by the parity check (game.rs:2445-2472). | game.rs, api.rs | `malformed /api/check response` |
| c. Telemetry | The sender, the MetricKit drain and the `/v1/flags` fetch are not compiled (telemetry/mod.rs:361, 445, 501-531, 576, 653). `init` sets the route Off and clears whatever an earlier build queued (mod.rs:672). `flush` only clears (mod.rs:557). Settings hides "Help improve SpellGame" (settings.rs:431, `telemetry::COMPILED`). The pure routing/schema code and its tests stay. | telemetry/mod.rs, settings.rs | `/v1/events`, `/v1/flags` |
| d. Say-It server STT | The "server" rung is never chosen (spell_aloud.rs:849). The native recognizer is never handed a server URL (spell_aloud.rs:1325-1333). The on-device recognizer is unchanged. | spell_aloud.rs | `/api/stt` |

Proof, both directions (rerunnable):
- `node scripts/seam-absence-check.mjs dist`: OK on the App Store dist.
- `node scripts/seam-absence-check.mjs --services-present dist-dev`: all 6 needles found. On `dist` the same command reports all 6 MISSING.
- `--selftest` adds a case: a dist carrying `/v1/events` fails.
- e2e `release-langs: English grades locally and no compiled-out path is called`, on the release-profile bundle. It checks that `accountBtn`, `climbBtn`, `setAccountRow` and `telemetryRow` are hidden, then answers an English word correctly and requires zero requests to /api/check, /api/auth/, /api/climb/, /api/stt or /v1/{events,aggregate,clips,flags}. On dist-test (all four paths on) it fails, the positive control.

What broke, and what changed:
- `word_lists::tests::jr_ignores_persisted_stars...` broke because of item 4, not this item: its fixture holds a German word. The expectation now drops "hund" in the release profile only.
- No test asserted telemetry sending or accounts at host level. The Climb module's own tests (`entrance_tests`) now compile only with `accounts_net`. They pass in `cargo test --lib --features all_langs,net_services`.
- New dead-code warnings in the release build were silenced at the source (telemetry/mod.rs:26, celebrate.rs). No other warning was added.
- No runtime flag was removed, so `config/flags.release.json` is unchanged and flags-audit is green.

Left in place, unreachable in the App Store build:
- index.html keeps the front door, the account sheet, the leaderboard markup and the `SpellTelemetryPost` helper. Only wasm calls that helper.
- The Swift side of server STT is still in the iOS binary. Nothing hands it a URL.
- `ghostStart` (Spell Racing, status hidden) clicks `climbBtn`, which used to open the leaderboard, not a run. In the App Store build that click does nothing. This is pre-existing and not changed.

## 3. Parity: local English grade vs the server's /api/check

    python3 tools/parity/en_check_parity.py            # requires 0 diffs
    python3 tools/parity/en_check_parity.py --mutate   # self-test: a wrong server MUST diff

How it works:
- A Rust `#[ignore]` test (`src/en_check_parity.rs`) writes the local verdicts: `en_route` + `en_grade_local`.
- The driver lifts `check()` out of `backend/app.py` with `ast` and runs that function body in-process. There is no Flask and no network.
- It compares the verdict a player gets in each build: release (local) vs dev (server).
- `other_sense` and `variant` are decided before any server call in both builds, so only `grade` cases reach the comparison.

Result (2026-11-07):
- 43,896 cases from 4,366 bank words, plus 209 UK/US pairs from data/en/accepted-variants.tsv. tests/fixtures/en-uk-variants.json does not exist.
- Tiers covered: legacy easy 762, medium 798, hard 800, expert 797; retier easy 825, medium 2466, hard 596, expert 278.
- Routes: grade 43,022, variant 738, other_sense 136.
- 0 diffs.
- `--mutate` (a server that accepts any prefix) gives 8,694 diffs, so the check can fail.

Typed forms per word, each graded both normal and Spell Jr:
- exact, UPPER, Capital
- a dropped last letter and a swap of letters 2 and 3
- curly apostrophe, no space, no hyphen and accents stripped, but only where the word has one of those features

No bank word has an apostrophe, space, hyphen or accent, so those last four forms produced no cases. That is where the two graders would disagree if such a word were ever added: `fold_strict` folds apostrophes and drops whitespace, `fold_lenient` strips accents for Spell Jr, and the server does `lower().strip()` equality.

## 4. Hidden-language My Words lists: kept, never played

- `consts::speak_tag_servable` (consts.rs:387): in the release profile a saved word is served only if its speak tag's base is a listed language. That is English, plus untagged words, which CustomSet treats as English. A voice-only language (ro, id, ca) is no longer offered by the release "Speak in" picker, so it counts as unservable too.
- Filtered at play time:
  - `word_lists::play_entries_with` (word_lists.rs:452) and `starred_words` (:660)
  - the legacy CustomSet fallback in `game::pool_for_tier` (game.rs:127)
- `word_lists::refuses_play` (word_lists.rs:465) and `lists_ui::play`: a list that holds words, none of them servable, does not start. The toast is `mode.unavailable`, "Not ready in this language yet", and the saved selection is untouched. An empty list behaves as before.
- Nothing is deleted. `all_words` and the list screen still show every word and star.
- Word Search and WordCross from a list already filter to the current play language.
- Test: `word_lists::hidden_lang_tests::the_release_profile_keeps_hidden_language_words_but_never_plays_them`. It also checks that the lists are unchanged (`assert_eq!(l, before)`). The mirror test runs in all-language builds.

## Decisions for Eric

1. **spellgame.net keeps all four paths** (`web = ["net_services"]`), so the site build is unchanged. If the site should drop any of them, remove it from `web` in Cargo.toml. seam-absence-check then needs SPELL_WEB=1 to stop allowing those needles.
2. **The education build drops telemetry entirely.** Before, it sent daily aggregate counts (D7). Accounts were already gated off there.
3. A launch whose saved study language is My Words, with only Hidden-language words chosen, serves the placeholder "word". That is the existing empty-My-Words behaviour, and none of the Hidden words are served.
