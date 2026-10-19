# CC-IMPOSSIBLE-LAUNCH v1: section 0 recon (R1-R9)

Run 2026-10-19 against origin/main 0a7471df. Read-only: no source, bank, flag, server or key was changed, and the private key's contents were never read (path only). I treated the pasted spec as signed for this read-only step only; Phases 1-4 and Track V have not started.

## Contradictions with the spec (read these first)

| # | Spec says | Repo and decisions say | Needs |
|---|---|---|---|
| C1 | Feature 7 turns ON "the Climb x1.5 multiplier"; AT-7.4 tests it | Eric ruled 2026-10-18 **no Climb effect**; F9 and Phase E were dropped, and the Climb code is untouched. It scores a chain length, not points. There is no flag for it | Strike the multiplier and AT-7.4, or re-open the ruling. I stopped here, per the file's own rule |
| C2 | L-D7: the private key never goes on the server | The key is at `~/spellgame-server/keys/kill-flags.pem`, inside the server's own folder on the **same Mac that runs the server** (the folder holds deploy.sh, the Caddyfile and the tunnel config). It is not in the repo, the docker volumes or the web root. Signing today happens on that machine | Decide what "not on the server" means when the server and Eric's Mac are one machine (see R7) |
| C3 | Phase 1 treats the key as backed up once Eric stores a copy | `bin/backup.sh` backs up exactly three things (`data/pack-signing.pem`, `climb.db`, the clip cache) and **not** the kill key. So it has no copy anywhere, not even on the same disk. My earlier note that it had a same-disk backup was wrong | Eric's manual offline copy is the only copy until made |
| C4 | Feature 7: Impossible Daily "with its own board per language" | Eric ruled 2026-10-19: no leaderboard, no Climb account. Built: a local Impossible Daily with its own per-language streak and history | Reword to "its own record per language" |
| C5 | L-D3 and AT-4.1: every Impossible attempt stored `tier = impossible` | Today Impossible attempts are stored as **expert** (R6). Only Misses carry the `impossible` tag | Build item (Feature 4) |
| C6 | AT-2.3: the client "logs" an attempt to turn a flag on | A document has no way to say "on": those fields are never read. There is no log, and no telemetry may be added (I12 of the Complete file) | Change AT-2.3 to "ignored", or allow a console line |
| C7 | F2 says "within one poll interval" | There is no polling: the file is fetched once at app start (R7) | Define a poll interval, or accept "next launch" |
| C8 | R4 gate: English re-tier "passed its promotion audit" | I found no promotion record (R4) | Feature 6 is blocked until one exists |

## R1. Flags

All defaults are in `src/flags.rs`, read from `localStorage["spell_flag_<name>"]`, compiled-in default shown. "Turning a flag on in a build" means flipping that default in code (the server file can only kill).

| Purpose | Flag name | Build default |
|---|---|---|
| English re-tier | `en_retier` | off |
| Impossible, master | `impossible` | off |
| Impossible, per language | `impossible_<lang>` (en, es, fr, de, pt, pl, vi, ko, ja, fil, zh, ru, ar, sw, hi) | off |
| Mix | `impossible_mix` | off |
| Impossible Daily | **no flag of its own.** It follows `impossible` + `impossible_<lang>` and the level picker | n/a |
| Climb x1.5 | **does not exist** (see C1) | n/a |
| BoardGame Impossible trap | **no flag of its own.** It follows the language's Impossible availability | n/a |
| F10 drawn entry | `quickbee_draw_zh`, `quickbee_draw_ja`, `quickbee_draw_ko` (key `quickbee_draw_<lang>`) | off |
| German in the trap | **not a flag.** Code rule: `engine::needs_shift("de")` keeps German out until the board keyboard has a Shift key | hard-coded off |

So AT-7.1 ("generate the flag table from code") has to generate it from these: the Daily, the trap and the German exclusion cannot be switched individually.

## R2. Launch languages (reports/impossible-availability.md, after Phase B)

Pool gate result per language (entitlement, flags and profile are run-time and not part of this table):

| Lang | Result | Reason code | Pool |
|---|---|---|---|
| en | PASS | (opens only with en.retier on) | 271 |
| es | PASS | | 2,995 |
| ru | PASS | | 2,997 |
| fr | PASS | | 1,826 |
| de | PASS | (trap falls back to Double Expert) | 2,964 |
| pt | PASS | | 2,625 |
| pl | PASS | | 2,932 |
| ko | PASS | | 1,399 |
| fil | PASS | | 1,927 |
| sw | PASS | | 679 |
| vi | FAIL | language_locked:no_exclusion_gate | 0 |
| ar | FAIL | language_locked:no_exclusion_gate | 0 |
| hi | FAIL | language_locked:no_exclusion_gate | 0 |
| ja | FAIL | language_locked:no_drawn_entry | 0 |
| zh | FAIL | language_locked:no_drawn_entry | 0 |

With the spec's exclusions the TestFlight 2 set is en, es, ru plus fr, de, pt, pl, ko, fil, sw. The launch condition (English plus one other) holds on paper; en depends on `en.retier`. These pools were built before the quarantine in Feature 3, so they will move.

## R3. Flag dependencies

- `impossible_en` needing `en_retier` is enforced in one place, as an availability rule, not as a flag check: `src/impossible.rs` `lang_availability`: `if lang == "en" && !i.en_retier { return Availability::Hidden }`. The pool artifact is built from the re-tier Expert tier, so it is also stale-checked against that tier's hash.
- If `en_retier` is killed while `impossible_en` stays on: English returns `Hidden` (no entry in the picker), is filtered out of Mix's qualifying list, and a serve that already had the Impossible level selected falls back to plain Expert (`impossible_langs` returns None). **Hidden carries no reason code**, so AT-2.5's "English disappears from Mix with the right reason code" cannot be met as written: it disappears, silently. Mix itself only shows `needs_two_languages` if that drops it under two languages.
- A Misses entry tagged `impossible` for an English re-tier word still replays after `en_retier` is killed (the review path does not check availability). Harmless, but the word may no longer be in the English bank.
- The kill file works on any flag name, so killing `en_retier` also reverts English tiers for everyone who was on the re-tier.

## R4. en.retier promotion gate: OPEN

- Evidence for: `reports/en-retier-plan.md` records tests A8-A11 and the partition test passing (median score Easy 0.305 < Medium 0.394 < Hard 0.436 < Expert 0.694), the full gate passing (e2e 422/422 app and 17/17 site, at that time), and PURITY passing on the 1,210 new rows.
- Evidence against / missing: the CC-EN-EXPERT-GROWTH spec is not in the repo, and I found **no record of the F7 promotion audit** (R8 sample plus your spot check) in `reports/` or `docs/`. The plan itself says "no tiers changed yet" and lists the flag as off.
- Also open, from the same plan: TTS audio for about 1,200 new words was unverified when billing was off (billing has since come back; I have not checked those clips).
- Verdict: UNKNOWN to me, **treat as open** until you point me at the record. Per the file, Feature 6 and so the launch wait on this.

## R5. The 41 rows

Source: `reports/impossible-bank-hygiene-from-loopback.csv` (41 rows, all in the **Expert** tier of their language).

| Lang | Rows | In a launch pool today | Notes |
|---|---|---|---|
| fr | 6 | 0 | apparement (misspelling), masson (surname), sin, statu, lisée, telecom (non-headwords or unaccented) |
| de | 1 | 1 | Wales (proper noun) |
| pt | 9 | 6 | adi, cês (clipping), fev (abbreviation), palace, portuguese, temple (English words); objectivo, utilizá, visitá not in the pool |
| pl | 2 | 2 | acta (Latin/English), ny (not a headword) |
| fil | 12 | 4 | inuming, kadalasang, kailangang, simbahang (linker forms) are in the pool; the other 8 are not |
| ar | 11 | 0 | proper nouns, dialect words, transliterations, missing-hamza spellings. Arabic is RTL-blocked, so no mode serves it today |
| **Total** | **41** | **13** | |

By the CSV's own wording the reasons are: 1 misspelling, 4 proper nouns, 2 abbreviations/clippings (pt cês, fev), the rest non-headwords, English words, dialect words, nonstandard spellings or linker forms. The spec's three-way split (misspelling / proper noun / abbreviation) does not cover most of them.

Other modes that serve them: every mode reads the same bank through `words::tier_for`, so each row is served wherever its language's Expert tier is: Normal Expert, the Daily's expert slots, the Climb's expert band, the BoardGame Expert tier and Long Word band, Spell Search, Spelluzzle, word-based modes, and Impossible for the 13 above. The other 28 are already out of the Impossible pools (rules net or reviewed KEEP) but are still served at Expert.

Mechanism (the Russian precedent, commit b24fcf58): a quarantine touches the bank file, `assets/words-quarantine/<lang>/<tier>.txt`, `assets/words-quarantine/manifest.csv`, `tools/bank/purity_overrides.csv`, `assets/words/purity/<lang>.tsv`, `src/word_data.rs` (tier hashes), the language's sound-alike table and the shared sound-alike report, plus any pinned digest (Spell Search's grid digest was re-pinned in host and e2e). Expect AT-3.4's "bank diff contains only these 41 moves" to hold for the bank but not for the generated and pinned files around it. Quarantine rows already restorable only by a signed exception (`scripts/build-bigbank.py` never re-admits them).

## R6. Attempt storage

- `game::finalize_*` and the correct path call `stats::record(state, lang, tier, correct)`, which stores counts per **language and tier** (`stats[lang][tier] = {seen, correct, tone_partial}`) under the stats key. For an Impossible word `cur_tier` is `"expert"`, so **Impossible attempts are stored as expert today**.
- Misses: `record_miss_routed` files the miss under tier `"impossible"` (the Mix spec M9 tag), so misses are the only place the tag exists.
- `wordstats::record(lang, word, correct)` and `learner::note_attempt*` take no tier. The BoardGame ledger records trap words under the `expert` key. The Impossible Daily keeps its own record (`spell_daily_imp_v1_<lang>`), which holds a per-day correct count and no tier.
- Readers: `stats::render` and the achievements iterate the known tiers. Writing a new `impossible` tier key will drop those attempts from the displayed totals unless the stats screen sums it into Expert, so AT-4.3 needs an explicit read-side change, not just a write.

## R7. Kill switch

- **Fetch:** once per app start (`killflags::refresh`, spawned from `start()`), `GET {api_base}/api/kill-flags`. There is **no poll interval**; a long-running session never refetches.
- **Unreachable / 204 / error:** nothing changes; the cached, re-verified document (localStorage `spell_kill_flags_v1`) and the build defaults stay in force. No UI, no crash (tested).
- **Verification:** Ed25519 (`ed25519-dalek`, verify-only) over `spell-kill-v1\n<issued>\n<sorted names>`. A document older than the cached one is refused. Unknown fields, including any "on" list, are never read.
- **Public key:** `PUBLIC_KEY_HEX` in `src/killflags.rs`.
- **Private key:** `~/spellgame-server/keys/kill-flags.pem` (path only; mode 600, directory 700). Server-side route `/api/kill-flags` is written in `backend/app.py` but **not deployed**.
- See C2 and C3: that path is on the machine that serves production, and no backup contains it.

## R8. Vietnamese audio

- **Are the growth rows live?** Yes, in every app build. All 2,500 Vietnamese Expert rows, 1,499 of 1,500 Hard rows and 419 of 833 Medium rows contain a space. They have been in the bank since before the Sept cut of 71 non-words (commit dbbfd081). The site is English only (`web` feature), so spellgame.net does not serve them; the app, including TestFlight, does.
- **Validation that rejects spaces:** `backend/app.py` `validate_word` accepts only characters whose Unicode category starts with L or M, plus `-`, `'`, `’` (`_EXTRA_WORD_CHARS`). Space is category Zs, so `GET /api/speak?word=rượt%20đuổi&lang=vi` returns **400 invalid word** (checked live: the same word joined without a space returns 200).
- **What the client does:** `api::speak_url` sends the word raw. Whether the app then falls back to on-device speech (a `NativeTts` source exists in the play order) or stays silent I did not resolve. UNKNOWN: it needs a device check.
- **Cache keys:** `cache_path_for` hashes `"{lang}:{word}"` (plus an IPA suffix when a lexicon entry exists, plus the voice for en, de, ru, ko). "a b" and "ab" already hash differently, so AT-8.3 holds by construction. Voice-in-key languages mean a changed word text only creates new clips; nothing else moves.
- **Other languages' punctuation (all tiers):** Filipino has 1 hyphenated row in Easy, 12 in Hard and 54 in Expert; the endpoint accepts hyphens, so they are served. No other language has a space, apostrophe or hyphen in any tier. So Feature 8 changes behaviour for Vietnamese only, and the "row types R8 flagged" clause is empty today.
- My loopback runs sent Vietnamese with hyphens in place of spaces, which the endpoint accepts but is not what the app sends.

## R9. A-15 procedure

The Complete spec's A-15 ("stability tests f821855c ... 10 runs in a row") names three groups in `tests/e2e/run.mjs`. From a gate-ready checkout (node_modules present, `bash scripts/build-web-test.sh` run):

```
for i in 1 2 3 4 5 6 7 8 9 10; do node tests/e2e/run.mjs spelldoku  || break; done
for i in 1 2 3 4 5 6 7 8 9 10; do node tests/e2e/run.mjs boardgame  || break; done
for i in 1 2 3 4 5 6 7 8 9 10; do node tests/e2e/run.mjs spelluzzle || break; done
```

Groups: `spelldoku` (spelldoku + spelldoku-tier), `boardgame` (7 specs), `spelluzzle` (6 specs). Commit f821855c touched the three specs that were flaky (`boardgame-boost`, `spelldoku`, `spelluzzle`). Caveats: `scripts/gate.sh` is the one gate battery and it also runs cargo tests; a worktree needs `node_modules` linked first; the e2e-sleep-ratchet forbids new `waitForTimeout`. Each group takes several minutes, so the 30 runs are a long job. UNKNOWN: whether "clean" means zero failing tests per run (assumed).

## Addendum 2026-10-19: decisions and a second look at R4

**Decisions (Eric):** the Climb x1.5 multiplier is struck from Feature 7 (and AT-7.4 with it). The server-side key work is a work in progress and is deferred, so C2 and C3 stay open as written. The other contradictions stand as listed.

**R4, second look.** What exists, with dates:

- 2026-10-16: Eric ruled on the 200-row staged-word sample (`reports/en-staged-review-sample-graded.csv`, ruling recorded in `reports/en-staged-definition-review-2026-10-16.md`): keep all 200, agree with the estimated tiers. Keep rate 200/200 meets the 95% bar for admitting staged words. The CSV's own verdict column is still blank (the ruling lives in the report and the commit message, 2b564f86). This is the closest thing to an "R8 sample" in the repo.
- 2026-10-16 to 10-18: about 2,700 English words graded and used to build the re-tier. The calibration reports say the grades began as model suggestions that Eric chose to record as his, so they measure the suggestion model more than a blind human grader.
- 2026-10-17/18: the re-tier bank was built behind `en.retier` with tests A8-A11 and the partition test passing and the full gate passing (`reports/en-retier-plan.md`).

What I still cannot find: a record that the **built re-tier bank** (Easy 825, Medium 1,753, Hard 1,310, Expert 279) was spot-checked and promoted, and any check that the audio for the roughly 1,200 new words exists and sounds right. The word-admission ruling covers whether the words are real; it does not cover whether each sits in the right tier of the final bank. So R4 is **partly met**: admission yes, final-bank spot check and audio no.
