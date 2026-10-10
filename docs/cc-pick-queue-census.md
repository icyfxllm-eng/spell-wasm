# CC-PICK-QUEUE v1 - Section 0 census (RECON ONLY)

Base: origin/main 8fb45c32, branch cc-pick-queue. Nothing in this file is implemented.
Written 2026-10-10. File:line cites are against that commit. "UNVERIFIED" marks anything not
confirmed by reading code.

No prior CC-PICK-QUEUE file existed (`docs/` listing checked). Convention note: census files
live both as `docs/CC-*-census.md` and `docs/census/*.md`; this one is at the path the task named.

## 0. Headline findings (read these first)

1. **Misses has no list screen at all.** Misses is a mode (`app.review = true`), entered from
   `#missesBtn` (retired chip, still the live element; drawer row proxies a click to it,
   `src/lib.rs:917`, `index.html:2114`). There is no row of miss words to tap, so
   "tap words to line up" in Misses needs a NEW surface. My Words has one
   (`src/lists_screen.rs`), app-only.
2. **Misses serves only DUE entries, across all languages, one at a time**
   (`src/game.rs:1571-1603`, `src/misses.rs:34-43`). A picked word that is not yet due is
   currently unreachable, and `enter_review` refuses to start when nothing is due
   (`src/game.rs:3574-3579`). Picking non-due misses is a behaviour change, not an insertion.
3. **Correct answers already promote ANY queued miss, due or not**, from normal play as well as
   from review (`src/game.rs:2084-2093` calls `misses::promote_miss` with no due check;
   `src/misses.rs:89-97`). So D3 ("picked correct counts as practice but does not skip the next
   spaced check") is NOT today's behaviour for a not-yet-due word; see section 2.
4. **My Words already has an "In my order" mode with a per-entry star** (CC-MYWORDS-LISTS D3,
   signed). `ListEntry.starred` leads the list (`src/word_lists.rs:54-56`, `:396-400`). The new
   "focus stars (cap 10 per language)" collide with it by name and by meaning. Needs Eric's call (O-A).
5. **No schema bump is required.** A new storage key avoids the ship-order risk completely.
   Bumping `word_lists` VERSION would be destructive for an older build (section 4).
6. **Telemetry carries no product events**, only errors and perf (section 9). D11 "no new
   events" is satisfied by doing nothing; the gap list is short.
7. **Sign-in exists** (Climb account), but sign-out clears only the token and user, not any
   local word data (section 5). Spell Jr has no sign-out at all.

## 1. Queue builders

### 1a. Misses practice

- Entry: `game::enter_review` `src/game.rs:3566-3600`. Sets `review=true`, disables `langSel`
  and `levelSel`. Exit `exit_review` `:3602`. Toggled by `#missesBtn` `src/lib.rs:917-925`.
- Per-word serve: inside `game::next_word` (`src/game.rs:1534`), branch `else if s.review`
  at `:1571`.
  - Inputs: `state.misses: Vec<MissEntry>` (`src/model.rs:250`), `misses::due_misses` =
    indices where `due <= now` (`src/misses.rs:34-43`), the persisted-then-rebuilt Deck keyed
    `"__review"` (`src/game.rs:1593-1594`, `src/deck.rs:51`).
  - Order today: Deck over the due keys (`miss_key`), no-repeat shuffle, random index
    (`deck.rs next_with`). NOT sorted by anything; recency is not used.
  - Outputs: `s.cur_lang/word/cur_tier` from the chosen `MissEntry` (`:1600-1603`),
    `note_serve(..., DueReview)` (`:1604`).
  - Tone drill fallback: served only when no real misses are due (`:1575-1588`).
- **Insertion point for a lineup**: at `:1571-1603`, before the Deck call. Proposal: if
  `lineup_nonempty()`, take the first lineup key present in `misses` and serve it; else fall
  straight into the existing lines 1593-1603 untouched. Zero picks then executes no new code,
  which is what makes "byte-identical" provable (also by a flag-off path).
- Stale-comment note: `src/model.rs:270-272` says decks are "session-only, not persisted";
  they are persisted (`src/game.rs:1723`, `src/lib.rs:233`, `DECKS_KEY` `src/model.rs:154`).

### 1b. My Words practice

- Not a mode: My Words is the study language `MINE = "__mine"` (`src/consts.rs:24`). The
  draw is the ordinary solo branch of `next_word`, the final `else` at `src/game.rs:1624`.
- Pool: `active_word_list` `src/game.rs:116-129` -> `pool_for_tier` `:89-112`. When
  `state.list_words` is non-empty (a list was chosen), that list is the pool; otherwise the
  flat `state.custom.words`. Filtered by length tier, falling back to the whole set if the
  tier is empty.
- Draw: `src/game.rs:1685-1695`. Two cases today:
  - `in_order = s.lang == MINE && s.list_sequential`: cursor walk through the list
    (`s.list_cursor`, session-only, reset by `lists_ui::refresh_pool` `src/lists_ui.rs:142`).
  - otherwise the persisted no-repeat Deck keyed `"{lang}:{tier}"` (`:1633`, `:1692`).
  - Learner-model promote runs before the draw when `flags::learner_select()` (`:1655-1682`).
- Lists feed the pool via `lists_ui::refresh_pool` `src/lists_ui.rs:128-144` and
  `word_lists::play_entries` `src/word_lists.rs:391-409` (starred first when InMyOrder).
- **Insertion point**: a single guard at `:1685`, i.e. `let w = if let Some(p) = lineup_next(...)
  { p } else if in_order { ... } else { deck.next }`. Because the no-repeat deck is persisted and
  keyed by pool length, a picked word served OUTSIDE the deck must not call `Deck::next`, or the
  deck's pass bookkeeping shifts. Zero picks never enters the new branch.
- Gotchas: (i) `pool_for_tier` filters by length tier, so a picked word whose length tier is not
  the current level would be dropped from `pool`; the lineup must bypass `pool_for_tier`, not
  ride on it. (ii) `list_words` is `Vec<String>` with language stripped
  (`src/lists_ui.rs:139`); per-entry lang is in `ListEntry.lang` and `custom.word_lang`.
  (iii) Misses recorded from My Words carry `lang == "__mine"`, so they are NOT per-language
  in the misses store (section 2).

## 2. Misses data model

- Storage: localStorage key `byear_misses_v1` (`src/model.rs:150`), JSON array of
  `MissEntry`, cap 300 (`MISS_CAP` `:151`; `misses::save` takes first 300, `src/misses.rs:30-33`;
  new entries are inserted at index 0, so the cap evicts the oldest).
- Fields (`src/model.rs:107-124`): `word`, `lang`, `tier`, `misses: u32`, `box_` (legacy
  Leitner, 0 on new), `due: f64` (ms), `ts: f64` (last miss time), `review: Option<ReviewState>`
  (FSRS-4.5 + learning step, `src/review.rs:36-40`). Identity key `misses::miss_key` =
  `word_id0(lang, word)` = `"{lang}::{lowercased word}"` (`src/misses.rs:8-14`,
  `src/word_id.rs:34`).
- Learned state: there is none stored. "Learned" = graduation = the entry is DELETED
  (`src/misses.rs:106-130`, `review::Outcome::Graduated`, `GRADUATE_AFTER_DAYS = 7`
  `src/review.rs:35`). A "star until learned" for a miss therefore ends by the entry vanishing.
  For My Words words there is no learned flag at all (see O-D).
- Per-language scoping: **none**. One flat list; the Misses mode serves all languages mixed
  (`cur_lang = m.lang`, `src/game.rs:1600`). My Words misses are recorded under `"__mine"`
  (cur_lang is MINE in that mode). A "cap 10 stars per language" has to be computed from
  `ListEntry.lang` (My Words) or `MissEntry.lang` (Misses), not from a store partition.
- Spaced schedule: only `review.rs` computes due times (L0 I5, enforced by a source scan,
  `src/review.rs:232-262`). Miss -> `review::on_miss` due now (`src/misses.rs:53-62`,
  `src/review.rs:80-86`). Correct -> `review::on_correct` -> `Outcome::Due(t)` or `Graduated`
  (`src/review.rs:90-107`); learning steps are now, +10 min, then FSRS days.
- Where a correct answer updates next-due: `misses::promote_miss_at` `src/misses.rs:93-103`
  (`state.misses[idx].due = due`), called from `game::on_correct` `src/game.rs:2093`. Grade is
  Hard if audio was replayed, else Good (`:2090`). Retry-rescued words are excluded (`:2078`).
- **I6 (correct answer on a picked miss leaves next due date set)**: HOLDS today on every
  non-graduating path, because `promote_miss_at` only ever assigns `due` or removes the entry
  (`:96-103`, `:106-130`). A due date can be cleared only by graduation. So I6 needs no code
  change for a due word; it needs a regression test, and the one risky edge is that a picked
  word which is graduating still vanishes (acceptable? see O-C).
- **D3 (picked correct counts as practice, does not skip the next spaced check)**: needs a code
  change. Today a correct answer on an entry whose `due` is in the FUTURE still calls
  `review::on_correct` (`src/game.rs:2093` has no due guard), which advances learning steps or
  runs FSRS early (`src/review.rs:90-107`). Minimal change point: in `promote_miss_at`
  (`src/misses.rs:89`), when the answer was a LINEUP pick of a not-due entry, skip the
  `on_correct` call and only bump `ts`/stats. This must be gated on "served by lineup" so the
  existing normal-play promotion stays byte-identical (it is existing behaviour, not ours to change).
- Misses recorded for a picked word that was wrong: `add_miss_at` (`src/misses.rs:53-82`) resets
  the schedule via `on_miss`; unchanged by picking (invariant "grading untouched" holds).
- Other consumers of the same list (must not be perturbed): reports `src/reports.rs:36`,
  learner_query `src/learner_query.rs:177-254`, spelldoku `src/spelldoku_ui.rs:342`,
  drawer badge `src/game.rs:791-819`.

## 3. My Words list model

- Two stores:
  - Flat set `CustomSet` key `byear_custom_v1` (`src/model.rs:74-92`, `:148`): `words`,
    `speak_lang`, `word_lang` map, batch ids. Written by `src/importer.rs:50`.
  - Lists `word_lists::Lists` key `byear_word_lists_v1` (`src/word_lists.rs:20`): `v`, `lists`,
    `next_id`, `migrated`, `selection`. `WordList { id, profile(reserved ""), name, created_at,
    updated_at, source, entries, deleted_at, order }`; `ListEntry { text, lang, added_at, starred }`
    (`:46-72`).
- Scoping: per DEVICE (signed, CC-MYWORDS-LISTS "Scope"); `profile` slot is empty. Per-list: a
  word may live in several lists, once per list (D2). Per-language: per ENTRY (`ListEntry.lang`),
  not per list; a list can hold mixed languages.
- Entry identity inside lists: `(nfc(text).lowercase, lang)` (`src/word_lists.rs:99-101`).
  This is NOT `word_id0`. Progress/misses key by `word_id0("__mine", word)`. Any lineup key
  must pick one scheme; proposal is `word_id0(lang_or_MINE, text)` to match miss/learner identity
  (L0 I4, `docs/CC-LEARNING-ENGINE-L0.md:239`).
- UI: `src/lists_screen.rs`. Home cards `.lists-card[data-l-open]` (`:56-76`), detail rows
  `entry_row` `:147-196` -> `.lists-entry > .le-word + .lists-chip + (star/up/down) + edit +
  remove`, all via `data-e-*` attributes, not DOM ids (only `#listsEdit`, `#listsRename`).
  Container `#listsScreen`, `#listsBody`, `#listsUndo`, `#listsClose` (`index.html:3539-3548`).
  Whole screen is app-only: `#[cfg(not(feature = "web"))]` at `src/lib.rs:1122-1130`; the site
  build keeps the paste sheet. Row CSS `index.html:1859-1871`.
- Existing play affordances: "Play this list" (`data-l-play`), "Play all words"
  (`data-l-play-all`), Mixed / In my order switch (`data-l-order`) `:252-258`, `:285-288`.

## 4. Session persistence, force-quit, schema risk

- There is NO session-persistence layer for Misses or My Words. `AppState.review` is not in
  prefs (`src/model.rs:161`, absent from `Prefs`); `list_cursor` and the current word are memory
  only. After a force-quit the app boots with `review=false` and no word; the player re-enters
  Misses via the button. What survives: `byear_misses_v1` (written on every miss and every
  promote, `src/misses.rs:29,79,130`), `byear_word_lists_v1` incl. `selection`, the persisted
  decks, prefs.
- localStorage wrapper: `src/storage.rs` (`get_json`, `set_json`, silent failure, no-op on host
  tests `:14-23`). No transaction; per-key atomic writes only.
- Version rules seen: `word_lists::load` discards ANY stored blob whose `v != VERSION` and
  starts clean (`src/word_lists.rs:495-501`), then `store` would overwrite. So an OLDER build
  reading a NEWER `v` silently loses the user's lists on next write. This is the ship-order
  risk. `learner.rs` instead keeps bytes and reports `Loaded::Newer` (`src/learner.rs:540-555`).
  `MissEntry` has no deny-unknown-fields, so extra `#[serde(default)]` fields are readable by
  old builds but are DROPPED when an old build rewrites the array.
- Consequence for the lineup: store it in its OWN new key (proposal `spell_lineup_v1`), written
  on every pick change, with a `v` field, tolerant reader (unknown version -> keep bytes, behave
  as empty, never overwrite), never altering `byear_misses_v1` or the lists blob. That needs no
  schema bump of any existing store, so no ship-order hold applies. An old build simply ignores
  the key, which is also the downgrade path. Do NOT add `picked` to `MissEntry`/`ListEntry`.
- Lifecycle events: no `visibilitychange`/`pagehide` handler touches game state (grep:
  none outside audio, telemetry flush). Write-through-on-every-change is therefore the only
  safe design for "survive app close, backgrounding and crash"; there is no hook to rely on.
- Reset paths found: `#resetStats` clears learner + stats + `wordstats` for ONE language
  (`src/lib.rs:1452-1459`). It does not touch misses or lists. Whether the lineup should clear
  there is an open question (O-E).

## 5. Sign-in / sign-out (O1)

- Exists: Climb leaderboard account. Logout handler `src/climb.rs:202-210`
  (`do_logout`: POST `/api/auth/logout`, `clear_token`, `set_user(None)`, close scrim,
  `reflect_auth`). Sign-in `src/climb.rs` ~`:190-200`. Drawer account row `src/drawer.rs:176-205`.
- It clears NO local learning data: misses, lists, decks and prefs are device-scoped and survive
  sign-out. So there is no existing "wipe on sign-out" to extend.
- Spell Jr gets no Sign out row (`src/drawer.rs:176-192`, test `:1164-1169`): a child cannot
  sign out of a parent's account.
- O1 resolution input: "cleared only by explicit end-session or sign-out (if sign-in exists)"
  would, if literal, be the ONLY place a sign-out starts touching local word state, and it would
  break "per device". Recommendation: sign-out must NOT clear the lineup (data is device-scoped
  like misses and lists); clear only on explicit End session. Eric to sign (O-B).

## 6. Spell Jr vs Standard

- One boolean `AppState.kid` / `Prefs.kid` (`src/model.rs:13`, `:226`), read through
  `experience::of_kid(kid) -> Experience::{Junior,Standard}` (`src/experience.rs:72`,
  `resolve` `:60`, `allowed_tiers` `:86`). `allowed_tiers` is the one tier authority; per-mode policy is data in
  `config/modes.json` `juniorPolicy`; `my_words` and `misses` are `"ungated"`
  (`config/modes.json:277-320`, golden `config/allowed-tiers.golden.json:142-163`: junior gets
  easy+medium).
- Jr differences in these screens today: list delete needs the parent gate
  (`src/lists_screen.rs:547-558`, `:81`, `:280`); built-in-language pools get the kid word
  filter but My Words does not (`src/game.rs:116-129`); tier cap `clamp_base_tier` `:1648`.
- Jr's Misses entry is the same `missesBtn`; Jr is not hidden from either surface.
- For the new feature: Standard/Jr split should be ONE capability function next to
  `experience::allowed_tiers`, e.g. `experience::pick_caps(exp) -> { max_picks: 5|None,
  sort_search_stars: bool }`, returning the same shape for every language. No per-language
  branches are needed: nothing in picking depends on language. Proposal only.

## 7. UI and DOM ids, RTL

- Misses: `#missesBtn` (retired chip, `index.html:2114`), label from
  `top.misses` / `top.missesExit` (`src/game.rs:791-819`). In-play affordances are the orb and
  `#feedback` ("Practicing your misses - N due now.", `src/game.rs:3597`, hardcoded English).
  No rows exist. A new list surface needs its own ids; `scripts/dom-id-check.mjs` (uniqueness)
  and `dom-id-live-check.mjs` (a `dom::el(id)` for a missing id is a panic) both gate this.
- My Words: see section 3. Rows are `.lists-entry` with `.le-word`; no per-row id (attribute
  hooks only). Tapping the word text is not currently a handler; the row's controls are buttons.
- RTL: `dir`/`lang` are stamped per word surface via `reflect_play_direction`
  (`src/game.rs:376-414`, `consts::dir_attr`). `RTL_SUPPORTED = true` (`src/consts.rs:147`).
  Document-level `dir` switching for the UI chrome: UNVERIFIED (not found in `i18n.rs`/`lib.rs`
  by grep). The lists CSS already uses logical properties (`text-align:start`,
  `margin-inline-start:auto`, `margin-inline:auto`, `index.html:1850-1871`).
- Badge/star placement: the spec says badge top-left, star top-right. In logical terms those are
  `inset-inline-start` (badge) and `inset-inline-end` (star), so in RTL the badge sits top-RIGHT.
  Use `inset-inline-*`, `padding-inline-*`, `margin-inline-*`, never `left/right`. Numerals
  inside a badge are digits, keep them in `<bdi>` / `unicode-bidi:isolate` so the Arabic
  neighbour does not reorder them (proposal; mixed-script word text in the same row is the risk).
- Gate: `scripts/css-logical-check.mjs` scans only `index.html` and `privacy.html`, allowlists
  three idioms (`left:50%`+translateX, orb specular, safe-area env), runs in GitHub Actions
  (`.github/workflows/css-logical.yml`) and in the pre-push hook `scripts/hooks/pre-push:38`,
  NOT in `scripts/gate.sh`. Geometry diff tool: `tests/e2e/css-logical-geometry.mjs` (manual).
  Any new CSS must live in `index.html` to be covered, or the check must be extended.

## 8. Localization

- Path: flat key->string JSON, `src/i18n/locales/{code}.json`, embedded via `build.rs:34-43`
  into `src/i18n.rs:25-46`; read with `i18n::t(key)` / `i18n::tp(key, &[("n", ..)])`
  (`src/i18n.rs:157`, `:172`). Fallback chain: active locale -> en -> the key itself.
  Static markup uses `data-i18n` / `data-i18n-aria` and `translate_page()`.
- Locale count: **15** files: ar de en es fil fr hi ja ko pl pt ru sw vi zh. (Telemetry's
  `Lang` also lists `fa`, but there is no `fa.json`, `ls src/i18n/locales`.) en.json has 934 keys.
- Gates:
  - `scripts/i18n-check.mjs`: key parity (missing/extra keys, empty values, `{param}` drift
    versus en). Run by `npm run build`/`i18n:check` (`package.json:7,23`). NOT in `gate.sh`.
  - `scripts/i18n-translated-check.mjs` (+ `--selftest`): for keys on `translated` in
    `src/i18n/translation-status.json`, a non-en value equal to the English fails
    (`gate.sh:141-142`, pre-push hook). Brand names live in `do_not_translate`; unaudited drafts
    in `needs_audit` (per-locale, must be a subset of `translated`).
  - `scripts/hardcoded-label-check.mjs` (pre-push): visible text in `index.html` with no i18n key.
    Rust-built HTML such as `lists_screen.rs` is not scanned, but must still go through `t()`.
- Cost of one new string: 1 en entry + 14 translations = 15 edits, plus a `translated` entry
  (and `needs_audit` rows for the machine-drafted locales). The existing `lists.*` namespace
  has 29 keys (`en.json:351-354` etc). Estimated key budget for the feature, UNVERIFIED until
  design: ~20-25 (badge aria, star aria, "Practice N first", "Just these", sort labels x3-4,
  search placeholder, Toughest 5, session banner, end-session, Jr cap message, empty states,
  undo). That is ~300-375 locale values; this is the dominant cost of the feature and the
  reason to keep copy minimal.
- Existing violations to NOT copy: Misses feedback strings are hardcoded English
  (`src/game.rs:1580`, `:3597`, `:798` area, "All caught up ...", "Practicing your misses ...").
  Reusing those surfaces for the lineup status line would ship English to 14 locales.

## 9. Telemetry (CC-TELEMETRY-FOUNDATION)

- Records are only `error_event`, `event_batch`, `perf_row`, `agg_*`, `flags`
  (`src/telemetry/schema.rs:264-304`). Event kinds: errors (`ErrorCode` `:46-52`) and perf
  metrics (`:60-62`). No usage/product event exists, by design (I3: "No word, answer, miss,
  attempt", `src/telemetry/mod.rs:1-25`; L0 I1/I2 forbid learner fields).
- What touches these surfaces: only the coarse `Mode` context. `set_mode(Review)` at
  `src/game.rs:3567`, `set_mode(MyLists)` at `src/lists_screen.rs:381`. `Lang::Mine` hides My
  Words content (`schema.rs:98-100`).
- Local (non-network) provenance: `learner_query::NextWordReason` (`src/learner_query.rs:39-58`)
  has `DueReview`, `InOrder`, `Deck` etc. A picked serve would need a new variant; that enum is
  an L0 contract ("an enum, never prose", `:37`) so adding `Picked` is a contract edit, not free.
- Gap list for D11 (no new events): (a) no way to measure pick adoption, session length, missed-
  pick re-entry, or abandon; (b) a panic inside the lineup path would be tagged `Review` or
  `MyLists` with no distinguishing feature. Both are accepted under D11. If a future decision
  wants (b), a new `Mode` variant needs schema + JS binding + Worker change
  (`telemetry-schema.js`, `workers/telemetry`). Not proposed.

## 10. Feature flags

- Each flag is `pub fn name() -> bool { resolve(stored("name").as_deref(), DEFAULT) }`
  (`resolve` `src/flags.rs:20`, examples `:157-158`, `:331-335`). Storage `localStorage['spell_flag_<name>']`
  = on/off/1/0/true/false/yes/no; absent -> compiled default (`:20-28`).
- Dark-by-default precedent: `translate` (`:257`), `sd_tier_mode` (`:333`), `word_stories`,
  `offline_packs`, `online_spelloff`.
- If a flag is also a registry mode it needs an arm in `is_on` (`:337`); test
  `every_registry_mode_has_an_arm` (`:412`) enforces it. A pick flag is NOT a mode, so it should
  be a plain function and NOT added to `modes.json`.
- Host unit tests: `set_test_override(Some("on"|"off"))` (`:49`) is ONE global override for
  all flags (not per-flag). Phase flags A-D tested together would see the same value, so
  a dual-config test cannot independently set B on while A is off in one process; each phase
  flag test must be separate, or the override must be extended to a per-name map (a small
  test-only change, outside this census).
- E2E has no cfg(test) seam: it sets `localStorage['spell_flag_<name>']` before boot
  (`tests/e2e/specs/spelldoku-tier.mjs:20`, harness `tests/e2e/harness.mjs:109-161`).
- Proposed names (proposal): `pick_queue_a` ... `pick_queue_d`, each default false; phases nest
  (B reads A && B) so a stray B cannot ship dark-A behaviour.

## 11. Existing tests that baseline the queue

- Host (cargo test --lib, run by `gate.sh:236-242`):
  - `src/review.rs` tests: schedule rules (miss due now, +10 min, learning->FSRS,
    graduation, Hard sooner than Good, carry-over, determinism) and the I5 second-rule scan
    (`:~270-311`); a ru homograph case drives `add_miss_at`/`promote_miss_at` (`:301-305`).
  - `src/attempts.rs` tests a1-a12, i2, i8: "spaced-rep identical regardless of retry outcome"
    (`:231-246`) is the closest to a "grading untouched" baseline.
  - `src/word_lists.rs` tests: lossless migration, `play_entries` ordering (`:735-770`+).
  - `src/tone_drill.rs` tests (`:234`, `:253`).
  - `src/deck.rs` tests for no-repeat (`next_with`, deterministic via injected rand).
  - `src/flags.rs` tests (`:420-479`).
  - **Gap**: `src/misses.rs` itself has NO `#[cfg(test)]` module. `due_misses` and `promote_miss_at`
    are exercised only indirectly. A2/A12 baselines for "zero picks byte-identical" do not exist
    for the serve ordering in `next_word` (it uses JS RNG and the DOM; untestable on host as is).
    A baseline needs either the injected-RNG seam (`Deck::next_with`) or an e2e snapshot.
- E2E specs (`tests/e2e/specs`, run by `tests/e2e/run.mjs`):
  - `review-queue.mjs`: miss enters queue due now; pre-R2 carry-over keeps due; Hard vs Good.
    Closest A2 baseline for Misses; reads stored bytes (`byear_misses_v1`).
  - `mywords-lists.mjs`: migration, save sheet, cards, edit/undo, delete+undo, Jr parent gate
    (`:339`), `mywords_playing_a_list_serves_only_its_words` (`:364`),
    `mywords_play_falls_back_...` (`:388`), `mywords_in_my_order_serves_the_players_order`
    (`:431`). The A12 baseline for My Words.
  - Adjacent: `drawer.mjs`/`drawer-close.mjs`, `hub-inventory.mjs`, `economics.mjs`,
    `learner-surfaces.mjs`, `learner-reset.mjs` all reference misses/My Words.
- Static gates a new screen will trip or must satisfy: `dom-id-check`, `dom-id-live-check`,
  `mode-exit-check` (every mode needs an exit, an explicit WORD `aria.exit`), `modes-check`,
  `css-logical-check`, `hardcoded-label-check`, `e2e-sleep-ratchet` (no new fixed sleeps in e2e),
  `i18n-translated-check`, `scroll-check`, `tier-list-check`.

## 12. Risks, conflicts, decisions needed

Conflicts with signed decisions or invariants in the repo:

- **CC-MYWORDS-LISTS D3** (signed): "In my order" is "the one permitted change to how a mode
  draws words" (`docs/CC-MYWORDS-LISTS.md:28`, `:55-56`, `:309`). A lineup is a second change to
  the draw. Needs an explicit amendment: the lineup is an overlay applied BEFORE the list order,
  list order remains for unpicked words. State this when this spec is signed.
- **CC-MYWORDS-LISTS D3 star** vs "focus stars" (O-A).
- **L0 I3** "Ordering only: may reorder legal candidates" (compatible; lineup is order only),
  **I4** identity-keyed (lineup key must be `word_id0`-based), **I5** one review rule (compatible
  only if D3's "skip" is implemented by NOT calling `on_correct`, never by computing a due),
  **I6** contract-only access to learner data (the lineup must not read `state.misses` / learner
  directly in a new consumer, or go through `LearnerQuery`; note `game.rs` itself already reads
  `s.misses`, so this applies to NEW modules), **I7** deterministic (a lineup is deterministic;
  "re-enter 3-5 positions later" must use a fixed offset or a seeded value, not `Math.random`,
  or it breaks I7 for scheduling. Pick a rule: e.g. position `4`, or `3 + (hash(key) % 3)`).
- **L0 I1/I2 and telemetry**: lineup contents never leave the device; do not add fields to
  `schema.rs`.
- **Misses graduation deletes the entry** (`src/misses.rs:130`): a picked word that graduates
  disappears mid-session; the lineup must tolerate a key with no backing entry, and "Just these"
  must not loop on an empty set.
- **MISS_CAP 300** eviction (`src/misses.rs:29-33`) can silently drop a lineup target.
- **Due semantics**: Misses is due-only. Serving non-due picks reshapes "All caught up" and the
  drawer badge (`src/game.rs:791-819`, `src/drawer.rs:665`); the badge must remain the due count.
- **Web build**: My Words screen is app-only; "Misses and My Words" on the site would have no My
  Words surface (`src/lib.rs:1122-1130`). Scope decision needed (O-F).
- **Mode-exit law** (memory: every game mode needs an exit): the lineup/session has to expose a
  WORD exit and an end-session control, including on its end screen; `mode-exit-check.mjs` gates it.
- **Concurrent sessions** share the repo (memory): worktree + check `git log` before merge.
- No document edits to `docs/CC-MYWORDS-LISTS.md` were made.

Open questions for Eric (not guessable from code):

- **O-A** focus stars vs the existing per-entry `starred` in My Words "In my order". Merge into
  one concept, or keep two? If kept, copy must distinguish them.
- **O-B** (O1) sign-out: clear lineup or not. Recommendation: no (per-device, section 5).
- **O-C** a picked word that graduates: leaves lineup silently, or is kept until end-of-session?
- **O-D** "until learned" for My Words words: there is no learned flag. Candidates: graduation
  of the matching miss, `wordstats` streak `>=3` (`src/wordstats.rs:92-102`), learner mastery
  >=0.8 after 2 reviews (`src/learner_query.rs:64-70`). Pick one and name the source.
- **O-E** should `resetStats` ("Reset this language") clear that language's lineup/stars?
- **O-F** web build scope: app-only like the lists screen, or add a pick surface to the site?
- **O-G** Misses non-due picks: confirm the player may practise a word that is not due, and that
  D3 (no schedule skip) is the rule for exactly those.
- **O-H** per-language star cap "10 per language": in Misses, languages mix in one flat list, and
  My Words misses are all `"__mine"`. Cap by `MissEntry.lang` / `ListEntry.lang` as the spec
  implies, or per "study language" screen?
- **O-I** copy budget: ~20-25 keys x 15 locales; confirm the translation route (auditors as for
  `needs_audit`) before phase A, since `i18n-translated-check` will fail on machine drafts left
  equal to English.

## 13. Proposed insertion design per phase (PROPOSAL, not decided)

- **Phase A - lineup core, no stars/sort/search.** New module `pick_queue` (pure, host-testable,
  no DOM): `Lineup { v, picks: Vec<Key>, just_these: bool, ended: bool }` in new key
  `spell_lineup_v1` (+ separate key per surface: misses vs my_words list id). API:
  `toggle(key)`, `next_served(&available) -> Option<Key>`, `reinsert_after_miss(key, offset)`,
  `prune(&live_keys)`. Wire at the two guards named in 1a and 1b only. Flag `pick_queue_a`
  off -> neither guard compiles into the path (early return before any read). Pick UI: Misses
  needs a new list sheet (data from `state.misses`); My Words reuses `entry_row` with a tap target
  and a badge. Write-through on every toggle.
- **Phase B - session lifecycle + persistence.** Explicit "Practice N first" start and an
  End session control (WORD exit). Lineup survives relaunch by reading `spell_lineup_v1` at boot
  after `misses::load` / `word_lists::boot` (`src/lib.rs:225-233`). Prune picks whose backing
  entry vanished. Clear on End session; per O-B not on sign-out.
- **Phase C - Standard extras.** Focus stars (cap 10 per language) as a second list in the
  same key, `Toughest 5` computed from `MissEntry.misses` desc then `ts`, sort and search over
  the already-loaded rows (no new store reads). "Just these" = serve only picks then end.
  Missed pick re-entry via `reinsert_after_miss` with a deterministic offset (I7). D3 guard in
  `promote_miss_at` for not-due picks, gated on "served by lineup".
- **Phase D - Spell Jr + polish.** Jr: tap-to-line-up only, `max_picks=5` through a single
  `experience::pick_caps`; sort/search/stars absent (ABSENCE, not locks, as `src/modes.rs:229-232`
  does for hidden modes). RTL pass with inset-inline-*, localization of all strings, the A2/A12
  baseline specs wired, and the css-logical check extended if CSS leaves `index.html`.
- Cross-phase test plan: dual-config (flag off byte-identical to main snapshot of serve
  order using `Deck::next_with` seeded RNG, flag on with zero picks identical to flag off);
  e2e on stored bytes of `byear_misses_v1` asserting grading/streak/learned untouched after a
  picked session; mutation check per the memory "test apparatus traps".

## Not verified

- Document-level RTL `dir` switching for chrome (section 7).
- Whether any iOS-native layer persists session state outside localStorage (no Swift/Kotlin read).
- Exact `is_on` Tools-hub rendering of a new flag (only the dispatcher was read).
- e2e specs were read for names and headers, not executed. No tests were run in this census.
