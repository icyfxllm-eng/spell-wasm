<!--
Stashed 2026-10-27 from the spec Eric pasted (and signed) on 2026-10-26. The text
below the rule is VERBATIM: it has not been edited to reflect the amendments.

Amendments already signed (these override the verbatim text where they conflict):

- G-D16 / A12: "close" outcomes dropped. No close class exists in the game;
  Kid Mode's lenient match logs as Correct. Exclusion is satisfied vacuously.
  Signed as L-D2 in docs/CC-LEARNING-ENGINE-L0-CR1.md.
- A14: now reads "top-up never includes a word seen since CR-1 or still in any
  learner store". The seen-set backfill cannot recover words seen before CR-1
  that have left the 512-entry log ring. Signed as L-D4 in
  docs/CC-LEARNING-ENGINE-L0-CR1.md.
- F7: the deep-link scheme is spellgame://, not spell:// (census C8). The ping
  opens spellgame://guardian-report?week=<monday>.

Census HALTs C1, C2, C3 and C6 are cleared by CC-REPORTS-SHARE v1,
L0 CR-1, CC-HUB-NAV AE-1 and CC-MYWORDS-LISTS KC-1 (all built on main).

Phases A-D are built (2026-10-27): see "As built" at the end, below a rule
that separates the signed text from the build record.
-->

# CC-GUARDIAN-REPORT v1 — Weekly Guardian Report, Word Types, Practice List, Monday Ping

**Status:** REVIEW-GATED. Post-build-56. Must not delay build 56 or the 15-language release.
**Executable on signature:** §0 census only (read-only).
**Gated on Eric's census review:** Phases A–D.
**Design-ahead, blocked:** Phase E.
**Amends:** CC-REPORTS-SHARE v1. Also amends CC-GUARDIAN-DASH F2 copy (see G-D8, G-D13).
**Signed:** Eric, Oct 26 2026: "I like both the chat's recommendations and revised recommendations."

---

## Intent

A parent wants to know what kinds of words tripped their child this week, and what to do about it.

Today they can export a list of words. They can't see a pattern, and they get no prompt to act. This file adds four things:
- a fixed Monday–Sunday week;
- word types;
- a practice list built from those types;
- a Monday reminder.

The parent gets one tap to share and one tap to practice. The report is a door, not a verdict.

Children's app first. When unsure, show less, store less, and send nothing.

---

## Ownership

| Concern | Owner | This file |
|---|---|---|
| Report composer, closed lexicon, share sheet, adult gate, entitlement | CC-REPORTS-SHARE | Extends F1/F3 and adds sections. Does not fork them. |
| Learner data | CC-LEARNING-ENGINE-L0 R1 `LearnerQuery` | Read-only consumer. Field gaps go to L0, never here. |
| Parent dashboard, PDF, weekly digest | CC-GUARDIAN-DASH | Its F2 digest and this ping become one notification (I11). |
| Kid / adult resolver, unknown age = Jr | CC-ONBOARD-JR | Consumed as-is. |
| Drawer groups, row registry | CC-HUB-NAV | Adds one row to Your Words. |
| Sound-alike table | CC-SENSE-CUE v1.1 read API | Read-only. Uses `verified` status only. |
| Trap classes | CC-RU-ORTHO F6, CC-AR-HI-PREAUDIT | Reads `TRAP_MISS(rule_id)` when it exists. |
| Bank purity | CC-BANK-PURITY | Top-up reads the verdict ledger only. |
| Dated My Words lists | CC-MYWORDS-LISTS | Writes through its API only. |
| Practice lineup | CC-PICK-QUEUE | Opens its lineup. Does not change it. |
| Email delivery | Successor CC-GUARDIAN-EMAIL (not started) | Out of scope. |

---

## §0 Blocking census (read-only, executable on signature)

Report every item. Do not write code.

**C1 — Composer present.** Confirm the CC-REPORTS-SHARE F1 composer and F3 lexicon are in the repo. **HALT** if absent. Build REPORTS-SHARE Phase A first.

**C2 — Query fields.** Confirm `LearnerQuery` exposes, per attempt:
- UTC instant
- word id
- language
- outcome class
- mode tag, enough to identify voided "Can't hear it" rounds, Spell It voice results, and Impossible-tagged attempts

Also confirm it exposes a per-profile seen-word set. **HALT** on any missing field. Report the gap to Eric for L0.

**C3 — Gate and row.** Locate the adult gate entry point. Confirm the HUB-NAV registry can show a Your Words row by profile kind (standard vs Jr). **HALT** if row visibility can't key on profile kind.

**C4 — Bank columns.** For each language, confirm these columns exist:
- tier
- stored length unit (the same unit Randospell D3 reads)
- audited flag
- proper-noun flag
- profanity verdict
- PURITY verdict

List the languages that have a PURITY ledger. Top-up is disabled where it is absent.

**C5 — Type signals.** Per language, report the SENSE-CUE table status (`verified` / `none-by-construction` / `missing`). Also report whether `TRAP_MISS(rule_id)` outcomes exist.

**C6 — My Words lists.** Locate the CC-MYWORDS-LISTS dated-list API. Report two things:
- whether a list can hold more than one language;
- whether it supports an idempotency key.

**HALT** Phase C if the API is absent.

**C7 — Lineup and profile switch.** Locate the CC-PICK-QUEUE lineup entry and the active-profile switch path. If either is absent, "Practice these" is absent. Add still ships.

**C8 — Notifications.** Report, per platform:
- the local notification plugin;
- support for foreground notification actions;
- every current call site that requests notification permission.

Adding a local-only plugin is allowed. Name it and its licence.

**C9 — Length bands.** Per language, show the distribution of bank words across short (≤4 units), mid (5–7) and long (≥8). **Stop and ask** if any language has more than 80% of rows in one band.

**C10 — Dry run.** On the DEV fixture profile, print the type groups and practice list for the last 4 weeks, per language. Eric reviews this output before Phase A.

---

## Features

### F1 — Week window

**Intent:** Parents think in school weeks, not rolling windows.

- A week is Monday 00:00:00 through Sunday 23:59:59.
- Attempts are bucketed by converting each UTC instant with the device's current IANA zone at compose time (G-D14).
- The week starts on Monday in every locale (G-D15).
- The report is "ready" Monday 08:00 local for the week that just closed.
- "This week so far" can be viewed anytime and is labelled as partial.
- The header shows explicit dates in the uiLang date format, e.g. "Oct 19 – Oct 25, 2026".
- This resolves REPORTS-SHARE D-R6: the share window becomes the calendar week.

### F2 — Word types

**Intent:** A pattern teaches more than a list.

**Missed set W.** Words with at least one `miss` or `TRAP_MISS` outcome in the window. These are excluded (G-D16):
- voided "Can't hear it" rounds;
- Spell It voice results;
- Impossible-tagged attempts;
- `close` and `valid_other_sense` outcomes, which count as correct.

**Closed type registry (v1).** No other types exist.

| Id | Label key | Signal |
|---|---|---|
| `T-TRAP:<rule_id>` | rule-card label | `TRAP_MISS(rule_id)` (ru/ar/hi, when present) |
| `T-SOUND` | `type.sound_alike` | member of a set in a `verified` SENSE-CUE table |
| `T-LONG` | `type.long_words` | ≥8 units in the bank's stored unit |
| `T-TIER:<tier>` | `type.tier.<tier>` | the word's bank tier |

**Primary type.** Each word in W gets exactly one primary type, using this priority (G-D17):

TRAP → SOUND → LONG → TIER

**Display rules.**
- A group with fewer than 3 words is hidden. Its words still appear in the Working on bucket and the practice list.
- At most 4 groups are shown per language.
- Groups are ordered by size descending, then by registry order.
- Each group shows its label, its word count, and up to 5 words.
- Languages are sections. Language is not a type.

### F3 — Practice list (revised per G-D10)

**Intent:** Turn the pattern into practice without the parent hunting for words.

The list is built every week, per language, with at most 10 words (G-D18). Order:

1. **Missed words from W.** Sorted by miss count descending, then most recent miss, then word id. Take up to 10.
2. **Top-up, if under 10.** Add same-language words of the same primary type, round-robin across the shown groups by group size. Every top-up row must be:
   - audited and PURITY-passing;
   - not a proper noun;
   - profanity-clean;
   - within the tier cap;
   - unseen by this profile.

   For `T-SOUND`, top-up words come only from verified sets.
3. **Tier cap.** Spell Jr: Easy and Medium only. Standard: the highest tier this profile played in that language this week.
4. **Determinism.** Selection is seeded by `hash(profile_id, week_start, language)`.
5. **No padding.** If eligible rows run out, the list is shorter. Never relax a filter to fill it.
6. **Saving.** The list is not saved until the parent confirms (F6).

If a language has no PURITY ledger, there is no top-up. The list is that language's missed words only.

### F4 — Placement

**Intent:** Parents find it where words live, and kids never find it.

- Row: "Guardian Report", in the Your Words drawer group (G-D4).
- Tapping the row passes the adult gate first.
- The row is absent under Spell Jr and under unknown age.
- The row is absent without SpellGame Complete (REPORTS-SHARE F6, absent not locked; G-D22).
- The row is absent in the Education Edition in v1 (O2).
- The app is the only surface. spellgame.net gets no row. REPORTS-SHARE's web behaviour is untouched.
- The row label is localized through the closed lexicon (G-D21). It is not a brand name.

### F5 — Report screen and share

Behind the gate, one screen per profile, with a profile picker if there is more than one.

**Sections, in order:**
1. Header with profile name and week dates.
2. Word types (F2).
3. Mastered / Working on / New this week, from REPORTS-SHARE F2 with the F1 window.
4. Practice list (F3).

**Buttons:** Add to My Words · Practice these · Share.

**Share** sends the composer's text through the OS share sheet. The F2 and F3 sections render through the closed lexicon. It stays pull only (REPORTS-SHARE D-R1).

**Empty week:** a short in-app report. The buttons are never disabled (D-R7).

### F6 — Add to My Words and Practice these (G-D9, G-D10)

**Add:**
- Writes one dated list per language through the CC-MYWORDS-LISTS API, named "Week of {Monday date}", localized (G-D19).
- Idempotency key: `(profile_id, week_start, language)`.
- A second Add writes nothing. It shows "Already in My Words" and opens that list.

**Practice these:**
1. Performs Add, which is idempotent.
2. Switches the active profile to that child.
3. Opens the CC-PICK-QUEUE lineup on the saved list (G-D20).

Practice these is absent if C7 finds no lineup or no profile switch.

No My Words write happens without a passed adult gate in the same session.

### F7 — Monday ping (G-D2, G-D8, G-D11)

**Intent:** Remind the parent the report exists. Never carry the report itself.

- Off by default. The toggle lives on the gated report screen.
- OS notification permission is requested only when the parent turns the toggle on. Never at launch.
- If permission is denied, the toggle reads off, with one line pointing to the OS settings.

**Body copy:**
- One profile: "Guardian Report ready · 8 practice words".
- Several profiles: "Guardian Reports ready · 2 profiles".
- Never a word. Never a profile name (G-D13).

**Action:** "Review & add to My Words". Foreground only. It opens `spell://guardian-report?week=<monday>`. The deep link passes the adult gate before rendering.

**Scheduling:**
- Local notifications fix their content at schedule time.
- So on every session end, and on every app foreground, cancel and re-schedule one non-repeating notification for the next Monday 08:00 local, with the current count.
- No sessions in a week means nothing is scheduled. An empty week gets no ping (G-D12).

**Device scope:** v1 fires on the play device only (G-D11). Reaching a parent's separate phone waits for accounts and CC-GUARDIAN-EMAIL.

**One notification:** this ping and the CC-GUARDIAN-DASH F2 weekly digest are one notification. If DASH F2 is built later, it reuses this scheduler and this copy rule (I11).

---

## Decisions

### Signed (Eric, Oct 26 2026)

- **G-D1:** Monday–Sunday calendar week. Resolves REPORTS-SHARE D-R6.
- **G-D2:** v1 delivery is pull via the share sheet plus an opt-in Monday ping. Keeps REPORTS-SHARE D-R1.
- **G-D3:** Email is a later, separate file, link only. It is gated on parent accounts (CC-ONBOARD-JR Phase B) and the consent mechanism (CC-SECURITY-BASELINE O2, which counsel decides). SMS never. **Do not re-propose SMS.**
- **G-D4:** The row lives in the Your Words drawer group, behind the adult gate.
- **G-D5:** v1 types use existing signals only. English pattern tags come later (Phase E).
- **G-D6:** Rides SpellGame Complete (REPORTS-SHARE D-R4).
- **G-D7:** The report never says "struggled". "Working on" is the label. It joins the F3 prohibited list.
- **G-D8:** The notification never contains a word.
- **G-D9:** Import happens only after the adult gate plus an explicit Add. No background notification action.
- **G-D10:** The practice list is not saved until the parent confirms. This revises the first-pitch F3.
- **G-D11:** v1 pings on the play device only.

### Defaults applied, unsigned

- **G-D12:** An empty week gets no ping. The in-app short report still shows.
- **G-D13:** No profile names on the lock screen. Several profiles share one notification. This amends CC-GUARDIAN-DASH F2's sample copy ("Maya mastered…").
- **G-D14:** Bucket by the device's current IANA zone at compose time.
- **G-D15:** The week starts on Monday in every locale.
- **G-D16:** Excluded from W: voided rescue rounds, Spell It voice results, Impossible-tagged attempts, close and valid_other_sense outcomes.
- **G-D17:** Primary-type priority TRAP → SOUND → LONG → TIER. Groups under 3 words are hidden. At most 4 groups per language.
- **G-D18:** Per language, at most 10 words: missed words first, then unseen same-type top-up, no padding. Tier cap: Jr Easy/Medium; standard, the highest tier played that week.
- **G-D19:** One dated My Words list per language per week.
- **G-D20:** Practice these performs Add, switches to the child, and opens the lineup.
- **G-D21:** "Guardian Report" is localized. It is not a brand name.
- **G-D22:** Absent, not locked, without Complete.

### Open

- **O1:** Whether pattern tags extend beyond English after Phase E.
- **O2:** Education Edition. Default: absent until rostering exists (CC-GUARDIAN-DASH D4).
- **O3:** Ping time. Default: fixed at 08:00. Alternative: parent-chosen.

---

## Invariants

- **I1:** Read-only over `LearnerQuery`. Any write from this module to learner state fails the build.
- **I2:** Zero network egress. A symbol scan of the module finds no network API.
- **I3:** Words and integer counts only. No percentages, grades, rankings, or screen-time and session metrics.
- **I4:** Unreachable from any Jr or unknown-age profile, including by deep link.
- **I5:** Every practice word resolves to an audited, PURITY-passing, non-proper-noun, profanity-clean bank row within the tier cap.
- **I6:** No cross-child comparison anywhere.
- **I7:** Notification payloads contain no bank row text and no profile name.
- **I8:** No My Words write without a passed adult gate in the same session.
- **I9:** Add is idempotent per `(profile_id, week_start, language)`.
- **I10:** The composer is pure: clock, zone and bank view are injected. The same inputs give byte-identical output.
- **I11:** At most one Guardian notification is pending per device.
- **I12:** Each word in W appears in exactly one type group, or in none if its group is hidden.
- **I13:** Every type label comes from the closed registry. No free text.

---

## Non-goals

Do not touch:
- the learner engine, FSRS scheduling, the missed-words queue, or the confusion matrix;
- bank rows: no edits, no tier changes;
- Climb, Daily, streaks, shields, or scoring.

Do not add:
- a server, stored phone numbers or emails, or email/SMS sending;
- a new diagnosis model (that belongs to CC-LEARNING-DIAGNOSIS);
- a teacher class rollup;
- a web surface;
- a PDF (that belongs to CC-GUARDIAN-DASH);
- telemetry.

---

## Phases

- **A — Composer.** F1 window, F2 type registry, F3 practice list builder. Pure functions with golden files.
- **B — Screen.** F4 row, F5 screen, extended Share.
- **C — Import.** F6 Add and Practice these.
- **D — Ping.** F7 scheduler, permission flow, deep link, and DASH F2 merge.
- **E — English pattern tags (design-ahead, blocked).** A closed rule set: silent letters, double consonants, -tion/-sion, vowel teams. Tags are derived from CC-EN-EXPERT-GROWTH pronunciation keys. It is blocked on that file's Phase A plus Eric signing the rule list. Tags become new `T-PATTERN:<id>` registry rows, slotted between SOUND and LONG.

---

## Acceptance tests (CI unless marked)

**Week window**
- **A1:** In the America/Indiana/Indianapolis zone, an attempt at Sun 2026-10-25 23:59:59 lands in the week of Oct 19. Mon 2026-10-26 00:00:00 lands in the week of Oct 26.
- **A2:** The DST fall-back on Sun 2026-11-01: two attempts at 01:30 (once at EDT, once at EST) both land in the week of Oct 26, each counted once.
- **A3:** Under `fr-FR`, `en-US`, `ar` and `ja` locales, the week starts on Monday.

**Composer and lexicon**
- **A4:** Golden files: the same fixture gives byte-identical output across 3 runs and both platforms.
- **A5:** The prohibited-lexicon test passes in all 15 languages and includes "struggled" and its translations.

**Access**
- **A6:** A Jr profile and an unknown-age profile show no row. The deep link is refused.
- **A7:** Without Complete, the row is absent, not greyed.
- **A8:** The symbol scan finds no network calls in the module.

**Word types**
- **A9:** A type group with 2 words is hidden. Those words still appear in Working on and in the practice list.
- **A10:** A word that is both a trap word and long sits only in its `T-TRAP` group.
- **A11:** Every rendered type label key exists in the registry. A lint fails on any other key.
- **A12:** Excluded outcomes (voided, Spell It, Impossible, close) never appear in W.

**Practice list**
- **A13:** Every practice word passes I5. A Jr list contains no Hard or Expert rows.
- **A14:** Top-up never includes a word in the profile's seen set.
- **A15:** With the PURITY ledger removed for a language, top-up is off and the list equals the missed words.
- **A16:** With eligible rows exhausted, the list is shorter than 10 and no filter was relaxed.

**Import**
- **A17:** Tap Add twice. Exactly one "Week of Oct 19" list exists per language, with no duplicate words.
- **A18:** Open the action and dismiss the gate. My Words is unchanged.
- **A19:** Imported words keep their language and NFC form.

**Ping**
- **A20:** Every scheduled payload, scanned against the bank and against profile names, gives zero matches.
- **A21:** No permission prompt before the toggle is turned on. Permission denied leaves the toggle off.
- **A22:** A week with zero sessions leaves no pending notification.
- **A23:** Two profiles give one pending notification, with the multi-profile copy.

**Device (Eric, TestFlight)**
- **A24:** On Eric's phone:
  1. Play several days.
  2. On Monday at 08:00 the ping arrives with a count and no words.
  3. The action opens the gate, then the report.
  4. Add creates the list.
  5. Practice these opens the lineup.
  6. Share sends readable text through Messages and Mail, with non-Latin scripts intact.

---

## Done

1. §0 is reported and Eric has reviewed the C10 dry run.
2. A1–A23 pass in CI.
3. A24 passes on Eric's phone.
4. No signed decision from another file was reversed.

## Stop and ask

Stop and ask if any of these comes up:
- a conflict with REPORTS-SHARE D-R1;
- a conflict with CC-ONBOARD-JR D13 (no phone);
- a conflict with CC-LEARNING-ENGINE D5 (zero telemetry);
- a conflict with CC-GUARDIAN-DASH D2/D3;
- any need for a new learner field;
- any need for network access.

Never infer a reversal.

---

<!-- Appended 2026-10-27 by the Phase A–D build. Everything above this rule is
the signed text, unedited; everything below is a record of the build. -->

## As built (Phases A–D, 2026-10-27)

**Phase A — composer.** `src/guardian_report.rs`, pure, on top of `reports_share::compose` (it adds two fields to each language section and does not fork the composer).
- **W** is the miss-class attempts in the window (Misspelled, TimedOut, GaveUp, Trap), excluding Speech, Impossible and any attempt without its CR-1 instant or class. There is no close class (L-D2).
- **Types** follow the closed registry in priority order. Only bank rows get SOUND, LONG or TIER, so a missed word from a player's own list is in no group (I12 allows that). SOUND reads only `kind: "true"` sets of a `verified` table (en, es, ru today). **T-TRAP is built but never fires:** nothing records a `TRAP_MISS(rule_id)` yet (census C5).
- **Practice list:** missed words first, filtered to bank rows within the tier cap that are clean (not a proper noun, profanity-clean, kid list for Jr). **Reading of I5:** PURITY and the audited flag bind the top-up rows. Missed words are exempt from those two, because A15 says that without a ledger the list *equals* the missed words. Flag this if you read I5 as binding missed words too.
- **Top-up is built, tested and OFF everywhere.** No bank has a per-row audited flag (census C4), and the filter may not be relaxed (F3.5). `config/guardian-topup.json` holds a per-language auditor claim, all empty, the same pattern as SENSE-CUE D9. Naming an auditor for a language turns its top-up on.
- **PURITY** comes from `config/guardian-purity.json`, generated from the ledgers and checked against them in CI. A row with no verdict counts as not passing. Three zh bank rows have no verdict: `jie1zhu4|接著`, `ta1|牠` and `you3zhu4|有著`, all traditional characters in the simplified bank. vi has no ledger, so it gets no top-up.
- **Seed:** FNV-1a over `profile|monday|lang`, so it is stable across builds and platforms.

**Phase B — screen.** The REPORTS-SHARE row (`progress_report`) now uses `greport.name`, "Guardian Report", localized (G-D21). The title and row keys `rshare.name`, `rshare.desc` and `rshare.title` are retired. The screen shows, per language, Word types, then the three buckets, then the practice list. Share carries the same text.
- 21 `greport.*` strings in all 15 locales. 20 are machine drafts in `needs_audit`; `greport.group` is a frame, held identical.
- **No profile picker and no name in the header:** there is one profile, the device (census C7).

**Phase C — import.**
- **Add** writes one list per language through `word_lists::create_keyed` with key `guardian:0:<monday>:<lang>`, then shows "Added to My Words" or "Already in My Words" on the My Words screen. The app's only toast reads "Achievement —", so a small note line was added there instead.
- With several languages, each list's name carries the language name ("Week of Oct 19 · Español") rather than a "(2)" suffix.
- Add requires a gate passed in this app session (I8). It is checked inside the write, not only by the screen being behind the gate.
- **Practice these is absent:** there is no profile switch (census C7; the spec's own rule).

**Phase D — ping.**
- **One notification, id 3, which is DASH F2's id.** The report screen toggle and the dashboard's "Weekly digest" toggle are the same opt-in (`spell_digest_on`), so existing opt-ins carry over. That makes the merge visible to them:
  - It now fires Monday 08:00 instead of Sunday 17:00.
  - It is one-shot, rescheduled, instead of repeating.
  - It is skipped for an empty week.
- **Copy:** the Guardian copy and action when the report row is reachable. Otherwise DASH's own count line ("{n} new words mastered this week"), with no action, so a device without Complete is never sent to a row it does not have.
- **Permission:** asked only when a toggle is turned on, including the dashboard's, which used to schedule without asking. A refusal reads off with one line on the report screen, and a toast on the dashboard.
- **Rescheduled** at boot and on every `visibilitychange`, which covers both foreground and leaving the app (session end).
- **Deep link:** iOS `AppDelegate` now forwards the query (`spellgame://guardian-report?week=…` → `#guardian-report?week=…`), with host and query kept to `[A-Za-z0-9=&_-]` because they are written into a JS string. The router hands the launcher the route, so the report opens on the named week. The notification action sets the same hash, so it passes the gate the same way.

**Acceptance coverage.**
- **Host tests:** A1–A3, A4 (host only, not "both platforms"), A5 over the new strings, A8–A16, A20, A22 and A23.
- **e2e:**
  - REPORTS-SHARE's access tests cover A6 and A7.
  - The new tests cover A17–A19 and A21, plus a live A20/A22 and I11 against a recording notification plugin.

**Not covered:**
- A5's "translations of struggled": the non-English prohibited lists are still English-seed drafts awaiting the native pass.
- A23 live: there is one profile.
- A24, the device run on your phone.
