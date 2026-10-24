# CC-ECLIPSE-HOTFIX v1 — §0 recon

Read-only. Taken at origin/main `110eef6a` (2026-10-24). No code was changed.

**Result: STOP.** Recon item 2 found a token-naming conflict that the stop rule names, and several fixes would collide with shipped, Eric-requested features. The questions are in [Stop-and-ask](#stop-and-ask). The item-by-item findings follow it.

## Stop-and-ask

| # | Conflict | What recon found | Recommendation |
|---|---|---|---|
| S1 | `--surface-raised` (F5: fixed `#1C1C1F`, neutral, saturation ≤ 6%) | The existing name is `--panel-raised = color-mix(--controls-color 25%, --bg)`, and `--edge-strong` is built the same way. `--controls-color` is the player's Settings > Controls colour (build 290), and the brighter panels were Eric's request in his 288 review (build 289). The default Controls colour is violet `#7a66c2`, so I9 fails on the default by construction. | Make F5's ramp the Eclipse default and **exempt raised surfaces built from a player's pick from I9**. Do not neutralise `--controls-color`. |
| S2 | `--bg` (F5: `#0A0A0B`; the html/body background follows it) | Two tokens share the job. `--bg` (`#14111f`) colours mode screens. The page, html and overscroll paint `--bg-color` (`#1c1830`), the default of Settings > Background, whose first swatch is labelled "Violet background". | Retint both. Make the Eclipse default `--bg-color` `#0A0A0B`, and keep the Background swatches as the player's choice. The swatch label and default need your OK. |
| S3 | F4: App Store build compiled out, TestFlight compiled in | Both stores get the **same binary**. `fastlane submit_review` runs `skip_binary_upload: true` and submits an existing TestFlight build number. No App Store lane builds anything. | Choose one: (a) compile the bar out of every release build and keep it only in `build-web-dev.sh` / dev builds; or (b) cut a separate store build each release. (a) is simpler and makes I7 true on every shipped build. |
| S4 | F6 / I11 premise | `orb.listen` = "listen…" **is** the orb's idle label for a word in play (`game.rs:2051`, `3168`). In one-press letter capture, native code swallows a per-cycle recognizer error (1110 included) and keeps listening (`cycleEnded`, `NativeLanguageKitPlugin.swift:1191`). JS gets no `letterError`, so no event exists to measure "within 300 ms" from. The session ends on its own after 6 s of quiet (`noSpeechTimeout`) through the normal stop path. Then the Rust side resets cleanly and shows "Didn't catch that". | Redefine I11 as: N s after the mic is tapped with no speech, the mic is enabled and not `.listening`, and the status shows the retry hint. Then decide whether 6 s is too long. Shortening it changes recognizer timing, which F6 forbids, so it needs your ruling. A repro on device is needed for the "dimmed" mic (see item 6). |

Not stops, but you should know:

- **"Build 56".** The signing sheet already deleted the "after build 56" clause; the app is at build 298. P0 would ride the next build (299+).
- **axe-core.** Phase E deliberately used a hand-written contrast pass instead of `@axe-core/playwright`, because adding it is a new npm dependency (`reports/visual-eclipse-phase-e.md`). Done-when #3 needs that dependency added. Saying go on this spec counts as approving it.
- **Done-when paths.** The repo has no `.css`, `.scss`, `.ts` or `.tsx` files. All CSS is one inline `<style>` in `index.html`, so `stylelint "src/**/*.{css,scss}"` would match nothing. The equivalents are `scripts/token-ratchet.mjs` (already in pre-push) plus a stylelint run over the extracted `<style>` block.
- **Orb underglow (F3).** "Awaiting glows toward the input" is Phase B F3 (`da73f036`). D2 keeps the glow but changes its colour, so the signed behaviour survives. I'm reading it that way unless you object.

## 1. Bundler, CSS, where theme values live

- **No JS bundler.** `scripts/build-web.sh` runs `cargo build --target wasm32-unknown-unknown`, then `wasm-bindgen --target web`, then copies static files into `dist/`. JS files (`native-language-kit.js`, `audio-native.js`, …) ship verbatim. Nothing defines or tree-shakes them.
- **Build-time constants that exist:** Cargo features (`--features web` for the site) and the sentinel cuts in `build-web.sh` (e.g. `/* BOARDGAME:BEGIN */`), which strip markup and CSS from the site build. A `__SPELL_DIAG__` flag would need a Cargo feature for the Rust side plus a sentinel strip for the JS and markup.
- **CSS:** one inline stylesheet in `index.html` (about 5,100 lines in the file overall). Rust also writes some inline `style=` strings.
- **Theme values:**
  - Source of truth: `design/tokens.json` (223 colour tokens, each mapped to a CSS name via `$css`).
  - `scripts/tokens-build.mjs --write` generates the `/* tokens:begin */ … /* tokens:end */` block (`index.html:128-451`) and `src/tokens.rs` (Rust constants).
  - Themes are `:root[data-theme=<id>]` overrides (`index.html:315-323`), set by `src/themes.rs`. "Light mode" is the **Daybreak** theme, behind `visual.themes`, which is on in build 298.

## 2. CC-VISUAL-ECLIPSE status and token names

- **In the repo, Phases A–E merged and shipped.** A = 287 (A1 token migration `651dd3c5`, 2026-10-20), B = 288, C = 295, D = 296, E = 298. The spec itself is not in the repo. The decision record is `reports/eclipse-keyboard-signing-sheet.md`.
- **Mapping to this file's names:**

| Hotfix name | Existing name | Notes |
|---|---|---|
| `--ink` | `--ink` (`color.text`) | Same. `--ink-on-bg` also exists for text on the page background. |
| `--ink-muted` | `--muted` (`color.text-muted`) | Also `--muted-on-bg` (a color-mix). |
| `--ink-faint` | `--muted-2` (`color.text-subtle`) | |
| `--icon` | **missing** | `--icon-stroke` exists but is a stroke **width** (1.75), not a colour. A new `--icon` doesn't clash, but the names are confusingly close. |
| `--border` | `--edge` (`color.edge`) | Also `--edge-strong` (from the Controls colour, see S1). |
| `--surface` | `--panel` (`color.surface`) | Also `--panel-2`. |
| `--surface-raised` | `--panel-raised` | **Conflict S1.** |
| `--bg` | `--bg` + `--bg-color` | **Conflict S2.** |
| `--accent` | `--amber` (`color.accent`) | `var(--accent, var(--ui-chrome))` is read at `index.html:1842`, but nothing defines `--accent`. |
| `--on-accent` | `--button-ink` / `--palette-on-gold-text` | `--button-ink` follows the Buttons picker. |

## 3. Raw colours in components

- **Literal colours are almost gone.** The pre-push ratchet passes: 0 raw colours, 63/63 exempt. My independent scan also counts named colours and JS. Outside the token block it finds:

| File | Raw | What they are |
|---|---|---|
| `index.html` | 33 hex + 6 named (`white`) | Hex: the swatch data values of the Glow, Background and Orb pickers (inside an exempt block), plus the `theme-color` meta `#1c1830`. Named: `white` inside `color-mix` (lines 488, 1754, 1755). |
| `index.html` | 1 hex in a data URI | `%23968cb4`, the `<select>` arrow (line 674). **Both scanners miss it.** It's the Eclipse muted colour, so it stays violet-grey on Daybreak. |
| `privacy.html` | 14 hex + 4 rgba | A standalone page with its own dark styling. |
| `notices.html` | 10 hex | A standalone page; it already has light and dark variants. |
| `src/drawing.rs` | 2 (exempt) | OCR canvas black on white, correctly exempt. |
| `src/randospell_fx.rs` | 2 (exempt) | Runtime alpha, mode-owned art. |
| JS files | 0 | |

- **The real cause of white-on-white is theme-blind tokens, not raw colours.**
  - Daybreak overrides 17 tokens, and 20 vary once `var()` chains resolve. **128 colour tokens keep their Eclipse value on every theme**, and component CSS references them **351 times**.
  - The most used: `--palette-game-gold` ×48, `--teal` ×37, `--palette-mode-screen-bg` ×25, `--palette-on-gold-text` ×17, `--color-white` ×17, `--coral` ×17, `--ui-chrome` ×13, `--alpha-white-x14` ×13, `--orb-color` ×11, `--palette-slate-text` ×7. About 40 `--alpha-white-*` / `--alpha-black-*` overlays account for most chip, card and divider fills.
  - This is the white-on-white: Eclipse's white overlays and pale ink, painted on Daybreak's white panels.
- **Canvas does not read CSS at all.** 51 sites use compile-time `crate::tokens::X.value` constants, so canvas output and Rust-built styles never see a theme:
  - `settings.rs` 12, `wordpic.rs` 10, `share.rs` 9, `spellpic_export.rs` 7, `model.rs` 4
  - 2 each in `yearbook_ui.rs`, `drawing.rs`, `guardian_dash.rs`, `orb.rs`; 1 in `result_card.rs`
  - **No canvas code calls `getComputedStyle`.** Spell Picture line art (`wordpic.rs`) is one of these, which is why its thumbnails don't follow the theme.
- **Answer-box letters (F2):** `.ltr{color:var(--glow); text-shadow:…var(--glow)}` at `index.html:1875`. The fill is the glow, as the spec says.

## 4. Glow on the orb's lower rim

It is **an orb pseudo-element that reads the glow token**, not the input glow layer overflowing:

- `body.visual-orb #orbWrap .orb::after` (`index.html:1811`) is a blurred `radial-gradient(var(--glow))` ellipse at `inset-block-end:-20%`. It shows at opacity .6 in `data-orb="awaiting"` (`:1812`).
- It was added in Phase B F3 (`da73f036`).
- The milestone state also reads `--glow` (`box-shadow … 0 0 64px 16px var(--glow)`, `:1817`).
- **Containment:** neither `contain: paint` nor `isolation: isolate` is needed. The fix is to point these two rules at a fixed orb token.

## 5. The `diag:` bar

- **Mounted:** `#voiceSpellDiag` at the app root (`index.html:3310`). It is `position:fixed` at the safe-area top, `z-index:9999`, has its own `--alpha-black-72` background, and ships `hidden`.
- **Written by:** `native-language-kit.js:440-505`, the `letterDiag` and `trail` handlers fed by the native plugin's `letterDiag` event (`NativeLanguageKitPlugin.swift`, `diagHandler`). The text includes rms, thresholds, buffer counts and the raw transcript trail.
- **Gate:** every write is behind the dev door (`localStorage spell_dev_entitlements === 'on'`). `scripts/diag-dev-door-check.mjs` enforces this in pre-push; it was the fix for build 270 (`8676ba00`).
- **Why it follows the player:** once the dev door is on, the first mic use unhides it and **nothing ever hides it again**. No route change or mic stop re-hides it. That matches Eric's report: his device has the dev door on.
- **Dev menu: exists.** Five quick taps on the SPELL logo (`src/dev.rs`) open `#devMenu` with the "dev entitlements" switch. The D8 fallback (7 taps on the version) isn't needed; F4's toggle can be a new row there.
- **App Store:** the dev door ships in every build, App Store included, because the binaries are the same (S3).

## 6. Recognizer error path for `kAFAssistantErrorDomain 1110`

- **Native.** 1110 has no case of its own. It reaches the task callback's error branch (`NativeLanguageKitPlugin.swift:1091-1104`), is logged as `task-err=…`, and becomes `.failure(.noSpeech)` when nothing was heard.
- **One-press letter capture** (the Spell screen mic) is `continuous = true`, so `cycleEnded` (`:1191`) **swallows the failure and starts the next cycle**. The session keeps listening and nothing reaches JS.
- **Self-stop.** After 6 s with no partials (`noSpeechTimeout`, `:496`, `:674`), native calls `stop()`. JS gets `letterFinal` with `end:true` and an empty token, not `letterError`.
- **Rust.** `spell_aloud::on_final` with nothing committed calls `end_capture_ui()`: `CAPTURING` and `LISTENING` become false, `.listening` comes off `#voiceSpellMic`, and the chip is hidden. The status then reads "Didn't catch that — try again." `on_error` (`:1618`) does the same reset for real error codes (PERMISSION_DENIED, UNAVAILABLE, NETWORK, MIC_SILENT, default).
- **State afterwards:**
  - **The orb:** unchanged. It still shows "listen…", which is its idle label for a word in play. The voice mic doesn't drive the orb's state machine.
  - **The mic button:** for up to 6 s it keeps pulsing as `.listening` (teal ring), by design, then it resets. No rule ever disables or dims it: it has no `disabled` attribute and no opacity rule.
  - **Probable cause of "dimmed":** in Daybreak, its default look is a `--panel-2` fill with an `--edge` border, plus a theme-blind `--chrome-mic-listening-ring`. That reads as a greyed button on a light field. Unconfirmed: needs a device repro (S4).

## 7. Routes

There is **no URL router**. Screens are DOM panels opened by launcher buttons. The only hash routes are the deep links `#daily` and `#practice` (`lib.rs:347`). The registry the suites should iterate is **`config/modes.json` (29 modes)**, through `drawer::route_for`, plus the chrome screens.

- **Live or core (23):**
  - practice, photo_list, def_match, letter_forge, word_chains, impostor, bee_sim
  - word_picture, my_words, misses, reports, calendar, translate
  - spelldoku, spell_search, spell_cross, spelluzzle, boardgame, randospell, quickbee_draw
  - standard, climb, daily, versus
- **Hidden (6):** ghost_racing, syllable_replay, spell_aloud (iOS mic, lives on the standard screen), word_stories, online_spelloff.
- **Web-only subset:** practice, def_match, my_words, misses, standard, climb, daily, versus. The rest are iOS-only.
- **Chrome screens (not in modes.json; need an explicit list in the suites):**
  - the front door and age gate
  - the drawer and the setup sheet
  - Settings: Appearance, Controls, Buttons, Background, Orb, Glow
  - the Stats sheet and the Learning report (`statsGuardian`)
  - the Tools hub, the leaderboard (`climbBtn`), and the coming-soon panel (`comingPanel`)
- **Existing test harness:** a custom Node runner over `playwright` (`tests/e2e/run.mjs`, `harness.mjs`, 75 specs). `playwright test` and `*.spec.ts` are not used; the new suites would be `tests/e2e/specs/*.mjs`.

## 8. Stats scope (report only, O1)

| Surface | Source | Scope | Language |
|---|---|---|---|
| "Your accuracy" card (`#statsBody`, `src/stats.rs`) | `spell_stats` map, written only by `stats::record` / `record_tone_only` in `game.rs` | **Base game only** (the standard screen: Normal, Daily, Climb, Impossible). No other mode writes it. | **Written** under `cur_lang`, the language of the word served. **Read and labelled** with `state.lang`, the study language. In Mix, and wherever those two differ, the card shows another language's bucket. |
| "Learning report" (`#statsGuardian`, `learner::guardian_report_html`) | the learner log via `learner_query::review_stats(DEVICE, state.lang)` | **Every surface that calls `learner::note_attempt*`:** the base game (`game.rs`), Spell Search, Spell Cross, Spell Picture. Behind `learner_surfaces()`. | `state.lang` |

**Why 36/25 and 18/12 disagree:** the report counts attempts from more modes than the card, and both are labelled with no scope. The "English during a Português session" label is `name_for(state, &state.lang)`: either `state.lang` was still `en` while the words came from `pt`, or the sheet wasn't re-rendered. `stats::render` is called from 6 sites in `lib.rs`. Not fixed here.

## 9. Native background

| Setting | Value |
|---|---|
| Capacitor `ios.backgroundColor` | **Not set.** `capacitor.config.json` holds only appId, appName and webDir. |
| LaunchScreen | The storyboard background is `systemBackgroundColor`, which follows the system. It is fully covered by the `Splash` image (`scaleAspectFill`). **Both splash variants, light and dark, are solid `#1c1830` violet** (sampled). No named colour asset exists (`Assets.xcassets` holds only AppIcon and Splash). |
| WKWebView background | Not set in the repo. `MainViewController` only registers the plugin, so Capacitor's default applies (not measured). |
| Page and overscroll | `html{background:var(--bg-color)}` (`index.html:485`), which follows the theme (Daybreak overrides `--bg-color`). The `<meta name="theme-color">` is fixed at `#1c1830`. |

So I10 needs three things: the splash PNGs replaced (or the image removed in favour of a colour set), `ios.backgroundColor` set, and the `theme-color` meta made to follow the theme.
