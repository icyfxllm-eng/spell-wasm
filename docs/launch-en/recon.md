# CC-LAUNCH-EN v1 — §0 Recon

Read-only. Main @2bf45f41, 2026-11-07. No product code changed. Detail sections below are from three parallel passes (A: R1/R2/R4/R10, B: R3/R8/R9/R11, C: R5/R6/R7); each claim carries file:line or sha. Items I could not verify are marked unknown.

## Summary for Eric's review

**HALT conditions**
- R2 local grant path: NOT triggered. The only grant door (`spell_dev_entitlements`) is `dev_preview`-gated and cut from release dist by `scripts/cut-diag.py`; `seam-absence-check.mjs` fails the build on any trace. `entitlements:check` passes.
- R5 SENSE-CUE Phase B: NOT triggered. Merged on main (every language `cueMode: "off"`).
- R7 zero working parent features: effectively TRIGGERED today (see below). D4 says Complete SKU is not created unless this changes.

**Findings that change the plan**
1. **There is no StoreKit on main.** No Restore, no `currentEntitlements`, no `.storekit` file. `live_entitlements` hard-codes `purchased=false` (src/play_hub.rs:122-124). Only StoreKit code is on `feature/entitlements-ios` (d382f505), never compile-verified. F4 is a from-scratch build. `english` and `complete_upgrade` products do not exist; only `net.spellgame.complete` (src/entitlements.rs:49).
2. **Free tier is wrong for the plan.** English is Full and free today (src/entitlements.rs:125). The Easy-only preview rule applies to non-English only. D1 needs a core entitlement change.
3. **D4 / R7.** Parent features are built (photo lists, progress reports, Guardian Report) and registry-gated, but with no purchase path nothing unlocks, and the tester dev door was removed from release binaries (d2a7ed32; builds 310-312). Kid profiles: not started. Custom lists: no cap exists (Eric's 2026-09-17 "no free limit" ruling), so cannot be a Complete benefit. Complete becomes sellable once StoreKit is wired AND photo lists + reports are verified on device. Eric confirms O4 after reading R7.
4. **15 languages are Active in the app** (only Persian gated). F3 needs a play-language registry change in the app itself, not just the web.
5. **Audio chain F6/D13 is unbuilt.** No Pi (audio is the Mac mini via Cloudflare tunnel). No CDN code. No English audio bundled (`runtime.json` `"langs": {}`). Bundling Easy ≈ 14.6 MB. ~1,191 retier words have no cached audio (Hard 132/596, Expert 34/278, Medium 2038/2466, Easy 770/825). No timeout/offline screen on the audio path; A6.1 (3 s) unmet.
6. **Web gating (D7).** English already Full and free on web; other 15 languages still show coming-soon tiles (a test asserts it), conflicting with D3. Medium effort.
7. **Flags.** ~35 default-ON on main, nearly all post-56. `release/online-testflight` turns ~30 more on by editing src/flags.rs. The release base must be chosen explicitly. Remote kill file is kill-only (src/killflags.rs), but any flag can be set locally via `localStorage['spell_flag_x']`; F10 and I4.4 must account for that (none of those flags touch the entitlement set today). `en.retier` default ON.
8. **Placeholders.** User-visible "Coming soon" in en.json (8 lines), index.html:3513, privacy.html:122. "struggl": 0 user-visible hits.
9. **Store text contradicts app/policy.** `store/app-review-notes.md` ("no personal data", "mic inert", 4+) and `RELEASE_CHECKLIST.md` ("Data Not Collected") vs `privacy.html` (Climb accounts, telemetry on by default 13+, optional speech egress to Google). Note also C5.10 "no account anywhere" vs Climb accounts — needs checking. No `PrivacyInfo.xcprivacy` in repo.
10. **EN-VARIANTS** only partly on main (table + 117 staged pairs); live wiring on branch `en-variants-live` (c7530bf0), unmerged, no flag. Open census questions unruled.
11. **iPad is enabled** ("1,2", Split View on); two committed iPad shots, one portrait layout test. D16 default outcome: iPhone-only.

**Needs Eric (cannot be read from repo):** ASC age-rating and App Privacy answers; whether IAP exists + status; latest uploaded build; Paid Applications agreement; Small Business Program; whether live server runs VOICE-CLARITY code.

**Decisions this recon asks of you:** release base branch/flags (R1.3); whether to reopen the stale store text; whether to proceed to Phase A knowing it is a StoreKit build from scratch; O4.

---

# Launch-EN recon, part A (R1, R2, R4, R10)

Worktree /Users/eric/repos/spell-launch-en, main @2bf45f41, recon date 2026-11-07. Read-only; no product code changed. "unknown" = not determinable from the repo.

Build-56 dating: release note docs/build-56-release-note.md:1-3 says it cuts as build 56. Its commits land 2026-07-19 to 2026-07-23 (`git log`: 3381daff 2026-07-19, b24cff1a 2026-07-23). "After 56" below means introduced after 2026-07-23.

## Headline findings

1. **No StoreKit exists in the repo.** The resolver is always called with `purchased=false` and no regional grants (src/play_hub.rs:122-123). See R2.
2. **No local flag or path grants an entitlement in a release build.** The only one, the "Test entitlements" dev door, is cut at compile time and at dist time (R2.2). Not a HALT, but the guarantee rests on build discipline (R2.4).
3. **Structural mismatch (HALT-class for the plan).** `free_tier()` gives English **Full** (all tiers, Daily, Climb) for free (src/entitlements.rs:125-131 and its doc at :110-124). The planned model needs English limited to Easy and a paid `english` unlock. The core has no `english` or `complete_upgrade` concept. Its only product id is `net.spellgame.complete` (src/entitlements.rs:49). Preview (Easy tier only) exists but is applied only to non-English languages (src/entitlements.rs:321-330).
4. **Flag defaults on main differ from what has been shipping.** Release branches flip many OFF flags to ON by editing src/flags.rs (R1.3). The launch base (main or release/online-testflight) decides what ships.
5. **Remote flag file is kill-only; telemetry has a separate remote flag that can turn ON.** See R1.4.
6. **Placeholder strings:** user-visible "Coming soon" copy exists in en.json and index.html. It is intended for non-English languages and is reachable only if a ComingSoon language is selectable. See R4.

## R1 Flags

### R1.1 Mechanism
- Every runtime flag is a `pub fn -> bool` in src/flags.rs, read from `localStorage['spell_flag_<name>']`. Absent, it falls back to the compiled default (src/flags.rs:1-6, resolver at :16-22, `stored()` at :26-35).
- Values "on/1/true/yes" and "off/0/false/no" are honoured; anything else falls back to the default (src/flags.rs:16-22).
- Any local value can turn a flag ON or OFF, because `stored()` returns localStorage as-is after the kill check (src/flags.rs:26-35).
- The Tools hub writes `spell_flag_*` from UI toggles (src/tools_hub.rs:105-112). index.html:5282 documents the key.
- `is_on(name)` (src/flags.rs:767) is the hub dispatcher. `_ => false` for any id without an arm.
- Site build (`--features web`) forces `pick_queue_a..d` to false (src/flags.rs:617-665) and gates many arms with `#[cfg(not(feature="web"))]`.

### R1.2 Runtime flags on main @2bf45f41
"Remote" is y only through the kill-only file, which can turn a flag OFF (R1.4). All rows are y for kill, none for ON. "Post-56 ON" marks a default-ON flag introduced after build 56. Dates are first commit adding the flag or its `true` default, from `git log -S`.

| Flag (flags.rs line) | Default | Post-56 ON? | Introduced |
|---|---|---|---|
| en_retier :67 | **ON** | **YES** | 2026-10-17 added; default ON 2026-10-19 (4e83f15c). Doc :60-66 |
| ghost_racing :109 | ON | no | 2026-07-13 (e92ae6e9) |
| syllable_replay :115 | ON | no | 2026-07-13 |
| photo_list :120 | ON | no (shipped by 54 per release note) | unknown date; before 56 |
| spell_aloud :438 | ON | no | 2026-07-15 (bdf887d9) |
| attempts_shields :450 | ON | no | 2026-07-15 |
| def_match :478 | ON | **YES** | 2026-07-27 (64219e3f) |
| practice :555 | ON | **YES** | 2026-07-27 (55918323) |
| word_picture :488 | ON | **YES** | 2026-07-28 (ec621eba) |
| learner_select :564 | ON | **YES** | 2026-08-02, ON 2026-08-05 |
| learner_surfaces :593 | ON | **YES** | 2026-08-02, ON 2026-08-05 |
| streak_focal :573 | ON | **YES** | 2026-08-04 |
| learner_insight :584 | ON | **YES** | 2026-08-05, ON 2026-08-06 |
| reports :506 | ON | **YES** | 2026-08-07 (fb917726) |
| calendar :517 | ON | **YES** | 2026-08-07 |
| letter_forge :129 | ON | **YES** | 2026-08-12, ON 08-13 |
| word_chains :138 | ON | **YES** | 2026-08-12, ON 08-13 |
| impostor :144 | ON | **YES** | 2026-08-12, ON 08-13 |
| bee_sim :152 | ON | **YES** | 2026-08-13 |
| spelldoku :534 | ON | **YES** | 2026-09-18 |
| spell_search :542 | ON | **YES** | 2026-09-19 |
| spell_cross :549 | ON | **YES** | 2026-09-19 |
| boardgame :162 | ON | **YES** | 2026-10-08 |
| board_stretch :171 (key `boardStretch`) | ON | **YES** | 2026-10-09 |
| board_boosts :239 (`boardBoosts`) | ON | **YES** | 2026-10-09 |
| board_streak :248 (`boardStreak`) | ON | **YES** | 2026-10-09 |
| spelluzzle :181 | ON | **YES** | 2026-10-09 |
| spelluzzle_pencil/_strip/_ripple :190-198 | ON (and parent) | **YES** | 2026-10-10 |
| spelluzzle_par :205, _expedition :213, _mywords :217, _duel :224 | ON (and parent) | **YES** | 2026-10-10 |
| spelluzzle_lang(lang) :231 | ON only for `en` | **YES** | 2026-10-09 |
| randospell :261 | ON | **YES** | 2026-10-14 |
| randospell_marathon :268 | ON (and parent) | **YES** | 2026-10-14 |
| progress_report :513 | ON | **YES** | 2026-10-27 (4bb3835f) |
| guardian_report_days :75 | OFF | n/a | 2026-11-07 |
| cant_tell :83 | OFF | n/a | |
| impossible :91, impossible_lang(lang) :98, impossible_mix :104 | OFF | n/a | |
| randospell_daily/h2h/jr/heat/call/stretch/doubles/ear/npc/steal/lastcall/fx :275-345 | OFF (each also needs `randospell`) | n/a | |
| quickbee_draw_lang :354, quickbee_draw :360 (keys `quickbee_draw_zh/ja/ko`, src/quickbee_draw/mod.rs:88-90) | OFF | n/a | |
| word_stories :366, online_spelloff :373 | OFF | n/a | |
| online_enabled :381, online_enabled_built :388, online_kids :395, online_boardgame :402 | OFF | n/a | |
| accounts_enabled :409, accounts_sync :416, accounts_online_gate :421, accounts_family :427 | OFF | n/a | |
| offline_packs :468, translate :526, sd_tier_mode :603 | OFF | n/a | |
| pick_queue_a/b/c/d :613-665 | OFF (always false with `web`) | n/a | |
| visual_type :673, visual_feedback :683, visual_orb :691, keyboard_flight :698, keyboard_constellation :705, keyboard_react :712, visual_sound :719, visual_haptics :727, visual_icons :734, visual_celebrate :741, visual_cards :749, visual_themes :756, visual_accents :763 | OFF on main | n/a | |

- The 4 core ids `standard|climb|daily|my_words|misses|versus` are unconditional true in `is_on` (src/flags.rs:781). They are not flags.
- The default-ON set above is "everything shipped dark then turned on".
- Whether each ON flag's feature is wanted in the English launch is out of scope for this recon.

### R1.3 Main vs release-branch divergence (important)
- `git diff 2bf45f41 release/online-testflight -- src/flags.rs` (tip 39c816b7, 2026-10-31) flips these from OFF on main to ON, by editing the default in code:
  - impossible, impossible_lang (`launch`), impossible_mix
  - randospell_daily/h2h/jr/heat/call/stretch/doubles/ear/npc/steal/lastcall/fx
  - quickbee_draw_* (`c.measured`)
  - online_enabled, online_enabled_built, and the next entries (online_boardgame is visible in the diff)
  - all 13 visual_* / keyboard_* flags
- The doc comments for those flags say "the build turns it on". There is no config or define for this. The mechanism is a source edit on a release branch.
- main @2bf45f41 therefore does not describe what testers or past builds carried. The launch branch must pin an explicit flag table. Today it has none.
- The ON/OFF list for the launch base is unknown until the base branch is chosen.

### R1.4 Remote flag file
- The signed file is src/killflags.rs, fetched from `/api/kill-flags` (backend/app.py:758-761), verified with an Ed25519 key compiled in, cached under `spell_kill_flags_v1`.
- It is **kill-only**: only `kill` names are read and there is no "on" list (src/killflags.rs:1-14, 66-80). Test: src/killflags/tests.rs:82 `the_file_can_never_turn_a_flag_on`.
- `flags::stored()` returns "off" for killed names before reading localStorage (src/flags.rs:28-35).
- It is fetched at startup (src/lib.rs:289-290) and on online screens (src/online_screen.rs:278, 302).
- **Second remote flag, not kill-only:** telemetry reads `/v1/flags` from the telemetry Worker, and `telemetry_enabled=true` writes `spell_flag_telemetry_enabled=on` (src/telemetry/mod.rs:50, 216-221, 625-640). That can turn telemetry ON remotely. Whether the player's opt-in (`OPT_KEY`) still gates upload: unknown, not read.
- There is no remote-config channel for any other flag.
- Local override path: `localStorage['spell_flag_x']='on'` (via Tools hub toggle or devtools) beats the compiled default, for any flag not killed.

### R1.5 Build-time features (Cargo.toml `[features]`, lines 16-50)
| Feature | Effect | Default off? | Set where |
|---|---|---|---|
| testseam | `window.__spelltest` seam | yes | scripts/build-web-test.sh:21-23 |
| dev_preview (implies theme_lab) | dev menu, Test entitlements, inspector, burger | yes | build-web-dev.sh:16-17; build-web-test.sh |
| theme_lab | Theme Lab (internal TestFlight only) | yes | `SPELL_THEME_LAB=1` in scripts/build-web.sh:25-33; ledger fastlane/lab-builds.txt |
| audit_preview | un-gates ComingSoon languages, loads draft banks | yes | scripts/build-web-audit.sh |
| web | spellgame.net English-only site | yes | `SPELL_WEB=1` |
| education | education edition, no purchase surface | yes | scripts/build-web-edu.sh |
| native | room-server grader parity | yes | |
- `MIC_DIAG` is not a define or feature. The equivalent is the `SPELL-DIAG:BEGIN/END` sentinel regions in index.html and native-language-kit.js, cut by scripts/cut-diag.py (called at scripts/build-web.sh:90, build-web-audit.sh:48, build-web-edu.sh:55).
- `DEV_PREVIEW` and `THEME_LAB` appear only as the cargo features `dev_preview` and `theme_lab`.
- Env vars that matter: `SPELL_WEB`, `SPELL_THEME_LAB`, `AUDIT_MODE`. `AUDIT_MODE` is only a CI lane variable and is not read by any src or JS code (grep: only .github/workflows/*.yml and scripts/audit-mode-release-check.mjs).

## R2 Entitlements

### R2.1 Where entitlement is resolved
- THE resolver: `resolve_entitlements(purchased, regional_grants, audit_override)` (src/entitlements.rs:164-170) and `resolve_for_edition` (:179-232).
- `free_tier()` (:125): English `Full`, every other shipped language `Preview`, all parent-premium off.
- Purchase only raises everything to Full plus `photo_ocr`, `multiple_profiles`, `progress_reports` (`raise_to_complete`, :273).
- Preview = Standard mode, tier 1 only (:321-330). Daily, Climb, GhostRace denied at Preview.
- Product id: `COMPLETE_PRODUCT_ID = "net.spellgame.complete"` (:49), compiled out of the education edition.
- There is **no** `english` or `complete_upgrade` product, no upgrade-price logic, and no English-Preview path.
- Consumers of the live set:
  - src/play_hub.rs:122-123 `live_entitlements()`, the single call
  - src/photo_list.rs:61 (photo OCR)
  - src/yearbook_ui.rs:96 (progress_reports)
  - src/translate_ui.rs:408, src/quickbee_draw_screen.rs:132, src/randospell_screen.rs:273, src/wordpic_screen.rs:365, src/widgets.rs:36
  - src/defmatch.rs:156 (`allowed_tiers`)
  - src/modes.rs hub gating (:393-395, premium list)

### R2.2 Every path that can grant (FINDINGS)
| Path | Status in a release build | Evidence |
|---|---|---|
| "Test entitlements (Complete)" dev door: `localStorage spell_dev_entitlements=on` (5 taps on logo) | **Not present** in release: Rust read is `#[cfg(feature="dev_preview")]`, the `#[cfg(not)]` twin returns false. The HTML row and the JS reader sit in SPELL-DIAG regions cut by cut-diag.py | src/play_hub.rs:131-139, src/dev.rs:94-103, index.html:3855-3861, native-language-kit.js:425-436, scripts/cut-diag.py |
| `audit_override` (AUDIT_MODE) | Only reachable as the `dev_grant()` argument above. No env or build define feeds it. Ignored in education (:188) | src/play_hub.rs:123, src/entitlements.rs:188 |
| Regional grant (CF-IPCountry / `country-language-map.json`) | **Not wired into the client.** Backend route exists (`/api/entitlements/regional`) but no src/JS caller; `live_entitlements` passes `&[]` | backend/entitlements.py:1-40, 76-87; grep of src/index.html/sw.js for "api/entitlements" returns nothing; src/play_hub.rs:123 |
| `education` build | Grants all active languages and premium. Compile-time, separate bundle id | src/entitlements.rs:249-258, Cargo.toml:25 |
| URL params | None found: grep for URLSearchParams / location.search in src, index.html, JS: no hits | |
| localStorage `spell_flag_*` | Changes features, **not** entitlements (no flag feeds the resolver) | src/flags.rs |
| StoreKit / Play Billing / receipts | **Do not exist** | R2.3 |
- **No local flag or path grants an entitlement in a release build. No HALT on that criterion.**
- Caveats:
  - The protection is compile-time (`dev_preview` absent) plus dist cut. Both are asserted by `scripts/seam-absence-check.mjs` (needles `dev_preview`, `devEntitlements`, `spell_dev_entitlements`, `diag: `, scripts/seam-absence-check.mjs:17-21), which `npm run build` runs.
  - `scripts/diag-dev-door-check.mjs` (gate.sh:278-279) checks the same door.
  - `dist-test-web/` is **tracked in git** (27 files) and contains the dev door (`dist-test-web/native-language-kit.js:461`, `index.html:2076`). It is not shipped (capacitor webDir=`dist`, capacitor.config.json:3), but it is a stale artifact carrying the grant row. Recommend confirming it is excluded from every artifact.
  - E2E and Maestro flows depend on the door (`tests/e2e/hotfix-lib.mjs:54`, `.maestro/photo-import.yaml:12-13`), so a reviewer walkthrough that needs Complete will fail on a release build. Needs a different mechanism (a real sandbox purchase).

### R2.3 StoreKit integration today: none
- grep for StoreKit, SKProduct, Product.products, currentEntitlements, Transaction., restorePurchases, SKPaymentQueue and revocation across ios/, android/, src/, index.html, native-language-kit.js, capacitor.config.json, package.json: **zero** purchase-API hits. The only mentions are doc comments, src/entitlements.rs:7,15,20.
- ios/App/App has no StoreKit swift file. Plugins: NativeLanguageKitPlugin and extensions (FamilyVoices, Ink, Keychain, Packs, PhotoList, SavePicture, Widgets), SpellIntents, AudioSessionOwner. The `+Keychain` extension is not a purchase path.
- No Restore Purchases button. The code comment at src/play_hub.rs:164 reads "When the purchase adapters land they feed the same call".
- No `.storekit` configuration file found.
- No revocation or refund handling.
- No Capacitor purchase plugin in package.json (dependencies at package.json:35-44).
- Hardcoded price strings: grep for `$4.99|$12.99|$7.99|4.99|12.99|7.99|price` over src/, index.html, i18n locales, JS, ios/App/App: **no price literals**. Hits are unrelated ("priced", "price of that digit"), and one deliberate rule: Spell Jr says "no prices" (src/i18n/locales/en.json:630, src/calendar_ui.rs:184, src/themes.rs:9 "never a price").
- Product prices must come from StoreKit `displayPrice`, with no existing code for it.
- `COMPLETE_PRODUCT_ID` is referenced by nothing but its definition. `scripts/entitlement-core-purity-check.mjs` forbids the id string outside the core.
- Store text: store/app-review-notes.md and fastlane/metadata/en-US exist. Their IAP content was not read, so whether they mention a purchase is unknown.

### R2.4 Existing guards
- `npm run entitlements:check` passes (run 2026-11-07): map check, selftest, core purity, audit-mode release check.
  - Output: "entitlement-core-purity-check: OK"; "audit-mode-release-check: OK — release lanes pin AUDIT_MODE=0 and none enable the bypass".
  - It does not run the Rust entitlement tests.
- Rust tests: src/entitlements.rs:400-720 (proptests: monotonic, edition parity, audit dominates, RTL clamp). CI: `cargo test --lib entitlements` (.github/workflows/entitlements.yml:65).
- Dev-door law: seam-absence-check.mjs and diag-dev-door-check.mjs.
- No test asserts the new product model (English Easy free, `english`, `complete`, `complete_upgrade`), because it does not exist.

## R4 Placeholder strings
Command (case-insensitive) over src/, index.html, i18n (root dir holds only REVIEW_NEEDED.md and STRING_AUDIT.md), ios/App, ios/NativeLanguageKit, ios/ScanStack, *.js, sw.js, manifest.json, privacy.html, notices.html: `grep -rniI "coming soon\|not yet available\|under construction\|lorem\|TODO\|FIXME"`. Result: 50 raw hits, 35 of them false positives (Spanish/Portuguese "todo/todos", plus a CSS class `"todo"`).

### User-visible (English)
- src/i18n/locales/en.json:189 `coming.badge` "Coming soon". Rendered at index.html:3513 inside `#comingPanel`, shown when the study language is not active.
- src/i18n/locales/en.json:191 `coming.notice` "...coming back soon." (matched by "soon", not by the strict pattern).
- en.json:549 `racing.soon` "Live racing is coming soon." Use site unknown (not traced).
- en.json:574 `sd.avail`, :866 `ws.avail`, :889 `xw.avail` "Coming soon in this language" (play_hub.rs:327-333, wordsearch_ui.rs:213, 623, wordcross_ui.rs:241). Shown when a mode lacks a language.
- en.json:720 `tools.defmatch.avail` "All platforms · coming soon" (play_hub.rs:337).
- en.json:730 `tools.practice.avail` "Coming to this language soon". (Not matched by the strict pattern; found by "soon".)
- en.json:962 `bg.soon`, :1066 `rs.soon` "Not available in this language yet".
- privacy.html:122 mentions the "coming soon" language label and the "Vote for this language" control.
- Non-English locales: es.json and pt.json hits are the word "todo/todos" only. Whether other locales carry a "coming soon" equivalent: not checked.
- English-only launch: the Coming Soon panel is reachable only if non-English languages remain selectable. consts.rs marks fa as `ComingSoon` (src/consts.rs:194-197). Which other languages are Active/ComingSoon on the launch base: unknown, depends on the base.

### Comment-only
- src/consts.rs:173, 297, 318 (doc comments), src/play_hub.rs:315, src/boardgame_pools.rs:78, 177, 191 (eprintln in a test).
- src/lib.rs:1652 `TODO once the app adds back-handling` (the only real TODO/FIXME in src; comment).
- src/quickbee_draw_screen.rs:388 and src/practice_screen.rs:836 CSS class "todo"; src/spell_aloud*.rs, tests and word_data.rs: Spanish "borrar todo" / word-list data.
- ios/App/App/es.lproj/InfoPlist.strings:3: Spanish "todo".
- No lorem, "under construction", "not yet available" or FIXME hits anywhere searched.

### "struggl"
- `grep -rniI struggl` over src, index.html, ios/App/App, ios/NativeLanguageKit, store, fastlane: only src/learner.rs:1835 and :1838 (test fixture names) and src/reports_share.rs:574 (a test of the prohibited-word list). **No user-visible surface carries "struggl".** No hit in fastlane metadata or store/.

## R10 Commands (status: exists / partial / missing)

| Need | Status | Command and note |
|---|---|---|
| Unit + entitlement tests | exists (no new-model tests) | `cargo test` (full; run inside scripts/gate.sh:297-305). Fast: `cargo test --lib entitlements` and `cargo test --lib modes` (CI .github/workflows/entitlements.yml:65, modes.yml:55). Also `npm run entitlements:check` (ran, passes). |
| Flags audit (feature flags) | **missing** | No script lists flags or compares them to an expected table. `scripts/flags-check.mjs` is **not** this: it bans flag emoji and national symbols (scripts/flags-check.mjs:1-8). Unit tests exist in src/flags.rs (:800+) for the resolver only. |
| Placeholder grep | **missing** (manual) | The R4 grep above. `npm run i18n:check`, `scripts/i18n-translated-check.mjs` and `scripts/hardcoded-label-check.mjs` cover other string faults, not "coming soon". |
| Price-string grep | **missing** | The R2.3 grep. Nothing automated. |
| Lint | **partial** | `cargo clippy` is used for the server crates only (.github/workflows/game-server.yml:35); none for the app crate. `npm run viewport:check`, `kb:check`, `check:contrast`, `i18n:check`. |
| Full gate | exists | `bash scripts/gate.sh` (preflight, ~60 checks, cargo test, app build, e2e app+site, web wall scan). Slow; not run. |
| Build | exists | `npm run build` (i18n, human-audio, viewport, wordlists, build-web.sh, seam-absence, bundle scans, manifest, picture/chrome/vision checks, registry test, tracer-core check). `npm run sync:ios`. Ship: `bundle exec fastlane ship` (fastlane/Fastfile:233, runs must-contain-check then `npm run sync:ios`). |
| E2E (web) | exists | `npm run e2e` (app + site; uses testseam build) or `node tests/e2e/run.mjs`. 88 files in tests/e2e/specs. |
| Maestro / iOS e2e | **partial** | .maestro/ has 5 flows (photo-import, photo-replace-effect, tool-photo-effect, tool-spellaloud-effect, tool-spelloff-effect). They are tool-effect tests, not a reviewer walkthrough. They depend on the dev-entitlement door (`.maestro/photo-import.yaml:12-13`). tests/README.md names `tests/ios-ui/flows/*.yaml` and a `ui_tests` lane, but tests/ios-ui is effectively empty (1 entry) and no `ui_tests` lane was seen. |
| Maestro reviewer walkthrough (purchase, restore, free tier) | **missing** | Needs a StoreKit-config or sandbox flow, which cannot exist before StoreKit does. |
| Repetition sim | **missing** (unknown) | No script named for a repetition sim. Nearest: `tests/learner_simulator.rs` (learner/spaced-repetition simulator, `cargo test --test learner_simulator`) and the FSRS vectors in tests/fsrs_reference_vectors.rs. Whether either matches your intended "repetition sim": unknown. |
| Whisper loopback gate | **partial** | `tools/audio-verify/verify.py` (TTS clips through whisper.cpp) and `tools/spell-aloud-loopback/loopback.py` (letter accuracy >= 95%). Both run on Eric's machine, not in CI or gate.sh; tests/README.md Part 3. Needs whisper.cpp and models. |
| Release build fails if DEV_PREVIEW / THEME_LAB / MIC_DIAG present | **exists** (partial for naming) | `scripts/seam-absence-check.mjs dist` runs inside `npm run build` and `npm run seam:check`. It scans dist for `dev_preview`, `testseam`, `__spelltest`, `devEntitlements`, `spell_dev_entitlements`, `voiceSpellDiag`, `devDiagBar`, `spell_diag_bar`, `diag: ` and, outside Lab builds, the Theme Lab needles. Exit 1 on any hit. Selftest: `node scripts/seam-absence-check.mjs --selftest`. Mic diag is cut by `cut-diag.py`, with the law in scripts/diag-dev-door-check.mjs. There is no "MIC_DIAG" token. A guard against the Lab build being submitted: fastlane/lab-builds.txt. |
| Audit-bypass in release lanes | exists | `npm run entitlements:audit-mode` |
| Must-contain fixes | exists | `node scripts/must-contain-check.mjs` (`npm run must-contain`) |

Fast checks run this session: `npm run entitlements:check` returned OK on all four sub-checks. Nothing else was run.

# Recon part B: R3 audio path, R8 bank census + no-repeat, R9 spellgame.net, R11 iPad
Worktree /Users/eric/repos/spell-launch-en, main @2bf45f41. Read-only; only this file was written.
Method notes: cache coverage computed by importing the DEPLOYED backend (~/spellgame-server/backend/app.py, venv python, dummy TTS key, AUDIO_CACHE_DIR=~/spellgame-server/cache) and calling its own `cache_path_for(word, variant, "en")` then `os.path.exists`. Scratch script: /private/tmp/claude-501/-Users-eric--claude-spares-spare-NBTXdy/b6feaad0-8b47-4c90-bb71-f359530f2667/scratchpad/cov.py. No server started, nothing synthesized. (The LaunchAgent plist holds live API keys in plaintext; I did not copy them.)

## Corrections to the brief (important)
1. **There is no Raspberry Pi.** The audio origin is the Mac mini: docs/CC-GAME-SERVER.md:31 ("TTS comes from the Mac mini backend (/api/speak) over Tailscale, not a Pi"); ~/Library/LaunchAgents/net.spellgame.backend.plist runs gunicorn 127.0.0.1:8000 from ~/spellgame-server/backend; net.spellgame.tunnel.plist is the Cloudflare tunnel. Older docs (CC-OFFLINE-PACKS.md:4, CC-PERSIAN-FOUNDATION.md:31) still say "Pi".
2. **No CDN code exists.** `grep -i cdn|r2.|pages.dev` over src, index.html, sw.js, scripts/build-web.sh, cloudflared, Caddyfile finds only a font CDN (index.html:36). There is exactly one audio base URL, `window.SPELL_API_BASE = "https://spellgame.net"` (index.html:5379), read by `api_base()` (src/api.rs:148-160). The planned "bundled -> CDN -> Pi" chain is NOT implemented; only "bundled -> origin -> device voice" exists (below).
3. **No English audio is bundled today.** assets/human-audio/runtime.json has `"langs": {}` with comment "English human audio is OFF (Eric, 2026-09-20)"; assets/human-audio/en/ holds only bundle.json + verdicts.json (no .m4a; `ls | wc -l` = 2). build-web.sh:99-104 would copy .m4a files into dist/human-audio/ only if they existed. `dist/` and `ios/App/App/public` do not exist in this worktree, so the as-built binary contents are unverified; the source-level answer is zero clips.
4. **English is not "Easy-only free" anywhere.** `free_tier()` gives English `Full` (src/entitlements.rs:~118-135 doc + body) and every other shipped language `Preview` (Easy tier only, DEFAULT_PREVIEW_TIER=1, entitlements.rs:294). `live_entitlements()` hard-codes `purchased=false` (src/play_hub.rs:122-124). There is no StoreKit adapter: no Swift file references StoreKit, and COMPLETE_PRODUCT_ID is used only inside entitlements.rs and CI scripts. So today no purchase can happen on any platform.
5. **The app is not English-only.** `BUILTIN_LANGS` marks 15 languages Active (src/consts.rs:179-193); only Persian is ComingSoon (test consts.rs ~line 520-545 asserts gated == ["fa"]). An English-only App Store build needs a registry change; that is outside R3/R8/R9/R11 and is flagged as a gap.
6. **`en.retier` is default ON**, contrary to the memory note ("off"): src/flags.rs:67 `resolve(stored("en_retier").as_deref(), true)`, commit 4e83f15c "en.retier default ON". So the English bank players get is the RE-TIERED one (non Jr/Kid); see R8.

## R3 Audio path
### What is bundled (English)
- Human audio: nothing (see correction 3). Source order = Human, Pack, ServerCache, NativeTts (src/api.rs:90-96, test api.rs:108-117).
- Wasm-compiled data only: word lists, `audio_verdict`, lead-in list (src/audio_verdict.rs:49 comment about 250 KB bundle). No mp3/m4a for TTS words ships in the app.
- Size to bundle Easy, for planning: current EN_EASY (762 words) = 1524 clips (normal+slow) = 14,559,936 bytes (14.6 MB); RETIER Easy (825 words) = 1533 clips present = 14,647,872 bytes (14.6 MB). All measured from ~/spellgame-server/cache.
- Offline "Pack" source exists in code (src/packs.rs, ios NativeLanguageKitPlugin+Packs.swift, backend build_packs.py on the server) but is a user-initiated download from the server, signed with PACK_PUB_KEY (packs.rs:16); docs/CC-OFFLINE-PACKS.md is the spec (38 lines, "REVIEW-GATED"). It is not a bundled-in-binary mechanism and has no CDN leg. Whether pack download UI is live in release: unknown (not traced).

### What is fetched, from where
- Each word: GET `{SPELL_API_BASE}/api/speak?word=&variant=normal|slow&lang=en[&v=li]` (src/api.rs:166-183). Server responds with `Cache-Control: public, max-age=31536000, immutable` (backend/app.py ~line 841-845), so the Cloudflare edge/HTTP cache may hold clips, but no purpose-built mirror exists.
- Native app path: `audio-native.js` `ensureCached` stats the on-device file (Directory.Cache, name = djb2 hash of assetId, audio-native.js:114-121, 138-177); on miss does a plain `fetch(url)` with NO timeout/AbortController, writes base64 to disk, preloads into NativeAudio, plays. Next word is prefetched (`preload_word_with`, api.rs ~lines 600-630; game.rs:2000-2010 uses deck.peek()); app-open warms first 20 words of the tier (PRELOAD_AT_OPEN=20, game.rs:166-192). Cache.directory is OS-purgeable ("Cache"), so on-device persistence is not guaranteed (inference from the Capacitor enum; not tested).
- Server miss behavior: if the mp3 is not in cache the backend SYNTHESIZES it live (app.py `_speak` ~lines 826-836: `if os.path.exists(path) ... else synthesize_to_cache`), 502 on failure. EN voice = en-US Neural2-E via Google; memory notes Google billing was off 2026-09-30 (unverified today). Uncached words therefore depend on live TTS being healthy.

### English cache coverage (cache is inspectable locally: ~/spellgame-server/cache, 277,913 files: 186,645 v4, 89,634 v3, 1,633 meaning json; current CACHE_VERSION="v4", backend/app.py:226; EN key includes voice Neural2-E, render profile vc1, and lead-in suffix for 124 lead-in words)
| bank | rows | normal cached | slow cached | both |
|---|---|---|---|---|
| EN_EASY (legacy, Jr/Kid) | 762 | 762 | 762 | 762 |
| EN_MEDIUM | 798 | 798 | 798 | 798 |
| EN_HARD | 800 | 800 | 800 | 800 |
| EN_EXPERT | 797 | 797 | 797 | 797 |
| **EN_RETIER_EASY (live default)** | 825 | 770 | 763 | 763 |
| **EN_RETIER_MEDIUM** | 2466 | 2038 | 2031 | 2031 |
| **EN_RETIER_HARD** | 596 | 132 | 128 | 128 |
| **EN_RETIER_EXPERT** | 278 | 34 | 34 | 34 |
=> With retier ON (default for non-Jr), uncached normal clips: Easy 55, Medium 428, Hard 464, Expert 244 (= 1,191 words need live synthesis on first play; slow variant a few more). Examples of uncached retier Easy: baseline, bloat, bookie, bulldog, certify, checkpoint, chirp, choppy, clown, cookware, cowgirl, downstream. This is the biggest launch risk for the CDN-mirror plan: the mirror must be built from a COMPLETE warm, and the warm (bin/warm-run.sh, memory note) has not covered retier Hard/Expert. Caveat: my path function is the deployed one but bank membership came from src/word_data*.rs in this worktree.
Mirror sizing: files are ~9.5 KB each (14.56 MB / 1524).

### Behavior when tunnel/server is down (trace)
1. Human: no clip -> skip. Pack: no active pack -> skip (api.rs:323-342; packs::src_for returns None unless active).
2. ServerCache native: `SpellAudio.playWord` -> on-disk hit plays offline; on miss `fetch` with no timeout. A dead tunnel returns a quick error (Cloudflare 530/1033) or a hung socket -> only the OS/WebView default timeout bounds it (unknown value, not tested). Rejection -> `play_word_html` fallback (api.rs:425, 497+): `<audio src=speak_url>` with onerror + play() rejection (once-guard), again no explicit timeout.
3. NativeTts (AVSpeechSynthesizer) rescue (api.rs:450-500): EN uses `native_lang::session_voice` / family voice; speaks the word on-device. So English never goes silent in the app if an iOS voice exists. Source order is fixed; no spinner is shown by this code path (grep for AbortController/timeout in api.rs and native_audio.rs: none).
4. If all sources fail: `set_source("none")` writes banner `voice.audioFail` = "Couldn't reach the audio server for this word — spelling still counts." (src/i18n/locales/en.json:786, api.rs:57-59) and Settings note "NO SOURCE — every source failed" (api.rs:50-51). zh excluded from native-TTS (api.rs:252-254).
5. There is no "offline state" screen, no network reachability check (grep navigator.onLine: none relevant), no CDN hop. Telemetry records Resolved/Unavailable (api.rs:220-231). Latency while the server hangs: unknown (no timeout in code).
6. Web: no native audio; `<audio>` -> Web Speech fallback (api.rs:459-466).
Gap vs plan: fallback "clear offline state" is not implemented beyond the one-line banner; CDN leg and per-hop timeouts need building. Single seam to change: `api_base()`/`speak_url()` (api.rs:148-183) and `ensureCached` fetch (audio-native.js:151).

## R8 English bank census
Files: tier source lists assets/words/en/{easy,medium,hard,expert}.txt (762/798/800/797 lines = legacy banks) and assets/words/en/retier/{easy,medium,hard,expert}.txt (826/2467/597/279 lines, includes header; held.txt 7, quarantine.txt 203). Compiled: src/word_data.rs (EN_EASY etc.) and src/word_data_en_retier.rs (EN_RETIER_*). Counted by parsing the .rs consts:
| tier | legacy (Jr/Kid, and fixed-bank puzzle modes) | RETIER (default for standard profile) |
|---|---|---|
| easy | 762 | 825 |
| medium | 798 | 2466 |
| hard | 800 | 596 |
| expert | 797 | 278 |
All unique within tier (0 dups), 0 uppercase, and 0 overlap between retier tiers (python check). PURITY: `python3 scripts/build-wordlists.py --check` -> "purity-gate: OK — 0 problem(s)" and "build-wordlists: OK (check) — 78328 words across 15 locales, gates green." Ledgers: assets/words/purity/en.tsv = 3157 rows all `dictionary` verdict (legacy, 3163 lines incl. 6 header); en-retier.tsv = 1209 rows all `dictionary` (1211 lines). 3157 legacy rows = 762+798+800+797 exactly. Retier purity ledger covers 1209 rows, NOT the 3,565 retier words (825+2466+596+278) — the rest are presumably carried from en.tsv; I did not verify that coverage ("unknown"). Note the purity gate output also printed 6 length-drops in es/de/pl (not English).
Selection: `words::en_tier()` (src/words.rs:75-96) returns retier when `retier_active()` = flag ON and not Jr/Kid profile (words.rs:101-106). Puzzle modes pin to the LEGACY bank via `tier_for_fixed` (words.rs:52-70): Spelluzzle, Spell Search, WordCross, Randospell. Spot oddity (not judged): legacy Easy includes tokens like "en", "don", "ad", "am".

### No-repeat mechanisms per mode
- **Normal / Climb / Spell Jr (free-play)**: persisted per-(lang:tier) `Deck` (src/deck.rs). Full shuffled pass over the pool, drawn down before reshuffle (deck.rs:57-75), persisted via DECKS_KEY (game.rs:2090-2092), key `"{lang}:{tier}"` (game.rs:1961); plus RECENT_CAP=5 and a boundary guard R=min(3,|pool|/2) (deck.rs:132-150). Pool = `active_word_list` (game.rs:133-147; kid filter applied for kid). Climb uses same path with tier from `serve_climb_tier` (game.rs:230-240; Jr Climb is a fixed ramp). Test: `deck_no_repeat` (deck.rs:177-205) checks pools 2,3,10,41,200 for 5 passes: no repeat inside a pass, no repeat within R across reshuffle. Also `deck_handles_pool_growth`.
  - **200 draws/tier without a repeat?** Yes for any tier with >=200 unique words, because it is draw-without-replacement over the whole pool: legacy min 762; retier Expert 278 (smallest), Hard 596, Easy 825, Medium 2466. 200 < 278, so OK even for retier Expert (278 -> reshuffle only after 278 draws). Kid pool is kid-filtered (size unknown, smaller than 762). Caveats: (a) if `pool.len()` changes the deck reshuffles immediately (deck.rs:61-63: `pool_len != pool.len()`), so a returning player whose deck was built on legacy bank (e.g. before retier default-ON) gets one early reshuffle and may repeat — irrelevant to a fresh install, relevant to existing TestFlight testers; (b) Jr and standard profiles share deck key `lang:tier` but different pool sizes => switching profile forces reshuffle (inference from game.rs:1961 and :142-146; not tested); (c) `pick_queue::already_served` loop (game.rs ~2034-2043) only skips more; (d) learner_select `promote` reorders within the tail window, excluding recent (deck.rs:93-125), flag default OFF per comment (not re-verified).
- **Daily**: deterministic by date+locale, no stored history. `build_words` (src/daily.rs:~262-290): arc easy2/medium3/hard3/expert2 (daily.rs:15), Kid arc easy5/medium5; per-tier cycle walk `tier_slice_ns` (daily.rs:~175-215): pool sorted, Fisher-Yates permutation seeded `FNV("daily-v2|lang|tier|cycle|pool_hash|salt")`, consumed `w` words/day, so no repeat for L=floor(n/w) days; day index = days since 2026-01-01 from LOCAL date string (daily.rs:128-147); cycle-boundary guard re-salts up to 8 times so day 0 of a new cycle shares nothing with yesterday. Uses `words::tier_for` (so retier when ON). Tiers are disjoint, so 10 distinct words/day. **30 consecutive dates -> 30 distinct words per slot: yes by construction** (L, standard profile, retier banks: easy 825/2=412 days, medium 2466/3=822, hard 596/3=198, expert 278/2=139; all >30). Legacy: 381/266/266/398. Caveat: `pool_hash` is in the seed, so ANY bank edit (or flipping retier) reshuffles the cycle once; a 30-day window spanning that shipped change could repeat. Only a ~8-retry cap on the boundary guard ("then accept").
  - July bug: introduced pre-fix by commit 4cf80929 (2026-07-10 Daily, seeded by date) and fixed by eb966f38 (2026-07-12 "no-repeat word selection — Daily cycle walk + persisted free-play deck"). I did not find a doc or issue text titled "Daily repeated across days"; the commit is the evidence.
  - Tests: `daily_no_repeat` (daily.rs:453-484: 11 langs x kid/standard, walks one full cycle per tier, asserts no word twice in a cycle and none repeated from yesterday), `daily_deterministic_400_days` (521-530: en/de/ja/zh byte-identical on recompute), `standard_seed_is_unchanged` (551, pins seed descriptor), plus `deterministic_per_date_locale`, `date_changes_the_set`, `kid_arc_is_ten_easy_medium`, Jr daily tests. NOT tested: a 30-consecutive-dates distinctness assertion on `build_words` output across the cycle boundary with retier ON; `daily_no_repeat` tests `tier_slice` directly with `tier_for` (retier-on state depends on flag default and test-thread state; not checked). I did not run cargo tests (would create target/ in the worktree).
- **Impossible / Mix**: own decks `"{lang}:{LEVEL}"` over `impossible::pool` (game.rs:1917-1930), same Deck.
- **Online Spell Off**: seed-derived `build_words_from_seed` (daily.rs:~300-330), one-shot permutation, no history.
- **Spelluzzle / Spell Search / WordCross / Randospell / Quick Bee / Board Game / Impostor / SpellPic**: not traced in depth. Hints only: wordsearch_ui.rs:22-23 has a seen-word ledger `spell_wordgrid_seen_v1`; wordcross_ui.rs:152-162 keeps recent grid hashes `spell_xw_daily_seen_v1`; bee.rs:131 "I3: one per round, no repeats"; boardgame_pools.rs has a ledger (test line ~273). Window sizes: unknown.

## R9 spellgame.net (web build, SPELL_WEB=1)
- Build: `SPELL_WEB=1` -> `cargo build --features web` (scripts/build-web.sh:17-24); Dockerfile:12-15 is the source of truth for the site; app-only markup stripped between sentinels (build-web.sh:50-72).
- Language gate: ONE chokepoint `is_active_lang`: `if cfg!(feature = "web") && lang != EN { return false; }` (src/consts.rs:394-396). Test `the_web_build_is_english_only` (consts.rs ~line 535): active == ["en"], registry still 16 so the other 15 render as **coming-soon tiles, not hidden**.
- Entitlement on web: same resolver; English = Full (free_tier), so the web gives full English: standard (all 4 tiers), Daily, Climb, Versus, etc. No Preview clamp on web.
- Modes exposed on web (config/modes.json platforms contains "web"): practice, def_match (live), my_words, misses, standard, climb, daily, versus (core), plus hidden ghost_racing, syllable_replay, word_stories, online_spelloff. App-only (18): photo_list, spell_aloud, letter_forge, word_chains, impostor, bee_sim, word_picture, reports, progress_report, calendar, translate, spelldoku, spell_search, spell_cross, spelluzzle, boardgame, randospell, quickbee_draw. Enforced by test modes.rs:883-900 and web wall scans (package.json build:web).
- Purchases on web: none in code. `purchases_available()` (consts.rs:103) is only EDITION-based (Consumer => true) and has no caller besides itself; there is no purchase UI and no StoreKit anywhere. CC-OFFLINE-PACKS I5 says web has no packs.
- Audio on web: streams from same origin (`SPELL_API_BASE=https://spellgame.net`, index.html:5379), then Web Speech fallback.
- **D7 effort** ("English preview free, everything else locked, purchases app-only, other languages hidden"): estimate MEDIUM, a few focused days, not hours; main items: (1) Web-only entitlement clamp: English at `Preview` (Easy tier, standard mode only; `preview_allows`, entitlements.rs:321-329) instead of Full — the resolver is deliberately single-source, so add a `cfg(feature="web")` branch in `free_tier`/`resolve_for_edition`, and expect to update entitlements tests, scripts/entitlement-map-check, entitlement-core-purity-check, allowed-tiers.golden.json; (2) Hide, don't tile, the 15 other languages on web (needs the picker to skip ComingSoon rows when web; the existing test asserts the tiles remain, so it must change); (3) a "locked" state + "get the app" link for Daily/Climb/Medium+ tiers on web (new copy in all locales, i18n-check/translated gates; mode registry already filters by platform so locked rows may be easier as `status`/gate changes than new UI); (4) site wall scans/e2e (`npm run e2e:web`, TEST-REPORT-site.md) updated; (5) privacy/store copy if any. "Purchases app-only" is already true (nothing exists). Caveat: since the app also has no purchase adapter, any real "locked until purchase" story in the app is a bigger separate build.
- Note: Dockerfile/Caddy are the real prod path for the site; I did not read Dockerfile past the `--features web` lines.

## R11 iPad layout
- Enabled: YES. `TARGETED_DEVICE_FAMILY = "1,2"` in all 4 build configs (ios/App/App.xcodeproj/project.pbxproj:547, 571, 599, 627).
- Info.plist (ios/App/App/Info.plist:54-64): iPhone = Portrait only; `UISupportedInterfaceOrientations~ipad` = Portrait, PortraitUpsideDown, LandscapeLeft, LandscapeRight. No `UIRequiresFullScreen` key => iPad multitasking (Split View / Slide Over) is implicitly supported, with all four orientations. capacitor.config.json has no orientation lock; index.html has no orientation lock (grep none).
- Fastlane: Fastfile `deliver` lanes (lines ~152-200) run `scripts/store-assets-check.mjs` and upload metadata/screenshots from fastlane/screenshots/en-US; no Deliverfile. Committed iPad assets: fastlane/screenshots/en-US/ipad13-1-play.png, ipad13-2-settings.png (2064x2752, `file` output) alongside iphone69-* (1290x2796). Only 2 shots per device. NOTE mismatch: config/store-scenes.json says shots must be `<device>-<n>-<id>.png` mapping to scenes (orb-speaking, wrong-diff, celebration, hub-icons, multi-script, themes, boardgame, ...), so the 4 files named `...-play`/`...-settings` in this tree would fail `store-assets-check` ("maps to no scene"). The b299 scene screenshots live on branch release/eclipse-hotfix-b299 per memory; not in this checkout (unverified). Also, fastlane metadata is English-only (commit 9521c977).
- Tooling that targets iPad: scripts/screenshots.mjs:36, scripts/store-screenshots.mjs:23, tests/e2e/appstore-shots.mjs:18 define ipad13 at 1032x1376 css px, dpr 2 (Playwright Chromium, `isMobile:false`). These are browser viewports, not an iPad simulator or real iPad rendering.
- Existing layout tests: tests/e2e/specs/play-fold.mjs (AT-4) at 320x568, 375x667, 390x844, 440x956 and **1032x1376** for standard and Kid Mode (TEST-REPORT-app.md:529,534 shows pass for 1032x1376, portrait only); tests/e2e/specs/boardgame-polish.mjs:10 SIZES includes 820x1180; spelldoku geometry census (docs/census/spelldoku-geometry-v1.2-census.md:39-40) computed fit for iPad mini 744 (9x9, 12x12) - paper calc. css-logical-geometry.mjs exists (RTL, not iPad). No iPad screenshot diff/golden, no landscape test, no Split View narrow-width (e.g. 320/507/678) test, no iPad simulator run found.
- Layout approach: width-driven CSS only, no isPad / UA branch (spelldoku census C6, line ~58); media queries in index.html: only (max-width:480px) x3 and (max-width:360px) x2 — i.e. no tablet breakpoint, so iPad uses the phone layout stretched/centered as CSS dictates (inference from grep; visual result unseen).
- Known failing screens: none recorded. Untested (no iPad coverage found): everything except play screen + Kid play at 1032 portrait, and Boardgame at 820x1180; i.e. hub/drawer, Settings (only screenshotted), Daily/Climb result screens, Misses, My Words, Def Match, Spelldoku/Spelluzzle/Spell Search/WordCross/Randospell/Quick Bee draw, photo import, Guardian report, accounts, onboarding/age gate; all landscape orientations; Split View. Whether the Info.plist-implied landscape support actually renders correctly: unknown.
- Suggested tooling: Playwright with the existing harness at 1024x1366 / 1032x1376 / 820x1180 / 744x1133 portrait + landscape (1366x1024) and 507/678 wide Split View widths; for real rendering use the iOS Simulator (iPad Pro 13" M4) via xcodebuild + the control tool; run `play-fold.mjs` with added sizes; extend appstore-shots/store-screenshots for scene shots. Alternative if iPad support is not wanted at launch: set TARGETED_DEVICE_FAMILY=1 and drop the ipad13 shots (App Store requires iPad screenshots whenever the iPad family is enabled).

# Part C: R5 dependencies, R6 App Store Connect, R7 parent features

Recon 2026-11-07, read-only. Worktree /Users/eric/repos/spell-launch-en, detached HEAD = origin/main = 2bf45f41.
"main" = origin/main. Release branches are local branches in the shared repo (release/*). The repo cannot show what is deployed or uploaded; those are marked unknown.

## HALT-class findings (read first)

**H1. No purchase code exists on main, so nothing can unlock on purchase day.**
- `src/play_hub.rs:122-124`: `live_entitlements()` is `resolve_entitlements(false, &[], dev_grant())`. `purchased` is hard-coded false.
- `dev_grant()` (`play_hub.rs:131-139`) compiles only under feature `dev_preview`; release builds return false. d2a7ed32 (2026-10-28, "Cut the Test entitlements switch from every release build") removed the tap-the-logo Complete unlock from release binaries.
- `git ls-tree -r origin/main | grep -i storekit` returns nothing; `ls ios/App/App` has no StoreKit dir; `grep -rIn "StoreKit|SKPayment|Transaction.currentEntitlements" src ios` finds only comments and CI-gate scripts. Docs agree: `docs/recon/CC-GAME-SERVER-recon.md:332` ("No StoreKit at all"), `docs/CC-GAME-SERVER.md:29`.
- The only StoreKit code is on branch `feature/entitlements-ios`, commit d382f505 (2026-07-22): `ios/App/App/StoreKit/{EntitlementsPlugin,StickyGrantKeychain,StorefrontCountry}.swift` and `entitlements-native.js`. Its message says "NOT compile-verified". `git merge-base --is-ancestor d382f505 origin/main` is false. Whether it builds against current main: unknown.
- `COMPLETE_PRODUCT_ID = "net.spellgame.complete"` is defined at `src/entitlements.rs:49`. Nothing else references the product (no .storekit file; Fastfile comment `fastlane/Fastfile:165` says "app has no IAP").

**H2. Every parent feature is hidden in every current release build.** With `purchased = false` and no dev door, `photo_ocr`, `progress_reports`, `multiple_profiles` are all false (`entitlements.rs:119-137`, `free_tier`). Builds containing d2a7ed32 (release/online-b310, board-online-b311, sense-ortho-b312; checked with `merge-base --is-ancestor`) cannot show them to anyone. Builds 304, 307, 309 (guardian-report-b304, guardian-own-words-b307, guardian-topup-en-b309) do not contain d2a7ed32, so testers could still reach them there through the dev door. Consequence: no TestFlight tester can exercise these features on b310+ unless the build is a dev_preview build.

**H3. Store text and policy disagree with each other and with the app** (R6): `store/app-review-notes.md` says no personal data collected, no accounts, mic inert on iOS; `store/RELEASE_CHECKLIST.md:31-35` says privacy label "Data Not Collected" and age 4+; `privacy.html` (effective 18 Sep 2026) describes optional Climb accounts, crash/speed reports on by default for 13+, and optional speech egress to Google. Submitting from the checklist answers would contradict the policy.

## R5. Dependency status

Flag mechanism: `src/flags.rs` (localStorage `spell_flag_<name>`, compiled default; signed kill file can only switch OFF; `flags.rs:20-65`). Release branches flip defaults in "RELEASE BRANCH ONLY" commits (e.g. 447ede60); main keeps the dark defaults.

**(a) SENSE-CUE Phase B (fairness / merger rules): merged on main.**
- v1.1 Phase A e1452d6b, F6 sheet 0687c03b, Phase B 59dfefb2 (en/es/ru merger rules, kinds, flag-only loopback). v1.2: census 3af4895c, 19ef3008, 6205f033, 99f1ec83, 8305f999, d015cfb7. All ancestors of main.
- No runtime flag. The cue mode is data: `config/sense-cue.json` has en, es, ru, pl, de, fr, pt, ko all `cueMode: "off"`, auditor null; D9 is enforced in code. So no player-visible cue is on.
- `docs/CC-SENSE-CUE-v1.1.md:3` still says "Phases not started" (stale; commits exist). v1.2 resolutions: `docs/CC-SENSE-CUE-v1.2-mergers.md` (MG1-MG5). Spanish seseo pairs stay accepted by Eric's ruling (`reports/sense-cue-phase-b.md:70-81`).
- Shipped: changelog for build 312 (226bb83f, release/sense-ortho-b312 only, never merged) says SENSE-CUE v1.2 rode build 312. Upload state: unknown.

**(b) VOICE-CLARITY Phase A: merged on main (code); deployment unknown.**
- 8826f12b (English 250 ms trail, profile `vc1`), 64e4a6ea (7 re-scored verdicts), addfdc31 ("The word is" lead-in for 124 words).
- `backend/app.py:136` `PAD_TRAIL_MS_BY_LANG = {"en": 250}`, `:137` `RENDER_PROFILE = {"en": "vc1"}`, `:412-413` cache key suffix `|rp:vc1` and `|rp:vc1li`. Rate is unchanged at 0.85 (`app.py:118`): Eric chose 0.85 over the spec's 0.9 (8826f12b message), so there is no slower-rate change. Lead-in words: `config/audio-leadin-en.txt`; client requests `&v=li` (`src/api.rs:173`, `src/audio_verdict.rs:75`).
- No client flag. Whether the running spellgame.net server has this code and warmed cache: unknowable from the repo (the 8826f12b message says all 6,316 English clips were warmed locally; I did not verify a deploy).
- Not Phase A, still off: `flags::cant_tell` default off (`flags.rs:83`).

**(c) ECLIPSE-HOTFIX: merged on main.** fdeb6f14 (recon), 4a1ebbf3 (P0: every word readable on every theme, diag bar leaves releases), 59f301bf (P1: orb colour, Eclipse near-black). Shipped as build 299 from release/eclipse-hotfix-b299 (changelog e53518ed, release-only). Visual flags (`visual_*`) are switched on only in release-branch commits (9b7ecb4b, c8381b9c, 3ee1af7e, b9a9e5f0, df3479dd); `config/store-scenes.json` names the flags each screenshot needs and `store-assets-check.mjs` enforces it. Per-flag main defaults: not individually checked.

**(d) EN-VARIANTS v1 (British/US spelling): partly on main, rest on branch `en-variants-live` (also remote).**
- On main: census 6c1bb9ef, apostrophe fold 71d62995, SCOWL checks 5499eaa9 / f9a3dc2b, Phase B1 11f1449a. English now reads `data/en/accepted-variants.tsv` (215 lines, includes the 209 earlier symmetric pairs) through `src/accepted.rs:21`; 117 further pairs sit in `data/en/accepted-variants.staged.tsv` (121 lines), unread.
- On branch only (not on main): fe9d3687 and c7530bf0 (2026-11-04): 115 approved pairs live, 3 staged, "pedophilia" quarantined, sexual/medical words moved out of Kid Mode and Spell Jr; branch table has 344 lines.
- No flag; this is grading data. English grades both spellings correct (`data/en/accepted-variants.tsv:3-6`). Settings "British" switch shows the British form on a miss (`config/settings-effects.json:50`, `src/variants.rs`).
- Open: census HALT items in `reports/en-variants-census.md:6-30` (11 British-stored bank rows, 35 hand pairs not dictionary-tagged). I found no record of those rulings beyond the c7530bf0 commit: unknown.

**(e) GUARDIAN-REPORT v1 (A, B, D) + v1.1: on main.**
- v1 A-D: 7750bb3b (same work cherry-picked as 380608e6 on release/guardian-report-b304, -b307, -b309); own-list misses 863aee2e; English top-up ON ff8f0974 (Eric named en auditor).
- v1.1: e00c7e06 (tz_off_s), c7ab6998 (frozen golden), 490fb34a (Phase A), 0663c181 and c97b108f (Phase B), 2bf45f41 (Phase D ping). `docs/CC-GUARDIAN-REPORT-v1.1-amendments.md:3`: signed 2026-11-07.
- Flags: v1 has no flag of its own; its entry is mode row `progress_report` (`config/modes.json:474-490`), `flags::progress_report()` default on (`flags.rs:513`), but hidden unless `progress_reports` entitlement (H1/H2), adult-only, consumer edition only. v1.1 `guardian_report_days` default OFF (`flags.rs:75`). Top-up on for en only (`config/guardian-topup.json`; `docs/CC-GUARDIAN-REPORT.md:425-427`).
- Not built: "Practice these" (no profile switch, `CC-GUARDIAN-REPORT.md:440`), T-TRAP never fires (`:424`), A24 device run on Eric's phone open (`:386,461`). v1.1 Phase C: no commit found; unknown whether the spec defines one.
- `src/guardian_dash.rs`: last touched by 7750bb3b; it has no entitlement reference (grep), so the dashboard digest is not behind Complete.

**(f) Daily/Normal repetition fix (July): merged on main.** eb966f38 (2026-07-12) "no-repeat word selection": Daily cycle walk per (lang,tier) (`src/daily.rs:128-261`, no repeat for floor(pool/W) days, boundary guard) and free play through the persisted no-repeat Deck (`spell_decks_v1`, `src/deck.rs`, `game.rs`, reshuffle guard R=min(3,pool/2)). Test `daily_no_repeat` at `src/daily.rs:453`. No flag. Later related: 4e36ad8e, 60ec9a2d, 65454f60 (`learner_select` default on, `flags.rs:564`). I did not run tests. That the repeat is gone on a device: unverified (no device note found).

## R6. App Store Connect, from the repo only

| Item | What the repo says |
|---|---|
| Marketing version | `CFBundleShortVersionString` 1.1 (`ios/App/App/Info.plist:22`); `MARKETING_VERSION = 1.1` (`project.pbxproj:540,565`). |
| Build number | Repo value `CURRENT_PROJECT_VERSION = 266` (`project.pbxproj:532,557`) is stale. The `ship` lane overwrites it with (latest TestFlight build + 1) at build time (`fastlane/Fastfile:249-259`). Highest build in the repo: `fastlane/changelog/312.md` on release/sense-ortho-b312. What ASC holds: needs Eric to read from ASC. |
| iPad | Supported: `TARGETED_DEVICE_FAMILY = "1,2"` (`project.pbxproj:547,571`), iPad orientations in Info.plist, iPad 13" screenshots exist. |
| Encryption | `ITSAppUsesNonExemptEncryption` false (`Info.plist:32-33`); lanes pass `export_compliance_uses_encryption: false` (`Fastfile:169,194`). |
| Entitlements | `ios/App/App/App.entitlements`: only app group `group.net.spellgame.app` (also `SpellWidgets.entitlements`). No in-app-purchase related change in repo (the capability itself is an ASC/Xcode item: unknown). |
| PrivacyInfo.xcprivacy | **Absent** (`git ls-files` and `git log --all -- '*PrivacyInfo.xcprivacy'` empty). Whether SDK manifests cover required-reason APIs: unknown. |
| Privacy policy URL | `https://spellgame.net/privacy` (`fastlane/metadata/en-US/privacy_url.txt`); served from `privacy.html` via `Caddyfile:58-61`. Policy effective 18 Sep 2026 (`privacy.html:49`). It does not mention a purchase / Complete (grep for purchase found nothing relevant): needs updating if IAP ships. |
| Support / marketing URL | Both `https://spellgame.net` (the home page, not a support page). |
| Purpose strings | `Info.plist:36-45` and `en.lproj/InfoPlist.strings` (es.lproj also exists). Mic: "Spell listens so you can say or spell a word out loud. Usually your voice is recognized on this device. For some languages, only if you agree first, short clips are sent to Spell's server and Google's speech service to be turned into text. Spell never saves them." Speech: "Spell uses speech recognition on this device to understand what you say. Your voice isn't recorded or saved." Camera: "Spell uses the camera so you can photograph a spelling list and turn it into your own words. The photo is read on your device and never leaves it." Photo library read: "Spell can read a spelling list from a photo you choose. The photo is read on your device and never leaves it." Photo add: "Spell saves the picture you spelled to your photos. It never reads your library." Note: `en.lproj/InfoPlist.strings` has no PhotoLibraryAdd line (grep showed 4 keys, mic, speech, camera, library): minor, unverified for other locales. |
| Age rating | Recorded only as prose: "4+ / Education" (`store/RELEASE_CHECKLIST.md:16,35`), "age rating 4+" (`store/app-review-notes.md:44`). No answers file. The app now has accounts, online head-to-head (flag on in b309+), camera/photo input and an IAP; whether 4+ still fits the questionnaire: needs Eric to read from ASC. |
| Privacy labels | Recorded only as prose: "No data collected / Data Not Collected" (`RELEASE_CHECKLIST.md:31-34`), and `store/IOS_RUNBOOK.md:104-113` says it must instead declare Name, Email, Phone, User ID, App activity for Climb accounts. Telemetry (13+ default on), speech egress, and the Complete purchase are not in either. What ASC currently holds: needs Eric to read from ASC. |
| IAP products | Only `net.spellgame.complete` is named (`entitlements.rs:49`). Whether it exists in ASC, its price, and its review status: needs Eric to read from ASC. `Fastfile:165` says precheck skipped because "app has no IAP". |
| Review info | `fastlane/metadata/review_information/*` exists; `store/app-review-notes.md` is stale (says mic inert on iOS, no personal data, 4+). |
| Other languages in metadata/screenshots | **No.** Only `fastlane/metadata/en-US/` exists (name "Spell — hear it, spell it", subtitle, description, keywords, promo "Free, offline, no ads"). Description does not mention other languages. Screenshots on main: 2 iPhone 6.9" + 2 iPad 13" in `en-US` only, plus a scene manifest (`config/store-scenes.json`) whose scenes include a four-script "multi-script" shot (Korean, Arabic, Hindi, Vietnamese); release-branch screenshot commits (e8113daf, 70a47231, previews 82a5ec38) are not on main. `submit_localized` lane exists (`Fastfile:177`) but there are no localized folders. Promo text "Free ... no ads" and release_notes ("First release of Spell") will need revisiting for a paid unlock. |
| Category / copyright | Primary EDUCATION, copyright "2026 Eric Hillman". |
| Other | `UIRequiredDeviceCapabilities` is `armv7` (`Info.plist:50-53`) with deployment target iOS 15.0: odd, effect on App Store device filtering unknown. |

Needs Eric to read from ASC: current age-rating answers; App Privacy answers; IAP product existence/price/status; latest uploaded build number and which build is "Ready for Sale"; localized metadata actually live; Paid Applications agreement and tax/banking status; review notes currently on file; whether the support URL on file differs.

## R7. Parent features

Evidence caveat: `tests/e2e/TEST-REPORT-app.md` reports 522/522 across 63 areas but was last regenerated 2026-10-24 (4a1ebbf3). It contains no `progress-report` or `report-days` area, although `tests/e2e/specs/progress-report.mjs` (Guardian report and ping tests, lines 107-328) and `report-days.mjs` are wired into `tests/e2e/run.mjs:90-91`. So current pass state of those specs: unknown (I ran nothing). Those specs use the dev entitlement door, which release binaries no longer have.

| Feature | Class | Evidence | Gate today |
|---|---|---|---|
| OCR / photo word lists | **Shipped, working on device at some past build; currently unreachable** | Code: `src/photo_import.rs`, `photo_list.rs`, `ios/App/App/NativeLanguageKitPlugin+PhotoList.swift`. Device history in commits: f37ce039 (2026-07-25, capture reverted to plain camera because the scanner was "blurry/dark on device"), 13d3fa2d (camera/library chooser), and `docs/CC-SNAP-CLEAN-v1.1.md:153` / `CC-SNAP-BOXES.md:373-381` cite build 219 on TestFlight importing a merged line, i.e. Eric ran it on device. SNAP-BOXES says its Swift side was written but never compiled by that author; `CC-SNAP-ROADMAP.md:20` says Phases A-C built and shipped in build 260. Rust-side tests exist; e2e report line 108 `photo_split_proposes_and_never_auto_applies` passes. No device note after d2a7ed32. | `photo_ocr` (`modes.json:125`, `photo_list.rs:34-35`). Also off in Kid Mode, native only, language needs OCR support (`consts::ocr_support`). The My Words screen has a second door, `data-l-snap` (`lists_screen.rs:114,689`), which calls `open_camera()`; I did not confirm that button is also premium-gated (unknown). Flag `photo_list` default on. |
| Custom lists beyond cap | **Shipped; there is no cap to go beyond** | `src/entitlements.rs:51-56`: "no free limit" (Eric, 2026-09-17, CC-MYWORDS-LISTS D6); the cap was "declared here and enforced nowhere". `grep` for a list-count limit in `word_lists.rs`, `lists_ui.rs`, `lists_screen.rs` finds none (only `MAX_WORD = 24` characters per imported token, `photo_import.rs:190`). `custom_lists_unlimited` appears only as a premium string in tests (`modes.rs:660,995`), not in `EntitlementSet`. e2e: 18/18 `mywords-lists` (report lines 139-156). | None. Free for everyone, so it cannot be a purchase-day unlock; any "beyond cap" wording must be dropped from copy. |
| Kid profiles (multiple profiles) | **Not started (entitlement flag only)** | `multiple_profiles` bool exists (`entitlements.rs:101,134`) and is pushed into the hub premium list (`play_hub.rs:181`) but nothing consumes it. Learner data is keyed profile 0 = the device "until multi-profile ships" (`src/learner_query.rs:13-15,32-36`). Guardian Report states "there is one profile, the device" and no profile switch (`CC-GUARDIAN-REPORT.md:434,440`). Kid Mode / Spell Jr exist as an audience setting, not as parent-managed profiles. | `multiple_profiles` (nothing to gate). |
| Progress reports | **Shipped in code; hidden in all current release builds** | CC-REPORTS-SHARE v1: 4bb3835f on main (also efc599d5 on release copies); composer to share sheet `src/reports_share.rs`, `reports_share_ui.rs`; spec `tests/e2e/specs/progress-report.mjs:107-131` (gate first, then the week; deep link lands on the gate). Related: `reports.rs` (kid tab FREE, conquest framing, `modes.json:461`), printable PDF / weekly digest f3cb42a8, yearbook export `yearbook_ui.rs:96`, calendar beyond the free week `calendar_ui.rs:199`. Device evidence: none found (Guardian A24 is the open device run). | `progress_reports` (parent-premium). Also adult-only, consumer edition, app only (`modes.json:474-490`). The kid Reports tab and the Guardian Dash digest are not gated. |
| Guardian Report | **Shipped in code and released to TestFlight (b304, b307, b309 changelogs); device verification open** | See R5(e). Host tests A1-A16, A20, A22, A23; e2e A17-A19, A21 and live A20/A22 against a recording notification plugin (`CC-GUARDIAN-REPORT.md` "Acceptance coverage"). Not covered: translations of struggled words for non-English (English-seed drafts awaiting native pass), A24 device run (`:461`). 20 of 21 `greport.*` strings are machine drafts in `needs_audit` (`:431`). v1.1 report-days is flag-off. Device working: unverified. | Same row as progress reports: `progress_reports` + adult gate. English top-up on; other languages reports only the missed words. |

## D4 conclusion

**Not as the repo stands.** On main, and on every release branch from b310 on, nothing sets `purchased = true`, so no parent feature unlocks on purchase day: the Complete SKU would sell a product that changes nothing, which App Review would likely reject (guideline 3.1.1 / unlock that does nothing) and which would be a refund risk.

What is plausible once purchase wiring exists (H1):
- Plausible on purchase day: the Guardian Report / progress report (`progress_reports`) and the photo word list (`photo_ocr`). Both are built, registry-gated, tested in Rust/e2e against the dev door, and need only `purchased` to flip. Each has an unverified device run (A24 for Guardian; no post-gating device note for photo).
- Not available: kid profiles (not started). Custom lists have no cap, so they cannot be marketed as part of Complete.
- Languages: the resolver also raises every language to Full on purchase (`entitlements.rs:241-247`), but only audit-passed languages are active, so this depends on the active-language list (not checked here).

Minimum path: merge or rebuild the StoreKit layer (d382f505, unverified), call `resolve_entitlements(purchased,...)` from it, create the IAP in ASC, update privacy label / policy / review notes / description (R6), and run A24 plus one photo import on a device with a real or sandbox purchase.
