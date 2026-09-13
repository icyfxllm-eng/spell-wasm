# CC-ONBOARD-JR — Step 0 inventory

Branch `cc-onboard-jr` off `e89d0b4d`. Measured 2026-09-10. **No code written.**
Every number below was read from the tree, the live server, or the live
database — not inferred. Two of the file's stop-and-ask triggers fired; they
are §0.

---

## §0 Stop-and-ask — two triggers fired, two did not

| Trigger | Result |
|---|---|
| Password hashing weaker than argon2id/bcrypt/scrypt | **Not fired.** bcrypt (`backend/auth.py:48`). |
| Codes not rate-limited | **Not fired**, with a caveat — see §4. |
| Spell Jr / Kid Mode / Little Speller are separate systems with conflicting rules | **FIRED.** One system, but its own rules contradict. §0.1. |
| A mode whose Jr policy is not obvious from F2's defaults | **FIRED.** Seven modes have no F2 default, and F2's three named targets are not registry entries. §0.2. |

### §0.1 One system, two contradictory tier ceilings

There is exactly **one** runtime system: `AppState.kid: bool`. "Little Speller"
is not a second system — it survives only in comments and as
`GameMode::LittleSpeller` in `src/entitlements.rs:83`, an enum variant that is
**never constructed** (it appears only in a match arm and a test loop). Dead.

But that one flag carries **two tier ceilings that disagree**:

| Surface | Kid ceiling today | Source |
|---|---|---|
| Base game | **Hard** — Expert is swapped for Hard, Hard is kept | `game.rs:173`, `:1309` ("Kid Mode caps at Hard") |
| Standard Climb | **Hard** — bands run to Expert, clamped to Hard | `game.rs:216` `band_to_tier` + the cap above |
| Word Bee | Medium | `bee.rs:37` `KID_CEILING`, commented "matching the standing kid tier cap" |
| Definition Match | Medium | `defmatch.rs:166` |
| Spell Picture | Medium | `wordpic.rs:412` `kid_ok` |
| Daily Challenge | Medium | `daily.rs:18` `KID_ARC` = easy 5 + medium 5 |
| Impostor | has its own rule | `impostor.rs:105` `effective_tier(t, kid)` |

Bee's comment calls Medium "the standing kid tier cap"; the base game's comment
calls Hard the cap. **Both claim to be the rule.**

C3 resolves the direction (Jr = Easy + Medium, no Jr Hard), so this is not a
blocked decision. **It is a player-visible behaviour change that F0 must not
disguise as a rename:** a Spell Jr player today can play Hard words in the base
game and in the Climb. After this file, they cannot.

And it is worse than the ceiling table suggests, because **the level selector
is not filtered at all**. `build_level_options` (`game.rs:774`) always renders
all five of `LEVEL_OPTS` — Climb, Easy, Medium, Hard, **Expert** — for every
player, and the default level is `"climb"` (`model.rs:218`). A Spell Jr player
can pick Expert today; the serve path quietly swaps it for Hard. The UI offers a
tier the game then refuses to deliver.

**Question for Eric:** confirm C3 applies to the base game and the Climb, i.e.
Spell Jr players lose Hard there. (Recommend yes — it is what the label
promises and what four of the six surfaces already do.)

### §0.2 F2 cannot be implemented on `modes.json` as written

F2 adds `juniorPolicy` to each `modes.json` entry and makes a missing policy a
build failure (I4). But **the three surfaces it names first are not in
`modes.json`:**

| F2 target | Where it actually lives |
|---|---|
| Base game | the default screen; tiers from `consts::LEVEL_OPTS` |
| The Climb | a *level option* (`LEVEL_OPTS[0]`) plus a hub tile (`config/hub-tiles.json`) |
| Daily Challenge | a hub tile plus `src/daily.rs` |
| Blitz | **does not exist.** Only `docs/CC-SPELL-O-PHONES.md` and one comment mention it. "Already hidden" is moot. |

So I4 as written would pass while leaving the base game, the Climb and the Daily
— the surfaces a child actually plays — ungoverned. The resolver needs a
registry that covers them. Two ways, Eric's call:

- **(a)** add `standard`, `climb` and `daily` as entries in `modes.json` so I4
  reaches them, or
- **(b)** keep `modes.json` for tools and give the base game its own policy
  source alongside `hub-tiles.json`.

Recommend (a): one registry, one grep gate, and I4 means what it says.

**Proposed `juniorPolicy` — the seven with no F2 default are flagged ⚠:**

| Mode | Status | Existing kid rule | Proposed |
|---|---|---|---|
| base game *(not in registry)* | live | caps Hard ✗ | `ceiling:medium` |
| climb *(not in registry)* | live | bands clamp at Hard ✗ | `variant:jr-climb` |
| daily *(not in registry)* | live | `KID_ARC` easy+medium ✓ | `variant:jr-daily` |
| def_match | live | medium ceiling, Relaxed ✓ | `ceiling:medium` |
| word_picture | live | `kid_ok` ≤ medium ✓ | `ceiling:medium` |
| bee_sim | live | `KID_CEILING` medium ✓ | `ceiling:medium` |
| impostor | live | `effective_tier(t, kid)` | `ceiling:medium` ⚠ confirm its clamp |
| practice | live | **none** | `ceiling:medium` ⚠ |
| letter_forge | live | **none** | `ceiling:medium` ⚠ |
| word_chains | live | **none** | `ceiling:medium` ⚠ |
| translate | live | reads the kid's own pools | `ceiling:medium` ⚠ |
| reports | live | kid surface vs guardian surface (I4) | `ceiling:medium` ⚠ not a tiered mode — may want `n/a` |
| calendar | live | kid-only goals (I7) | `ceiling:medium` ⚠ not a tiered mode — may want `n/a` |
| say_it | hidden | `is_offered = flag && !kid` | `hidden` |
| photo_list | hidden | `button_allowed(.., !kid, ..)` | `hidden` |
| spell_aloud | hidden | `&& !kid` | `hidden` |
| online_spelloff | hidden | `!kid && logged_in` | `hidden` |
| ghost_racing | hidden | none | `hidden` |
| syllable_replay | hidden | none | `hidden` |
| word_stories | hidden | none | `hidden` |

Reports and Calendar are not difficulty-tiered, so `ceiling:medium` is
meaningless for them. F2's enum has no "not applicable" value — worth adding
`ungated` rather than forcing a false ceiling.

---

## §1 Names

**The player-facing surface is already clean.** The rename is almost entirely
internal.

| Surface | Kid Mode / Little Speller occurrences | Notes |
|---|---|---|
| Locale values, all 15 | **0** | the only hit is `settings.kid` = `"Spell Jr"`, identical in all 15 — already untranslated (D7a already true) |
| Locale keys `kidMode.*` / `littleSpeller.*` | **0** | none exist; the keys are `settings.kid`, `settings.kidSmall` |
| `index.html` renderable text | **0** | 11 raw hits, all inside HTML/CSS comments (verified by stripping comments and scripts) |
| Rust literals that reach a player | **0** | 2 raw hits, both test assertion messages |
| Web bundle / site | **0** | |
| Comments | many | not player-facing; F0 may clean them but I11 does not require it |

So **I11 already passes for player-facing text.** Scanning for translated forms
(Modo Niños, Mode enfant, Kindermodus, 키즈 모드, 儿童模式, детский режим, and
the rest) found none in any locale.

`settings.kidSmall` reads "Bigger text, friendly words, no prices" — the three
Spell Jr promises, and one of them (a gentler game) the code does not keep. §0.1.

**Internal rename scope, if F0 renames identifiers:**

| Identifier | Occurrences |
|---|---|
| `kid` as a word in Rust | 262 |
| distinct `*kid*` identifiers | 33 — `kid_ok` 13, `kid_allowed` 10, `kid_register` 7, `kid_mode` 6, `for_kid` 5, `kid_filter` 4 … |
| DOM id `kidToggle`, body class `kid` | read by `settings.rs`, `climb.rs`, `calendar_ui.rs`, `online_spelloff.rs` |
| `GameMode::LittleSpeller` | dead variant — delete |

**Persisted storage — recommend NOT renaming.**

| Key | Holds |
|---|---|
| `byear_agegate_v1` | the verdict (§2) |
| `byear_prefs_v1` | the prefs blob, containing field `kid: bool` |

Neither key is named `kidMode` or `littleSpeller`. The only rename available is
the `kid` field inside the prefs JSON, which needs F0's read-old-write-new
migration and buys a player nothing. F0 says stop and ask if a migration cannot
be proven lossless; the cheaper answer is not to migrate. **Recommend: leave
persisted names alone.**

---

## §2 Age gate

| Aspect | Current |
|---|---|
| Storage | `byear_agegate_v1` = `{ verdict: "kid" \| "full", checkedAt }` (`agegate.rs:19`) |
| Raw DOB | **never stored** — I7 already holds |
| Cutoff | `MIN_FULL_APP_AGE = 13` (`agegate.rs:17`) — D6's boundary already |
| Prompted | at launch when no verdict exists (`lib.rs:251`) |
| Lock | boot sets `kid = true, age_locked = true` for a `kid` verdict (`lib.rs:168`) |
| Parent gate | fresh worded-math problem, `open_parent_gate()` (`lib.rs:381`) + `agegate::parent_problem()` |
| Toggle vs lock | unchecking while `age_locked` re-checks the box and opens the parent gate (`lib.rs:596`) — F5's lock already behaves |
| Reset | clearing app data wipes the verdict; the prompt returns |

F1's `{experience, locked, source}` maps onto this directly:
`verdict=kid` → `junior, locked, age-gate`; prefs `kid` without a lock →
`junior, unlocked, toggle`; a passed parent gate → `source: parent-gate`.

---

## §3 Modes

Registry: **17 entries** in `config/modes.json` — 10 live, 7 hidden. Full
per-mode table with proposed `juniorPolicy` is in §0.2.

Existing kid rules outside the tier ceilings:

| Rule | Where |
|---|---|
| Definition Match defaults to Relaxed | `defmatch.rs:114` — F0 carry-over ✓ |
| Climb + account icons hidden | `climb.rs:146` via `btn-hide` |
| Share hidden | `index.html:727` (D4: "celebrates and saves, but does not share") |
| Camera absent | `photo_list.rs:29` |
| Extra attempts default ON | `lib.rs:606`, `settings.rs:286` |
| Zero purchase surfaces | `calendar_ui.rs:184` (AUDITPASS F12) |
| Daily friendly-words filter | `daily.rs:229` via `kid_filter` |

**Jr Daily already exists in all but name.** `KID_ARC` is easy 5 + medium 5,
kid-filtered, deterministic per date. It is not seeded `date + ':jr'` as F4
specifies, but the slice seed hashes the pool, and the kid pool is filtered, so
kid and standard dailies already draw different sets on the same date. F4 is a
rename plus a seed-string change, not a new mode.

---

## §4 Accounts

| Aspect | Current |
|---|---|
| Where | **Flask on the Mac mini** (`~/spellgame-server/backend`, PIDs 1144/1145) — neither Workers + D1 nor the Pi |
| Hashing | **bcrypt** — passes |
| Sessions | opaque tokens, `sessions` table, cookie or `Authorization: Bearer`, **90-day rolling** — matches F12 |
| Signup verification | email **link** token, 24 h — not a code |
| Password reset (email) | **link** token, 30 min, single-use |
| Password reset (SMS) | 6-digit code, 10 min, bcrypt-hashed, **3** attempts |
| SMS sending | `send_sms()` is a **stub that only prints** (`auth.py:223`) — never sends |
| Rate limits | per-IP, in memory, per process: signup 5/h, login 10/15 min, reset-email 5/h, reset-sms 5/h, confirm-reset 10/15 min |
| Enumeration | reset-email response identical for known and unknown ✓; `CLIMB_DEV` confirmed **absent** from `.env` and the live process, so the dev-token leak is off |
| Server password rule | length ≥ 8 only — no digit, symbol or blocklist check |
| Account deletion | server `/api/auth/delete-account` + client UI ✓ — reached from the 👤 meta icon, password-confirmed; **not from Settings** as F12 specifies |
| Phone | DB column, accepted on signup, SMS reset route — **no client UI** |

**The rate-limit caveat.** Codes *are* rate-limited, so the trigger does not
fire. But the limiter is an in-memory dict per process and **two worker
processes are running**, so every limit is effectively doubled and resets on
restart. And it is per IP, not per email — D4's "≤ 5 sends per email per hour"
and 60-second resend cooldown do not exist.

F8 and F10 specify **6-digit codes**. Today both email flows use **links**.
That is new server work, not a tweak: new code table semantics, attempt
counting on the email path, and per-email limiting.

### Phone accounts — the number F12 is waiting for

| Count, live database | |
|---|---|
| tables present | 11 |
| users | **0** |
| with any phone | **0** |
| phone-only (no email) | **0** |
| sessions, verifications, resets, leaderboard, matches | **0** each |

Verified as the live database — the running server holds this exact file open —
and as genuinely empty rather than a stale read: all eleven tables exist and
every one has zero rows.

Phone-only accounts are also **impossible by schema**: `email TEXT NOT NULL`
(`db.py:27`).

**So F12's "zero phone-only accounts" branch is the one that applies: delete
phone auth entirely.** No player is stranded, because no player exists.

That second fact deserves its own line: **nobody has ever created a Climb
account in production.** Every migration concern in F12 about existing accounts
is moot. The only existing state that must survive the update is device-local —
guest progress and age verdicts.

---

## §5 Tier literals outside the registry

The nominal registry is `consts::LEVEL_OPTS`. Hard-coded tier strings elsewhere,
by file (excluding the generated `word_data.rs`, 65):

| File | Literals |
|---|---|
| `defmatch.rs` | 31 |
| `game.rs` | 29 |
| `wordpic_screen.rs` | 28 |
| `words.rs` | 19 |
| `translate.rs` | 17 |
| `tone_drill.rs` | 13 |
| `word_data_audit.rs` | 12 |
| `wordpic.rs` | 11 |
| `attempts.rs` | 11 |
| + `bee.rs`, `climb.rs`, `daily.rs`, `forge.rs`, `learner.rs`, `model.rs`, `practice.rs`, `reports.rs`, `say_it.rs`, `spellpic.rs`, … | |

Plus tooling added this week: `tools/retier_cjk.py` (`BUDGETS`) and the
`every_tier_ladder_climbs` test in `words.rs`.

Many of these are legitimate — deck keys, word-data headers, server leaderboard
categories (`db.py` notes "NO 'easy'"). **I3's grep gate as written would fail
the build on day one.** It needs an allowlist of data-bearing literals, or it
needs to target *tier lists* (arrays of several tiers) rather than every
occurrence of `"hard"`.

---

## §6 Spell Picture

| Aspect | Current |
|---|---|
| Tier entering layout | the **picture's own** `p.tier`, not the player's: `params(&s.tier)` sets `band_max`, `pool(lang, &s.tier)` draws `[tier, "easy"]` (`spellpic.rs:201-203`) |
| Player-chosen tier | does not exist |
| Save key | `spell_wordpic`; each `Run` keyed by `(pic, lang)` (`wordpic.rs:467`) |
| Tier in the save | **none** — `Run` has no tier field |

So F16's `(subject, tier)` keying is a schema change to `Run` plus a migration:
every existing run gets the picture's own tier.

Relevant evidence from this week: `band_max` is keyed on the picture's tier
because the calligram packer was tuned per tier, and the ko/ja re-tiering showed
how easily a change to *which words* a tier supplies starves STACK slots.
Offering Hard words on an Easy picture is exactly the combination F16's
tier-axis render sweep exists to catch — it should run before the sheet offers
anything.

---

## Signed after review (Eric, 2026-09-11)

- **§0.1 — SIGNED: yes.** Spell Jr loses Hard in the base game and the Climb.
  Jr is Easy + Medium on every surface, per C3.
- **§0.2 — SIGNED: option (a).** `standard`, `climb` and `daily` become
  entries in `config/modes.json`, so I4 reaches the surfaces a child actually
  plays and one registry governs every tier decision.
- **Inventory committed** to `cc-onboard-jr` for review.

- **Items 3, 4, 6 — SIGNED ("yes, use your proposals and start phase a").**
  Read as covering every recommendation in the list below that Phase A needs:
  the seven ⚠ policies as proposed plus `ungated` for Reports and Calendar
  (item 3); persisted storage names are not renamed (item 4); I3 ships as an
  allowlisted gate rather than a bare grep (item 6). Item 5 is Phase B.
  Phase A started on this branch the same day. If that reading is wider than
  intended, item 4 is the one to revisit: it is why F1 derives
  `{experience, locked, source}` from the stored `kid` and `age_locked` rather
  than storing that shape.

## Signed during Phase B review (Eric, 2026-09-12)

- **Guest leaderboard — SIGNED: D1 wins.** "use D1, guests keep the
  leaderboard." CC-ONBOARD-JR F7 item 6 and Done 22a asked that a guest who
  opens the Climb leaderboard be routed to the front door; D1 (signed
  2026-09-02) keeps the standings exactly as they were. The two conflicted, the
  gate caught it (the hub-row e2e), and the branch kept D1 pending this call.
  Guests view the board; posting a run still requires an account. The rest of
  Done 22a stands: a guest's runs land on a new account exactly once.

## Decisions this inventory asks Eric for

1. **§0.1** — confirm Spell Jr loses Hard in the base game and the Climb.
2. **§0.2** — put `standard`, `climb` and `daily` in `modes.json` (a) or give
   them a separate policy source (b). Recommend (a).
3. **§0.2** — the seven ⚠ policies, and whether to add an `ungated` value for
   Reports and Calendar.
4. **§1** — recommend **not** renaming persisted storage names.
5. **§4** — the email flows move from links to 6-digit codes (F8/F10), which is
   new server work including per-email limits and a shared limiter across both
   workers.
6. **§5** — I3's grep gate needs a scoped target or an allowlist before it can
   be switched on.
