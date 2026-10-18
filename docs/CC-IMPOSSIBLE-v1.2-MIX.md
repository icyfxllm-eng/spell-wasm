# CC-IMPOSSIBLE v1.2 amendment: Impossible Mix (all languages in one bank)

**Status:** REVIEW-GATED draft, written 2026-10-16 from Eric's choice "2" (mixed bank, per-language pools underneath). Renamed from v1.1 on 2026-10-17: v1.1 is the sound-alike filter pointer (`docs/CC-IMPOSSIBLE-v1.1-SOUNDALIKE.md`), which this file builds on.
**Amends:** CC-IMPOSSIBLE v1 and v1.1. Section 10 (2026-10-17) replaces v1's top-slice pool with 100% of Expert; nothing else in v1's rules or invariants is removed.
**Sequencing:** after CC-IMPOSSIBLE v1 Phase B (the Normal-surface tier) and before its Phases C and D. Not before build 56.
**Flags:** every new flag defaults OFF.

---

## 0. Intent

Impossible Mix is one Impossible bank that draws from every language the player is entitled to and that has an available Impossible pool. Whatever language the next word is in, the audio, the keyboard and the reading direction switch to that language.

It is for multilingual players who want one run that tests everything they have unlocked. **The player cannot narrow the mix** (Eric, 2026-10-16: letting players pick languages would dilute the difficulty, and the surprise is the point).

This knowingly relaxes v1 section 0's "solvable from the audio alone" for players who do not know every language they have unlocked. The mitigations are in M3: entry is gated by entitlement, the player is told what is in the mix before they start, and they can leave at any time.

---

## 1. Scope and non-goals

**In scope:** a mixed draw over the v1 per-language Impossible pools; per-word language, keyboard and direction switching; a pre-run notice, with no chooser; locked states; flags; badges.

**Non-goals:**
- No new words, no new pool rules. A mixed bank is the union of v1's per-language pools and nothing else.
- No change to any per-language pool, slice, homophone filter, or availability gate.
- No change to Expert-tier membership, definitions, or entitlements.
- No new input method and no new telemetry events (reuse the existing ones with a mix tag).
- Never in Kid Mode, Little Speller, Spell Jr or Randospell (v1 I3 applies).

---

## 2. Standing rules carried

- v1 section 2 in full (single registry, no silent fallbacks, NFC, bank purity, shared no-repeat history, signed Vietnamese D2, stop-and-ask on conflicts).
- Mix reads the **same registry result** as v1 F3. UI code reads availability from the registry only.

---

## 3. Features

### M1. Pool = union of available per-language pools

- The mix pool is the union of `impossible_pool.<lang>.json` for every language that is (a) available under v1 F3 (pool >= 150, not zh or ja, release gates open, `impossible.<lang>` ON), (b) entitled for this profile (v1 F11), and (c) has at least one pool word not yet served this cycle.
- A language contributes only its own pool. No word is re-tiered, re-scored or re-filtered here.
- The mix pool is computed at run time from the artifacts. It adds no generated artifact and no bank rows.

### M2. Per-word language switching

Each word carries its language. Before the audio plays:
- the keyboard rebuilds for that language,
- the text direction is set from the registry (a Spanish word followed by an Arabic word flips direction),
- the TTS voice is that language's voice,
- the grading rules are that language's (NFC equality, Vietnamese tone placement, German case, and so on).

The interface language does not change. Precedent in the code: Misses already replays words across languages by setting `cur_lang` per word and rebuilding the keyboard (`src/game.rs` around line 380, `src/keyboard.rs`). M2 reuses that path and adds nothing parallel.

### M3. No chooser; notice, entitlement and exit

- There is **no language chooser** and no setting that narrows the mix. Every language that passes M1 is in.
- Before the first Mix run (and again when a newly eligible language joins the mix) the player sees one plain notice listing the languages in it, e.g. "Impossible Mix: words from Spanish, English, ... Any of them can come up next." They tap to continue or to cancel. This informs; it does not let them choose.
- Entitlement is the gate. A language enters the mix only if the profile is entitled to it (English free; others need SpellGame Complete or a regional grant, v1 F11), so the mix grows with what the player has unlocked.
- Mix needs at least 2 qualifying languages to start; otherwise it shows locked with "Needs two Impossible languages".
- There is an exit at every point, including the end screen (standing rule: every mode has an exit).

### M4. Draw and no-repeat

- Draw a language uniformly from the qualifying set, then draw the next word from that language's v1 no-repeat cycle (v1 F5, per profile and language).
- A language whose cycle is exhausted resets on its own, as in v1, with v1's toast.
- When every qualifying language's cycle has just reset in the same run, show one toast: "You've heard every Impossible word. Starting a new round."

### M5. Rules

Unchanged from v1 F4: initial play plus 1 replay, one attempt per word, no hints, strict grading. Replay and attempt counts reset per word.

### M6. Surfaces

| Surface | Mix? |
|---|---|
| Normal / standard practice | Yes, as an option under the Impossible picker |
| Impossible Daily, BoardGame trap, The Climb | No in v1.1 (open O3) |
| Kid Mode, Little Speller, Spell Jr, Randospell | Never |

### M7. Locked states

Locked means visible but not selectable, with a one-line reason: "Needs two Impossible languages" (fewer than two languages pass M1 for this profile), "or v1's existing reasons.

### M8. Badges and flags

- Cosmetic badges only: first Mix word, a Mix session cleared with no misses. Per profile, not per language. No unlock, shield or entitlement references Mix (v1 I4 applies).
- New flag: `impossible.mix` (OFF). It requires the master `impossible` flag and at least two `impossible.<lang>` flags ON.

### M9. Misses

A missed Mix word enters Misses tagged `impossible` and its language, and is re-served at Impossible rules in its own language (v1 D13).

---

## 4. Decisions

### Signed

| # | Decision |
|---|---|
| X1 | Impossible Mix exists as a single mixed-language bank, as Eric proposed. The per-language pools underneath (option 2, 2026-10-16). |
| X2a | No language chooser (Eric, 2026-10-16): letting players pick would saturate the "impossible" theme. This reverses the draft's fairness gate. |

### Defaults applied (unsigned; Eric can object)

| # | Default |
|---|---|
| X2 | No language chooser: every qualifying language is in the mix (M3). Mitigations: notice, entitlement gate, exit. |
| X3 | Draw: uniform over qualifying languages, then v1's cycle within a language (M4). Not weighted by pool size. |
| X4 | Mix starts only with at least 2 qualifying languages. |
| X5 | Mix is Normal-surface only in v1.1 (M6). |
| X6 | RTL and LTR languages may mix in one run; direction flips per word (M2). |
| X7 | Badges are per profile (M8). |

### Open

| # | Question | Build default |
|---|---|---|
| O1 | Show the pre-run notice every run, or only the first run and when the language set changes? | First run and on change |
| O2 | Should the mix be weighted so a small pool is not exhausted first? | Uniform (X3) |
| O3 | Should the Impossible Daily get a Mix variant later? | No |
| O4 | Does Mix need its own score or leaderboard? | No |

---

## 5. Invariants

- **M-I1** The Mix pool is a subset of the union of v1 per-language Impossible pools, and every word in it comes from a language the profile is entitled to.
- **M-I2** On every word, the keyboard, direction and voice match the word's language.
- **M-I3** Mix is unreachable from Kid Mode, Little Speller, Spell Jr and Randospell (registry-enforced).
- **M-I4** No Mix word is served from a language that is unavailable or unentitled.
- **M-I7** The player has no control that removes a qualifying language from the mix.
- **M-I5** Every v1 invariant I1 to I10 still holds per language.
- **M-I6** No non-cosmetic unlock, achievement, shield or entitlement depends on Mix.

---

## 6. Phases

| Phase | Work | Exit gate |
|---|---|---|
| M-A | Registry entry for Mix, M1 union, M3 notice and entitlement gate, M7 locked states, `impossible.mix` flag | Tests X1 to X4 green |
| M-B | M2 per-word switching, M4 draw and cycles, M5 rules, M9 misses | Tests X5 to X8 green; Maestro run across 3 languages including one RTL |

---

## 7. Acceptance tests

| # | Test | Asserts |
|---|---|---|
| X1 | `mix_pool_is_union` | The Mix pool equals the union of the qualifying languages' v1 pools; no row outside them (M-I1) |
| X2 | `mix_unavailable_languages_excluded` | A language with its flag OFF, unentitled, closed (Arabic RTL gate) or below 150 never appears (M-I4) |
| X3 | `mix_no_chooser_notice` | No UI narrows the languages (M-I7); the pre-run notice lists exactly the qualifying languages; fewer than 2 qualifying shows locked with "Needs two Impossible languages" |
| X4 | `mix_kid_surfaces` | Kid Mode, Little Speller, Spell Jr and Randospell show no Mix entry (M-I3) |
| X5 | `mix_keyboard_follows_word` | Maestro: across a 3-language run, the keyboard, voice and direction match each word's language on every word (M-I2) |
| X6 | `mix_rules` | The F4 rules hold per word: 1 replay, 1 attempt, accent-stripped answers wrong |
| X7 | `mix_no_repeat` | A fixture with 2 qualifying languages x 150-word pools never repeats a word within a language's cycle and fires the reset toast once per exhausted cycle (M4) |
| X8 | `mix_misses_language` | A missed Mix word is re-served in its own language at Impossible rules (M9) |

---

## 8. Stop-and-ask triggers

- The answer surface cannot rebuild the keyboard per word without a rewrite (would be a new-input-method change).
- Any need to change a per-language pool, an entitlement or a release gate.
- Fewer than two languages ever qualify under v1 F3, which would leave Mix permanently locked. Report this before building it.
- Any conflict with a signed CC file that implies a strategy reversal.

## 9. Do not re-propose

- A language chooser, or any control that lets the player narrow the mix (rejected by Eric: it dilutes the difficulty).
- Padding the mix with Expert words that are not in a v1 per-language pool.
- A separate Mix leaderboard or non-cosmetic rewards.

---

## 10. Amendment 2026-10-17: the pool is 100% of Expert (Eric)

Signed in chat: Mix takes **all** of each language's Expert tier, not a top slice.

### P1. Pool
A language's Impossible pool is every row of its live Expert tier that passes, in order:
1. PURITY (v1 section 2, unchanged).
2. The v1.1 sound-alike filter: drop every member of every `kind: true` or `kind: accent` set, read through `soundalikes(lang, voice)` (v1.1 A1, A2).
3. The proper-noun assertion (v1 F2 step 6).

No ranking and no slice. v1 F2 steps 2 and 3, D1 and I6 (the 5-20% slice and its bounds) **do not apply to Mix**. Since no per-language difficulty score is needed any more, the census C2 gap (no score for ru, ar, hi, sw, zh) no longer blocks a language.

### P2. English
English contributes only from the **re-tiered** Expert tier (`EN_RETIER_EXPERT`, 279 rows, graded by Eric), and only while `en.retier` is on. With the flag off, English is not in the Mix: today's 797-row Expert tier is mostly common long words and must not be served as Impossible (v1 I5).

### P3. Per-language floor
A language enters the Mix only if its filtered pool holds **at least 150 rows** (v1 D3's number, kept). With 100% of Expert every language that has a table clears it easily, so the floor only guards against a thin or broken bank.

### P4. Availability, unchanged from v1 and v1.1
A language must also: have a sound-alike table that is not `missing` (v1.1 A3); have its `impossible.<lang>` flag on; be entitled for the profile (v1 F11); and pass its own release gates (v1 F3). Mix still needs at least 2 qualifying languages (section 4, X4).

### P5. What qualifies today
Approximate filtered pool sizes, measured 2026-10-17:

| Language | Expert rows | After the sound-alike filter | Status |
|---|---|---|---|
| English (re-tier) | 279 | about 271 | table `missing`; `en.retier` off |
| Spanish | 3,000 | about 2,994 | table `missing` |
| Russian | 3,000 | about 2,997 | table `missing`; its key only models devoicing and assimilation, so it finds fewer sound-alikes than exist |
| all others | | | no sound-alike table data yet |

**Nothing qualifies yet.** Every table is still `missing`, because the SENSE-CUE v1.1 F5 loopback (Phase B) has not run, and v1.1 A3 keeps Impossible locked on a `missing` table. Mix unlocks once at least two of English, Spanish and Russian reach `verified` (and, for English, `en.retier` is on).

### P6. Tests (replace v1 A1, A4 and A2 for Mix)
- **X9** `mix_pool_is_all_expert`: every filtered Expert row of a qualifying language is in its pool; nothing outside its Expert tier is.
- **X10** `mix_english_needs_retier`: with `en.retier` off, English contributes nothing; with it on, English's pool is drawn from `EN_RETIER_EXPERT` only.
- **X11** `mix_floor`: a fixture language with 149 filtered rows is left out; 150 is in.

### P7. Status, 2026-10-18: Phase M-A built (flags off)
SENSE-CUE Phase B made the en, es and ru tables `verified`, so P5's "nothing qualifies" no longer holds. Built pools (`assets/impossible/impossible_pool.<lang>.json`, `tools/impossible/build_pools.py`): English (re-tier) 271, Spanish 2,995, Russian 2,997. Every other language is locked: zh and ja by D4, the rest by a `missing` table. Spanish and Russian need Full access (SpellGame Complete or a regional grant), so on the free tier Mix shows "Needs two Impossible languages" even with every flag on. The registry is `src/impossible.rs`; the picker entry, the notice's strings and play arrive with M-B. See `reports/impossible-phase-a.md`.
