# CC-VISUAL-ECLIPSE v1 — §0 Recon report

**Date:** 2026-10-20. **Tree:** origin/main `1e602477`. Read-only: nothing under `src/` was changed.
Recon scripts, data and screenshots are in `reports/visual-eclipse-recon/`.

**Status: STOPPED for Eric's review.** Phases A–F have not started.

---

## 1. Decisions needed before Phase A

These are the items where the recon either fires a stop-and-ask trigger or conflicts with the spec. Each one cites the R-section that holds the evidence.

| # | Item | Why it blocks | Evidence |
|---|---|---|---|
| Q1 | **"Build 56" does not match the numbering.** The iOS project is at build 266 and builds 280–285 have shipped. | Sequencing (V-D8) can't be checked against a build that doesn't exist under that number. | pbxproj `CURRENT_PROJECT_VERSION = 266`; build 285 xcarchive 2026-10-19 |
| Q2 | **Trigger 1 fires on iOS for the orb envelope.** Server clips play through `@capacitor-community/native-audio` → `AVAudioPlayer(contentsOf: fileURL)`. On a cache **hit**, JS never sees the bytes. The plugin also exposes no playback clock: the `play()` promise resolves when play is queued, not when sound starts. | V-D13 / AT-8 (≤ 50 ms p95) can't be met without one of three options. (a) A side `Filesystem.readFile` + `decodeAudioData` in JS, synced to `play()` resolve. This leaves the path unchanged, but the start offset is unknown. (b) A server-precomputed envelope served next to the mp3. (c) Changing the playback path, which needs your approval. | R5 |
| Q3 | **F6 "never more than 2 haptics per word" contradicts current behaviour.** `key_tap()` fires a light impact on every keypress, from 47 call sites. | Meeting F6 means removing per-key haptics. That is a strategy reversal of shipped behaviour (D-P6 lineage). | R7 |
| Q4 | **AT-4 assumes the iOS system keyboard. Play screens don't use it:** every mode has its own on-screen keyboard and no `<input>`. The real layout problem is different. On SE sizes (320×568, 375×667) the custom keyboard and the Check button sit below the fold. `#feedback` sits below the fold at every size tested, including 440×956. Landscape is allowed on iPhone, and there the spellbox is entirely off screen. | AT-4 needs rewording to "spellbox + custom keyboard + Check fully visible". Fixing the fold will likely change the main play screen's structure, not just its spacing (trigger 10). It may also be a P0 candidate for the current build (V-D8 exception). | R11 |
| Q5 | **The app supports iPad** (`TARGETED_DEVICE_FAMILY = "1,2"`). | The spec's device matrix (AT-4, AT-19, screenshot matrix, F12) is iPhone-only. App Store Connect requires an iPad 13" set. | R11, R16 |
| Q6 | **A feedback registry already exists:** `src/feedback.rs`, a four-state grammar `Success/Close/Miss/Neutral`. | Single source of truth: F4's policy fields (`verdict`, `revealCorrect`, `diff`, `lengthBeforeAnswer`) should extend `feedback.rs` or `config/modes.json`, not live in a parallel registry. Recommend `modes.json` columns, gated by `modes-check.mjs`. | R8 |
| Q7 | **Red is shipped in several modes not covered by "mode-owned" protection:** DefMatch `.wrong`, Spelldoku `#ff6b6b` + `sd-flash`, Impostor `#7d3a3a`, Spell Cross coral, and the base orb `.bad` coral + shake. | Is F4 "amber, never red" meant to reach these? Spelldoku, Spell Cross, DefMatch and Impostor aren't in the protected list, but they do own their own screens. | R8 |
| Q8 | **V-D19 unlock criteria vs. stored data.** "7-day streak" can only be computed retroactively from the **Daily** streak (`spell_daily_v1.best_streak`). "500 lifetime correct" and "100 at Hard+" come from `byear_stats_v1`. That store is base game only, is wiped by the per-language reset button, and records the tier loosely (Climb band, length guess, forced expert). | "Retroactive + never revoked" works only if the unlock itself is stored once granted, and the counters undercount anyone who plays other modes. Do you accept base-game-only counters? | R13 |
| Q9 | **V-D18 triggers have no data behind them:** perfect run is not stored anywhere. The Climb rank is server-only, shown in a toast, never stored on device. "Personal best" exists only as the local top-10 chain list. | F7 needs new local records for perfect run and rank-up. That is a small new store and touches no grading, but it is new data. | R13 |
| Q10 | **Child-safety gaps found (relevant to I9).** (a) Kid Mode can share. Only `#wpShare` is hidden; Chain, Daily, Practice, Randospell and BoardGame share buttons have no kid check. (b) The Yearbook PDF prints `hardest_word` / `first_words` from the learner log. That log includes My Words and photo-imported words, so a child's own name could appear. `kid_filter` is not applied. | Is the Yearbook in scope as a "result card"? Should these be fixed now, as a separate safety fix outside this file? | R14 |
| Q11 | **I3 (no tofu) is already violated on the min OS.** Emoji 14/15 code points in `config/wordpic/pictures.json` (🪭 U+1FAAD, 🪬 U+1FAAC, 🪺 U+1FABA) can't render on iOS 15.0–15.3 (🪭 also fails before 16.4). | Raise the deployment target, or swap those icons? (The "stork" picture also uses 🪺, which is a nest.) | R9 |

**Pre-existing defects found in passing.** These are not Eclipse work, and none of them is fixed:

- **Arabic, Hindi and Urdu answer letters bypass the bundled script fonts.** `.ltr { font-family: Space Mono }` (index.html:1528) beats the inherited `:lang(ar|hi|ur)` rule. `.feedback .reveal` behaves the same way.
- **Vietnamese mixes two typefaces mid-word.** Instrument Sans has no Vietnamese subset: 48 bank and 78 UI code points fall back to system-ui.
- **Both App Store screenshot generators silently shoot the wrong screen.** They click `#setBtn` with `.catch(()=>{})`, and `#setBtn` is now `.retired`. See `tests/e2e/appstore-shots.mjs:42` and `scripts/screenshots.mjs:67`.
- **Code-read only, not run:**
  - The Bee Kid reveal is cleared in the same tick (`bee_screen.rs:155`).
  - Impostor's green "right" state is never visible, because `advance()` re-renders immediately.
  - `#wpStatus.pulse` has no CSS rule.
- **NativeAudio sets AVAudioSession itself,** outside `AudioSessionOwner`. `audio-session-owner-check.mjs` only scans `ios/App/App` and doesn't see it.
- **The app ships three copies of the wasm** (raw, `.br`, `.gz`), about 6.8 MB of dead weight on iOS.
- **`check-contrast.mjs` is not wired** into gate.sh, pre-push or CI.
- **Telemetry D1 query auth has expired** (wrangler error 7403), so `tap_to_audio_ms` couldn't be read.

---

## 2. Recon items

### R1 Styling inventory

**Method.** The scanner (`style_inventory.py`) covered 236 shipped files; the full table is `inventory.csv`. The repo has **no .css files**: the styles are one inline `<style>` in `index.html`, lines 34–2436.

| | Count |
|---|---|
| Raw style literals | **2,877** |
| Colours | 586 (455 hex, 125 rgb/rgba/hsl/color-mix, 6 named) |
| Dimensions | 2,181 (27 built at runtime in Rust `{}px` strings) |
| Durations (transition/animation only) | 110 |
| Colours already inside a `--var:` definition | 81, so **505 are raw usages** |
| Durations inside definitions | 0 |
| `var(--…)` uses | 630 |

| File | Total | Colours | Dims | Durations |
|---|---|---|---|---|
| index.html `<style>` | 2567 | 454 | 2004 | 109 |
| index.html markup (102 `style=` attrs) | 81 | 33 | 48 | 0 |
| privacy.html | 48 | 18 | 30 | 0 |
| notices.html | 34 | 10 | 24 | 0 |
| src/c3_probe.rs (dev) | 26 | 0 | 26 | 0 |
| src/share.rs | 18 | 10 | 8 | 0 |
| src/wordpic.rs | 16 | 16 | 0 | 0 |
| src/boardgame_screen.rs | 15 | 0 | 15 | 0 |
| 19 more Rust files | 72 | 45 | 26 | 1 |

**Existing token system.** `:root` at index.html:115–152 defines 33 custom properties:

- Surfaces: `--bg`, `--bg2`, `--panel`, `--panel-2`, `--edge`
- Text: `--ink`, `--muted`, `--muted-2`, `--ink-on-bg`, `--muted-on-bg`
- Accents: `--amber`, `--amber-soft`, `--teal`, `--coral`, `--glow`
- State: `--success`, `--warn` (`#ff9e57`, orange), `--glow-streak-1..3`
- Tier badges: `--band-*`
- `--ui-chrome`
- Fonts: `--display`, `--body`, `--mono`

It has **no spacing, radius or duration tokens**. Other theming already in place:

- Per-language overrides: `:root[data-lang="ja"|"zh"]` at :1525–1526.
- User colour pickers: `settings.rs:99–125` sets `--glow`, `--bg-color`, `--orb-color` and `--orb-text` through `dom::set_css_var`, with a WCAG check in `best_contrast_ink`.
- Body classes: `big-text` (121 rules), `kid` (18), `coming-soon`, `readable`, `versus`.

What is missing:

- **No light theme, no `prefers-color-scheme`, no `data-theme`.** The app is dark-only, so Daybreak would be wholly new.
- No Edu-specific CSS and no `body.jr` class.

Untokenised palettes:

- The largest raw clusters are mode palettes. `#f2c14e` appears ×48 across BoardGame, Randospell and Spelluzzle, and `#0e1420` ×25.
- 23 raw hex uses duplicate an existing token (for example `#ffb14d` for `--amber`).
- Rust hard-codes palettes in `wordpic.rs:110–129`, `pinyin.rs:70–74` and `share.rs:218–266`.

**Existing gates:** `css-logical-check` (CI + pre-push), `chrome-separation-check` and `viewport-check` (in `npm run build`), `font-selfhost-check`, `wordpic-export-parity` (gate.sh), and `check-contrast` (not gated).

### R2 Framework and render path

- **UI:** hand-rolled Rust on wasm-bindgen 0.2.126 + web-sys. No JS framework and no bundler.
  - Static markup lives in index.html.
  - Rust fills it through `dom::set_html` / `append_html` (216 call sites) and CSS class toggles.
- **Animation:** CSS keyframes and transitions only. There are **40 `@keyframes`**, all in index.html; the core set is `emit, breathe, pulse, shake, fbHold, wrongFb, pop, blink` at :1430–1535. Rust drives timing with 21 `after_ms` timers.
  - **No animation libraries** in Cargo.lock or the shipped JS, and no `element.animate()`.
- **Canvas (all 2D, no WebGL):**

  | File | Use |
  |---|---|
  | `drawing.rs:139` | Handwriting pad |
  | `randospell_fx.rs:91` | Particle layer; the only Rust rAF loop |
  | `share.rs:214` | Share card |
  | `spellpic_export.rs:268` | Spell Picture export |
  | `yearbook_ui.rs:264` | Yearbook |

### R3 Fonts

**Delivery.** There are no remote fonts; `font-selfhost-check.mjs` enforces this. The app, the site and the Edu build all ship the same 15 files: 968 KB on disk, with the script faces lazy-loaded by unicode-range. There are no native iOS fonts.

| Face | Files / KB | Licence | Role |
|---|---|---|---|
| Bricolage Grotesque (var) | latin / latinext / viet: 75 + 30 + 13 | OFL | `--display`: brand, headings, scores, orb glyph |
| Instrument Sans (var) | latin / latinext: 29 + 11 (**no viet**) | OFL | `--body`, UI |
| Space Mono 400 / 700 | 6 files, 7–16 each (with viet) | OFL | `--mono`; **700 is the answer letters `.ltr`**, reveal, hint |
| Noto Naskh Arabic | 92, lazy | OFL | `:lang(ar,fa)` on #letters, #feedback, #meaning, #hintLine |
| Noto Nastaliq Urdu | 436, lazy | OFL | `:lang(ur)`; **ur is not a registered language** |
| Noto Sans Devanagari | 161, lazy | OFL | `:lang(hi)` |
| OpenDyslexic | 16 | **CC BY 3.0** (not OFL) | `body.readable` only |

No CJK, Cyrillic or Korean faces are named; those scripts reach system fonts through `system-ui` and `monospace`. The comments claiming ar/hi are blocked are stale, because `RTL_SUPPORTED = true` and AR and HI are Active.

### R4 Glyph coverage

`tools/font-coverage.py` doesn't exist, so the recon wrote `reports/visual-eclipse-recon/font_coverage.py` (fontTools). It reads banks from `word_data.rs` / `words.rs`, the Impossible pools and the i18n locales, after NFC normalisation. macOS system fonts stand in for iOS. The full list is in `coverage_missing_full.json`.

**With bundled fonts plus the system fallback, 0 code points are missing for every language** on the macOS proxy. Bundled fonts alone:

| Lang | Bank, answer role | Bank, word role | UI |
|---|---|---|---|
| en es fr de pt pl fil sw | 0 | 0 | 30 (shared symbols + emoji) |
| **vi** | 0 | **48/89** | **78/187**; 948 of 1,120 UI strings mix typefaces |
| ru | 33/33 | 33/33 | 91/168 |
| ko | 807/807 | 807/807 | 582/664 |
| ja | 79/79 | 79/79 | 587/666 |
| zh | 2234/2266 hanzi | same | 858/941 |
| ar | 36/36 (Space Mono wins, see Q-defects) | 0 | 79/151 |
| hi | 57/57 (same) | 0 | 89/166 |

After NFC, no Vietnamese stacked combining sequences remain; they are all precomposed. **Not measured:** shaping quality (Arabic joining in a fallback font, Devanagari conjuncts), the real iOS CoreText cascade, served def_pools, and canvas/SVG export text.

### R5 TTS playback path

**One router:** `api::play_word_with` (api.rs:233). It tries Human, then Pack, then ServerCache, then NativeTts. A `Fail` verdict skips the first three.

| Source | iOS app | Web |
|---|---|---|
| ServerCache | `audio-native.js` → `Filesystem` cache (`Directory.Cache/spell-audio/*.mp3`) → `NativeAudio.preload(fileURI)` (LRU 8) → `AVAudioPlayer`. Falls back to `<audio>`. | `<audio src=/api/speak…>`. HTTP cache only (`immutable`); the service worker bypasses `/api/*`. |
| Human | `<audio>`. **The runtime manifest is empty, so no clips ship.** | same |
| Pack | `<audio>` with a native file path; not wired to the gain graph | n/a |
| NativeTts | `AVSpeechSynthesizer` (+ family voice) | n/a |

**Can JS reach the bytes?**

| Platform | Cache hit | Cache miss |
|---|---|---|
| iOS | **No.** Only a file URI is passed. A side read is possible. | Yes, briefly (audio-native.js:151–156) |
| iOS PCM | Never | Never |
| Web | Yes in principle. At the default volume 1.35, every `<audio>` is routed MediaElementSource → Gain → limiter, so an AnalyserNode tap leaves playback unchanged. But the `AudioContext` is a Rust `thread_local` (audio_boost.rs:23), and at volume exactly 1.0 there is no graph. | same |

**Cache-hit latency, web.** Measured in Playwright headless Chromium with real cached mp3s, from the click to the `playing` event. A 300 ms server-delay mutation check confirmed that the instrument detects fetch latency.

| Case | p50 (ms) | p95 (ms) | n |
|---|---|---|---|
| Replay (same element) | 9.6 | 18.5 | 55 |
| Next word (pre-warmed) | 11.7 | 26.9 | 56 |
| All hits | 10.9 | 20.2 | 111 |

**iOS: unmeasured.** No device run was done, and the telemetry query failed on auth. On iOS, telemetry's `tap_to_audio_ms` times the `play()` promise, not the start of sound.

**Default volume 1.35 is not applied to iOS server clips.** NativeAudio bypasses the gain graph (code read).

### R6 Other audio

| What | Mechanism |
|---|---|
| Example sentence | `<audio>` `/api/sentence-audio` + boost (api.rs:701) |
| Meaning panel, Impostor replay (bypasses router), Practice coach | Web Speech |
| Syllable replay | AVSpeech `SyllableSpeaker` / Web Speech |
| Mode word audio (13 modes) | the R5 router |
| Completion chime (880/1320 Hz sines) | Web Audio, audio_boost.rs:118. Callers: wordpic, bee (app), **calendar.rs:226 (also on web)**, c3_probe (dev) |
| Spelluzzle ripple tick (660 Hz, +1 semitone per rune) | Web Audio, audio_boost.rs:146 |
| UI sound effects | **none exist** |

### R7 Haptics

- **Plugin:** `@capacitor/haptics` 8.0.2, through `src/haptics.rs`. There is no `navigator.vibrate` and no native generators.
- **Web:** a silent no-op, not logged.
- **Call sites: 76.** 47 `key_tap`, 17 `correct`, 12 `incorrect`.

| Function | Style |
|---|---|
| `correct` | light impact |
| `key_tap` | light impact |
| `incorrect` (kid) | medium impact |
| `incorrect` (adult) | ERROR notification |

- **Hard-coded kid argument:** `randospell_screen.rs:1327, 1361` always pass `true`; `:1369`, `quickbee_draw_screen.rs:363` and `spelluzzle_screen.rs:1599` always pass `false`.
- **No settings toggle.**
- **Reduce Motion** is honoured at only one site: the BoardGame hop at boardgame_screen.rs:1409.

### R8 Mode feedback

The full per-mode table with file:line references is in §A below. Summary:

- **Existing registry:** `src/feedback.rs` maps three outcome types onto `Success/Close/Miss/Neutral`. It records **none** of reveal, diff or length policy. One divergence is pinned on purpose: OtherSense maps to Close but renders green.
- **Base game, Daily, Climb, Impossible, Versus, My Words:**
  - Per-word verdict, and "It was {word}" on a miss. With shields (Climb) or the extra attempt (default ON in Kid Mode), the reveal is delayed.
  - **No letter slots**, but an opt-in **Hint** shows the first letter, bullets and "N letters". Impossible disables hints.
  - Diffs exist only for zh tones, ko jamo, and cursive scripts (the cursive path is unreachable).
- **Spelluzzle solo: CONFIRMED no per-word verdict** (play.rs:5–6). Duel gives "✓ +N / ✗", D23.
- **Length shown before answering** in Randospell (dice), Quick Bee Draw (cells), Spelluzzle, Practice, Word Picture, Spell Search, Spell Cross and Impostor.
- **Ambiguous or flag-dependent:** shields, extra attempts, zh tone retry, the Randospell Jr retry, tier-dependent checks in Spelluzzle, Spelldoku, Word Picture and Spell Cross, and flag defaults that differ between main and the release branch. These need explicit policy rows (trigger 3).

### R9 Emoji as UI

The scanner is `emoji-scan.mjs`. It decodes `\u{…}` escapes before matching, because most Rust icons are escaped. Output is in `emoji-scan.tsv`. It found **238 UI-icon uses**, mostly in these places:

- **`config/modes.json`:** 26 of 29 mode icons are emoji.
- **Drawer:** ⚙ 🎤 🎙 👤 ✕ ☰.
- **Home launchers:** ⚡ 🏔 🗓 🖼 🎤 🌍.
- **Other UI:**
  - 🔊 replay and 💡 hint buttons
  - ⌫ and ✓ on custom keyboards
  - 📖 💬 definition and sentence buttons
  - orb glyphs ✓ ✗ ⏱
  - BoardGame HUD and medals
  - achievements and calendar
  - settings rows
  - ceremony titles
- **Icons inside translatable strings** in all 15 locales. `drawer.rs:123` has a `dedupe_icon` workaround for this.

**Share text (allowed to stay):** `practice.shareText`, `share.chain`, `share.daily` and `share.dailyStreak`.

### R10 Orbs

**There is no shared orb component.**

| Orb | Owner | States / animation |
|---|---|---|
| Base `#orbWrap` | index.html:2559 + CSS :1413–1477; lib.rs:617–670; game.rs | idle `breathe`; `speaking` (`pulse` + `emit` rings, **only set by the Web Speech fallback**, so server audio never shows it); `good`; `bad` + `shake`; `timed`. Colour comes from user `--orb-color`. CSS only, no canvas. |
| Front door `.fd-orb` | index.html:3491 | idle only (reuses the base CSS) |
| BoardGame `#bgOrb` | boardgame_screen.rs:752 | `data-state` roll/replay/wait; no animation |
| Randospell `#rsOrb` | randospell_screen.rs:719 | `data-state` is set, but **no CSS reads it**; `rs-shake` plays on dice land. The Eclipse dial/corona is a separate SVG + canvas fx. |
| DefMatch `#dmOrb`, Impostor `#impOrb` | 🔊 buttons | pulse/good/heat; none |
| Share card | share.rs:221 | static canvas "S" |

`tests/e2e/orb-shot.mjs` is a manual screenshot of the idle state only. It has no assertions and is not wired into anything.

### R11 Layout integrity

The capture is Chromium web emulation, not WKWebView. The stubbed TTS added a ~50 px error note above the spellbox, so real fold positions are slightly higher.

**Main play screen** (screenshots `fold-afterOrb_*.png`, `fold-afterCheck_375x667.png`):

| Viewport | Keyboard rows visible | Check button | `#feedback` after Check |
|---|---|---|---|
| 320×568 | 0 of 4 | below fold (821) | 1041 |
| 375×667 | 1 of 4 | below fold (810) | 1030 |
| 390×844 | 4 of 4 | visible | 1030 (below) |
| 440×956 | 4 of 4 | visible | 996 (below) |

- **Landscape** (allowed in Info.plist): the spellbox is off screen at every size (`landscape-afterOrb_667x375.png`). There is no `scrollIntoView` anywhere.
- **Full-screen modes fit at all three sizes:** BoardGame, Randospell, Spelldoku, Spelluzzle, Practice and Spell Search (`randospell_320x568.png`, `boardgame_320x568.png`). No horizontal overflow.
- **Exactly one visible submit on every screen.** `#kbSubmit` is hidden on purpose, and e2e `submit-advance.mjs:108` already asserts this.
- **Safe area:**
  - `viewport-fit=cover` is enforced, and insets are applied to the body and to every full-screen mode.
  - Five fixed overlays have no insets: `.bee-over`, `.imp-over`, `.cn-over`, `.wp-modal`, `.scrim`. This is unverified, because Chromium reports the insets as 0.
- **Real system-keyboard inputs** exist only outside play: name entry, parent gates, Spell Off, Translate, search, account and lists. There is no `@capacitor/keyboard`, and their visibility was not measured.

### R12 Performance baseline

- **Minimum iOS:** 15.0 for the app (iPhone 6s / SE 1st gen, A9, 2 GB) and 16.0 for the widgets.
- **Existing instrumentation:**
  - No frame-time instrumentation exists.
  - Telemetry only has `wasm_init_ms`, `tap_to_audio_ms` and `audio_resolution`.
  - One wall-clock test caps BoardGame hops (`boardgame-polish.mjs:230`).
- **On-device frame times: not measured.** This needs an A9 device, the A5 frame logger, and you.
- **Desktop proxy (not device data):** headless Chromium at 4× CPU throttle.
  - 0 long tasks over 50 ms in any scenario: speaking, typing, result, BoardGame hop, Randospell roll.
  - The Randospell roll carries the most script time: 150 ms per 3 s window.
  - The headless rAF cadence is not vsync, so its p50/p95 values are not frame times.
  - GPU compositing (orb blur and glow) is not captured.

### R13 Streaks, bests, achievements

All of these live in localStorage on the device, except the Climb rank.

| Store | Key | Usable for |
|---|---|---|
| 7 achievements (first, chain5/10/25, timed10, cleared, importer) | `byear_ach_v1` (ids only, no dates) | small celebrations |
| Daily streak | `spell_daily_v1 {streak, best_streak, history}` (uncapped) | **7/30/100/365 tiers; Chalkboard (retroactive, exact)** |
| Per-tier lifetime counts | `byear_stats_v1` lang → tier → `{seen, correct}` | **Typewriter / Spelling Bee Stage, base game only, reset-wipeable** |
| Local top-10 chains | `byear_leaderboard_v1` | "new personal best" |
| Climb record / rank | server only | rank-up needs a new local record |
| Journal | `spell_journal_{lang}` (730 days, UTC) | approximate practice-day streak |
| Per-mode bests | Spelluzzle, Randospell, QBD, Bee, Chains, Impostor, Spelldoku | small celebrations |
| Perfect run | **not stored**; Daily `history[date]==10` could stand in | — |

### R14 Result cards

- **Code:** one helper, `src/share.rs`. It uses Capacitor Share, falls back to `navigator.share`, and always appends spellgame.net/app.

| Card | Format |
|---|---|
| Chain result | canvas PNG **1080×1080**; **the card text is hard-coded English**; web sends text only |
| Daily, Practice, Randospell daily, BoardGame | text only |
| Spell Picture | ~1747×2048 PNG card + 1536×1536 keepsake |
| Yearbook / guardian report | JPEG pages 1240×1754, parent-gated |

- **Child identity:** no profile name or avatar exists to leak; the agegate stores only kid/full. The exceptions are in Q10.

### R15 Bundle

Fresh builds at HEAD, in bytes:

| | wasm raw | gzip | brotli | index.html |
|---|---|---|---|---|
| App | 16,089,156 | 4,213,879 | 2,636,996 | 309,112 |
| Web (`--features web`) | 7,783,388 | 2,224,287 | 1,529,247 | 230,703 |

- **Other assets:** fonts 967,720 (Nastaliq alone 446,088), icons 458,193, other JS 41,336.
- **dist totals** (summed from parts): app 17.95 MB, web 9.56 MB (13.3 MB with `.br`/`.gz`).
- **Build 285 xcarchive:** App.app 28.0 MB (public/ 24.3 MB). The latest `.ipa` found is build 284 at 14.6 MB. The App Store download size was not measured.
- **Live spellgame.net:** 460 commits behind main (deployed 2026-09-18); web wasm 6.55 MB then, 7.78 MB now (+19%).
- **V-D11:** the 2 MB app font budget currently has ~1 MB used. A web font budget can't be set from first-load transfer, which wasn't measured.

### R16 App Store requirements

- **Requirements:** per developer.apple.com "Screenshot specifications" (fetched 2026-10-20), required sizes are:
  - iPhone 6.9" (1260×2736, 1290×2796 or 1320×2868), or 6.5" as a substitute
  - iPad 13" (2064×2752 or 2048×2732), required here because iPad is supported
  - Format: PNG/JPG with no alpha, 1–10 per size
- **Ambiguity:** Apple's page names 6.3" as required in its intro, then says scaled shots fill in for it.
- **Repo assets:** `fastlane/screenshots/en-US/` has 2 × iPhone 6.9" (1290×2796) and 2 × iPad 13" (2064×2752), all without alpha.
- **Problems:**
  - These shots are from 2026-07-11, before the drawer navigation.
  - There is no Snapfile; upload runs through the `deliver` lane `submit_localized`.
  - The generators are broken (see the defects list).
  - What is actually live in App Store Connect was not checked.

---

## §A Mode feedback table (R8 detail)

| Mode | Verdict | Reveal correct | Diff | Length before answer | Right/wrong styling | Evidence |
|---|---|---|---|---|---|---|
| standard / My Words / Misses | per word | yes, "It was {word}" (+ zh hanzi, ru stress, es syllables) | zh tones, ko jamo hint, cursive (unreachable) | no slots; opt-in Hint shows "N letters" | orb teal / coral + shake; spellbox `--success` / `--warn` orange | game.rs:2317–2646, 3144–3180 |
| climb | per word | after the final miss; a shield offers a retry with no reveal | as standard | as standard | + streak tiers | game.rs:3051–3091 |
| extra attempt (default ON in Kid Mode) | per word | on the 2nd miss | — | as standard | ✗ coral | game.rs:3033–3045 |
| daily (+ Jr) | per word | always | none | Hint as standard | ✓/✗ | game.rs:3642–3670 |
| versus | per word | yes (hard-coded English chain text) | as standard | as standard | as standard | game.rs:4126–4152 |
| impossible | per word | yes | as standard | **no**: hints are disabled | as standard | game.rs:2044–2156 |
| bee_sim | no explicit "correct"; a miss rings a bell | yes (Kid: likely cleared in the same tick) | no | no | no red, by design | bee_screen.rs:151–396 |
| boardgame | chip ✓ / ✗ | yes (private `#bgMiss` in pass-and-play) | no | no | neutral chips | boardgame_screen.rs:1700–1716 |
| randospell | +N / loss | yes (Jr: on the 2nd miss) | no | **yes**: dice = length | `rs-shake` on dice land; fx shatter | randospell_screen.rs:1292–1395 |
| quickbee_draw | per character | the failed character only | no | **yes**: cells | green done / amber current | quickbee_draw_screen.rs:319–390 |
| spelluzzle solo | **none** (I6) | no | no | **yes**: slots | amber wobble on Check (Jr/Easy) | spelluzzle/play.rs:1–31 |
| spelluzzle duel | ✓ +N / ✗ (D23) | no | no | yes | haptic only | spelluzzle_screen.rs:1580–1600 |
| practice | per letter, live | n/a (hint ladder) | wrong letters refused | **yes**: slots | green underline | practice_screen.rs:511–762 |
| def_match | per round | yes | n/a | n/a | `.wrong` **red** (calm blue in Kid Mode) | defmatch_screen.rs:438–519 |
| letter_forge | valid / dup / invalid | n/a | no | n/a | shake / bounce | forge_screen.rs:327–395 |
| word_chains | three error messages | n/a | no | n/a | shake / bounce | chains_screen.rs:276–349 |
| impostor | 4-card choice | yes | no | yes | wrong `#7d3a3a`, right green never seen | impostor_screen.rs:184–229 |
| word_picture | none | no | trimmed to the good prefix (expert: stays) | **yes**: dashes | dim / orange | wordpic_screen.rs:2089–2477 |
| spelldoku | per placement (hard+: silent until Check) | **no** | no | Fragment clues only | `sd-flash` **red** `#ff6b6b` | spelldoku_ui.rs:518–1060 |
| spell_search | find phase / Lock-It-In | Lock-It-In only | no | **yes** | teal / coral | wordsearch_ui.rs:271–525 |
| spell_cross | when the word fills | keystone only | **yes, positional** (live below hard) | **yes** | coral cells | wordcross_ui.rs:419–544 |
| translate, lists, photo_list | hand off to the base game | — | — | — | — | — |
| ghost_racing, online_spelloff, spell_aloud | hidden; not analysed | | | | | |

---

## Caveats on the whole report

- **No device was used.**
  - Layout, performance and latency figures come from Chromium emulation or a desktop proxy.
  - iOS keyboard, safe-area, frame-time and audio latency are all unmeasured.
  - The macOS fonts are a proxy for iOS.
- **Flag defaults are main's,** and they differ from the release branch (Randospell sub-modes, QBD languages, Impossible).
- **Some findings come from reading code** rather than running it; each is labelled where it appears.
