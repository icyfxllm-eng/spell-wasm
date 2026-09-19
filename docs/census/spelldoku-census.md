# CC-SPELLDOKU §0 census

Run 2026-09-18, before any mode code, on branch `cc-spelldoku`.

| # | Item | Finding |
|---|---|---|
| 1 | **Name collision** | **None.** A case-insensitive search for `spelldoku` across the repo (excluding build output) finds nothing. |
| 2 | **Registry** | Languages: `src/consts.rs` — `BUILTIN_LANGS` and the `is_active_lang` / status accessors. Which a player may play: `entitlements::EntitlementSet::lang_level` via `play_hub::live_entitlements()`. Which modes a player sees: `config/modes.json` filtered by `modes::visible(&all, &HubCtx)` with `flags::is_on`. |
| 3 | **Audio resolver** | CC-BUILD219-FIXES is not in this repo; the one resolver is `api::play_word` / `api::play_word_with` (`src/api.rs:186`, `:199`), as signed for Translate. It exposes "no audio for this item" as an **`on_fail` callback** after every source has failed — callers can branch on it at play time (Translate does). It has **no up-front query**, so a generator cannot ask before building a board. Phase A needs it only for hint level 3, where the play-time failure is enough; F3 echo givens (Phase B) will need a per-language pre-check, which the Spell Cross E3 ruling already defines as "the language has a server voice". Not treated as a stop. |
| 4 | **Keyboard** | CC-PLAYER-CONTRACT is not in this repo; the keyboard is `src/keyboard.rs`, layouts in `assets/keyboards/`. It covers all 15 study languages (en es fr de pt pl ru vi ko ja fil zh sw ar hi). The keyboard itself is not mountable outside the play screen; other surfaces take its **data** through `keyboard::unit_rows(locale)`, as Word Chains does — SpellDoku will too. **English has no hyphen key**, so twenty-one and similar compound words cannot be typed as written; that matters only for Phase B's 0–45 range. |
| 5 | **Jr resolver** | `src/experience.rs`: `resolve(kid, age_locked) -> Resolved`, `of_kid(kid) -> Experience`, `allowed_tiers(exp, mode_id) -> Vec<&str>`, `serve_tier(exp, mode_id, requested) -> &str`, `leaderboard_allowed(exp) -> bool`, and `TIERS`. |
| 6 | **Leaderboard / score service** | `climb::open_leaderboard` (GET `/api/climb/leaderboard`), `climb::submit_run` (POST `/api/climb/submit-chain`), `climb::upload_local_bests`, `/api/climb/report-name`, and `game::end_chain`, which calls `submit_run`. Online Spell Off posts results to `/api/match/*` (`src/online_spelloff.rs`). Done #10 will forbid all of these from SpellDoku code. |
| 7 | **App-only registration** | Reusable and in use: `#[cfg(not(feature = "web"))]` on the modules and the hub's launch entry (as Spell Picture, Translate, Calendar and My Words lists do), `"platforms": ["ios"]` in `config/modes.json`, and the site build's wall scan. |
| 8 | **Number-word tables** | **None exist** for any language. |

## Two things D3 depends on that the repo does not have

- **No per-language dictionary authority list.** D3 requires every row to cite
  its entry in that list. For English I propose **Merriam-Webster** (one entry
  per number word, cited by URL); the table ships as `status: sourced` until you
  sign it off.
- **TestFlight and production are the same build.** The ship lane builds without
  `audit_preview`, so the binary on TestFlight is the production binary. D3's
  "served in TestFlight, production after sign-off" cannot be told apart at run
  time. So English SpellDoku serves in dev builds (`audit_preview`) as soon as the
  table is sourced, and in TestFlight builds once you sign English off.
