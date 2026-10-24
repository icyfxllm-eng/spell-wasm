# CC-GAME-SERVER — Phase 0 Recon

Status: **STOP for Eric's review.** Read-only recon on `origin/main` @ `d4be437b`
(worktree `~/repos/spell-wasm-game-server-recon`, branch `recon/game-server`).
Nothing outside `docs/recon/` was changed. Date: 2026-10-24.

## 0. Verdict up front

Five of the seven §4 stop-and-ask triggers fire. Phase A cannot start on the
spec as written. Each item below needs your call, not mine.

| # | Trigger | Fires? | One-line reason |
|---|---|---|---|
| 1 | Grader can't compile natively without logic changes | **YES (partly)** | The crate compiles natively with 0 errors, but there is no single grading function. Grading is an inline `if` ladder inside the DOM-bound `game::submit_guess`, and English is graded by the Flask backend. Extracting it is a logic move, not a cfg line. |
| 2 | Local H2H rules ambiguous/contradictory vs instruction file | **YES** | Local Head-to-Head is alternating turns with a longest-chain win, 2 players only, and its tier and timer come from the solo settings. The spec assumes simultaneous rounds with a `deadline_ms` and a speed score. |
| 3 | TTS only fetchable by word text, no ticket path can feed the player | No | `/api/speak` takes the word in clear (fine server-to-server), and the low-level players already take a URL. A ticket path needs one new entry point, plus a cache-key change (see R4). |
| 4 | No secure token storage without a new plugin | **YES** | No keychain/secure-storage plugin exists, and no `SecItem` appears anywhere in `ios/`. The Climb token lives in localStorage today. |
| 5 | A field outside the §9 inventory | Not yet | Nothing to store yet. Note that local H2H takes free-text player names, which the spec drops (G-D4). |
| 6 | i18n needs human translation | **YES** | Translations are hand-written into 15 locale JSONs; there is no machine pipeline (R11). |
| 7 | Instruction-file conflict implying strategy reversal | **YES** | An **async online "Spell Off" already exists** (flag `online_spelloff`, `backend/matches.py`, `DEPLOY-matches.md`). It is adult/account-gated, and its follow-up list names "real-time head-to-head". The spec does not mention it. Its facts about the TTS host and the release rule are also stale (§2). |

## 1. Facts the spec gets wrong

- **TTS is on the Mac mini, not a Pi.** spellgame.net (Flask + clip cache +
  Google/Azure keys) runs on the Mac mini at `~/spellgame-server`, with gunicorn
  on :8000 and cloudflared tunnel `spellgame-mac`. `DEPLOY-matches.md` still
  says "the Pi", so it is stale too. `SPELL_TTS_BASE` would be the Mac mini's
  tailnet address, and the Mac mini is not on Tailscale today as far as my
  notes show.
- **"Post-build-56" release rule.** TestFlight is at build 299. The rule needs
  a current anchor (e.g. "nothing on any `release/*` branch").
- **There is no StoreKit in the app** (R8), so O2 "client-asserted entitlement"
  has nothing to assert. Purchase is hard-coded `false`.
- **DEV_PREVIEW does not gate languages** (R5). Language gating is
  `consts::is_active_lang` (registry status + RTL block + `web` feature).
  A8.3 must be rewritten against that function.
- **Profiles: one per device.** "Kid profile" in this codebase means a
  device-wide `AppState.kid` flag (Spell Jr), set by the user or locked by the
  age gate. There is no multi-profile model (R7).

---

## R1 — Grading entry point and native compile

**Crate.** The core is the root package `spell_wasm` (`Cargo.toml`,
`crate-type = ["cdylib","rlib"]`), with workspace members `.`, `tracer-core`,
`scanlock` and `tonal`.

**Native compile: clean.**
```
cargo check -p spell_wasm --lib --target x86_64-unknown-linux-gnu
  -> Finished, 0 errors, 256 warnings (all dead-code)
cargo check -p spell_wasm --lib            (aarch64-apple-darwin host)
  -> Finished, 0 errors, 256 warnings
```
`wasm-bindgen`, `js-sys` and `web-sys` are non-optional deps but compile
natively as stubs. `src/storage.rs` already has `#[cfg(target_arch="wasm32")]`
twins, and CI runs `cargo test --lib` natively (`editions.yml`). `cargo check`
does not link, so link-time behaviour of a native binary is unproven until
Phase A builds one.

**The grading entry point is not a function.** Local Head-to-Head grades
through the same path as solo play, `game::submit_guess` (`src/game.rs:2131`).
The verdict is an inline ladder (`src/game.rs:2178-2245`):

| Branch | Code | Native-pure? |
|---|---|---|
| English, not other-sense/variant/impossible | `spawn_local(backend_verify(..))` → `api::check_answer` → **Flask `POST /api/check`** (`backend/app.py:769`), which returns `answer.lower().strip() == word.lower().strip()`; on network error it falls back to `norm::answer_matches` (`game.rs:2276-2279`) | **No**: it is a Python server call |
| English other-sense | `homophones::accepts(EN, word, typed)` | yes |
| English variant (colour/color) | `variants::accepts(word, typed)` | yes (fn is pure; the module's display half uses web_sys) |
| Impossible word | `impossible::engine::grade(lang, typed, word)` | yes |
| zh | `pinyin::grade_sandhi_aware(typed, word, zh_sandhi::lookup(word\|spoken), ToneMode::for_kid(kid), tier_accepts_surface(tier))` | yes, but needs `spoken` and the tier from AppState |
| ko | `jamo::grade(typed, word).correct` | yes |
| all others | `norm::answer_matches_in(lang, typed, word, kid) \|\| homophones::accepts(..)` | yes |

All of `norm`, `homophones`, `pinyin`, `jamo`, `zh_sandhi`, `viet` and
`impossible::engine` are free of web-sys, js-sys and storage.

**Why this is trigger 1.** F6 allows only `#[cfg]` and feature lines in the
core. A server cannot call a function that doesn't exist, and every route to
one is a logic diff:

- Pull the ladder out of `submit_guess` into a pure
  `pub fn grade(lang, typed, word, kid, ctx) -> Verdict`. This is a refactor of
  the most load-bearing function in the app; the WASM path would call the same
  function afterwards.
- Make modules `pub`. They are all private today (`src/lib.rs:112 mod kid_filter;`,
  `:119 mod norm;`, `:127 mod profanity;`, `:176 mod word_data;`,
  `:225 mod words;`). A visibility change is not a cfg line.
- **English parity is undefined.** Today's English verdict comes from Python
  (`lower().strip()`, no NFC fold, no internal-whitespace drop). The native
  fallback is `fold_strict`. A native server would differ from the app on any
  English input where those two disagree. You must decide which one is "the"
  English grader.

**Minimal plan, if you approve a non-cfg diff:**
1. Add a pure `grade::verdict(lang, typed, word, kid, GradeCtx { impossible, zh_spoken, tier })`
   holding the existing ladder verbatim, with English resolved locally by the
   fold (or keep the backend call client-side only and declare server English =
   fold).
2. `submit_guess` calls it; no verdict changes. Prove it by running A6.1 on WASM
   before/after.
3. Gate a `pub mod grade` re-export behind a `native-grader` feature.

## R2 — Local Head-to-Head rules

Source: `src/versus.rs` (pure state, 122 lines, written "so the exact same logic
can back a future online version") and `src/game.rs:4192-4400`, plus
`src/lib.rs:1504` (`wire_versus`). Only two players are supported.

| # | Rule | Value | Evidence |
|---|---|---|---|
| 1 | Players | Exactly 2 (`p1`, `p2`), on one device | `versus.rs:39-45` |
| 2 | Names | Free text, `maxlength=14`; blank → "Player 1"/"Player 2" | `index.html:4333`, `game.rs:4194` |
| 3 | Structure | **Alternating turns**, not rounds. A turn = keep spelling until the first miss | `versus.rs:1-5, 88-106` |
| 4 | Turns each | 3; **2 if Kid Mode** (`app.kid`) | `game.rs:4212` |
| 5 | Attempts per word | 1. Extra-attempt and shields are excluded in versus | `game.rs:2910, 2930`, `consts.rs:450` |
| 6 | Correct | `current += 1`; `best = max(best, current)` | `versus.rs:78-85` |
| 7 | Miss / timeout / give-up | Ends the turn: `current = 0`, `turns_left -= 1`, hand to the other side if they have turns left | `versus.rs:90-106`, `game.rs:2745` |
| 8 | Turn order | Strict alternation while both have turns; else the side with turns plays out | `versus.rs:97-105` |
| 9 | End | Neither side has turns left | `versus.rs:102` |
| 10 | Winner | Higher `best` (longest single chain); equal → tie | `versus.rs:110-121`, `game.rs:4375` |
| 11 | Tier | **If the selected level is `climb`**: rises with the active player's current chain, 0-2 easy, 3-6 medium, 7-11 hard, 12+ expert. **Otherwise the player's selected fixed tier.** Kid Mode is clamped by `clamp_base_tier` (Spell Jr ceiling) | `game.rs:230-232, 268-277, 1865-1876` |
| 12 | Timer | **Only if the global `timed` toggle is on**: 12 s easy / 16 s medium / 22 s hard / 32 s expert. Otherwise untimed | `game.rs:3980`, `consts.rs:3-10` |
| 13 | Speed component | **None.** Score is chain length only | `versus.rs` |
| 14 | Word display | No length or slots: the letters row shows only what has been typed (placeholder "type what you heard") | `game.rs:355-380` |
| 15 | After a correct word | Shows the **meaning/definition** (`show_meaning`), then auto-advances after `CORRECT_DELAY_MS` = 2200 ms | `game.rs:4337`, `consts.rs:448` |
| 16 | Turn start | Idle until the active player taps the orb ("<name> tap for a word") | `game.rs:4281-4305` |
| 17 | Persistence | Writes nothing: no stats, Misses, word stats, achievements or leaderboard | `game.rs:4313-4316, 2636-2647` |
| 18 | Rematch | Same two names, fresh match | `game.rs:4270` |

**Where this contradicts the spec (trigger 2):**
- F1/F7 model **simultaneous rounds** (`round_start{deadline_ms}` to everyone,
  everyone `submit`s). Local H2H is **one active speller at a time**; the other
  player watches.
- G-D17/F7/A7.3 spec a **speed component**. There is none.
- O7 room size 2–4: local H2H is fixed at 2. The O7 default already says "or
  R2's fixed count", so **2**.
- Tier and timer are **not match rules**: they inherit the solo level selector
  and the `timed` toggle. Online needs them fixed per room (`create_room{tier}`
  exists, but "tier" here can mean "climb-style chain ladder" or "fixed").
- G-D14 drops the post-correct definition (rule 15), so online already differs
  from local.

You need to pick between:
- **(a)** port turns-and-chains verbatim (then `round_start` is a per-turn
  event, only the active player submits, and G-D17 is moot), or
- **(b)** sign a new simultaneous-rounds mode, which is not "the same rules as
  local Head-to-Head" and makes the A7.1 rules table a new design.

## R3 — Remote flag system

- **Flags:** `src/flags.rs`. Each flag is
  `pub fn x() -> bool { resolve(stored("x").as_deref(), <default>) }`; a local
  override comes from localStorage `spell_flag_<name>`.
- **Remote file:** `GET {api_base}/api/kill-flags` (`src/killflags.rs:126-132`),
  where `api_base` is `https://spellgame.net` (`index.html:4917`). The server
  returns `KILL_FLAGS_FILE` verbatim (`backend/app.py:682-695`), with no logic
  and no key.
- **Kill-only enforcement:**
  - `flags::stored` checks `killflags::is_killed(name)` first and can only
    return `"off"` (`src/flags.rs:28-35`).
  - The verifier reads only `v`, `issued`, `kill` and `sig`; there is no "on"
    field (`killflags.rs:59-79`).
  - Ed25519 signature over `"spell-kill-v1\n<issued>\n<sorted names>"`, with a
    replay guard (`issued` must not go backwards, `killflags.rs:94-104`).
  - Test `the_file_can_never_turn_a_flag_on` (`src/killflags/tests.rs:79`).
  - Signer: `tools/killflags/sign.py`, key at
    `~/spellgame-server/keys/kill-flags.pem` (outside git).
- **Refresh cadence: app launch only** (`src/lib.rs:247-249`). There is no
  interval and no resume/visibility refetch.
  - **A13.3 ("remote kill in lobby → resting card within one flag refresh")
    cannot pass today**: one refresh = next cold launch. A periodic refresh,
    or a refresh on entering the online screen, is a client change.
- **Caveat:** "only a build turns it on" holds for the remote channel only. The
  Tools hub can write `spell_flag_<x>=on` locally (`src/tools_hub.rs:110`).
  G-D7's flags must stay out of the Tools hub, or the server must enforce its
  own gate.
- **Adding a flag:** add the fn to `flags.rs`, add an `is_on` arm if it is a
  hub mode (a test enforces this), and optionally a Tools-hub row. Kill needs
  no change.

## R4 — TTS fetch

- **Endpoint:** `GET /api/speak?word=<w>&variant=normal|slow&lang=<l>[&py=<pinyin>]`
  (`backend/app.py:703-766`; client `src/api.rs:166-178`). **The word travels
  in clear text.**
  - No auth. CORS is an origin allowlist that includes `capacitor://localhost`
    and `https://localhost`.
  - Response: `audio/mpeg` (MP3), `Cache-Control: public, max-age=31536000, immutable`.
  - zh requires `py`; `voice` is honoured only with `BAKEOFF=1`.
- **Voices (`LANG_VOICES`, Google):**

  | lang | voice | lang | voice |
  |---|---|---|---|
  | en | en-US-Neural2-E | vi | vi-VN-Wavenet-A |
  | es | es-ES-Neural2-G | ko | ko-KR-Wavenet-D |
  | fr | fr-FR-Neural2-F | ja | ja-JP-Wavenet-B |
  | de | de-DE-Neural2-A | fil | fil-PH-Wavenet-A |
  | pt | pt-BR-Neural2-B | zh | cmn-CN-Wavenet-A |
  | pl | pl-PL-Wavenet-B | ar | ar-XA-Wavenet-B |
  | ru | ru-RU-Wavenet-A | hi | hi-IN-Neural2-A |

  - sw uses Azure `sw-TZ-RehemaNeural`.
  - Rate 0.85 normal / 0.6 slow, gain +4 dB, 200/150 ms SSML pad.
- **Cache key** (`cache_path_for`, `app.py:323-342`):
  - Key: `word` (en) or `lang:word`, plus `|ipa:<ipa>` if a lexicon entry
    exists, plus `|voice:<v>` for `VOICE_IN_KEY = {en,de,ru,ko}`.
  - File: `md5(key)` → `v4_<md5>_<variant>.mp3`.
  - zh: `zh:<py>:<voice>`.
- **Can a ticket feed the player?**
  - Yes at the low level: `native_audio::play_word(asset_id, url)` /
    `SpellAudio.playWord(assetId, url)` and the HTML path
    `HtmlAudioElement::new_with_src(url)` take any URL.
  - Public entry points (`api::play_word*`) take only a word.
  - The native disk cache is keyed by `assetId = w:<lang>:<variant>:<word>`
    (`native_audio.rs:40`).
  - So a ticket path needs a new `play_ticket(ticket_url)` entry point with a
    ticket-derived `assetId`. A word-derived one would require the word on the
    client.
  - Fetches send no auth header today; F9 wants `Authorization: Bearer`, which
    `fetch` in `audio-native.js` would have to add.
- **Leak caveat for F9:**
  - Players also have `audio_verdict::servable(lang, word, variant)` and a
    Human → Pack → Server → NativeTts source chain, all keyed on the word.
  - Online play must skip that chain entirely: server-ticket audio only, with
    no pack or native-TTS fallback. "No silent fallbacks" makes this a hard
    `audio_unavailable` state.

## R5 — Banks, gating, loader

- **Banks are compiled-in Rust consts**, not files:
  - `src/word_data.rs` (@generated by `scripts/build-wordlists.py`) holds
    `EN_EASY` … for 14 languages.
  - zh is inline in `src/words.rs:180-6370` as `"pinyin|hanzi"`.
  - The en re-tier is in `src/word_data_en_retier.rs`.
  - Language packs (`src/packs.rs`) are **audio only**, never banks.
- **Loader:** `words::tier_for(lang, tier) -> &'static [&'static str]`
  (`words.rs:6382`).
  - It reaches `retier_active()` → `junior_profile()` → storage/agegate. That
    compiles natively, with storage returning None.
  - The server must pass "kid" explicitly rather than rely on that path.
  - Kid filtering is applied by the caller, `game::active_word_list`
    (`game.rs:133-146`) → `kid_filter::filter_kid(lang, pool)`.
- **PURITY:** enforced **at build time and in CI**, not at runtime. The ledger is
  `assets/words/purity/<lang>.tsv` and the gate is `tools/bank/purity_gate.py`,
  called from `build-wordlists.py --check` and `gate.sh`. Whatever is in
  `word_data.rs` has already passed PURITY, so the server inherits it for free.
- **Kid exclusions:** `assets/words/kid-exclude/<lang>.txt`, `include_str!`'d by
  `src/kid_filter.rs` (`kid_allowed`, `filter_kid`).
  - Spell Jr tier ceilings: `experience::allowed_tiers` / `serve_tier`, from
    `config/modes.json` `junior_policy`.
  - Golden file: `config/allowed-tiers.golden.json`.
- **DEV_PREVIEW:** a Cargo feature for dev tooling; **it does not gate
  languages**. The language gate is `consts::is_active_lang`
  (`consts.rs:369-390`): RTL block, then `web` feature → en only, then audit
  override, then registry `Active`.
  - All 15 `BUILTIN_LANGS` are Active; ar is blocked by RTL.
- **Proper nouns:** offline only (`purity_check.py`, `bank_complete.rs` gate 2).
  There is no runtime rule.
- **Tiers:** strings `easy|medium|hard|expert` (`consts::TIER_ORDER`), with no enum.
- **Native usability:** pure Rust, but all of these modules are private
  (`mod words;`, `mod word_data;`, `mod kid_filter;`). Same visibility problem
  as R1.

## R6 — Profanity filter

- **Client:** `src/profanity.rs`.
  - `is_blocked(word) -> bool`, `filter_allowed(Vec<String>)`,
    `rejection_message()` (hard-coded English).
  - 18 language lists, `include_str!`'d from `assets/words/profanity/*.txt`,
    plus constants. **No runtime overlay**; the "seed/custom/overlay" lists in
    the spec do not exist.
  - Pure Rust (std + unicode_normalization), so it runs natively. The module is
    private.
  - Built for word screening (My Words import), not for display names.
- **Server (Climb usernames):** `backend/usernames.py` (`is_offensive`,
  `is_acceptable`), using the `better_profanity` lib plus
  `backend/blocklist.txt`. This is the existing **name** filter. G-D4
  ("adult display names pass the profanity filter") must pick which filter
  applies.

## R7 — Profiles, parental gate, accounts

- **Profiles:** one per device (`learner_query.rs:31-35`,
  `DEVICE: ProfileId = ProfileId(0)`). `multiple_profiles` is an unimplemented
  entitlement bit.
- **Kid:**
  - `AppState.kid` lives in `byear_prefs_v1` and can be set by the user.
  - `agegate::is_kid_locked()` comes from the DOB gate (stores the verdict
    only; under 13 means locked into Spell Jr, `src/agegate.rs`).
  - So `hello.profile_kind` = `kid || age_locked`, as asserted by the client.
    An adult can trivially claim "adult"; that is a G-D13 design fact, not a bug.
- **Parental gate API:**
  - `lib.rs:1250 parent_gate_then(Box<dyn FnOnce()>)`: a localized
    multiplication problem from `agegate::parent_problem()`.
  - `guardian_dash.rs` has its own gate.
  - F10's per-kid "Play online with family" toggle fits behind
    `parent_gate_then`.
- **Accounts:**
  - Climb accounts are in Flask (`backend/climb.py`, `auth.py`, `db.py`;
    SQLite `climb.db`): username, email, bcrypt hash, 90-day rolling session
    tokens.
  - The client token is in **localStorage** `byear_climb_token_v1`.
- **Progress:** all local, in localStorage (`byear_*`, `spell_*` keys). There is
  no server sync except Climb leaderboard submits.
- **CC-SECURITY-BASELINE O6:** answered: accounts are server-side in
  `climb.db`, and progress is device-local.

## R8 — Entitlements

- **No StoreKit at all.** There is no StoreKit or IAP code in `ios/App/App/*.swift`
  and no purchase plugin in `package.json`. JWS is unavailable.
- **Model:**
  - Pure `entitlements::resolve_entitlements(purchased, regional_grants, audit_override)`.
  - One product, `net.spellgame.complete`.
  - Free tier: en = Full, others = Preview (tier 1 only in gated modes).
- **Live:** `play_hub::live_entitlements()` calls
  `resolve_entitlements(false, &[], dev)` (`src/play_hub.rs:120-123`), so
  purchase is always false.
- **The core standard game (the one versus uses) does not consult
  entitlements**; any active language at any tier is playable.
- **Feeds O2:** there is nothing to enforce. The honest build default is
  "online room languages = `is_active_lang`, same as local H2H". That is
  already what local H2H allows.

## R9 — Workspace, CI, registry

- **Workspace:** `.` (spell_wasm), `tracer-core`, `scanlock`, `tonal`
  (tool-only; "spell_wasm must never depend on this crate"). `spell-server` and
  `botload` would be new members.
- **CI (`.github/workflows/`):**

  | Workflow | What it runs |
  |---|---|
  | css-logical | CSS logical-properties check |
  | desktop | Tauri builds on tag |
  | e2e | Playwright on the web test build |
  | editions | wasm build + native `cargo test --lib`, ± `education` |
  | entitlements | entitlement checks |
  | font-coverage | font coverage |
  | modes | modes registry checks |
  | release | Android AAB to Play; iOS disabled |
  | wordlists | `build-wordlists.py --check` incl. PURITY |

  - Local gates: `scripts/gate.sh` and `scripts/hooks/pre-push`.
- **cargo-deny / cargo-audit / Dependabot:** none. The §11 `cargo deny check`
  needs a new `deny.toml`.
- **Container registry:** none; no GHCR. The existing Dockerfiles build on the
  host (`docker compose up -d --build`). F14's "Image= pinned by digest" needs a
  registry choice (GHCR is the obvious one; it needs a token, which is a secret
  Eric holds) or local `podman build` on Rocky with a local digest.
- **Linux CI runner:** GitHub-hosted ubuntu runners have Podman and shellcheck,
  so A14.1/A14.5 are feasible in CI. `quadlet -dryrun` needs Podman ≥ 4.4.

## R10 — Capacitor, WebSocket, origins

- **Plugins:** `@capacitor-community/native-audio`, `@capacitor/filesystem`,
  `haptics`, `local-notifications`, `share` (all v8; SPM, no Podfile), plus the
  custom `NativeLanguageKitPlugin*.swift`.
- **Secure storage: none → trigger 4.** No keychain plugin, no `SecItem` in
  `ios/`. Options:
  - (a) add a community secure-storage plugin. That is a new third-party
    plugin, which needs your OK under CC-SECURITY-BASELINE D4.
  - (b) add `keychainGet/Set/Delete` methods to the existing first-party
    `NativeLanguageKitPlugin` (~40 lines of Swift; no new dependency). Android
    would need an equivalent (EncryptedSharedPreferences/Keystore).
  - (c) accept localStorage, as the Climb token already does. That breaks
    G-D11's wording.
- **Origins:** `capacitor.config.json` has no `server` block, so the defaults
  apply: **iOS `capacitor://localhost`, Android `https://localhost`**. This
  matches the backend CORS list (`app.py:211-223`) and the F4 allowlist.
- **WebSocket:** used nowhere today. WKWebView supports `WebSocket` natively,
  and Cloudflare Tunnel proxies WebSockets. Not yet proven on a device.
- **Share:** `@capacitor/share` exists, so F12's share sheet needs no new
  plugin.

## R11 — i18n

- **Format:** flat JSON per locale, `src/i18n/locales/<code>.json`,
  `include_str!`'d via `build.rs`. 15 UI locales × ~1175 keys; en is canonical.
- **Translation:** **by hand.** There is no MT pipeline. Some locales were
  machine-drafted once by scripts with inline maps (`gen-ar-hi-locales.mjs`,
  `gen-ru-sw-locales.mjs`). `translation-status.json` marks unaudited drafts.
- **Gates:**
  - `i18n-check.mjs` (parity, params) runs in `npm run build`.
  - `i18n-translated-check.mjs` (sameness) runs in `gate.sh` and pre-push.
  - **No GitHub Action** runs either.
- **Trigger 6:** F12/F13 add roughly 40–60 keys (lobby, approvals, resting card,
  update-required, drain banner, reconnect overlay, match_ended reasons,
  emotes, kid name lists `<Color> <Piece>`). The keys aren't written yet, so
  there is nothing to list; every one needs 14 translations. The F10 kid-name
  lists (colors × pieces, per locale) are also translation work. **Decide**:
  - machine-draft + `needs_audit`, the same as the ar/hi/ru/sw precedent, or
  - wait for human translators.

## Other findings worth your eyes

1. **Existing async Online Spell Off.**
   - Code: `src/online_spelloff.rs`, flag `flags::online_spelloff()` (default
     off), `backend/matches.py`, `DEPLOY-matches.md`.
   - Design: 10 seeded words, both players play on their own time, the server
     compares submitted results. It **trusts client scores** (its own
     follow-up list says so), is adult/account-gated, and uses no WebSockets.
   - Questions for you:
     - Does CC-GAME-SERVER replace it, coexist with it, or retire it?
     - Should the new server use Climb accounts instead of G-D11 anonymous
       tokens? Spell Off chose accounts for COPPA reasons.
2. **G-D10 is already true locally.** Versus writes nothing persistent
   (`game.rs:4313`), so online inherits it.
3. **Kid tier ceiling.** Local versus clamps kid tiers through Spell Jr's
   policy, not a hard-coded "Medium". G-D13 should say "Spell Jr ceiling
   (`experience::serve_tier`)" so the two can't drift.
4. **Spell Off and climb both run on the Mac mini backend.** play.spellgame.net
   on Rocky would be a second origin. The cloudflared config lives on Rocky,
   and the app needs `SPELL_PLAY_BASE` alongside `SPELL_API_BASE`.

## Decisions needed before Phase A

| # | Question | My recommendation |
|---|---|---|
| D-R1a | Allow a non-cfg core diff to extract `grade::verdict` (verdict-identical, proven by A6.1 on WASM before/after)? | Yes. There is no other way to share a grader. |
| D-R1b | English online grader: the Flask `lower().strip()` comparison or Rust `fold_strict`? | `fold_strict`, applied server-side only (online only, so solo is untouched) |
| D-R2 | Online H2H = turns-and-chains verbatim (a) or simultaneous rounds (b)? | (a): it is what "same rules" means and drops G-D17's complexity |
| D-R2b | Online tier: chain ladder (climb) or fixed room tier? Timer on/off per room? | Chain ladder (capped by the Spell Jr ceiling for kid rooms); timer as a room setting |
| D-R3 | Kill-flag refresh on entering the online screen (client change) | Yes, or A13.3 can't pass |
| D-R10 | Secure storage: (a) plugin / (b) extend NativeLanguageKitPlugin / (c) localStorage | (b) |
| D-R11 | New strings: machine-draft + `needs_audit`, or wait for humans? | Machine-draft, matching precedent |
| D-X1 | Spell Off: replace, coexist, or retire? | **Eric 2026-10-24: Spell Off is kept, "with multiplayer option."** Not retired, not replaced. Open: what "multiplayer option" means (see D-X1a) |
| D-X1a | Does "multiplayer option" mean (i) Spell Off stays async and real-time Online H2H sits beside it as another option, or (ii) Spell Off itself gains more than 2 players? | **Eric 2026-10-24: (ii), Spell Off itself gets a 3+ player option.** See "Spell Off multiplayer" below |

### Spell Off multiplayer (D-X1a), what it touches

This is Flask work on the Mac mini, not the Rust room server, so it is outside
CC-GAME-SERVER's phases. It needs its own spec or amendment.

- **The schema is two-seat by construction.** `matches` has `player_a`/`player_b`,
  `score_a/b`, `correct_a/b`, `elapsed_a/b` and `winner IN ('a','b','tie')`
  (`backend/db.py:130-148`). N players needs a `match_players(code, user_id,
  seat, score, correct, elapsed, submitted_at)` table, and the migration runs on
  the live `climb.db`. That is a schema bump with ship-order risk, and that
  memory rule says to hold it until the order is planned.
- **Joining:** `join_match` refuses once `player_b` is set (`matches.py:146`).
  N players needs a seat cap, a "host starts / room closes" rule, and a
  decision on whether late joiners can still play after someone has submitted.
- **Winner rule:** today it is more correct, with ties broken by faster elapsed
  (`matches.py:220`). For N players that becomes a ranking, and ties among
  3+ need a rule.
- **Score trust:** results are client-submitted and trusted (`matches.py:17`).
  With two friends that is tolerable. With a group, one forged score decides
  the whole table. The seed is already server-side, so re-deriving the words
  and checking the submitted answers server-side is the natural fix. That
  check needs a shared grader too, so it ties back to D-R1a.
- **Client:** `MatchInfo` in `src/online_spelloff.rs` hard-codes
  `you: "a"|"b"` and `submittedA/B`. It needs a players list and a results
  table UI, and its new strings go through the hand-translation work in
  D-R11.
- **Compatibility:** `online_spelloff` defaults off on main. If no shipped
  build turned it on, the API can change shape freely. If one did, keep
  `playerA/B` in responses for 2-seat matches.
- **COPPA:** unchanged. It is still account-gated and adult-only, with no kids,
  so CC-SECURITY-BASELINE O2 doesn't block it the way it blocks kid rooms here.
| D-X2 | TTS base = Mac mini over Tailscale (needs Tailscale on the Mac mini) | Yes; fix the spec text |
| D-X3 | Release rule anchor (replace "build-56") | "Nothing on any `release/*` branch" |
| D-X4 | Image delivery: GHCR (token) or local build on Rocky | Local build first; GHCR when moving to a VPS |
