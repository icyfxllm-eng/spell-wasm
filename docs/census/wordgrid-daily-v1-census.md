# CC-WORDGRID-DAILY v1 — §0 census (read-only)

Ran 2026-09-22 against main at ab47295c (live build 239). Read-only. The C2
timings come from a temporary in-crate bench that was reverted, not committed.

**No HALT fired.** The headline: the shared server Daily that D-W1 retires was
specified but never built — both Dailies already generate on device. What makes
them shared is a date-derived seed, not a server.

## C1 — Current Daily path

| | Spell Search | Spell Cross |
|---|---|---|
| Entry | `wordsearch_ui.rs:198` | `wordcross_ui.rs:160` |
| Generator | `wordsearch::serve::daily(lang, tier, ymd, day)` | `wordcross::serve::daily(lang, tier, ymd, day)` |
| Words | `ledger::daily_pick(daily_pool(lang, tier), …)` | same, plus 6 spare words |
| Seed | `seed("daily:{lang}:{tier}", 0, ymd)` | same shape |
| Server | none | none |
| Cache | none | none |
| "Played today" gate | none | none |

`daily_pick` shuffles the pool by a fixed salt, cuts it into date blocks so a
word cannot return inside 90 days, then shuffles that day's block by `day`.
Everything is a pure function of (language, tier, date), so every device
computes the same puzzle without ever asking a server.

Both Dailies drop a word at a time rather than fail: Search retries without one
word (`serve.rs:85`), Cross draws 6 spare and redraws.

## C2 — On-device generation cost

200 Daily generations per cell, release build, this Mac (Apple silicon,
native — **not** wasm, and **not** the oldest supported iPhone):

| Mode | en p50 / p95 | es p50 / p95 | ru p50 / p95 |
|---|---|---|---|
| Spell Search Easy | 10.4 / 19.5 ms | 16.8 / 19.0 ms | 27.4 / 47.8 ms |
| Spell Search Medium | 13.5 / 27.6 ms | 17.4 / 20.5 ms | 28.7 / **79.5** ms |
| Spell Search Hard | 11.8 / 22.2 ms | 18.1 / 19.7 ms | 30.2 / 76.9 ms |
| Spell Cross Easy | 34.9 / 52.5 ms | 28.7 / 33.1 ms | 39.6 / 45.8 ms |
| Spell Cross Medium | 38.6 / 54.5 ms | 30.1 / 36.2 ms | 40.9 / 47.5 ms |
| Spell Cross Hard | 38.9 / 58.5 ms | 31.0 / 37.9 ms | 40.6 / 47.4 ms |

Worst p95 is 79.5 ms, roughly 19x under the 1.5 s threshold. Even allowing an
order of magnitude for wasm on an old phone, both modes clear it, so **D-W2
resolves to F1 (on-device) for both** — with the honest caveat that the census
asks for the oldest supported iPhone and this is a Mac. Closing that properly
needs a timing hook in a TestFlight build or a physical device; the margin is
wide enough that F2's server pool looks like work nobody needs.

**Incidental finding, worth a decision:** Spell Search en Medium returned no
puzzle on 3 of 200 dates (1.5%). Under today's shared Daily those dates simply
have no Spell Search Daily for English Medium. Under F1 a fresh seed per play
makes it self-healing — the next attempt draws differently.

Spell Cross produced zero puzzles under 5 interlocks in 1,800 attempts across
all three languages and tiers, so F5's redraw path is insurance, not a fix for
something currently failing.

## C3 — Shared-puzzle dependents

**None.** No leaderboard, no score comparison, no "X players solved today", no
share text, no push copy. Nothing to re-scope and nothing to break.

## C4 — Daily word source

The audited bank only. `lexicon::daily_pool(lang, tier)` unions the bank pools
across Jr/Easy/Medium/Hard/Expert and filters to the tier's maximum word
length. My Words never reaches a Daily. D-W3's default already holds in code.

## C5 — Web

No web Daily exists. All three modes are `"platforms": ["ios"]` in
`config/modes.json`; `build.rs` strips non-web modes from the site build, and
the gate's web-wall scan proves there is no trace of them in `dist`. D-W5 is
moot unless spellgame.net gains these modes.

---

## What Phase B actually needed, 2026-09-22

Implementing F1+F3 hit a conflict the census did not predict, and it is worth
recording because it changed the shape of the fix.

**The date block and per-play freshness cannot both hold.** The old Daily
picked its words from a block chosen by the date — that block schedule is
exactly what guaranteed a word could not return for 90 days (D9). Making the
Daily fresh per play (D-W1) while keeping the block means every replay draws
from the SAME small block, so two plays on one date shared 3 of 7 words and
blew F3's 25% cap. Measured, not guessed: the first attempt put a per-play
nonce into the word draw and the gap test failed with "day 19 repeats day 0
inside the 92-day gap", because salting the pool's global shuffle destroys the
partition the blocks are cut from.

**Resolution, and it is what D-W4 already signed.** A personal Daily is a
personal draw: both modes now serve the Daily through `serve::bank` at the
Daily's tier, using the player's own ledger — the same path every non-Daily
puzzle has always used. That gives fresh-per-player and fresh-per-play for
free, keeps it offline, and rotates words by the ledger's window. D-W4 says the
Daily's word guarantee is the 25% overlap cap rather than D9's 90 days, so the
weaker-but-personal guarantee is the one that was asked for. The F3 cap is
enforced on top, as a preference across attempts that relaxes rather than
fails.

**Removed:** `wordsearch::serve::daily` and `wordcross::serve::daily`, the two
date-seeded entry points, so the retired design cannot be served by accident
(D-W1: "Stop and ask if any code or test encodes the old reading"). The block
helpers stay — `daily_gap` still measures whether the bank is deep enough, and
that is worth holding regardless of who draws from it.

**Also settled here:** the streak. D-W6 matches SpellDoku's D-R11, which Eric
dropped on 2026-09-22 because SpellDoku has no streak to count toward. Neither
does Spell Cross or Spell Search, so F4's streak clause is dropped for the same
reason and the Daily simply has no gate: there was never an "already played
today" check to remove (census C1).
