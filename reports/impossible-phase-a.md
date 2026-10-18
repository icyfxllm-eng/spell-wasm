# CC-IMPOSSIBLE v1.2 Mix: Phase M-A

Built 2026-10-18. Every flag is OFF by default (`impossible`, `impossible_<lang>`, `impossible_mix`), so players see no change.

## What is built

- **Pool builder** `tools/impossible/build_pools.py` writes `assets/impossible/impossible_pool.<lang>.json` for all 15 languages. A pool is 100% of the served Expert tier (v1.2 section 10). The builder removes three things:
  - rows awaiting their PURITY C6 mark
  - every member of a `true` or `accent` sound-alike set, read through `tools/soundalikes.py`
  - any capitalised row, which fails the build (the proper-noun assertion)

  English uses the re-tier Expert tier and table. Each file records its source tier's wordListHash.
- **Registry** `src/impossible.rs`. It is the one place availability is decided:
  - `lang_availability` covers v1 F3, v1.1 A3 and v1.2 P2–P4.
  - `mix_status` covers M1, M3, M7 and M8, and lists the qualifying languages. It has no narrowing input.
  - `mix_pool` is the union of those languages' pools, with each word tagged by language.
  - `notice_due` follows O1: the notice shows on the first run and again when the language set changes.
  - Every gate is a pure function of `Inputs`, and `with_live_inputs` does the one storage read.
- **Flags** in `src/flags.rs`.
- **Gate**: `build_pools.py --check` runs in `scripts/gate.sh` and the pre-push hook. It fails when an artifact is stale, or when a removal count differs from the shared report's new `impossibleFilterRemovesExpert` (v1.1 A2.5).

## Per-language pools

| Language | Source | Expert rows served | PURITY removed | Sound-alike removed | Pool | Status |
|---|---|---|---|---|---|---|
| en | re-tier Expert | 279 | 0 | 8 | 271 | pool (only with en.retier on) |
| es | Expert | 2,999 | 1 | 3 | 2,995 | pool (needs Full access) |
| ru | Expert | 3,000 | 0 | 3 | 2,997 | pool (needs Full access) |
| zh, ja | | | | | 0 | locked: drawn entry (v1 D4) |
| fr de pt pl vi ko fil ar sw hi | | | | | 0 | locked: no sound-alike table yet |

What a player would get with every flag on:
- **Free tier:** only English qualifies, so Mix shows "Needs two Impossible languages".
- **SpellGame Complete:** Mix draws from English, Spanish and Russian (6,263 words).

## Tests (all green; `cargo test --lib impossible`)

| Spec test | What it checks | Rust test |
|---|---|---|
| v1 A1 / X9 | Pool comes only from Expert, and is all of it | `impossible_pool_subset_and_all_expert` |
| v1 A2 (v1.1 A2) | No sound-alikes in a pool | `impossible_pool_no_soundalikes` |
| v1.1 A2.1, A2.2 | The English pairs the spec names stay out | `english_named_pairs_absent` |
| v1.1 A2.3 | A missing table locks the language | `missing_table_locks` |
| v1.1 A2.5 | Removal counts match the shared report | `build_pools.py --check` |
| v1.1 A2.6 | No homophone code in the registry | `registry_has_no_homophone_code` |
| v1 A3 / X11 | 149 rows locked, 150 in | `impossible_gate_threshold` |
| v1 A9 | Stale artifact fails | `impossible_pool_artifact_fresh` |
| v1 A10 | Closed languages, zh and ja stay out | `impossible_closed_language` |
| v1 F11 | Entitlement gate | `entitlement_gate` |
| X1 | Mix pool is the union | `mix_pool_is_union` |
| X2 | Unavailable languages excluded | `mix_unavailable_languages_excluded` |
| X3 | No chooser; notice timing | `mix_no_chooser_notice` |
| X4 | Kid surfaces never get Mix | `mix_kid_surfaces` |
| X10 | English needs en.retier | `mix_english_needs_retier` |
| — | Flags default off | `defaults_offer_nothing` |

v1 A4 (slice bounds) does not apply to Mix (v1.2 P1). As a mutation check, a planted Russian sound-alike (боев) in the pool failed two tests.

## Not in M-A, deliberately

- **No picker entry, strings or notice screen yet.** Start, draw and per-word switching are Phase M-B. An entry with nothing behind it would be a dead end, and an unknown level value flowing into today's tier code is the risk census C1 warns about. The notice's languages and the locked reasons come from the registry, ready for M-B to render.
- **The flags are local, not remote** (v1 F12 says remote). Turning them on for players needs a build or a remote-flag channel.
- v1 Phases B–D (Normal picker and rules, Daily, BoardGame trap, Climb) are not started. The Climb (F7), Daily leaderboard (O1) and German case-folding questions are still open.
