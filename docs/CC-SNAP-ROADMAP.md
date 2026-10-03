# CC-SNAP-ROADMAP v1 — From a photo, Levels 1–5

> **Provenance and status, added when this was committed on 2026-10-03.**
> Everything below the rule is Eric's, verbatim as written. It had lived only
> in a chat session since 2026-10-01, which meant D1 and D7 were signed
> decisions with nowhere to point and three files named a layering parent that
> did not exist. That is the only reason this note exists; nothing in the
> document has been edited.
>
> Where things stand against it:
>
> - **Level 1.1** CC-SNAP-CLEAN v1 — shipped.
> - **Level 1.2** CC-SNAP-CLEAN v1.1 — built 2026-10-01, with auto-split
>   switched OFF pending Eric. See `docs/CC-SNAP-CLEAN-v1.1.md`.
> - **Level 1.3** CC-SNAP-HIGHLIGHT v1 — census still HALTs. See
>   `docs/CC-SNAP-HIGHLIGHT.md`.
> - **The gap this roadmap had**: Level 1 named CC-SNAP-LIST v1 as existing,
>   which is true and did not help — the word-box plumbing both Level 1 files
>   need is a CHANGE to it that no file owned. `docs/CC-SNAP-BOXES.md` now
>   owns it; Phases A, B and C are built and shipped in build 260.

---

**Status:** REVIEW-GATED. This is a roadmap and ownership map, not an executable file. Each level below becomes its own CC file; only the level files are executable.
**Layering:** sits above CC-SNAP-LIST v1 (capture, crop/rotate, language tagging, review screen) and CC-SNAP-CLEAN v1 (line cleanup). Neither is changed by this file.
**Owns:** the level order, the dependency gates between levels, and the doctrine every level file must obey. Nothing else.

---

## Intent

The camera is SpellGame's bridge between the real world and the audited bank. A child's homework, a book, a sign on a trip, a textbook chapter — any of these should become playable words, while every word still passes through the same audit gates as the shipped banks.

The product thesis, in one line: **nothing a player sees from a photo is less trustworthy than what they see from the bank.**

When a level file is incomplete, prefer the choice that:
1. keeps the word anchored to an audited bank row,
2. keeps the photo on the device,
3. keeps Spell Jr behind a parent, and
4. routes every new text path through `clean_ocr_lines` (CC-SNAP-CLEAN F0).

---

## Doctrine (binding on every level file)

- **DOC-1 Bank anchor.** A word enters a game mode only when it matches an audited bank row. An unmatched word lives in My Words only: no audio claim, no definition, no tier, no mode fan-out. CI gate: a symbol scan finds no mode-eligibility path that reads My Words rows without a `bank_id`.
- **DOC-2 On-device only.** Photos, crops, word boxes, highlight masks, classifier outputs: none of it leaves the device. No upload, no cloud OCR, no cloud vision. The only thing that can leave is a word list, and only via the existing share-sheet pull (CC-REPORTS-SHARE D-R1 pattern). Symbol scan + egress scan per level.
- **DOC-3 Zero telemetry.** No events about what was captured, stripped, matched or imported. Inherits CC-LEARNING-ENGINE D5 and CC-TELEMETRY-FOUNDATION v1.1 I3.
- **DOC-4 Spell Jr.** The camera flow is unreachable from a Jr profile. Parents capture; the child plays. Unknown age = Jr.
- **DOC-5 One entry point.** All text from any capture path (worksheet, book, sign, object label, handwriting) goes through `clean_ocr_lines` before anything else sees it. No level adds a second cleanup.
- **DOC-6 Suggest, don't silently alter.** Any transformation beyond CC-SNAP-CLEAN's signed defaults (lemmatization, segmentation in suggest-only languages, label choice) is offered as a one-tap action and recorded in `Candidate.flags`, never applied without a visible trace.
- **DOC-7 Premium.** The whole feature family sits under SpellGame Complete (July 2026 monetization, "OCR photo lists"). Education Edition unlocks it per seat. Free tier never sees a locked camera button: absent-not-locked.
- **DOC-8 Stop-and-ask.** A level file that finds it needs to reverse any signed decision in CC-SNAP-LIST, CC-SNAP-CLEAN, CC-MYWORDS-LISTS, CC-ONBOARD-JR, CC-LEARNING-ENGINE-L0 or CC-TRANSLATE-SCREEN halts and reports. It never infers the reversal.

---

## Level 1 — Capture cleanly

**Files:** CC-SNAP-LIST v1 (exists), CC-SNAP-CLEAN v1 (exists), CC-SNAP-CLEAN v1.1 (unbunching, to write), CC-SNAP-HIGHLIGHT v1 (to write).

**Intent:** a clean list imports with zero edits; a messy one needs the fewest taps possible.

### 1.1 Line cleanup — CC-SNAP-CLEAN v1
Already specified. Markers, punctuation, split/drop, bank canonicalization, flags.

### 1.2 Unbunching — CC-SNAP-CLEAN v1.1
Merged strings (`stucktogetherlikethis`) become their printed words.
- **Gate:** CC-SNAP-LIST must pass per-word bounding boxes from Vision / ML Kit into the core. Census C1 of v1.1 HALTs if only line text arrives.
- **Order:** (a) split on box gaps, since that is reading the page, not guessing; (b) dictionary split only for strings still merged after (a).
- **Dictionary split rule:** split only when the whole string is not a bank word, every piece is a bank word, and the best split beats the runner-up on piece count (fewer wins). Otherwise, suggest.
- **Per-language policy (recommended, open):** auto-split with one-tap Rejoin for en/es/fr/pt/pl/ru/fil; suggest-only for de/ko/vi (compounds and spacing conventions are real words); never segment zh/ja.
- **Invariant I-U1:** the concatenation of the pieces equals the input string byte-for-byte. Proptest.
- **Invariant I-U2:** a string that is itself a bank word is never split (`therapist`, `nowhere`, `carpet`).

### 1.3 Highlighted words only — CC-SNAP-HIGHLIGHT v1
A reader marks the words they don't know; only those import.
- OCR the page, keep word boxes; per box, mask the dark text pixels and sample the background; compare saturation to the page's paper estimate (median of non-text pixels across the page).
- A word is highlighted when ≥50% of its box exceeds the saturation threshold. No ML model.
- Rejoin hyphenated line breaks (`remem-` + `ber`) before cleanup.
- Review opens on "Highlighted (N)" when N > 0, with "All words" one tap away.
- Multiple highlighter colors → color chips, all selected; "yellow only" is one tap.
- **Out of v1:** pen underlines, circles, margin notes.
- **Acceptance:** 5 yellow words on a 300-word page → exactly 5; an aged yellow page under a warm lamp with no highlights → 0; a highlight wrapping two lines → one entry per word; an e-reader screenshot works the same.

**Level 1 done when:** CC-SNAP-CLEAN v1's done-check passes; v1.1 proptests pass; the highlight acceptance set passes on a 20-photo fixture folder Eric assembles (10 worksheets, 5 book pages, 5 e-reader screenshots).

---

## Level 2 — Understand the page

**Files:** CC-SNAP-LAYOUT v1, CC-SNAP-TAP v1, CC-SNAP-LEMMA v1 (to write). All three are independent of each other and can land in any order.

**Intent:** import only the spelling words, not everything on the page, and anchor each one to the bank.

### 2.1 Layout detection — CC-SNAP-LAYOUT
Recognize common worksheet shapes and pick the right column or token:
- numbered list (already handled by CLEAN)
- two-column `word — definition` or `word | sentence` (import left column only)
- sentence list with the target word bold, underlined or in a box (import the emphasized token only; emphasis read from the OCR engine's style hints where available, else from stroke weight)
- fill-in-the-blank (`The ___ sat on the mat.` → import nothing from that line; set `Blank`)
- Detection is per page, from box geometry (column alignment, consistent x-origins). Confidence below threshold → fall back to plain list import with every line present.
- **Invariant:** layout detection never removes a line; it only changes which lines start checked. (Extends CLEAN I6.)

### 2.2 Tap-to-collect — CC-SNAP-TAP
Show the photo with every OCR'd word as a tappable region; tapped words become the list.
- This is the preferred path for books. Highlight detection (1.3) is the no-hands version; tap is the precise version.
- Long-press a word → see it cleaned + bank-matched before committing.
- Works on the cropped image from CC-SNAP-LIST; no new capture UI.

### 2.3 Inflection matching — CC-SNAP-LEMMA
`running` in a book maps to bank `run`; `кота` maps to `кот`.
- **Gate:** depends on the bank census already blocking CC-RU-ORTHO §0 D3 and CC-TRANSLATE-SCREEN D11 (does the bank hold inflected forms or lemmas only?). This file does not re-run that census; it waits for it.
- Offered as a one-tap swap (DOC-6), never auto-applied. The scanned form is kept in My Words with a pointer to the bank lemma.
- Sourcing: per-language inflection tables from the dictionary authority list, never generated. Languages without a table get no lemma suggestions (absent, not locked).

### 2.4 Mixed-script pages
Already owned by CC-SNAP-LIST (per-line language tagging). Level 2 only adds: lines tagged with different languages produce separate My Words lists (one per language) rather than one mixed list. Confirm with CC-MYWORDS-LISTS; if that file's list model forbids auto-creating two lists from one import, stop and ask.

**Level 2 done when:** a 15-page fixture set (5 two-column, 5 bold-word sentence sheets, 5 book pages) imports with ≤1 manual correction per page averaged; tap-to-collect round-trips a tapped word to a bank id; lemma swap passes golden tests for en/es/ru.

---

## Level 3 — One photo, a whole week

**Files:** CC-SNAP-FANOUT v1, CC-SNAP-SCHEDULE v1, CC-SNAP-SHARE v1 (to write).

**Intent:** make the weekly spelling list the reason a family opens SpellGame.

### 3.1 Mode fan-out — CC-SNAP-FANOUT
After import, the list screen offers every mode the list qualifies for: Spell It, Spell Search, Spell Cross, SpellDoku Letters, Definition Match (when definitions are unlocked for that language).
- Eligibility is computed by each mode's **existing** gate, called read-only: CC-WORDGRID interlock count and decoy gates, CC-SPELLDOKU v1.2 F13 four-gate eligibility, definitions-dark registry flag. This file adds no new gate and edits none.
- Only bank-anchored words count toward eligibility (DOC-1). A list with 20 words of which 12 are anchored is a 12-word list for fan-out.
- Modes the list doesn't qualify for are absent, not greyed.

### 3.2 Test-date scheduling — CC-SNAP-SCHEDULE
"Test on Friday" turns a list into a dated goal.
- Date picker on the list (optional). The list's missed words are scheduled through CC-LEARNING-ENGINE-L0 R2 (FSRS) with a due-by constraint = test date − 1 day.
- **Gate:** L0 R2 must have landed. Census HALTs otherwise (same pattern as CC-REPORTS-SHARE C1).
- Local notifications only, on-device, opt-in per list. No server, no account.
- On-device streak "lists completed before test day" shown on the list screen only; never a leaderboard.

### 3.3 Teacher-shared lists — CC-SNAP-SHARE
A teacher captures once; every family imports.
- Export: a signed list payload (words + language + optional test date) as a QR code and a short code. Payload contains words only, never a photo (DOC-2).
- Import: scan QR or type code → normal review screen.
- **Gate:** short-code resolution needs a server endpoint and a sender identity. Both depend on CC-ONBOARD-JR Phase B accounts. QR-only ships first (no server, no account); short codes trail Phase B.
- Education Edition: teacher accounts can publish to a class roster. Consumer edition: QR/code only.
- No list on the server carries student identity. Ever.

**Level 3 done when:** a 20-word imported list fans out to ≥3 modes on en/es/ru; a scheduled list produces FSRS reviews that complete before the test date in the L0 simulator; QR round-trip reproduces the list byte-for-byte on a second device.

---

## Level 4 — The camera as language input

**Files:** CC-SNAP-OBJECT v1, CC-SNAP-SIGN v1, CC-SNAP-HANDWRITING v1 (to write). Highest asset and audit cost of the roadmap; sequenced last among executable levels.

### 4.1 Point-and-spell — CC-SNAP-OBJECT
Photograph an object → word in the study language → audited audio → spell it.
- On-device classifier (Vision / ML Kit image labeling). **The label set is constrained to bank rows:** a classifier label that maps to no audited bank row produces nothing. The mapping table (label → bank_id per language) is an auditor-reviewed sheet, ingested through the existing locked-sheet path.
- Ambiguous labels (`cup` vs `mug`) → show both as choices; never guess.
- Spell Jr: unreachable (DOC-4). A parent can run it and hand the phone over for the spelling step only.
- Acceptance: 50-object fixture photographed in good light → ≥80% produce the auditor-expected bank word in en; unmapped labels produce zero output, never a raw label.

### 4.2 Signs and menus — CC-SNAP-SIGN
Photograph a sign or menu → list in that language → Spell Translate's closed path.
- Reuses CC-SNAP-LIST capture + CLEAN + LEMMA. Adds only: default language = detected script, and a "Translate" affordance that opens CC-TRANSLATE-SCREEN with the word pre-searched (suggestion-commit, so a bank miss is still pre-commit).
- No OCR-of-translations: the sign's own text is the only input.

### 4.3 Handwriting check — CC-SNAP-HANDWRITING
Photograph a child's handwritten test → read the answers → mark against the chosen list.
- Platform handwriting OCR only; no custom model. Per-language quality is recorded in the census; languages below threshold are absent, not locked.
- Marking uses the same grader as the game (CLEAN → NFC → equivalence layer from CC-PLAYER-CONTRACT). Never a second grader.
- Output is a parent/teacher view only; results never write to the learner engine or missed-words (DOC-3, and the answers were not typed in-app so they carry no input provenance).
- Education Edition is the primary customer.

**Level 4 done when:** each file's acceptance fixture passes; DOC-1 scan shows every object label and sign word reaching a mode carries a bank_id.

---

## Level 5 — Research-grade (design-ahead; execution blocked on funding milestone)

**Files:** CC-SNAP-TEXTBOOK v1, CC-SNAP-SCRIPT-BRIDGE v1 (design-ahead only).

### 5.1 Textbook alignment — CC-SNAP-TEXTBOOK
Recognize a textbook chapter heading → load the matching auditor-prepared vocabulary set.
- Works only where a pre-built set exists (STARTALK critical languages, approved textbook lists). Recognition is heading-text match against a small on-device table; no cloud.
- Depends on auditors producing per-chapter sets: a Gig D in the Fiverr model, scoped and priced separately. Not in this file.

### 5.2 Script bridge — CC-SNAP-SCRIPT-BRIDGE
For hi/ar/ko/zh/ja learners: photograph a native-script page → transliterated reading list with audited audio.
- Transliteration is display-only and secondary (inherits CC-TRANSLATE-SCREEN F5 script primacy and I4).
- Serves learners who can hear a word but can't yet read the script; it is a reading bridge, not a spelling mode.
- Gated behind each language's existing audit gates (RTL gate for ar, Script Paths pilot for hi).

**Level 5 done when:** both design documents are reviewed and tied to a dated SBIR/STARTALK milestone (same binding pattern as CC-LEARNING-ENGINE-L0 D7). No code until then.

---

## Dependency map

```
CC-SNAP-LIST v1 ──► CC-SNAP-CLEAN v1 ──► CLEAN v1.1 (needs word boxes)
       │                   │
       │                   └──► SNAP-HIGHLIGHT, SNAP-LAYOUT, SNAP-TAP
       │
       └──► bank census (RU-ORTHO §0 / TRANSLATE D11) ──► SNAP-LEMMA
                                                               │
       L0 R2 (FSRS) ──────────────────────────────────────► SNAP-SCHEDULE
       mode gates (WORDGRID, SPELLDOKU F13, def registry) ─► SNAP-FANOUT
       ONBOARD-JR Phase B ──► SNAP-SHARE short codes (QR ships before)
       auditor label sheet ──► SNAP-OBJECT
       TRANSLATE-SCREEN ──► SNAP-SIGN
       PLAYER-CONTRACT equivalence ──► SNAP-HANDWRITING
       funding milestone ──► Level 5
```

---

## Decisions

**Signed (Eric, 2026-10-01):**
- **D1 Order — SIGNED.** Level 1 → 2.2 Tap + 2.3 Lemma → 3.1 Fan-out + 3.2 Schedule → 3.3 Share (QR first) → 2.1 Layout → Level 4 → Level 5. Layout detection moves after Level 3 because tap-to-collect covers most of its value sooner and cheaper.
- **D7 Share payload is words only — SIGNED.** No photo, no student identity, no progress. QR ships without any server.

**Recommended, applied unless Eric reverses:**
- **D2 Unbunching per-language policy** as in 1.2. German is the one most likely to be argued; keep it suggest-only.
- **D3 Highlight detection is heuristic, not ML.** Revisit only if the 20-photo fixture fails.
- **D4 Lemma swap is suggest-only** everywhere, including English.
- **D5 Fan-out reads existing gates read-only.** No fan-out-specific relaxation of any mode gate, ever. If a list is one word short of Spell Cross, it does not get Spell Cross.
- **D6 Scheduling is on-device, opt-in, local notifications only.**
- **D8 Object labels are an auditor-ingested mapping sheet.** No raw classifier label is ever shown.
- **D9 Handwriting results never enter the learner engine.**
- **D10 Entire family premium** under SpellGame Complete; Education Edition per seat.
- **D11 Naming.** "From a photo" stays the drawer row. Sub-paths appear as chips on the capture screen: *List · Book · Object · Sign · Handwriting*. Open if Eric wants different labels.
- **D12 spellgame.net.** Levels 1–3 ship on the web using the browser's camera + a WASM OCR path only if the census shows acceptable quality; otherwise app-only. Level 4+ app-only. Open.

**If Claude Code disagrees with any of these, stop and ask.**

---

## Non-goals (roadmap-wide)

- No cloud OCR, vision, or storage of photos.
- No new grader, no new cleanup function, no new mode-eligibility gate.
- No telemetry about captures.
- No changes to bank contents or audit gates from any level file.
- No AI-generated words, labels, definitions or lemmas anywhere in the pipeline.
- No camera path reachable from Spell Jr.

---

## Done (for this roadmap file)

This file is done when Eric has signed D1 and D7, and CC-SNAP-CLEAN v1.1 + CC-SNAP-HIGHLIGHT v1 exist as REVIEW-GATED files pointing back here. Every later level file names this roadmap in its layering line.

---

## Open against this roadmap, as of 2026-10-03

Recorded here rather than left in a chat log. These are findings and questions,
not edits to the doctrine above.

- **D12 has an answer the roadmap did not anticipate.** It frames the web
  question as OCR quality. The actual blocker is DOC-1: `bank_lookup` is
  `#[cfg(feature = "web")] -> None` because `word_index` is app-only, so on
  spellgame.net no word can be bank-anchored at all. Levels 1–3 on the web
  would be My Words only, whatever the OCR quality turns out to be.
- **DOC-1's CI gate has nothing to scan for.** There is no `bank_id` field
  anywhere in the tree; the bank anchor is currently expressed as a canonical
  string from `bank_lookup`. The gate needs a schema addition that no file owns.
- **3.2's gate is mis-stated.** FSRS has landed, but there is no due-by or
  deadline concept in the learner — the only `deadline` in the tree is the
  game's round timer. "Due-by = test date − 1 day" is a change request to L0,
  not a read-only use of it.
- **2.3 SNAP-LEMMA's bank census has still not run.** D1's signed order puts
  Lemma early, so this blocks Level 2 at its start rather than its end.
- **C4's answer for a frequency list is partly yes**: `en`, `ja` and `ko` have
  one under `tools/wordpipe/sources/`, but build-time only, not in `assets/`
  and so not reachable at runtime.
