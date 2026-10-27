# CC-REPORTS-SHARE v1 — Parent report composer, closed lexicon, pull-only share

**Status:** DECISIONS SIGNED. D-R1, D-R4 and D-R6 to D-R10 were signed as recommended (Eric, 2026-10-26: "I like the recs they seem reasonable", then "record D-AE1 to D-AE4 and D-R1, D-R4, D-R6 to D-R10 as signed as recommended"). Nothing is built yet.
**Why it exists:** CC-GUARDIAN-REPORT v1, CC-SNAP-ROADMAP and CC-CONTRIBUTE all cite this file, and until now it existed nowhere (see `docs/CC-REPORTS-SHARE-gap-note.md`). This v1 is written to the shape those files already assume. It is small on purpose, because most of what it needs is already built.
**Base:** origin/main `5ffeb19d`.
**Depends on:**
- L0 CR-1 (instants, outcome classes, seen set). Built.
- CC-HUB-NAV AE-1 (adult-only rows, Edu strip, gated routes). Draft.

---

## Intent

A parent should be able to take one week of their child's practice somewhere else — Messages, Mail, a teacher — as plain text they can read. It is written in the app's words and nobody else's, and it is pulled by the parent, never pushed.

**Children's app first:** when unsure, show less, store less, and send nothing.

## Ownership

| Concern | Owner |
|---|---|
| Learner data | L0 `LearnerQuery`, read-only. Gaps go to L0. |
| Row visibility, deep links, adult gate | CC-HUB-NAV AE-1 |
| Entitlement | `entitlements.rs`, via `requiresPremium: progress_reports` |
| Share sheet | `share::share_text`, the one share gate. It already refuses in Kid Mode (`share.rs:55-72`). |
| PDF | CC-GUARDIAN-DASH. Not this file. |
| Types, practice list, Monday ping | CC-GUARDIAN-REPORT, which extends this file. |

---

## Features

### F1 — The composer

```rust
pub struct ComposeInput<'a> {
    pub q: &'a dyn LearnerQuery,
    pub profile: ProfileId,
    pub langs: &'a [&'a str],
    pub now_s: u64,          // injected clock
    pub zone: &'a dyn Zone,  // injected IANA zone (offset at an instant)
    pub ui_lang: &'a str,
}
pub fn compose(input: &ComposeInput) -> Report;  // pure
pub fn render_text(r: &Report) -> String;        // pure, closed lexicon only
```

- **Purity:** the same inputs always give byte-identical output. No storage, DOM, clock or network is touched inside.
- **What it reads:** `attempt_detail_log`, `seen_set`, and the redemption store. Redemptions reach the composer through a new additive `LearnerQuery` method, `redemptions(p, lang)`. Today they are read directly in `reports.rs:24`, which is allowed there, but the composer stays contract-only (L0 I6).
- **Unknown fields:** an attempt with `ts: None` or `outcome: None`, meaning it was written before CR-1 or rewritten by an older build, never enters a window. It is unknown, not guessed (CR-1).

### F2 — Three buckets per language, for one window

| Bucket | Definition |
|---|---|
| **Mastered** | Redemptions whose `mastered_ts` falls in the window |
| **Working on** | Words with at least one miss-class outcome in the window that are not mastered in it. Miss-class means Misspelled, TimedOut, GaveUp or Trap. |
| **New this week** | Words whose **first** logged attempt with a `ts` falls in the window |

- **A word sits in at most one bucket.** Mastered beats Working on, which beats New.
- **Excluded from every bucket:** Speech-channel attempts and Impossible-flagged attempts. Voided rounds never log at all.
- **The "New" bucket is bounded by the log ring** (512 per language, L-D5). A word whose earlier attempts have scrolled out reads as new. That's the honest limit, and an upper bound on the count.
- **Counts and words only:** no percentages, grades, rankings or session metrics (R-I3).

### F3 — Closed lexicon

- **Every word of report text** comes from i18n keys in one pool, `rshare.*`, in all 15 languages. Words from the bank are the only other text. The renderer has no `format!` with free prose, and a lint fails on any key outside the pool.
- **Prohibited list:** `config/report-prohibited.json`, one list per language. The English seed is:

  > struggled, struggling, failed, failure, wrong, behind, worse, weak, poor, bad, error, %

  These are the existing kid-pool bans (`reports.rs:251`) plus GUARDIAN G-D7's "struggled".
- **The lint** fails if any `rshare.*` string, rendered with any fixture, contains a prohibited token for its language. It also fails if any language has no list. **The other 14 lists need a native pass** (see D-R8). An English list copied into a locale doesn't count, because presence is not translation, so `i18n-translated-check` applies to the lists as well.
- **"Working on"** is the label for the in-progress bucket in every language. It is never "struggled" (G-D7).

### F4 — Window

- **Default:** the calendar week, Monday 00:00:00 to Sunday 23:59:59 local. Each `ts` is converted with the injected zone at compose time.
- **Week start:** Monday in every locale (D-R6, following GUARDIAN G-D1/G-D14/G-D15).
- **"This week so far"** is available anytime and is labelled as partial.
- **The header** shows explicit dates in the uiLang date format.

### F5 — Share (D-R1)

- **Pull only.** It goes through `share::share_text`, the OS share sheet. There is no server, no email and no scheduled send.
- **It carries no profile name** unless the parent typed one into the report screen for this share. Nothing is stored.
- **Absent where it can't run.** If `share::available()` is false (Kid Mode, no share mechanism), the button is absent, not disabled.

### F6 — Placement and access

- **The row** is "Progress report", in the Your Words drawer group, through CC-HUB-NAV AE-1, with:
  - `adultOnly: true`
  - `requiresPremium: "progress_reports"` (D-R4)
  - `editions: ["consumer"]`
  - `platforms: ["ios"]`
- **The adult gate** (`parent_gate_then`) runs before rendering, and before any share.
- **Not on the web** in v1: `strip_app_only_modes` removes the row.

### F7 — Empty week (D-R7)

- A week with no qualifying attempts renders a short report: the header plus one line from the pool.
- Share still works and still sends that short text. Buttons are never disabled for emptiness.

---

## Decisions (signed as recommended, Eric 2026-10-26)

| # | Decision | Signed |
|---|---|---|
| **D-R1** | Delivery | **Pull only via the OS share sheet.** No push, email, SMS or server. The rest of the tree already practises this. |
| **D-R4** | Entitlement | **It rides SpellGame Complete** (`progress_reports`) and is absent, not locked, without it. |
| **D-R6** | Window | **Monday–Sunday calendar week,** in the device's current zone at compose time. |
| **D-R7** | Empty week | **A short report,** with buttons never disabled. |
| **D-R8** | Prohibited lists for the 14 non-English languages | **Draft from the English seed, then a native pass per language.** The lint blocks shipping a language until its list passes `i18n-translated-check`. |
| **D-R9** | DASH week start | **Move `reports::week_over_week` and the DASH digest count from Sunday to Monday weeks,** so the app has one week definition. That changes DASH's visible numbers, so it's yours to sign. |
| **D-R10** | Row name | **"Progress report",** localized, and not a brand name. GUARDIAN-REPORT later renames or extends it to "Guardian Report" as its F4 specifies. |

## Invariants

- **R-I1.** Read-only over `LearnerQuery`. Any write fails the build.
- **R-I2.** Zero network egress. A symbol scan of the module finds no network API, and the only way out is `share_text`.
- **R-I3.** Words and integer counts only.
- **R-I4.** Every rendered string is a `rshare.*` key or a bank word. The lint enforces it.
- **R-I5.** No prohibited token reaches rendered text in any language.
- **R-I6.** Unreachable from Junior or Unknown, including by deep link (AE-1 AE-I1/AE-I3).
- **R-I7.** The composer is pure. The same inputs give byte-identical output.
- **R-I8.** No cross-child comparison.

## Acceptance (CI unless marked)

1. **Golden:** a `Fake` fixture composes and renders byte-identically across 3 runs.
2. **Window:** in America/Indiana/Indianapolis, Sun 2026-10-25 23:59:59 falls in the week of Oct 19 and Mon 00:00:00 in the week of Oct 26. The DST fall-back at 01:30 is counted once per instant.
3. **Buckets:** a word mastered and missed in the same week appears only in Mastered. Speech, Impossible and `ts: None` attempts appear nowhere.
4. **Lexicon lint:** an injected free-text string fails it, and a key outside `rshare.*` fails it.
5. **Prohibited lint:** a planted "struggled" fails it in en. A language with no list fails it.
6. **Egress scan:** no network symbol in the module.
7. **Access** (via AE-1 A1/A5): Junior and Unknown see no row, and the deep link is refused. With Complete off, the row is absent.
8. **Empty week:** a short report renders, and Share is present.
9. **Device (Eric, TestFlight):** Share sends readable text through Messages and Mail, with non-Latin scripts intact.

## Phases

- **A — Composer.** F1, F2, F4, with golden files. Pure, so it needs no UI.
- **B — Lexicon.** F3: the `rshare.*` pool in en, the prohibited lint, and the D-R8 native pass running alongside.
- **C — Screen and share.** F5, F6, F7. Needs AE-1.

GUARDIAN-REPORT's Phases A–D then extend these. Its F2 types and F3 practice list add sections to `Report`; they don't fork the composer.

## Non-goals

- No PDF (DASH has it).
- No web surface.
- No email, SMS or server.
- No telemetry.
- No new learner fields.
- No teacher rollup.
- No change to the kid Quest Log (CC-REPORTS).
