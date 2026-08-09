# CC-PERSIAN-FOUNDATION — Persian (fa) foundation

Pasted by Eric 2026-08-08. **Queued, not started** — "when all this wraps up".
Saved verbatim here because a pasted spec was already lost to context
compaction once today; the transcript is not a safe home for law.

## Intent

Persian dictation (املا) is a graded school subject in Iran because the script
has homophone letters — spelling is genuinely hard and genuinely loved. Build
Persian so the app always displays perfect orthography while never punishing
the player for keyboard limitations, and give the player as many real options
as possible — where "real" means every option passes its effect test. No dead
switches (AUG6 settings-truth invariant applies to everything in this file).

## Decisions (all signed by Eric — if any conflict with existing files implies a strategy reversal, STOP AND ASK)

- **D1** fa canonical map as specified in F1. Exact table is law; extending it requires a new decision.
- **D2** ZWNJ Law per F2 — bank includes ZWNJ words from v1; accept-either input; canonical display; never a tile.
- **D3** fa frequency floor 15,000 (matches ar in the CC-BANK-COMPLETE floor table).
- **D4** Orthographic-depth difficulty weighting ON (F5).
- **D5** Freshness Invariant per-profile, window W derived per tier from pool floors (F6). Applies to ALL languages, Daily included.
- **D6** Azure is the fa TTS provider. Ship both fa-IR neural voices, player-selectable — the options-maximizing call, gated by per-voice effect tests.
- **D7** fa joins ar in the tap-5 secret menu DEV_PREVIEW roster for pre-gate testing. DEV_PREVIEW visibility never leaks to production surfaces — same lint that guards Hindi/Swahili/Russian.

## F0 — Step-0 discipline (prove before building)

Never tune a dead code path.

- Confirm the tap-5 secret menu currently gates ar under DEV_PREVIEW; dump the roster mechanism before adding fa. **If ar isn't wired there yet, STOP AND ASK** — adding fa to a nonexistent gate is a silent architecture decision.
- Prove the Azure provider path live end-to-end with one hardcoded fa word before any bank work: request → Pi cache → playback, with provider recorded in the cache entry provenance.
- Device check (informative, non-blocking per D2): does the iOS Persian keyboard expose a ZWNJ key? Record the answer in the file's notes.

## F1 — fa canonicalizer (shared Rust-core module, single write path)

One word, one byte representation, everywhere. Map, applied in order, then NFC:

| From | To |
|---|---|
| ي U+064A, ى U+0649 | ی U+06CC |
| ك U+0643 | ک U+06A9 |
| ٠–٩ U+0660–0669 | ۰–۹ U+06F0–06F9 |
| tatweel U+0640 | removed |
| harakat / combining vowel marks (U+064B–065F, U+0670) | removed |
| Arabic presentation forms U+FB50–FDFF, U+FE70–FEFF | folded to base letters |

Consumers: bank ingestion (the only write path), player-input acceptance, TTS
request text, cache keys, search, My Words import, photo/OCR import. No
scattered `if (lang === 'fa')` fixups anywhere else — symbol scan enforces
single-module usage.

fa-legal codepoint lint: define the closed set of legal fa codepoints (32
letters + ZWNJ + fa digits). Any bank word containing anything outside it fails
ingestion at CI.

## F2 — ZWNJ Law: orthography, not spelling

The player spells letters; the game owns orthography. Correct Persian is always
what the player sees, never what trips them.

- Bank stores canonical form WITH ZWNJ (کتاب‌ها، بی‌نظیر).
- Match key = ZWNJ-stripped canonical. Input with or without ZWNJ is accepted identically.
- Every rendered surface — resolving tiles, gallery, result cards, Spell Picture paths, My Words — displays canonical form with ZWNJ restored.
- Tile/letters-only modes: ZWNJ is never a tile, never a required keystroke, never counted in word length for the glyph-count CI (glyph-count invariant compares against letter count, ZWNJ excluded — coordinate with CC-RENDER-FIXPASS so the two invariants don't fight).

## F3 — Azure provider abstraction + voice options

Multi-provider TTS without touching the 14 working languages.

- TTS config gains per-language provider; fa → azure, all others unchanged. Cache key = (canonical text, voice id) — provider-agnostic Pi caching.
- Both fa-IR voices ship, player-selectable in fa settings. Settings-truth: the voice toggle enters the effects manifest with a per-voice loopback effect test (whisper.cpp fa transcription passes threshold for BOTH voices on the calibration set). A voice that fails loopback is hidden, not shipped dead.
- Replay + slow-rate inherit CC-AUDIO-REPLAY unchanged; the 0.7× slow test runs against Azure output too.

## F4 — Bank via existing U(lang) machinery

Breadth without new machinery. fa plugs into CC-BANK-COMPLETE: tier-4 frequency
floor + single-word + profanity/Kid seed + TTS-renders + game-usable sense;
floor 15k per D3. Authorities: Dehkhoda, Moin. Definitions launch dark per
standing policy until the fa Fiverr pair's audit ingests; standard Gig A/B
briefs, decoy rows, locked-spreadsheet ingest — zero deviations.

## F5 — Orthographic-depth difficulty (trap-letter tagging)

Difficulty should be the language's own logic. Computable tagging, no human round:

- Count each word's membership in the homophone sets {س ص ث}, {ز ذ ض ظ}, {ت ط}, {ه ح}, {غ ق}.
- Easy tier weights toward zero-trap words (پ چ ژ گ words shine); expert weights toward multi-trap words.
- Monotonicity lint: mean trap-count must be non-decreasing across tiers (sibling of the band-density lint pattern).

## F6 — Freshness Invariant (GLOBAL — all languages, all modes)

Fresh words every run; repeats are a build failure, not a vibe.

- Invariant: within rolling window W per (mode, language, tier), no word repeats. Per-profile ledger per D5; selection = seeded shuffle over (pool − recent-seen). Daily included — cross-day repeats were the reported bug; date-seeding composes with the ledger.
- Feasibility rule: pool floor must cover W with margin; an under-floored pool fails a feasibility check loudly — never silently repeats.
- Coordinates with (does not duplicate) the CC-SPELLPIC-AUDITPASS seeded shuffle — that file's Spell Picture shuffle becomes a consumer of this ledger, not a parallel mechanism. If merging them requires reversing anything signed there, STOP AND ASK.

## F7 — Secret-menu DEV_PREVIEW pairing with Arabic

Two RTL scripts testing one gate catches what one misses.

- fa added to the tap-5 secret menu roster next to ar; both flagged DEV_PREVIEW.
- RTL test surfaces run against BOTH: tile resolution, joined display, replay controls, Spell Picture curve layout when that class opens.
- Production-leak lint: no fa surface reachable without the flag; deliberate-failure test — a build with fa in a production menu fails CI.

## Done when (all checkable)

1. Canonicalizer: `canon(canon(x)) == canon(x)` on a deliberately polluted corpus (Arabic ye/kaf, presentation forms, tatweel, harakat, mixed digits); zero variant collisions post-canon.
2. fa-legal lint green on the full bank; a planted Arabic-ye word fails ingestion (deliberate-failure piece).
3. ZWNJ round-trip: input without ZWNJ → accepted → renders with ZWNJ; input with ZWNJ → accepted; tile mode surfaces zero ZWNJ tiles; glyph-count CI passes on a ZWNJ word.
4. Azure loopback green for both voices on a 20–50 word calibration set; per-voice toggle present in effects manifest with passing test.
5. Trap-count monotonicity lint green across tiers.
6. Freshness simulator: 10k simulated sessions per mode, zero in-window repeats; under-floored synthetic pool fails feasibility CI.
7. Secret menu shows fa+ar under DEV_PREVIEW; production-leak deliberate-failure test fails as designed.
8. Eric plays Persian end-to-end from the secret menu on device — this pass gates any further Persian work.

## Constraints and non-goals

- Persian's public release stays sequenced behind the Arabic RTL gate; this file changes testability, not sequencing.
- Naskh only; nastaliq is far-future and explicitly out.
- No strict-ZWNJ mode, no poetry/classical packs, no Word Stories content — future files.
- Do not touch scoring logic. Do not touch the 14 shipped languages' TTS config beyond adding the provider field with their current provider as default.
- F6 is the only cross-language change in this file; everything else is fa-scoped.

---

## Notes from the Aug 8 session (not part of Eric's text)

Three interactions worth resolving before F6 is built:

1. **F6's feasibility rule collides with the BD-G4 easy-tier rebuild.** That
   rebuild took every easy tier from ~850 scrape words to ~250–280 vetted
   concept words. F6 requires the pool floor to cover the rolling window W
   "with margin" or fail loudly. A ~250-word easy pool may not clear that bar,
   so either W is derived small for tier 1 or the easy tiers need to grow
   before F6 lands. This is a live conflict, not a hypothetical.

2. **CC-SPELLPIC-AUDITPASS D1–D5 are still unsigned.** F6 says to stop and ask
   if merging the shuffles reverses anything signed there. Nothing is signed
   there yet, so today there is nothing to reverse — but that also means the
   Spell Picture shuffle should not be built as a parallel mechanism in the
   meantime.

3. **Azure is a network call.** The gate already runs a family-voices network
   scan, so there is an existing boundary policing what may reach the network.
   F3 needs to be reconciled with that scan rather than bypassing it.
