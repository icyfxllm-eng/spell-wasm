# CC-LEARNING-ENGINE-L0 §0 census

Run 2026-09-18 against `82fdde48` (plus an uncommitted working tree from
another session, which this census did not touch). Read-only: no code changed.

## Outcome: STOP. All three stop triggers fire, and the file's premise is wrong

The L0 file says CC-LEARNING-ENGINE "has been sidestepped by every session"
and treats the engine as unbuilt. **It is built and shipping.** `src/learner.rs`
(1,299 lines, 31 tests) holds BKT, FSRS, a bounded attempt log, placement, the
guardian report and the insight line. The flags `learner_select`,
`learner_surfaces` and `learner_insight` all default **ON**.

| Commit | What landed |
|---|---|
| `63357342` | L0: learner model core, deterministic to the byte |
| `65454f60` | L1: selection policy (dark at first) |
| `b3b5b712` | placement flow + guardian report |
| `b99f1765` / `7801d5ad` | CC-REPORTS `ReportsQuery` + grapheme classifier; guardian dash core (ship 122) |
| `8052a764` | CC-CALENDAR journal, goals, planner (ship 125) |
| `7fad9650` | flags flipped on (ship 134) |
| `03eba21c`, `4a2ffa72` | placement-probe bug fixes |

So L0 is not greenfield. R1 would retrofit a contract over four live consumers,
and R2 would replace one live scheduler and reconcile two others. D2 ("defer
BKT") describes a BKT that is already in the build and already feeds shipped
surfaces. Every part of the L0 spec needs rereading with that in mind.

---

## Item 1: are parent D2 and D6 signed?

**Yes, both** (Eric, 2026-07-31, recorded in `docs/CC-LEARNING-ENGINE.md`).

- **D2:** English taxonomy is the template and ships first; Tagalog → Paul.
  Every other language stays flag-shut until a native speaker signs. "Not naming
  them now is the decision." So the owner list is *signed as open-ended*, not
  filled in.
- **D6:** placement is 10–14 words per language, ≤3 minutes, skippable.

For the record, D1, D3, D4, D5 and D7 are also DECIDED there.

**Parent D5 is under pressure elsewhere.** `docs/census/telemetry_census.md`
R1/R9 propose amending "zero telemetry" to "zero learning or gameplay telemetry"
for standard players. L0's I1/I2 assume the strict reading. Worth confirming
that the amendment keeps learner data out of telemetry.

## Item 2: what decides when a missed word comes back? **Three rules** (stop trigger)

| # | Rule | Unit | Store (localStorage key) | Algorithm |
|---|---|---|---|---|
| A | `misses.rs` | **word** (`word_id0(lang, word)`) | `byear_misses_v1` (one global key, cap 300) | Leitner, 5 boxes; `SR_INT` = 0, 0, 10 min, 1 d, 3 d, 7 d (`consts.rs:383-385`) |
| B | `tone_drill.rs` | word (zh tone misses) | `byear_tone_drill_v1` | same Leitner constants, **copied**, separate store |
| C | `learner.rs` FSRS | **skill** (hazard id), not word | `spell_learner_{lang}` | FSRS (see the defect below); biases fresh-word picks via `select_within` |

**Call sites.**
- **A writes:** `game.rs:2323` `add_miss`, `defmatch_screen.rs:598`,
  `attempts.rs:79` `add_miss_at`, `game.rs:1811` `promote_miss` (graduation also
  writes `spell_redemption_v1` and the journal).
- **A reads:** `game.rs:1329` (the review turn, deck-shuffled over due
  misses), `game.rs:759/3195` (badge counts), `reports.rs:36,51`.
- **B:** served in the review turn only once A's due set is empty
  (`game.rs:1334`).
- **C writes:** `note_attempt*` from `game.rs:1654,1712`, `wordpic_screen.rs:2377,2460`,
  `say_it.rs:220` (speech, logged but not scheduled).
- **C reads:** `game.rs:1419-1424` (fresh-word promote window of 8, behind
  `learner_select`, default ON), `calendar.rs:94-110` `planned_pick`.

So A decides *when a missed word returns*; C decides *which fresh word to show
in the band*. L0's R2 ("FSRS for the missed-words queue, keyed on entry
identity") replaces **A**. It would then run beside a *different*,
per-skill FSRS (**C**) that shares the name and the weights. B is the same rule
as A in a second store, so I5's "exactly one path" has to decide B's fate too.

**Naming trap.** `reports::rematch_set` is documented as "the SAME FSRS due
window", but it reads rule **A** (Leitner `m.due`), not FSRS. CC-CALENDAR's
"Ready-for-a-rematch (FSRS due forecast)" inherits the same mislabel.

**FSRS correctness defect (by reading; Phase 2 must confirm).**
- `learner.rs` pins the **FSRS-4.5** 17-weight set. But initial difficulty uses
  the **FSRS-5** formula `w4 − e^{w5·(G−1)} + 1`.
- With 4.5 weights, a correct first review gives 5.16 − e^{2.46} + 1 ≈ −5.5,
  which clamps to **1.0**, the minimum. So every skill first met correctly
  starts at minimum difficulty. FSRS-4.5's own formula `w4 − w5·(G−3)` gives
  5.16.
- Mean reversion pulls toward a constant 5.0. FSRS pulls toward D0(Easy).
- Placement (`apply_placement`) hand-sets stability and due dates outside FSRS.
- The file's own header says "reference-exact" means agreement with formulas
  evaluated in-test, not external vectors. **Acceptance test 3 (published
  vectors) would fail today.** Any per-word R2 should not copy this code as-is.

## Item 3: stub inventory. None of the four is a stub; all are live

| Consumer | State | Shape it assumes | Access path |
|---|---|---|---|
| **CC-REPORTS** (`reports.rs`, `reports_ui.rs`) | Shipped (122) | `LearnerState.skills[].mastery`, `.log[].typed/.correct/.word`, `AppState.misses[]` (word, lang, misses, due), `Redemption` | **Is** the read module (`ReportsQuery`), but it calls `learner::load_for` and walks raw structs. Gate enforces no writes (`gate.sh:13-15`). |
| **CC-GUARDIAN-DASH** (`guardian_dash.rs`) | Shipped core (122); digest + PDF in `f3cb42a8` | `GuardianReport { attempts, correct, skills: (id, mastery, days_overdue), strengths }`, `.log.len()` | **Direct:** `learner::load_for` + `guardian_report` (`guardian_dash.rs:26,128,167`). Its header claims "every read goes through ReportsQuery or the learner's own report." |
| **CC-CALENDAR** (`calendar.rs`) | Shipped (125) | `LearnerState.skills[].fsrs.{reps,due_day}` + `learner::hazards` | **Direct:** takes `&LearnerState` and reads FSRS internals (`calendar.rs:94-110`). The spec says it "reads through ReportsQuery." |
| **CC-PLACEMENT-IDEMPOTENCE** | F0/F4 implemented (`game.rs:1175-1240`, `testseam.rs:147-153`, `learner.rs:1279`) | `LearnerState.placed: Option<bool>`. **There is no separate `PlacementRecord`**; placement status lives inside the learner blob. | **Direct:** `should_offer_placement`/`note_placement`/`finish_placement`, all in `learner.rs` |

Other direct readers the L0 spec doesn't list: `stats.rs:42-44` (guardian HTML
in Stats), `yearbook.rs:146`, `game.rs:894` (insight line).

**Stop trigger fires:** at least five modules outside `learner.rs` reach learner
storage through `load_for` and read raw fields. None of them goes through an
interface.

**Spec files missing.** CC-PLACEMENT-IDEMPOTENCE, CC-LANG-SCOPE and
CC-TELEMETRY-FOUNDATION have no file in `docs/`. The first two exist only as
code comments and past session transcripts. Telemetry has only its census. L0's
ownership table cites all three as authorities.

**Ownership conflict.** L0 says "placement: reads the record, never writes it".
But the record is a field of the same `spell_learner_{lang}` blob the scheduler
writes, so any learner-state migration rewrites it. The boundary needs
restating: separate the store, or accept a shared writer.

## Item 4: learner state on device, and language scoping

| Key | Contents | Language scoping | Profile scoping |
|---|---|---|---|
| `spell_learner_{lang}` | `LearnerState` v1: skills (BKT + FSRS), 512-entry log (misses keep typed text), `placed` | **by key suffix** | **none** |
| `byear_misses_v1` | ≤300 `MissEntry` (word, lang, tier, misses, box, due, ts) | **no**: one global store, lang is a field inside each record | none |
| `byear_tone_drill_v1` | same shape, zh tone misses | no | none |
| `spell_redemption_v1` | ≤200 graduations | no (filtered by field) | none |
| `spell_served_v1`, `spell_seltrace_v1` | served-word exclusion + a 200-entry serve/outcome trace | tier key includes lang; trace doesn't | none |
| journal / plans / goals (`journal.rs`, `calendar.rs`) | per-lang keys | yes | none |

**Stop trigger fires.**
- **No language-scope registry exists.** There is no `languageScoped`
  classification, no store-scope CI, and no CC-LANG-SCOPE file. F5's
  classification CI (acceptance test 7) has nothing to register against.
- **No profile model exists.** `multiple_profiles` is only an entitlement bool
  (`entitlements.rs:101`). No store carries a profile id. L0's I4 key
  (profileId, languageCode, entry identity) cannot be built until profiles exist
  or D-something defines the key as "device = one profile" for now.
- Entry identity (CC-RUSSIAN-STRESS I8) *is* in use on the miss store through
  `word_id::word_id0`. Sense 0 keeps the legacy key, so за́мок/замо́к separate
  only once senses > 0 are assigned. The learner log keys on bare `word`.
- `spell_seltrace_v1` is labelled "served-word telemetry (local, anonymous)".
  It's local-only, but the word "telemetry" will confuse the I2 lint.

## Item 5: the PERSIAN F6 exposure ledger

**It doesn't exist.** CC-PERSIAN-FOUNDATION F6 is specced but unbuilt. The
Aug 8 notes in that file list three unresolved blockers: the BD-G4 easy-tier
floor conflict, unsigned CC-SPELLPIC-AUDITPASS D1–D5, and Azure network
reconciliation.

What does the freshness job today, in parallel:
- `deck.rs`: persisted no-repeat shuffle per (lang, tier)
- `selection.rs`: rolling per-tier exclusion (`spell_served_v1`)
- `daily.rs`: its own no-repeat
- `spelldoku`: a served-hash ledger

The scheduler has no ledger API to consume, so "consumer, never a parallel
path" can't be honored yet. It could consume the deck, but the deck isn't the
ledger F6 describes.

**The telemetry schema lint (I2, acceptance 8) doesn't exist either.**
CC-TELEMETRY-FOUNDATION v1.1 is itself stopped at its census, so there is no
schema to lint.

---

## Rulings needed before Phase 1

| # | Question | Census recommendation |
|---|---|---|
| C1 | L0 was written for an unbuilt engine. **Re-scope it as a retrofit**, and withdraw or re-sign D2 ("defer BKT") given that BKT ships and feeds Reports/Guardian/Trap Boss? | Re-scope. Keep BKT as-is (don't rip out shipped surfaces); CC-LEARNING-SKILL becomes "validate BKT", not "build it". |
| C2 | Three review rules (A misses Leitner, B tone-drill Leitner, C per-skill FSRS). Which merge? | R2 replaces A with per-word FSRS; B folds into it (same rule, different store); C stays as the *fresh-word ordering* signal, renamed so it isn't mistaken for a review rule. Or C goes per-word too. Your call. |
| C3 | Fix the FSRS version mix (4.5 weights + 5 formula) inside R2, or pin FSRS-5 with its 19 weights? | Pin one published version with its vectors, and apply it to C as well so there's one FSRS in the crate. Note this **changes live scheduling** for every current player (difficulty resets from 1.0). |
| C4 | Profile key for I4: there are no profiles. | Define "device = profile 0" in the contract signature now, so the key shape is frozen before multi-profile ships. |
| C5 | CC-LANG-SCOPE F5 and PERSIAN F6 are unbuilt. Does L0 build a minimal store-scope registry and a ledger, wait for them, or drop acceptance 7 and the ledger clause? | Drop both from L0 acceptance and record them as dependencies. Building them here is scope creep. |
| C6 | Telemetry lint (I2 / acceptance 8) doesn't exist. | Move acceptance 8 to CC-TELEMETRY-FOUNDATION as a requirement on *its* lint. |
| C7 | Placement's `placed` flag lives inside the learner blob L0 will own. | Either split it into its own key (a migration) or amend the ownership row to "L0 stores it; PLACEMENT-IDEMPOTENCE owns its semantics". |
| C8 | Relabel `rematch_set` / the Calendar "FSRS due forecast", which actually read Leitner? | Yes. It resolves itself once R2 lands, but the docs are wrong today. |
| C9 | Missing spec files (PLACEMENT-IDEMPOTENCE, LANG-SCOPE) | Stash them in `docs/` from the transcripts before L0 cites them as owners. |

The L0 file's own open decisions (D3–D8) still stand and are unaffected, except
that D3's grade table now also governs rule C if C3 unifies FSRS.

---

## Rulings SIGNED (Eric, 2026-09-19): "sign C1–C9 as drafted"

These are the drafted rulings, rechecked against `origin/main` on 2026-09-18
before signing. The table above holds the census's first-pass
recommendations; where the two differ, **these rulings govern**. C3 and C6
changed after the census: the FSRS formulas were fixed on main (b94dc712,
build 227), and the telemetry lint landed.

**C1: re-scope L0 as a retrofit.** L0 retrofits the engine that shipped in
builds 122–134. It does not build a new one. **L0 D2 ("defer BKT") is
withdrawn.** BKT stays as shipped, feeding Reports, Guardian Dash and the Trap
Boss bars. CC-LEARNING-SKILL's job becomes checking that shipped BKT is correct,
not building it. R1–R4 keep their intent, with "build" read as "retrofit".

**C2: three review rules become one.** R2 replaces rule A (the Leitner misses
queue, `misses.rs`) with per-word FSRS. Rule B (the tone drill) runs on the
same scheduler code with its own queue: one rule and one code path (I5),
while staying the separate study CC-ZH-TONE F3 requires. Rule C (per-skill
FSRS in `learner.rs`) is not a review rule. It stays as the skill-level signal
for ordering fresh words, renamed to skill due-ness. The I5 scan counts rules
that decide when a missed word returns, and C doesn't count.

**C3: one FSRS version.** Pin FSRS-4.5 (17 weights, as pinned today). This
ratifies the formula fix on main (b94dc712) and the stored-difficulty reset
(schema v2, PR 6). Word-level scheduling (R2) and the skill signal (C) share
one implementation. Acceptance 3 is met by vendoring reference outputs from a
published FSRS-4.5 implementation as a fixture. Moving to FSRS-5 later is a
reviewed change of its own.

**C4: profile key.** The device counts as profile 0 until multi-profile ships.
The `LearnerQuery` contract takes a profile id from day one, so the key shape
(profile, language, entry identity) is fixed now, and multi-profile becomes a
change to the stored data, not to the contract.

**C5: unbuilt dependencies.** Acceptance 7 (the language-scope check) and the
exposure-ledger clause are dropped from L0 and recorded as dependencies on
CC-LANG-SCOPE F5 and CC-PERSIAN-FOUNDATION F6. L0 builds neither. R2's new
store is scoped by language by construction: one key per language, unlike
today's single global misses key. It registers with F5 when F5 exists.
Freshness keeps using the existing deck and selection exclusion until F6
lands.

**C6: telemetry isolation.** `schema_lint` (`src/telemetry/schema.rs`) rejects
gameplay-data field names but no learner terms. L0 Phase 1 adds `learner`,
`mastery`, `fsrs`, `bkt`, `skill`, `stability`, `difficulty`, `lapse`,
`reps`, `due`, `review` and `placement` to its forbidden list. On 2026-09-18
none of these collided with a current field. Acceptance 8 stays as written:
its planted `mastery` field must fail the lint. Because this touches a file
CC-TELEMETRY-FOUNDATION owns, this signature is that file's consent.

**C7: the placement flag.** Not split out. The ownership row now reads: "L0
stores `placed`; CC-PLACEMENT-IDEMPOTENCE owns what it means." Only placement
code writes it, enforced by the I6 scan. Splitting it would need another schema
migration right after v2.

**C8: the "FSRS due" labels.** Correct the doc comment on
`reports::rematch_set`, and CC-CALENDAR's "FSRS due forecast" wording, now:
both read the Leitner misses queue. The wiring corrects itself in R2, when the
rematch set reads the per-word FSRS queue through `LearnerQuery`.

**C9: missing specs.** CC-TELEMETRY-FOUNDATION is now in `docs/`.
CC-PLACEMENT-IDEMPOTENCE and CC-LANG-SCOPE are still missing. Save both into
`docs/` from the session transcripts before Phase 1, each marked as saved
verbatim. L0 doesn't cite them as owners until they're there.

**C10 (found after the census): a failed load overwrites learner data.**
Acted on, not formally signed. Eric (2026-09-18): "hold PR #6 and do C10
first". Implemented in PR 7:
- a newer build's state is never written;
- unreadable bytes are backed up to `spell_learner_{lang}_unreadable`, then
  play continues fresh.

It must reach players before schema v2 does.

**Phase 1 unblocked (Eric, 2026-09-19):** "Phase 1 may start; D3–D8 gate only the phases they name."

**Also recorded (2026-09-18/19, Eric):**
- Stored FSRS difficulty resets to 5.1618 for existing players: schema v2,
  PR 6, held behind C10.
- "Reset this language" erases that language's learner state (Done #8 gap,
  branch `learner-reset`).
- "Reset this language" leaves the missed-words queue as is.

### Actions these rulings create

| When | Action | Ruling |
|---|---|---|
| now | Correct the `rematch_set` doc comment and CC-CALENDAR's "FSRS due forecast" wording | C8 |
| before Phase 1 | Save CC-PLACEMENT-IDEMPOTENCE and CC-LANG-SCOPE into `docs/` | C9 |
| Phase 1 | Add the learner terms to `schema_lint`; the `LearnerQuery` signature carries the profile id | C6, C4 |
| Phase 2 | Per-word FSRS replaces `misses.rs`; the tone drill moves onto it; rule C renamed to skill due-ness | C2 |
| Phase 2 | Vendor FSRS-4.5 reference outputs as the acceptance 3 fixture | C3 |

L0's own D3–D8 remain open. D3's grade table now also covers rule C (C3).
Today the game grades only pass or fail, so D3's recommended Hard rating is
new signal the game doesn't record yet.
