# CC-REPORTS-SHARE — gap note

**Status:** DRAFT for Eric, 2026-10-26.
**Raised by:** CC-GUARDIAN-REPORT v1 §0 C1 (HALT). Base: origin/main `f61ca447`.

## The gap

No file named `CC-REPORTS-SHARE` exists in the repo, in any sibling checkout, or in the code. It isn't in `docs/` or `docs/census/`, and grep finds no `reports_share` anywhere. CC-CONTRIBUTE's census found the same on 2026-09-30 (`docs/census/contribute_census.md:170`).

Three files cite it as if it were signed:

| Cited by | What it relies on |
|---|---|
| CC-GUARDIAN-REPORT v1 | F1 composer, F3 closed lexicon and prohibited list, F2 Mastered / Working on / New buckets, F6 entitlement (absent, not locked), D-R1 pull only, D-R4 rides Complete, D-R6 share window, D-R7 empty week keeps its buttons |
| CC-SNAP-ROADMAP (DOC-2, gate pattern) | D-R1 share-sheet pull; "C1 HALT" pattern |
| CC-CONTRIBUTE (C5) | F6 adult gate |

## What exists that covers parts of it

| REPORTS-SHARE piece | Nearest thing on main | Fit |
|---|---|---|
| F1 composer (pure text report) | None. `reports.rs` is CC-REPORTS' kid Quest Log plus `ReportsQuery` helpers (`week_over_week`, `hesitant_words`, `confusion_pairs`). `guardian_dash.rs` renders HTML and SVG pages. | **Absent.** Nothing composes shareable report *text*. |
| F2 Mastered / Working on / New | `reports::week_over_week` counts mastered words in **Sunday** weeks (`reports.rs:67`). Redemptions hold mastered words with a timestamp. "Working on" is approximated by `LearnerQuery::at_risk_set`. No "new this week" exists. | Partial, with the wrong week start. |
| F3 closed lexicon + prohibited list | The i18n pools are lint-audited (`reports.*`, `gdash.*`). CC-REPORTS I4 bars deficit framing on kid surfaces. **No prohibited-word list exists**, and "struggled" is checked nowhere. | **Absent** as a closed lexicon. |
| F6 adult gate | `parent_gate_then` (`lib.rs:1274`), plus a duplicate in `guardian_dash.rs:254`. Worded multiplication, operands 3–8, unlimited retries. | Exists, and is weak (see the CONTRIBUTE census). |
| F6 entitlement, absent not locked | `requiresPremium: progress_reports` in `config/modes.json` → `modes::permitted` (`modes.rs:375`), tested at `modes.rs:627`. Purchase is hard-wired `false` (`play_hub.rs:120`). | Mechanism exists. No purchase path yet. |
| D-R1 pull via share sheet | `share::share_text` is the one gate, and it refuses in Kid Mode (`share.rs:55-72`). GUARDIAN-DASH already shares a **PDF** via `share_file` (`guardian_dash.rs:155`), although its header says "no share affordance". | Exists. The DASH header comment is stale. |
| "Today they can export a list of words" (GUARDIAN intent) | No text export of a word list was found. The parent-facing exports are the DASH PDF and the Yearbook PDF. | The premise is **not true as written.** |

## Options

1. **Write REPORTS-SHARE v1 now, small.** Scope it to the composer (pure, text, injected clock, zone and bank view), the closed lexicon with a prohibited list, the three buckets, and the decisions GUARDIAN cites (D-R1, R4, R6, R7). Then GUARDIAN-REPORT extends it as designed. **Recommended.** GUARDIAN's ownership table is already built around this split, and the ideas behind those decisions (pull only, absent not locked) are already practised in the code.
2. **Fold the composer into GUARDIAN-REPORT Phase A.** Change its ownership row and drop "Does not fork them". This is faster, but SNAP-ROADMAP and CONTRIBUTE still point at a file that doesn't exist.
3. **Re-point GUARDIAN-REPORT at CC-GUARDIAN-DASH.** DASH already has the gate, the entitlement and a share path, but it's PDF and HTML, and GUARDIAN's non-goals bar a PDF.

## Questions only Eric can answer

- Were D-R1, D-R4, D-R6 and D-R7 signed in a chat that never reached the repo? If so, paste the text and it gets stashed verbatim, as L0 was.
- Week start: `week_over_week` uses Sunday weeks today. GUARDIAN G-D15 says Monday everywhere. Should the DASH row move to Monday too, so there's one week definition?
- Is the DASH PDF share, live today, consistent with D-R1? It's pull through the share sheet, so it probably is.
