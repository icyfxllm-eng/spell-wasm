# Script Paths - curricula DRAFTS + auditor handoff (REVIEW-GATED)

**Status: DRAFTS for Eric's red pen, then native-auditor review. Nothing here bundles
or ships.** These are the two time-critical *review artifacts* for the Script-Path
feature so they can ride the **current Fiverr audit round**. The engine, schema CI,
path UI, entitlement wiring, and all Rust/JS are **DEFERRED until after CC-ENTITLEMENTS
ships** - none of that is in scope here. This is drafting + packaging, not engineering.

## What a "Script Path" is (and is not)

A Script Path teaches a true beginner to **READ and SPELL a writing system**: recognize
the script's elements (letters / jamo / diacritics / pinyin pieces), then spell **real,
already-audited words** with them, in a pedagogically sane order.

- **In scope:** the writing system only.
- **NOT in scope:** grammar, word meanings, comprehension. It is not a language course.
- **Words are referenced, never created.** Every `spell` step cites a real word by its
  exact text + tier drawn from `assets/words/<lang>/`. No invented words, no duplicated
  audio or metadata.

## Files in this folder

| File | What it is |
|------|-----------|
| `th.json` `ko.json` `vi.json` `zh.json` | DRAFT curricula: units -> lessons -> steps (`element` or `spell`). Strictly linear; each lesson lists prerequisite lesson IDs. Stable/sorted for diff hygiene. |
| `th-SEQUENCE.md` `ko-SEQUENCE.md` `vi-SEQUENCE.md` `zh-SEQUENCE.md` | One-page human-readable sequence summaries for a native auditor: element order, rationale, first ~20 words, flags. |
| `README-REVIEW.md` | This file - the Fiverr-round handoff + the auditor-checklist line. |

The JSON `schemaVersion` is `0.1-draft`. The formal word-ID wiring is an engine-phase
concern; for the draft, words are cited by real text + tier.

## Budget compliance (schema caps, verified)

Caps: `element` steps carry **<=2 instructional strings, each <=120 chars**; ~100
instructional strings per language. `spell` steps carry no strings (word reference only).

| Lang | Instructional strings | Elements | Spell refs | Over 120 chars | Over 2 strings |
|------|----------------------:|---------:|-----------:|:--------------:|:--------------:|
| Thai (th)            | **57**  | 51 | 87 | 0 | 0 |
| Korean (ko)          | **40**  | 39 | 89 | 0 | 0 |
| Vietnamese (vi)      | **48**  | 45 | 100 | 0 | 0 |
| Chinese pinyin (zh)  | **64**  | 63 | 0 (see flag) | 0 | 0 |

All four are within the ~100-string budget. Card strings are **English source text**,
to be localized later.

---

## DELIVERABLE 1 - the auditor-checklist line (feature 7)

Add this **exact one line to EVERY one of the 16 auditor checklists**:

> **Does the difficulty progression make sense as a learning order for a beginner in this script/spelling system? Note anything out of order.**

The 16 audited languages (each has a word list under `assets/words/` and an auditor
checklist / packet):

`de` `en` `es` `fil` `fr` `it` `ja` `ko` `nb` `nl` `pl` `pt` `sv` `th` `tr` `vi`

(15 of these currently have an `audit/<lang>/auditor-packet.md`; `en` is the 16th. The
line above goes into all 16, regardless of packet state.)

### The 4 pilot languages get an extra named section

For **Thai, Korean, Vietnamese, and Chinese (pinyin)**, the auditor ALSO receives, as a
**named checklist section in their packet**, that language's:

- one-page sequence summary (`<lang>-SEQUENCE.md`), and
- the Script-Path card strings for that language (the `strings` in `<lang>.json`).

Same milestone-payment terms as the rest of that language's audit. The auditor is asked
to review the **element order + card strings**, on top of the difficulty-progression
line above. (Chinese pinyin has no `spell` words yet - see the zh flag - so its section
is element-order + card-strings review only.)

---

## DELIVERABLE 2 - the four pilot curricula (per-language notes)

### Thai (th) - flagship
Element order: **mid-class consonants -> high-class -> low-class -> vowels -> tone
marks + silencer**, then first ~20 words, then difficulty progression by tier. Teaches
the 30 consonants that occur in the audited th lists (5 rare letters deferred - noted
in the summary). 57 strings, 87 word references.
**FLAG (decision 12):** Thai consonant NAMES ("ko kai", "kho khai" ...) may sound poor
through the current TTS voice; **recorded human audio may be needed** for the consonant
intro cards. Flagged only - nothing commissioned.

### Korean (ko)
Element order: **basic consonants (jamo) -> basic vowels -> syllable-block composition +
batchim -> tense/complex jamo**, then first ~20 words, then progression. 40 strings,
89 word references.

### Vietnamese (vi)
Element order: **base Latin letters -> special letters (đ ă â ê ô ơ ư) -> digraphs/
trigraphs -> the 5 tone diacritics**, then first ~20 words, then progression. 48
strings, 100 word references. Cards stress that horned/circumflex letters are separate
letters, not tones.

### Chinese - pinyin ONLY (zh)
Element order: **initials -> finals -> tone marking (4 tones + neutral + placement
rule)**. Pinyin only - **no hanzi, no stroke order, no drawing input** (retired
app-wide, stays retired). 64 strings, full element inventory.
**BLOCKING FLAG:** `assets/words/zh/` does not exist - **there is no audited Chinese
word list**, so zh has **zero `spell` steps**. The element cards + structure are drafted
in full, but zh needs audited word content before its spelling steps can exist. Proposal
to Eric is in `zh-SEQUENCE.md` (commission a tiered, audited pinyin word list; the
spelling units then drop in with no structural change). No words invented.

---

## Reminder for reviewers

These are **DRAFTS**. Expected review path: Eric red-pens -> native auditors review
(element order via the checklist line + the pilot sections). Only after that, and only
after CC-ENTITLEMENTS ships, does any engine/schema/UI work begin. Nothing here is
wired to bundle or ship.
