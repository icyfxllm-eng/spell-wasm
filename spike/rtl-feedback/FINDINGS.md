# CC-RTL Phase 0 — findings + recommendation

**Status: P0.3 — STOP AND ASK. Awaiting Eric's decision. No Phase 1+ work has begun.**

Spike: `spike/rtl-feedback/` on branch `spike/rtl-feedback`. **Never merged**, per the
non-goals. Run it: `node spike/rtl-feedback/measure.mjs` (writes `spike.png`).

10 Arabic words × 4 font sizes (24/32/48/64) × 3 renderings. Measured in real
Chromium, not reasoned about.

---

## The blocker, quantified

SpellGame's answer surface (`game.rs:247`, `render_letters`) wraps every
character in its own `<span class="ltr">`. Shaping engines join cursive letters
within a text run; separate inline elements break the run.

Objective test: correctly-joined Arabic is **narrower** than the same letters in
isolated forms, so width is a proxy for joining that needs no human eye.

| | |
|---|---|
| Baseline wider than a single text node | **40 / 40 cases** |
| Overhang | min **7.3%** · median **26.6%** · max **55.7%** |

`بيت` is the worst at 55.7% — three letters, all joining, so isolating them costs
the most proportionally. The screenshot shows it plainly: كتاب renders as
**ك ت ا ب**, four disconnected letterforms. Not "ugly" — **unreadable**.

This is why CC-LINEUP-SWAP D2's gate is unconditional. Shipping ar/fa/ur today
would not be partial support, it would be gibberish.

---

## P0.2 evaluation

### A — single text node + markers beneath, positioned by `Range.getBoundingClientRect()`

| Criterion | Result |
|---|---|
| Joining always intact | ✅ **By construction.** One text node; nothing is ever split. Not "we tested it and it held" — there is no mechanism by which it can break. |
| Markers align at all sizes | ✅ Every cluster produced a marker with real width at 24/32/48/64px. Alignment is **exact by construction**: the marker's x/width come *from* the measured rect of that cluster in its real laid-out position. |
| Reveal animation | ⚠️ Markers animate freely. But the current per-letter **`pop`** animation cannot survive — you cannot animate a glyph you have not split. Reveal must become a marker/whole-word effect. |
| Missed-letter highlighting | ✅ As a marker/underline beneath the cluster. ⚠️ **Not** as a coloured glyph — colouring one letter means splitting it out, which is the bug. |
| Kid Mode chunked display | ✅ A `Range` can span *several* clusters, so a chunk box is the same measurement with different boundaries. |
| Implementation cost | **Low.** ~30 lines + re-measure on resize/font-load. |

### B — canvas/SVG rendering with per-cluster hit data

| Criterion | Result |
|---|---|
| Joining always intact | ✅ One `fillText` call shapes correctly. |
| Markers align at all sizes | ❌ **No.** See below. |
| Reveal / highlight / Kid Mode | ⚠️ All possible, but every one must be hand-drawn. |
| Implementation cost | **High**, and it carries an accessibility regression — see below. |

**Why B fails.** Canvas exposes no per-glyph geometry, so cluster boxes must be
derived by measuring prefixes and taking differences. That is broken for cursive
scripts: measuring `كت` shapes the ت as **final**, but inside `كتاب` the same ت is
**medial** — a different glyph with a different advance.

Measured against A's real shaped geometry:

| | |
|---|---|
| Max per-cluster error | min **4.19px** · median **11.15px** · max **29.10px** |
| Mean per-cluster error | median **6.31px** |
| Cases misplacing a marker by >2px | **40 / 40** |

At 32px, an 11px error is about a third of a letter — the marker points at the
**wrong letter**. That is worse than no feedback: it teaches the wrong thing.

> **A trap worth recording.** My first draft of this spike "measured" B's error as
> the difference between the sum of derived advances and the true width, and got
> **0.00px in every case** — a clean pass. It is a tautology: the differences
> telescope, so the sum *always* equals `measureText(word)`. Had I reported it, B
> would have looked flawless. The number only means something when compared against
> A's real geometry, which is the only ground truth available.

**Also, and the spec doesn't mention it: canvas text is invisible to screen
readers.** B would make the answer surface unreadable to VoiceOver unless an
entire parallel ARIA text layer is maintained alongside it. For a children's
education product with institutional buyers — the Education Edition's exact
market — that is close to disqualifying on its own.

---

## Recommendation: **A**

Accurate by construction, ~30 lines, no accessibility cost. B is inaccurate at the
one thing it exists to do, and expensive.

## The bigger question — Eric's call (P0.3)

**Should A replace the per-letter DOM for _every_ script, not just RTL?**

A is script-agnostic: `Range.getBoundingClientRect()` over cluster boundaries
works identically for Latin, Hangul, kana and Arabic. So the choice is:

- **Unify** — `render_letters`' span loop is **deleted**, all scripts get
  measured markers. One mechanism, less code than today, and RTL stops being a
  special case.
- **Parallel** — Latin keeps per-letter spans, RTL gets A. Two mechanics for the
  same feature, forever.

The spec's instinct ("deleting the per-letter DOM special casing rather than
adding a parallel path") is supported by the evidence. **But unify is a visible
UX change to every language**, and that is why it is your call, not mine:

1. The per-letter **`pop`** animation dies. It cannot be done without splitting.
2. Per-letter **colouring** dies — feedback moves *beneath* the word.
3. Korean's per-jamo coaching (`jamo::grade`) renders through the same surface and
   would need re-siting.

None is a regression in *capability* — the feedback still lands per-letter — but
all three change how the game **feels** in English, which is the only Active
language and 100% of current players. That is a product decision.

---

## If you choose A

Phase 1+ then proceeds as CC-RTL specifies (F1 direction plumbing → F2 CSS
logical-properties sweep → F3 bidi isolation → F4 the feedback mechanism → F5
keyboards → F6 fonts/harness/flip). Nothing there starts before this sign-off.

Two things this spike deliberately did **not** settle:

- **Fonts (D5).** Ran on macOS system Arabic (Geeza Pro). Whether Nastaliq's
  taller metrics break marker placement for Urdu is a real question, and D5
  already says to stop and ask with evidence rather than ship Urdu in Naskh.
- **Vocalised text.** All 10 words are unvocalised per CC-LINEUP-SWAP D5. Clusters
  carrying tashkeel would need re-measuring, though `Intl.Segmenter` already
  groups them correctly.

---

# Addendum — CC-RTL D5 fonts: Nastaliq. STOP AND ASK.

**Added 2026-07-17.** D5 says: *"If Nastaliq breaks layout metrics, performance, or
the D1 feedback mechanism, STOP AND ASK with evidence — do not silently ship Urdu
in Naskh."* Here is the evidence. It is not conclusive, and that is itself the
finding.

Reproduce: serve `spike/rtl-feedback/` over HTTP and open `fonts.html`
(`file://` silently blocks `@font-face` — see the trap below). Screenshot:
`fonts.png`, Naskh top 5 rows, Nastaliq bottom 5.

## What is measured and solid

| | Noto Naskh Arabic | Noto Nastaliq Urdu |
|---|---|---|
| Subset size (Google `/* arabic */` woff2) | **92 KB** | **234 KB** |
| Rendered height @44px | **75 px** | **110 px** — **+47%** |

Both OFL, which satisfies D6. 234 KB is **3× the largest face we ship today**
(bricolage-latin, 75 KB) and would be the single heaviest asset in the bundle
after the wasm. That is D5's "heavier", quantified — significant but survivable.

The +47% height is D5's "taller", and it is a **layout** fact: any row sized for
Latin or Naskh will be short for Urdu at the same `font-size`.

## What I could NOT measure — and why it matters most

**Nastaliq cascades.** `fonts.png` shows it plainly: مدرسہ and ٹماٹر slope
diagonally down-left across the word. Naskh sits flat.

I tried to quantify the cascade with per-cluster `Range.getBoundingClientRect()` —
the exact geometry approach A uses to place markers. It reported **baseline spread
= 0** and **cluster overlaps = 0** for Nastaliq, identical to Naskh.

**Those numbers are worthless.** A Range rect is the *line box* — advance-width
wide, line-height tall. It cannot see ink. So it reports a flat, non-overlapping
row of boxes for a script whose letters visibly are neither. My instrument was
blind to the exact property I was testing for.

**So the real question is open:** approach A positions a marker under each
cluster's *advance box*. In Naskh the letter sits in its advance box, so the marker
lands under the letter. In Nastaliq the letter may sit 40px higher and to the side
of its advance box — so **the marker would point at empty space, or at the wrong
letter.** That is worse for Urdu than the per-letter DOM was for Arabic, because it
would look plausible.

Answering it needs ink-level geometry, not layout geometry: canvas
`TextMetrics.actualBoundingBox*` per cluster, or rendering to canvas and finding
the ink. That is a second spike, and it is the thing to do before Urdu is promised
to anyone.

## Recommendation

1. **Bundle Noto Naskh Arabic (92 KB) for ar + fa.** Flat baseline, approach A's
   geometry holds, no open question. Low risk.
2. **Do NOT bundle Nastaliq yet, and do NOT ship Urdu in Naskh.** D5 forbids the
   second silently, and it is right to: Urdu readers expect Nastaliq, and Naskh
   Urdu reads as broken to them the way isolated letterforms read as broken to an
   Arabic reader. Either is a bad answer.
3. **Spike the cascade first** (ink-level per-cluster geometry, Nastaliq). If
   markers cannot be placed under cascading letters, Urdu needs a different
   feedback treatment from ar/fa — which is a product decision, not a font one, and
   it lands back at P0.3's unify-or-parallel question in a harder form.

**Urdu is the one language in the lineup whose feedback mechanism is still
unknown.** Arabic and Persian are ready pending the rest of CC-RTL. Urdu is not,
and the gap is not effort — it is an unanswered question.

## A trap worth recording

My first font measurement reported **identical metrics for both faces, to the
decimal**. Two typefaces cannot do that. `file://` blocks `@font-face` in Chromium,
so neither font loaded and both silently fell back to a system face — while
`document.fonts.status` cheerfully returned `"loaded"`, because it reports the
*load process* finished, not that the faces are usable. Serving over HTTP and
asserting `document.fonts.check('44px Nastaliq')` is what caught it.

That is the third measurement in this spike that returned a clean, confident,
meaningless number — after the tautological B-drift metric and the baseline-spread
above. All three looked like passes.
