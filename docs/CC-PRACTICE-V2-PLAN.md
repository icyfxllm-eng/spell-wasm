# CC-PRACTICE v2 — implementation ledger

Spec: Eric, 2026-07-27 ("after the hub clean up please add these improvments to
practice"). v2 supersedes v1 IN FULL: adds the interaction layer — tile
assembly, coach orb, trap micro-interactions, echo-spelling, syllable chunking,
hint ladder, choice beats, ghost-trace. Identity bar: within 5 seconds of word
1, Practice must look unlike the base game (tiles + ghost letters + coach).

## Decision → implementation map

| # | Decision | Where | Status |
|---|----------|-------|--------|
| D1 | Boundary with Script Paths | graduation "what's next" doors | ✅ with deviation: **CC-SCRIPT-PATHS does not exist in this repo** (repo-wide grep, 2026-07-27). Zero pilot languages ⇒ the Script Paths door renders for none; door plumbing ships dormant. Not a collision — nothing to collide with. |
| D2 | Ghost-trace 1–5 / cued 6–12 / audio 13–20 + keyboard fade-in + chunking | `practice.rs` consts `GHOST_UNTIL/CUED_UNTIL`, `phase()`, `chunks()`; screen swaps tray→keyboard at Audio | ✅ chunking deviation: repo syllable data is **es-only by design** (`syllable.rs` — other languages need pronunciation dictionaries the pipeline doesn't carry). Phase-3 chunking = es; all other languages deliver whole-word. Data-driven: any future `syllabify` coverage lights up automatically. |
| D3 | Tile tray 6–8, 1–3 decoys, deterministic, jamo/kana | `practice::units/tray` (splitmix64, FNV-1a seed on (lang,word)); ko tiles = compatibility jamo, assembly via `hangul::feed` (the repo's real IME) | ✅ |
| D4 | Coach orb, ≤40 audited lines/lang, deterministic pick | `practice::Coach` pools + `coach_line()`; spoken via existing TTS, bubble text; Practice-scoped | ✅ pools machine-drafted for 14 non-en languages — rides the standing Fiverr audit round like v1 strings |
| D5 | Micro-interactions: TAP_SILENT_UNIT / HEAR_PICK / TAP_STACK, unfailable, 2-miss auto-reveal | screen `micro_*`; curriculum `traps[].template{id,params}`; no-template trap → v1 intro card fallback | ✅ |
| D6 | Echo-spelling, ~2s, skippable | screen `echo_*`, per-unit TTS reuse | ✅ |
| D7 | Hint ladder 3 steps, free, no tracking | screen `HINT_STEP`; not persisted | ✅ |
| D8 | Choice beats every ~4 words, curriculum-encoded, curated emoji | `choiceBeats[{at,alt,emoji[2]}]`; pair = (words[at], alt), same trap class; unpicked skipped; pick recorded in Progress for exact resume | ✅ emoji machine-curated for non-en — audit-riding |
| D9 | Failure-proof | unchanged v1 posture; purity gate extended to new code | ✅ |
| D10 | Curriculum = data | `scripts/build-practice-curricula.py` v2 regenerates all 15 with decoys/templates/beats/coach pools | ✅ |
| D11 | No assessment writes | Progress record only (now incl. choice picks — still the one record) | ✅ |
| D12 | Availability/ceremony/upsell | unchanged from v1 (practice free at PREVIEW; upsell absent — no purchase surface exists; **audit-gate registry does not exist in repo** — v1's interim-content ruling by Eric stands, all 15 visible) | ✅ |

## Invariants

- I1 purity scan: `practice-purity-check.mjs` (unchanged scope, both files)
- I2 store diff: Progress under `spell_practice_<lang>` remains the ONLY write
- I3 schema: `practice-check.mjs` v2 — 20+5 from bank, alts in bank + same
  trap class, template ids ∈ {TAP_SILENT_UNIT, HEAR_PICK, TAP_STACK}, coach
  ≤40 lines ≤90 chars, decoys are single typing units, emoji per beat = 2
- I4 golden: `golden_tray_and_coach_are_deterministic` (en + ko pins)
- I5 property: `tray_always_completable_*` — sampled bank words per live
  language via host fs; tray ⊇ word units incl. duplicates
- I7 hint ladder + micro two-miss auto-reveal: enforced in screen logic
  (`hint()` always ghosts a next unit; `micro_tap` → `micro_done(auto)` at 2
  misses); exercised live in-browser 2026-07-28 (en run: wrong tile wiggle,
  hint-lit tile, TAP_SILENT_UNIT, choice beat pick, keyboard truncate-hold).
  Host unit tests can't reach web_sys — Playwright fixture remains the debt.
- I8 NFC everywhere; ko assembled-block matching via `hangul::feed`; phase
  boundaries + chunk threshold are consts

## Standing debts (same class as v1, non-blocking)

- Native-speaker audit: coach pools, template prompts, choice emoji (14 langs)
- Playwright en/es/ko run + ko block-assembly video: **no local browsers on
  this Mac** (standing CI debt, ledgered since CC-DEF-MATCH)
- Eric's behavioral bar ("teach, not test") needs a live tester pass

## Found & fixed while shipping (2026-07-28)

- **`practiceOpen` never existed in index.html** — the hub's Practice tile
  was a silent dead end on builds 96–98 (dom::click no-ops on a missing id).
  Hidden launch button added beside defMatchOpen; verified live.
- **Web deploys could serve mismatched wasm/JS for up to 4h**: the loader's
  `?v=DEV` cache-buster was never stamped and sw.js pinned CACHE_VERSION
  "v44", while Cloudflare edge-caches pkg/* for 4h — spellgame.net was down
  with a LinkError until this was found. build-web.sh now stamps both from
  the wasm content hash, so every deploy busts the edge cache atomically.
