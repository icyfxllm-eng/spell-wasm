# CC-THEMES-COSMOS v1 rev 2: §0 census

Read-only, run 2026-10-28 against origin/main 8c98c5a5. Nothing was built, changed or committed in the app.
The build-303 release branch (release/persian-coming-soon-b303) was read for flag states.
Phases A–D remain gated on Eric's review.

## Verdict

- **No HALT.** C1 and C2 are both merged.
- **Three stop-and-ask items:**
  - C3: the Lab cannot be compiled out of the App Store build without also being compiled out of TestFlight.
  - C6: 8 proposed accents fail I-T3.
  - I-T2 conflicts with the signed light-theme verdict pair.
- **Two items need Eric's numbers or definitions:**
  - O2: there is no Climb data at all.
  - C4: three counters are missing and need definitions.
- The "Post-build-56" line in the header is stale. The app is at build 303, the same clause the Eclipse signing sheet deleted.

## Eric's answers (2026-10-28)

- **C3:** an internal-only Lab TestFlight build, with a guard that refuses to submit it. ("Internal Lab TF build")
- **I-T2:** one verdict pair per lightness: dark themes keep the base pair, bright themes use the light pair.
- **I-T3:** ΔE ≥ 20 from **all** verdict colours, including the feedback-line variants and the wrong-answer coral.
- **Order:** "do the other findings after the theme changes". The Test entitlements switch is fixed after the theme work.

Still open: C4 counters (Phase B), C7, C9, O1, O2.

## C1 Theme registry: MERGED

- **Registry:** src/themes.rs.
  - `enum Theme` has 5 ids: eclipse, daybreak, chalkboard, typewriter, beestage. Eclipse and Daybreak are free.
  - `earned()` holds the rules, and `Store` is the unlock ledger.
- **Token schema:** design/tokens.json `$overrides`. Each theme is one `:root[data-theme="<id>"]` block of `color.*` overrides: bg, bg-2, surface, surface-2, edge, text, text-muted, text-subtle, text-on-bg, bg-user, accent, accent-soft, glow, controls, button, orb.core.
  - scripts/tokens-build.mjs generates the index.html block and src/tokens.rs.
- **Ledger:** localStorage `spell_themes_v1`.
  - It holds unlocked ids, the choice, a baseline frozen once from base-game stats, and `correct_since`.
  - It is idempotent and never revokes, and tests cover both. The stats reset does not wipe it.
- **Jr variant:** a `:root[data-theme=X] body.kid` block for each theme. Today it changes only glow, controls and accent-soft, never `accent`.
- **Appearance:** `render_picker()` shows System first, then every entry in `ALL`. A locked row shows its condition string. There is no "hidden" state yet, so T-D15 needs one (a visibility predicate per theme).
- **Gradients:** the bg tokens are flat colours (`--bg-color`). The Sky gradients and the metal sheen need a new token type, a background image or layer, plus an art layer. That is still token-only per I-T7, but it is new schema.

### I-T2 conflicts with a signed mechanism. STOP AND ASK

I-T2 says verdict tokens are "identical in every theme". The light themes already override them, and this was signed in the F9 and HOTFIX work:

| Theme | success | caution |
|---|---|---|
| Eclipse, Chalkboard, Bee Stage | #46d6b8 | #ff9e57 |
| Daybreak, Typewriter | #13806b | #b5540a |

The dark pair fails contrast on light grounds.

- **The question:** the 5 new bright themes (3 metals, Beach, Sky Day) would need the light pair too.
- **Recommended reading:** each verdict is fixed per lightness, so all dark themes share one pair and all light themes share the other. Eric's initial is needed, because this rewords a rule.

## C2 Theme-true tokens: MERGED

- P0 4a1ebbf3 and P1 59f301bf are ancestors of main.
- The token ratchet reports 0 raw colours. The 63 exempt colours are all marked token-exempt and sit at the ceiling.

## C3 Tap-5 menu: STOP AND ASK

- **The menu:**
  - src/dev.rs: five taps on `#brandMark` within 1200 ms of each other.
  - The DOM is `#devMenu` in index.html at line 3676.
  - The roster is static HTML: Racing, CJK ink probe, Arabic, Test entitlements, Frame logger, Mic diag (cut), Frame report.
- **It is in every build.** `dev::wire(app)` has no gate at lib.rs:541.
- **A build-type flag exists.** It is the cargo feature `dev_preview`:
  - used by build-web-dev.sh and build-web-test.sh;
  - absent from build-web.sh;
  - backed by the comment-marker cut in scripts/cut-diag.py;
  - proven absent by scripts/seam-absence-check.mjs.
- **The problem:** cut-diag.py says it outright: "TestFlight and the App Store ship one binary". Compiling the Lab out of the App Store build also compiles it out of every TestFlight build. Phase A's gate is Eric reviewing on his own device. So the decision is where he gets a Lab build:
  - **(a) Recommended:** a dedicated Lab TestFlight build cut with `--features dev_preview`. It goes to internal testers only and is never submitted for review. It follows the pattern of the release branches and per-language audit builds, with its own build number and a lane guard that refuses `submit_review` when the dist contains a Lab marker.
  - **(b)** A dev_preview build installed from Xcode onto his phone, or run in the simulator. This needs no lane work, but it is not "his own device via TestFlight".
  - **(c)** A runtime gate, for example a TestFlight sandbox-receipt check. This breaks T-D10's "compiled out" and is not recommended.
- **Found in passing, already in players' hands today:** the menu's "Test entitlements (Complete)" switch (`spell_dev_entitlements` in play_hub.rs:123) is in the App Store binary. A player who taps the logo five times gets the Complete entitlement for free. This is the same threat T-D10 describes. It should get its own fix whatever happens with Cosmos. **Fixed 2026-10-28:** the grant, its menu row and its storage key are now compiled into dev and test builds only. Release builds (TestFlight and the App Store) cut the row and carry no trace of the key, and seam-absence-check fails any store dist that does.

## C4 Progress counters

**There are no profiles.** Storage is raw localStorage with no namespacing. `ProfileId` is fixed at `DEVICE = 0` ("until multi-profile ships"). Kid / Little Speller / Jr is a mode flag (`body.kid`), not a store. So "per profile" means per device. Only Spelluzzle and Forge split their stores by jr/std.

| Counter | Status | Where | Scope | Reset-wiped? |
|---|---|---|---|---|
| Lifetime correct | EXISTS | `spell_themes_v1`, baseline.0 + correct_since. `note_correct` has 18 call sites across modes | device, all languages | no |
| Impossible correct | **MISSING** | Impossible words count as tier "expert" in TierStat. A trace exists in the learner log, which keeps 512 entries per language | — | — |
| Daily completions | derivable | `spell_daily_v1` Record.history (date → correct), `history.len()`. One key for all locales, so at most one completion per day. The Impossible Daily has its own key per language | device | no |
| Current streak | EXISTS | Record.streak / best_streak. **Not decayed on read**: a missed day still shows the old streak | device | no |
| Session count | **MISSING** | Nothing counts finished sessions or rounds | — | — |
| Per-day play history | **MISSING** | Yearbook days_played is derived from Daily history only. `spell_journal_{lang}` covers only the base game, is per language, uses **UTC** dates and rolls off after 730 days | — | — |

**Proposed, not added. Each needs Eric's approval, and none touches Daily, streak, Climb or shield logic:**

1. **`impossible_correct`:** an increment beside `note_correct` on Impossible correct answers. No retroactive baseline exists, so Diamond counts from this build.
2. **`play_days`:** a set of local dates (`daily::today()`), added when any word completes in any mode. Seed it once from the Daily history keys, which are already local dates. This one set serves Aurora, Lunar, Pink Moon and Blood Moon.
3. **`sessions_done`:** Beach needs a definition of "session". The rec: a session is one result screen reached, in any mode.
4. **Daily completions:** derive from `history.len()`. No new counter is needed.

**Two readings for Eric:**

- **Sky's "3-day streak":** read `streak` or `best_streak`? `streak` is never decayed, so the two behave alike. The rec is `best_streak`, as Chalkboard already does.
- **Dates:** the codebase uses two date conventions. Local is used by daily and spelluzzle. UTC epoch-day is used by the journal, calendar, yearbook UI and learner. All new counters must use local dates (T-D13).

## C5 Climb distribution: NO DATA

- The live db is ~/spellgame-server/data/climb.db, opened read-only.
  - `users` has 2 rows.
  - `leaderboard_entries` has 0 rows, and `leaderboard_archive` has 0 rows.
  - All 15 backups also have 0 entries.
- No Climb score has ever been posted, so there are no percentiles.
- **A score** is `chain`: the count of consecutive correct words in one solo run, capped at 500. Boards are split by difficulty (Medium, Hard, Expert) and by locale. Only personal bests are stored.
- **A "season"** is not a time period. It is an archive era created by a manual `archive_active_season()` call, which has never run. "Season scores" therefore do not exist.
- **Kid Mode** never submits to the server. Its only Climb record is the local top-10 board `byear_leaderboard_v1`.
- **For O2, the rec:**
  - Source the Great Wall unlock from the **local** best chain, so Kid and offline players can earn it.
  - Eric picks the number by judgement, since no data exists yet. A best Climb chain of 25 at any difficulty is a reasonable starting point.

## C6 ΔE2000 matrix (I-T3, threshold 20)

**Verdict tokens:**

- Primary: success and caution in both pairs. The dark pair is #46d6b8 / #ff9e57; the light-theme pair is #13806b / #b5540a.
- Also checked: the on-light and on-dark feedback-line variants, and coral (the wrong-answer colour).

The CIEDE2000 code was checked against Sharma pair 1.

| Accent | hex | min ΔE | nearest verdict | I-T3 strict (success/caution) |
|---|---|---|---|---|
| Bronze | #5A2F16 | 17.3 | warn-on-light | pass (22.0); fails only vs the feedback-line amber |
| Silver | #2E3540 | 27.8 | good-on-light | PASS |
| Gold | #5E440C | 18.2 | warn-on-light | pass (22.0); fails only vs the feedback-line amber |
| Diamond | #BFE6FF | 25.2 | success | PASS |
| Sky Sunrise | #F6C08A | **10.7** | caution | **FAIL** |
| Sky Day | #2F7FC1 | 30.5 | success (light) | PASS |
| Sky Sunset | #F2A07A | **7.6** | caution | **FAIL** |
| Sky Night | #8FA8FF | 37.1 | success | PASS |
| Beach | #1E8FA3 | **18.1** | success (light) | **FAIL** (the risk the spec flagged) |
| Lunar | #D8D4C8 | 24.7 | success | PASS |
| Pink Moon | #F2A6C8 | 17.9 | coral-on-dark | pass (32.8); fails only vs coral |
| Aurora | #9B8CFF | 34.4 | coral-on-dark | PASS |
| Great Wall | #C9B79C | **18.6** | caution | **FAIL** |
| Mercury | #B5A89A | 20.7 | caution | PASS (marginal) |
| Venus | #E3C98F | **18.7** | caution | **FAIL** |
| Earth | #4F9BD9 | 31.8 | success (light) | PASS |
| Mars | #D2693F | **11.0** | caution (light) | **FAIL** |
| Jupiter | #D8A877 | **10.2** | caution | **FAIL** |
| Saturn | #E6D3A3 | 20.9 | caution | PASS (marginal) |
| Uranus | #8ED6D8 | **13.1** | success | **FAIL** |
| Neptune | #4C6FE0 | 37.3 | success | PASS |
| Blood Moon | tuned (T-D18) | — | — | no value proposed yet |

**What this means:**

- 8 accents fail strictly. Most are warm sky and planet tones near caution amber. Uranus fails near success teal.
- T-D18 lets each failing hue move into the art and the accent be re-tuned, so no failure is "unpassable" yet. The stop-and-ask fires only if a re-tune cannot pass.
- **Scope question for Eric:** should I-T3 also cover the feedback-line variants and coral, as the existing language-accent test covers both pairs? If yes, Bronze, Gold and Pink Moon also need tuning.
- **Jr:** the spec proposes no Jr accents, and F9's Jr blocks leave `accent` alone. So the Jr results equal the standard ones until Jr accents are defined.

## C7 Impossible status

- Main: `flags::impossible()` defaults OFF.
- Build 303 (release/persian-coming-soon-b303): `impossible` and `impossible_mix` are both ON. They have been on in TestFlight since 286.
- **Question:** what does "Impossible is live" mean for Diamond? The rec: Diamond's visibility follows `flags::impossible()` at runtime, so it tracks whatever build it is in.

## C8 Keycap schema: ABSENT

- design/tokens.json has no keycap colour tokens. It has only key motion tokens (key-flight, key-wave, key-dim, key-pulse).
- index.html has only keyboard geometry vars (`--kb-key-max`, `--kb-cols`).
- Per the spec, the keycap tokens are defined but unused. That includes Diamond's frosted caps, which F3a.5 relies on for readability. Without the schema, Diamond's letters sit on the existing key fill. The fill is solid today, so this is safe but not the intended look.

## C9 Result cards: PARTIAL

- `result_card::live_palette()` reads the active theme from computed style: bg-user, orb core, glow, accent, orb text.
- Text and muted are fixed share-card tokens.
- The card background is one flat colour. Gradients, art, the pavé field and scrims will not appear on cards unless the F11 renderer learns to draw an art layer.
- That is a change to F11, which the spec says Cosmos "reads, never overrides". AT-T17 and F3a.7 need Eric to allow a renderer extension, or else to accept flat-colour cards.

## C10 Web: no stop

- themes.rs is compiled into the site with no web cfg gate.
- `visual.themes` is OFF on the site.
- Unlock storage is localStorage only. No backend route touches themes or progress, and the server has no child-profile table.
- Parity therefore needs no server-side storage, and the CC-SECURITY-BASELINE D2 stop does not fire.

## Not checked in §0 (Phase A work)

- The ephemeris and eclipse source and its licence (T-D12).
- The oldest CI device for the I-T13 60 fps check (memory says iPhone SE 2nd gen is the AT-19 device).

## Decisions needed before Phase A

1. **C3:** where the Lab build lives. The rec is (a), an internal-only dev_preview TestFlight build with a submit guard.
2. **I-T2:** the reading "per-lightness verdict pair".
3. **I-T3 scope:** success/caution only, or also the feedback-line variants and coral.
4. **C4:**
   - approve the three counters, `impossible_correct`, `play_days` (seeded from Daily history) and `sessions_done`;
   - define "session";
   - choose `streak` or `best_streak` for Sky.
5. **C7:** Diamond follows `flags::impossible()`.
6. **C9:** extend the F11 renderer for art, or accept flat cards.
7. **O2:** a Great Wall chain number with no data, sourced from the local best.
8. **Separate:** the "Test entitlements" switch ships to players. Fix it now?
