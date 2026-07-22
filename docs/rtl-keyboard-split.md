# Decision for Eric — the RTL keyboard split (Arabic)

**Rescoped.** This decision originally covered ar / fa / ur. Persian and Urdu are
cut from the lineup (`docs/CC-MASTER-PARITY.md`, Phase A), so only Arabic remains.
See **"What the cut removed"** below — roughly half the original risk went with it.

**One decision, three parts.** A keyboard file in this repo does two jobs at once.
I have split them and shipped the cheap, reversible half (the charset declaration)
for Arabic. What I need from you:

1. **Ratify the split** — accept that "which letters exist" (charset) and "how
   RTL input behaves" (F5) are now separate deliverables, or tell me to revert.
2. **Authorise (or hold) content work** against the Arabic charset before the input
   half lands — this is the part that inverts the spec's stated dependency order.
3. **Name who audits the charset inventory.** It is auditable content and I cannot
   sign it off myself.

Everything below is context for those three. If you only read one section, read
**"The decision"** at the end.

---

## What a keyboard file does today (the thing being split)

`assets/keyboards/<code>.json` currently serves **two unrelated purposes**:

- **A charset declaration.** The word-list build (`build-wordlists.py`) reads it
  to learn which characters are typeable, then **fails the build** if any word in
  a bank uses a character the keyboard can't produce. This is a content gate.
- **An input layout.** At runtime the on-screen keyboard renders these rows and
  routes taps. For Latin/Cyrillic/CJK that's the whole story.

For left-to-right scripts those two are the same file because nothing else is
needed. For right-to-left scripts they are **not** the same work: the charset is
a fixed linguistic fact (the alphabet), while correct RTL *input* — cursor and
backspace direction, the zero-width non-joiner, hamza sequencing — is real new
behaviour and is what CC-RTL **F5** is about.

## What I shipped (commit `3cb7bb4`)

The **charset half only**. As shipped this covered ar/fa/ur; post-cut, the part
that survives is:

- `assets/keyboards/ar.json` — the full Arabic alphabet, plus matching Rust layout
  data so the pipeline gate and the Rust gate agree.
- Tests pinning the inventory (`src/keyboard.rs`).

I did **not** build any RTL input handling. It is unreachable at runtime today
regardless — Arabic is `rtl_blocked` (`RTL_SUPPORTED = false`), so the game never
selects it and these layouts never render.

## What the cut removed

The original version of this decision carried a Persian/Urdu-specific hazard: the
answer-comparison does not unify look-alike codepoints, so declaring ك U+0643
where a Persian word uses ک U+06A9 would silently reject a correct word. That
class of bug **no longer exists in the lineup** — with fa/ur gone there are no
competing Arabic-script orthographies to confuse. Arabic simply uses its own
codepoints and nothing shadows them.

Two consequences, both cleanups owed to Phase A (not done here):

- `assets/keyboards/{fa,ur}.json` and their Rust layout data are now dead weight.
- `src/keyboard.rs` still carries the fa/ur codepoint-discipline tests
  (`src/keyboard.rs:655-681` — the ک/ی/gol-he/do-chashmi-he/bari-ye assertions) and
  the explanatory comments at `src/keyboard.rs:114-134`. These pass today and must
  be removed *deliberately* as part of CC-LINEUP-TRIM, not quietly.

Charset discipline for Arabic itself has not gone away — it has moved. The live
Arabic trap classes are hamza seats and unvocalized spelling, governed by
`docs/CC-MASTER-PARITY.md` **A2**, which puts an unvocalized-spelling CI check on
every merged word.

## Why this needs you, not just a merge

The content spec (CC-NEW-LANG-CONTENT) lists RTL keyboard work as a **non-goal**,
and treats Arabic content as blocked until CC-RTL F5. Splitting the keyboard
**inverts that**: with the charset declared, Arabic word banks can now be authored
and validated *before* F5 input handling exists. That is a genuine change to the
sequence the spec assumed, so it is your call, not mine.

This split is also what makes CC-MASTER-PARITY's **Track A two-half structure**
possible — the content half (A1–A3) starting immediately, independent of the
rendering half (A4–A6), is precisely the parallelism this decision unlocks. If you
revert the split, Track A collapses back into a serial track.

Two honest constraints that bound the upside:

- **Playable ≠ buildable.** Even with content, Arabic cannot be *played* until F5
  (input) and F6 (fonts + the `rtlSupported` flip) land. Authorising content now
  buys **parallelism** — word banks can be built and audited while the rendering
  track proceeds — not a shippable feature.
- **The charset itself needs a native/expert pass.** I built it from the standard
  national layout and the codepoint discipline is tested, but there is no
  `verified_by` I can set. A wrong or missing codepoint here silently mis-gates
  every bank built against it, so the audit should precede content, not follow it.
  Note this is **not** covered by Gig A / Gig B — those audit words and
  definitions, not the alphabet inventory. It needs its own named reviewer.

## The decision

Pick one:

**A — Ratify the split, authorise content, commission the charset audit
(recommended).** Keep the Arabic charset declaration. Have a native speaker /
linguist verify the inventory (the same `verified_by` gate word sources already
have). Once signed off, Arabic word-bank work may start in parallel with the RTL
rendering track. F5 input handling and F6 remain prerequisites for *playing*, not
for *building*. This is the most throughput for the least commitment: the charset
half is small and reversible, and nothing here can reach a player early because
`RTL_SUPPORTED` is still false — and G4 holds activation regardless.

**B — Ratify the split, but hold content until F5.** Keep the declaration as
groundwork, but do not start Arabic banks until RTL input handling lands. This
respects the spec's original order. Cost: the word-bank work (which is independent
of input behaviour) sits idle behind a dependency it doesn't actually have — and
CC-MASTER-PARITY A1–A3 stall behind A4–A6, contradicting Track A as written.

**C — Revert the split.** Drop the charset declaration; treat the keyboard as one
indivisible unit delivered whole at F5. Cost: Arabic content stays blocked on
input-handling work it does not depend on, and the small, tested groundwork is
thrown away.

I recommend **A**, gated on the native charset audit landing first.

## What is still blocked regardless of this decision

- **RTL input handling (F5, the other half).** Cursor/backspace direction, ZWNJ,
  hamza sequencing. Real new behaviour; not started.
- **Fonts (F6).** Naskh is bundled for Arabic. *(The Urdu Nastaliq stop-and-ask —
  the ink-level `getExtentOfChar` spike that was never run — is void: Urdu is cut.
  Arabic has no equivalent open rendering question.)*
- **The `rtlSupported` flip (end of F6).** The last step; needs every mode green on
  real content. Nothing before it makes Arabic playable, and per CC-MASTER-PARITY
  G4 the flip makes Arabic *eligible*, never live — activation still needs Gig A +
  Gig B.
- **CC-RTL Phase 0 findings approval (G2).** No RTL feature work merges before it.

---

*This doc lives on `feature/rtl-feedback` because that's where the code it
concerns lives. It should be folded into `docs/DECISIONS-PENDING.md` (currently
on `build-54`) when those branches converge.*
