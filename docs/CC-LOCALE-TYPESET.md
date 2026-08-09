# CC-LOCALE-TYPESET v1 — amendment to LOCALE-LAYOUT

Pasted by Eric 2026-08-09. **Queued, not started.** Saved verbatim here
because a pasted spec was lost to context compaction earlier in this work;
the transcript is not a safe home for law.

## Intent

Every language reads with the room its script needs, on every surface of the
app. Today, UI chrome (chips, tabs, settings rows) is hand-tuned to Latin
metrics; Arabic reads cramped, and the same latent problem awaits Vietnamese
diacritics, Devanagari matras, and Hangul blocks. The fix is not "pad the
Arabic chips" — it is: typography resolves through the LOCALE-LAYOUT cascade
by script class, so no surface can be Latin-tuned by accident again. When this
spec is incomplete, judge by that intent.

## F0 — Step-0: prove where today's values come from

Before changing anything, dump the resolved typography tokens (line-height,
letter-spacing, padding) for the ar home-screen chips and tagline. Identify
whether they come from a theme default, a per-component override, or hardcoded
literals.

- If values are hardcoded per-component in many places, **STOP AND ASK** — that
  is a refactor decision, not a style patch.
- If a tracking value from the "SPELL" header style is leaking into Arabic
  text, name it in the dump. That is a correctness bug (tracking breaks cursive
  joining) and gets fixed first.

## F1 — Typography axes in the cascade

Every UI text style resolves these tokens through the existing LOCALE-LAYOUT
cascade (per-language override > script-class default > global default):

- `lineHeightScale`
- `letterSpacing`
- `markHeadroomTop` / `markHeadroomBottom` (extra clearance for stacked marks / dotted descenders)
- `minPaddingV` (container vertical inset floor)
- `emojiGap` (see F5)

No component may hardcode these — enforced by lint (F6). Two orthogonal script
properties drive the defaults: **joining** (governs whether tracking is legal
at all) and **mark stacking** (governs vertical room).

### Starting defaults (proposals — D4 freezes them after device pass)

| class | languages | lineHeightScale | letterSpacing | headroom |
|---|---|---|---|---|
| latin | es fr de pt pl fil sw en | 1.20 | allowed | — |
| latin-stacked | vi | 1.35 | allowed | top |
| cyrillic | ru | 1.20 | allowed | — |
| arabic | ar fa ur | 1.55 | **0 enforced** | top + bottom |
| devanagari | hi | 1.50 | **0 enforced** | top + bottom |
| hangul | ko | 1.40 | allowed | — |
| cjk | ja zh | 1.40 | 0 default | — |

## F2 — Joining-script tracking lint

`letterSpacing` must resolve to exactly 0 for joined scripts (arabic,
devanagari classes) in every registered style. Any nonzero resolution =
**build failure**. A deliberate-failure test piece (a style that sets tracking
on an ar label) must exist and must fail, proving the lint is live.

## F3 — Mark-clip CI

A per-language worst-case golden string lives in the registry — chosen for
maximum vertical stress:

- ar: `التسلُّق` and `تهجَّاها` (shadda+damma / shadda+fatha stacks)
- fa: a ZWNJ word with stacked marks (coordinate with CC-PERSIAN-FOUNDATION goldens)
- vi: a word carrying `ệ`-class stacked diacritics
- hi: a conjunct with matra above and below
- ko: a dense syllable block; de: a long compound (doubles as F4 stressor)

CI renders each golden in **every registered text style** and pixel-asserts no
mark clipping at container bounds. A language with no golden string = CI red.
This is the checkable form of "each language can breathe."

## F4 — Longest-string layout sweep

Every fixed-size surface (home chips, tabs, settings rows, picker labels,
secret menu) is laid out against the longest localized label per language.
Overflow-by-truncation = **build failure**. Resolution is: surface grows per
cascade rules, or the string is shortened *by Eric* — STOP AND ASK, never
auto-ellipsize (D5). Reuses the pseudo-locale sweep machinery from
CC-PICKER-SEARCH. German compounds and `مبارزة التهجئة` are the named
stressors.

## F5 — Emoji/icon gap

Icons and emoji never share the label's bounding box; spacing comes from the
`emojiGap` token. Fixes the mountain/frame/bolt crowding on the mode chips as a
side effect of the general rule.

## F6 — Registry as source of truth

Rendered UI text styles ⊆ style registry (settings-truth pattern applied to
typography). A style rendered on device that isn't in the registry = build
failure, because unregistered styles escape F2/F3/F4. Lint proven by a
deliberate-failure piece.

## Decisions

- **D1 SIGNED (Eric):** ships as a LOCALE-LAYOUT amendment, not a standalone system.
- **D2 SIGNED (Eric):** script-class defaults cover every language — the fix is per-script, never per-bug.
- **D3 SIGNED per rec:** font audit is an informative check only (does the current typeface position Arabic/Devanagari marks well?). Any typeface swap is its own decision — stop and ask with side-by-sides.
- **D4 OPEN:** freeze the F1 numeric table only after Eric's device pass; values above are starting proposals.
- **D5 OPEN (rec: stop-and-ask):** longest-string overflow resolution — auto-grow vs escalate. Rec: escalate; silent ellipsis hides broken languages.

## Constraints and non-goals

- **Do not touch the Spell Picture render/word-path engine** — glyph layout there is governed by RENDER-FIXPASS / TONAL files. This file is UI chrome only.
- No font swaps in this pass (D3). No RTL mirroring changes — direction and shaping are already correct. No copy edits.
- If honoring these tokens would change any Spell Picture layout hash, STOP AND ASK.

## Done

1. F0 dump reviewed by Eric; any tracking leak fixed first.
2. Tracking lint green; deliberate-failure piece red.
3. Mark-clip CI green for all languages; every language has a golden.
4. Longest-string sweep screenshots attached for ar, fa, de minimum.
5. Registry lint green with deliberate-failure piece.
6. Eric's device pass: ar home screen, one settings screen, the picker — "breathes" is his call, then D4 freezes the table.

---

## Notes from the Aug 9 session (not part of Eric's text)

- The F1 table lists `fa` under the arabic class. `fa` is **not in the
  registry** (CC-MASTER-PARITY Phase A cut it; `registry_is_the_swapped_lineup_of_14`
  pins that), so an fa row in a script-class table has nothing to resolve for
  until that reversal is Eric's call. Same for the F3 fa golden.
- `ur` likewise appears in the arabic class and is also cut.
- F2's "letterSpacing must resolve to 0 for joined scripts" pairs with the
  Hindi finding already fixed in ship 149: `script_joins` now covers `ar` and
  `hi`, so the joined-script set this lint needs already exists as a single
  predicate rather than a hand-maintained list.
