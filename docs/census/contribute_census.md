# CC-CONTRIBUTE §0 census

Run 2026-09-30 against `main`. Read-only — nothing was modified. Every
load-bearing claim was re-checked by hand, including against the **live
server**, not only the repo.

**Outcome: HALT.** Four of the eight census items fail (C1, C2, C5, C7), and
the spec's central premise does not hold in production. Nothing past §0 is
built.

---

## The premise first: definitions are not dark

The spec opens with "The 15-language release ships definitions-dark: no
non-English definition renders until that language's definitions audit
ingests." That is not what production does today:

```
GET /api/meaning?word=gato&lang=es  -> {"definition":"cat (unspecified gender)","pos":"noun"}
GET /api/meaning?word=chien&lang=fr -> {"definition":"dog","pos":"noun"}
GET /api/defpool?lang=ja&tier=easy  -> 200
```

Non-English definitions are **lit, and have been since 2026-07-27**, under the
interim-content ruling in `docs/CC-DEF-MATCH-PLAN.md:16-22`: no Gig A artifacts
existed, so Eric directed that a mechanical prescreen stand in for the audit.
`backend/def_pools/*.json` (all 15 languages, 28,698 rows, built 2026-09-18) is
that stand-in, and `scripts/build-def-pools.py:11-13` says so itself —
"provisional stand-in for the Gig A prompt-grade column and the
CC-DEF-PRECHECK sweep — Eric-approved interim content, formal native audit
still open."

**This changes what CC-CONTRIBUTE is for.** It is not unlocking definitions
that are hidden. It is auditing definitions **already in front of players**,
including children. That is a stronger reason to build it, and a weaker reason
to gate it behind coverage: today's alternative to a contributor verdict is not
"nothing renders", it is "the prescreen's guess renders".

### What that costs right now, concretely

`backend/def_pools/en.json`, easy tier, both `prompt_grade: true` and
`kid_register: true`:

| word | definition shipped to players |
|---|---|
| cat | ISO 639-2 & ISO 639-3 language code for Catalan. |
| sun | ISO 639-2 & ISO 639-3 language code for Sundanese. |

These reach Definition Match, in English, marked kid-safe. The hint path is a
different code path (`/api/meaning`, live Wiktionary proxy) and returns the
right senses, which is why this has not been caught. A mechanical prescreen
(length, self-reference, blocklist, duplicate text) cannot detect a wrong-sense
pick; only a human can. That is the argument for this spec, and it belongs in
the spec's own intent section.

---

## Census items

| # | Item | Verdict |
|---|---|---|
| C1 | One definitions-dark gate, read by every surface | **FAIL** — six independent gates; the two that decide what a child sees are server-side |
| C2 | Locked sheet export/ingest format documented | **FAIL for definitions** — a hash-stamped format exists and is good, but it is audio-bound; five formats total, no definitions one |
| C3 | CC-DEF-PRECHECK output exists for ≥1 non-English language | **PASS on data, FAIL on identity** — candidates for all 15, but no stable key, no provenance, nowhere for a verdict to land |
| C4 | CC-ONBOARD-JR Phase B (accounts) landed | **PASS** |
| C5 | Adult gate (CC-REPORTS-SHARE F6) exists as a reusable component | **FAIL as named** — that doc and that gate do not exist; a reusable parent gate does, and it is weak |
| C6 | Backend accepts an authenticated POST from app and web | **PASS**, with caveats |
| C7 | Per-language "words that matter" scope list exists | **FAIL** — only the full bank |
| C8 | Per-language rows in scope with a candidate | **Reported below — and the pools are stale** |

### C1 — there is no single gate (FAIL)

Six mutually independent gates decide whether a definition renders:

| Gate | Where | Value today | To change |
|---|---|---|---|
| `api::meaning_supported` | `src/api.rs:618` | true for all 15 | app release |
| `consts::def_match` (= `is_builtin_lang`) | `src/consts.rs:275` | true for all 15 | app release |
| `MEANING_LANGS` | `backend/app.py:478` | 13 + en + zh | **server deploy** |
| `def_pools/<lang>.json` presence | `backend/app.py:744` | all 15 present | **drop a file on the server** |
| `translate::gloss_audited` | `src/translate.rs:176`, `config/gloss/*.json` | en true, 14 false | app release |
| `bee::definitions_available` | `src/bee.rs:80` | false everywhere | app release |

Consequences the spec must absorb:

- **The two gates that decide what definition text a child actually sees are server-side** (`MEANING_LANGS`, and whether a pool file exists). Neither needs an app release or store review. The audit-bearing gates are compiled constants.
- `src/consts.rs:272-274` claims to be "THE registry flag, no other per-language logic anywhere". It is not.
- Surfaces disagree: Spelldoku's definition card reads two gates (`spelldoku_ui.rs:953`), Spell Search's reads one (`wordsearch_ui.rs:330`). No test compares them.
- **`audit_pass` is synthesized, not carried.** `src/defmatch.rs:179` filters on `prompt_grade && audit_pass`, but the wire format has no such field, so the only production producer sets it as a literal `true` (`src/defmatch_screen.rs:270`, comment: "rows in the artifact passed the interim prescreen"). Definition Match's Invariant 1 is unenforceable in production, and **there is nowhere in today's artifact for a human verdict to land**.
  - Worth flagging: the test `no_artifacts_means_no_rounds` (`src/defmatch.rs:583-596`) sets `audit_pass = false` and calls that "the repo's true state today". It is a test fixture, and production now contradicts it. The test still passes because it mutates its own rows; nothing checks the real producer.
- Remote flags do not exist at all (confirmed in `docs/census/telemetry_census.md`), so F6's "the remote flag flips by the existing definitions-dark process" has no mechanism behind it. The telemetry Worker's `/v1/flags` is the only flag endpoint in existence and it serves one boolean.

### C2 — a locked-sheet format exists, but not for definitions (FAIL)

Better news than the spec's premise assumes, and still a fail. There is **no
Gig A definitions sheet**: no builder, no ingest, no column list, in any doc.
`docs/CC-DEF-MATCH-PLAN.md:16-23` and `src/defmatch.rs:10-16` both state that
no Gig A artifact has ever existed here.

But a genuinely good hash-stamped locked-sheet pipeline **does** exist for
**audio** — `tools/human-audio/sheet.py` + `ingest.py`:

- Columns `row, entry, clip, verdict, note`, preceded by a literal header line `# sheet <lang>-<tier>-<YYYYMMDD> stamp <sha256>`.
- The stamp (`sheet.py:55-61`) hashes the sheet id, then per row the row number, entry, clip name, and **the sha256 of the clip's bytes** — everything the auditor must not change.
- Ingest runs four gates — STAMP, AGREEMENT, COMPLETE, DECOY — and **any failure rejects the whole sheet and writes nothing** (`ingest.py:65-153`).
- Decoys: 8 per sheet, tolerance 1. A gap entry is placed as an ordinary row carrying a real passing clip of a *different* similar word (edit distance 1–2). Nothing on the sheet marks it; the marking lives only in `keys/<sheet-id>-DECOY-KEY.json`, written outside the served folder.
- Real artifacts exist, English only: six sheets (547/396/604/463/306/92 rows), 8 decoys each.

Four other formats exist (ru-stress, gloss packet, pronunciation overrides,
red-pen flags). Only the audio one is hash-stamped. Across them there are
three column sets, two incompatible decoy schemes (unmarked gap rows with
donor audio vs. corrupted real rows) and two decoy counts (8 vs 6).

Two docs instruct future work to "use the existing locked-spreadsheet audit
ingest. Do not build a new write path" (`docs/CC-SPELLDOKU.md:122`,
`docs/CC-SPELLDOKU-v1.2.md:174`). The thing they point at is the audio ingest,
which is hard-wired to clips — `clip_sha256`, `commons_sha1`, `speaker`,
`license`, six audio verdicts — and writes only
`assets/human-audio/<lang>/verdicts.json`. **Reusing it for definitions is a
rewrite, not a reuse.** F6's "hash-stamped exactly as Gig A sheets are" should
be restated as "matching the CC-HUMAN-AUDIO F3 stamp scheme", which is a real,
copyable design — the stamp function generalises cleanly if `clip_sha256` is
replaced by a hash of the candidate definition text.

The one existing ingest that is close in domain,
`scripts/ingest-audit-flags.py`, handles **word exclusions** in plain text: no
stamp, no decoys, not a definitions sheet.

Also worth noting: `tools/ru_stress_ingest.py:29` is the **only place in the
repo where a sheet gate is tied to compensation in code** ("gates payment").
If CC-CONTRIBUTE pays contributors, that is the precedent to read.

### C3 — candidates exist; identity does not

Pre-filled candidates exist for **all 15 languages**, git-tracked in
`backend/def_pools/`, shaped `{lang, tiers:{easy|medium|hard|expert:[...]},
exclusions}`. Each row has exactly five fields:

```json
{"word":"gato","definition":"cat (unspecified gender)","pos":"noun",
 "prompt_grade":true,"kid_register":true}
```

Three gaps block F2/F4/F6 as written:

1. **No stable identity.** Rows key on the bare surface string. There is no `word_id`, though `src/word_id.rs` exists and the telemetry census already settled `lang::lowercase` as the de-facto key. Homographs — the exact case a judged candidate list must represent — collide. The clearest proof: **zh bank rows are `pinyin|hanzi` composites** (`ai4|爱`), while the zh pool keys on bare hanzi, so the two artifacts cannot be joined without knowing that convention out of band. zh is the only language that does this (296 easy rows), which is precisely why a shared key has to be decided rather than assumed.
2. **No provenance and no confidence.** Two booleans; no source, no matcher score, no reviewer field.
3. **Nowhere to put a verdict.** No `audit_pass` column; the client invents it (see C1).

Each is a small change to `scripts/build-def-pools.py` and the pool schema —
but it is a change to an artifact this spec says it does not own.

### C4 — accounts have landed (PASS)

Email → 6-digit code → password → username, with login by email or username
(`src/climb.rs:539-702`, `backend/climb.py:68-190`). Identity is `users.id`, a
stable autoincrement integer (`backend/db.py:22-33`), exposed as
`ClimbUser.id` and resolvable server-side from the Bearer token. Present in
**both** app and web builds (`mod climb` is ungated, `src/lib.rs:7`). Votes
can be bound to an account id today.

Two facts for planning:

- As of `docs/onboard-jr-inventory.md:230-251`, **no accounts existed in production**. Every contributor must first complete a four-step signup.
- Signup email **does** send: `RESEND_API_KEY` is set in `~/spellgame-server/.env` and passed to the running backend. (`backend/auth.py:344` and an earlier reading of this tree both suggest production has no key; that comment is stale — I checked the live service.)

### C5 — the named adult gate does not exist (FAIL as specified)

`docs/CC-REPORTS-SHARE*.md` does not exist anywhere in the repo, and there is
no "F6 adult gate". What exists is a reusable parent gate,
`parent_gate_then(then)` (`src/lib.rs:1089`), already used by list deletion,
the credits screen and the telemetry toggle. Its challenge is a worded
multiplication with operands 3..=8 (`src/agegate.rs:137`).

That is **36 possible answers, unlimited retries, no lockout, no record**. It
is a child deterrent, not an adult check, and CC-CONTRIBUTE hangs "18+,
self-attested" (D6) and a paid-content grant (F5) off it. The spec should
either name `parent_gate_then` and accept what it is, or state what stronger
gate it wants — but per its own ownership rule it must not fork a second one.

### C6 — authenticated POST works from both builds (PASS, with caveats)

`Authorization: Bearer <opaque token>`, verified against a `sessions` table,
90-day rolling expiry (`backend/auth.py:184-247`). CORS already allows
`https://spellgame.net`, `capacitor://localhost`, `https://localhost`
(`backend/app.py:196-209`). Caveats for a vote route:

- `supports_credentials` is not set, so cookie auth is unusable cross-origin. Send Bearer.
- The generic rate limiter is **per-process, in memory** (`backend/auth.py:269`) and production runs two gunicorn workers, so every per-IP limit is effectively doubled and resets on restart. The DB-backed patterns to copy are `email_sends` and `submit_log`.
- Turnstile is a no-op (`backend/auth.py:291`), so signup and login have no bot protection today. A vote route inherits that exposure.

### C7 — no scope list (FAIL)

There is no per-language list of "every word that matters for the game". The
only per-language list is the full tier bank (`assets/words/<lang>/*.txt` →
generated `src/word_data.rs`), and the selection code serves the whole bank:
`src/selection.rs:41-60` bands the entire tier pool by length with no
allow-list, and the Daily "draws from the language's whole bank"
(`src/wordsearch/lexicon.rs:129-131`, `docs/CC-WORDGRID.md:50`). "Launch set"
in this repo always means a set of *languages*, never a set of words.

Runtime narrowing is negligible: `kid-exclude` 0–34 entries per language,
`exclusions` 0–7, `profanity` 33–318. Nothing reduces a 6,000-word bank to a
curated subset.

Narrower per-language lists exist but each is feature-specific, not a general
scope: gloss tables (246–299 rows, `audited:false` in all 14), practice
curricula (20 words, `$draft`), number words (13), the audio-clarity sample
(60), WordPic candidate pools, and the stale Jul-2026 pilot lexicons
(`data/<lang>/lexicon.jsonl`, 160–202 rows, `gloss` empty in all 17, and still
listing it/nb/nl/sv/th/tr which are no longer in the lineup). `audit/difficulty/`
is stale the same way (~2,574 words across 15 languages, two orders of
magnitude below the bank).

The one in-repo example of a deliberately narrowed audit scope is the
ru-stress audit: easy+medium, multi-syllable only → 1,135 of ru's 6,208 rows
(`tools/ru_stress_annotate.py:66-85`). It is defined by code, not a list file —
a reasonable precedent for how CC-CONTRIBUTE could define scope.

### C8 — rows with a candidate, per language

**The pools are stale against the bank.** 28,698 pool rows exist, but only
**21,894 name a word the bank still contains** — 6,804 rows (24%) would have
contributors judging words no player can be served. Both columns matter:

| lang | bank | pool rows | pool rows still in bank | bank coverage | prompt-grade & in bank |
|---|--:|--:|--:|--:|--:|
| en | 3165 | 2731 | 2695 | **85.2%** | 1941 |
| sw | 2845 | 1975 | 1526 | 53.6% | 1483 |
| hi | 2674 | 1892 | 1233 | 46.1% | 1181 |
| de | 6148 | 2729 | 2343 | 38.1% | 1358 |
| ru | 6208 | 2971 | 2269 | 36.5% | 1833 |
| es | 6099 | 2705 | 2145 | 35.2% | 1944 |
| pt | 6110 | 2595 | 2093 | 34.3% | 1913 |
| fil | 4083 | 1703 | 1384 | 33.9% | 1327 |
| fr | 6109 | 2592 | 2062 | 33.8% | 1688 |
| pl | 6176 | 2305 | 1709 | 27.7% | 1326 |
| vi | 4094 | 1274 | 653 | 16.0% | 610 |
| zh | 6181 | 1170 | 916 | 14.8% | 877 |
| ar | 6238 | 1041 | 478 | 7.7% | 431 |
| ko | 6233 | 531 | 238 | 3.8% | 225 |
| ja | 6160 | 484 | 150 | **2.4%** | 136 |
| **total** | **78,523** | **28,698** | **21,894** | **27.9%** | **18,273** |

No language has zero candidates, so no language is absent under F1. But the
shape of the problem is the headline:

- **Judging every in-bank candidate is ~18,273 prompt-grade rows.** At 3 Good votes each (5 for kid-eligible) that is **60,000–90,000 individual judgments**, from a player base with zero registered accounts. At 8 judged cards per session that is ~10,000 completed sessions. Spanish alone needs ~6,000 votes.
- **F7's fallback trigger (90 days, <50%, <3 active contributors) will fire for most languages on arithmetic alone**, regardless of contributor quality.
- **ja, ko and ar cannot reach meaningful coverage from this artifact at all** — 2.4%, 3.8% and 7.7% of the bank has any candidate. For those three the bottleneck is the *candidate builder*, not the contributors, and no amount of voting fixes it.
- Any count CC-CONTRIBUTE quotes should come from a **re-run of `scripts/build-def-pools.py`** against the current bank, not from the 2026-09-18 files.

Footnote: `fa` ships 8,340 bank rows in `assets/words/fa/` and in
`word_data.rs`, but is explicitly cut from the lineup (`src/consts.rs:486`
lists it among languages "gone") and has no def pool. It is out of scope, but
the dead bank files are worth cleaning up separately.

---

## What I recommend before Phase A

1. **Rewrite the premise.** State that definitions are live under an interim prescreen and that contributors are auditing shipped content. The "cat = Catalan" rows are the argument for the whole feature.
2. **Re-run `build-def-pools.py`** before any number in the spec is fixed; 24% of today's pool is words the bank no longer has.
3. **Decide the artifact's identity fields** (C3) — a stable `word_id`, provenance, and a verdict column — since F4 and F6 cannot work without them, and say who owns that change. Settle the zh `pinyin|hanzi` key explicitly.
4. **Restate F6 against the CC-HUMAN-AUDIO F3 stamp scheme** (C2), which is real and copyable, and budget it as a new definitions builder + ingest rather than a reuse of the audio one.
5. **Name `parent_gate_then` as the gate** (C5) and decide knowingly whether a 36-answer multiplication is the bar for an 18+ claim and a paid grant.
6. **Answer D12 before F5 is built** — and note a second problem: the entitlement system has **no account-bound grant mechanism at all**. Grants are regional, derived from the country code at launch (`backend/entitlements.py:66`, `src/entitlements.rs:164`), resolved fresh each time, with no per-user storage. "Sticky grant to a contributor" is new machinery, not a union into existing machinery.
7. **Size the ask against C8.** Consider a first pass targeting the kid-register subset or the most-served words per language rather than every matched row — and treat ja/ko/ar as a candidate-generation problem first.

Phases A–C stay blocked until Eric rules on these.
