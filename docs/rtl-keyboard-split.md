# Decision for Eric — the RTL keyboard split (ar / fa / ur)

**One decision, three parts.** A keyboard file in this repo does two jobs at
once. I have split them and shipped the cheap, reversible half (the charset
declaration) for Arabic, Persian, and Urdu. What I need from you:

1. **Ratify the split** — accept that "which letters exist" (charset) and "how
   RTL input behaves" (F5) are now separate deliverables, or tell me to revert.
2. **Authorise (or hold) content work** against these charsets before the input
   half lands — this is the part that inverts the spec's stated dependency order.
3. **Name who audits the charset inventories.** They are auditable content and I
   cannot sign them off myself.

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

The **charset half only**, for ar/fa/ur:

- `assets/keyboards/{ar,fa,ur}.json` — the full alphabet of each language, plus
  matching Rust layout data so the pipeline gate and the Rust gate agree.
- Tests pinning the inventory, including that Persian/Urdu use their own
  codepoints (ک U+06A9, ی U+06CC, gol-he ہ U+06C1, do-chashmi-he ھ U+06BE,
  bari-ye ے U+06D2) and **not** their Arabic look-alikes. This matters because
  the answer-comparison does not unify them: declare ك U+0643 where a Persian
  word uses ک U+06A9 and the gate silently rejects a correct word.

I did **not** build any RTL input handling. It is unreachable at runtime today
regardless — ar/fa/ur are `rtl_blocked` (`RTL_SUPPORTED = false`), so the game
never selects them and these layouts never render.

## Why this needs you, not just a merge

The content spec (CC-NEW-LANG-CONTENT) lists RTL keyboard work as a **non-goal**,
and treats ar/fa/ur content as blocked until CC-RTL F5. Splitting the keyboard
**inverts that**: with the charset declared, ar/fa/ur word banks can now be
authored and validated *before* F5 input handling exists. That is a genuine
change to the sequence the spec assumed, so it is your call, not mine.

Two honest constraints that bound the upside:

- **Playable ≠ buildable.** Even with content, ar/fa/ur cannot be *played* until
  F5 (input) and F6 (fonts + the `RTL_SUPPORTED` flip) land. Authorising content
  now buys **parallelism** — word banks can be built and audited while the
  rendering track proceeds — not a shippable feature.
- **The charsets themselves need a native/expert pass.** I built them from
  standard national layouts and the codepoint discipline is tested, but there is
  no `verified_by` I can set. A wrong or missing codepoint here silently
  mis-gates every bank built against it, so the audit should precede content, not
  follow it.

## The decision

Pick one:

**A — Ratify the split, authorise content, commission the audit (recommended).**
Keep the charset declarations. Have a native speaker / linguist verify each
inventory (this is the same `verified_by` gate word sources already have). Once a
charset is signed off, its word-bank work may start in parallel with the RTL
rendering track. F5 input handling and F6 remain prerequisites for *playing*, not
for *building*. This is the most throughput for the least commitment: the charset
half is small and reversible, and nothing here can reach a player early because
`RTL_SUPPORTED` is still false.

**B — Ratify the split, but hold content until F5.** Keep the declarations as
groundwork, but do not start ar/fa/ur banks until RTL input handling lands. This
respects the spec's original order. Cost: the word-bank work (which is
independent of input behaviour) sits idle behind a dependency it doesn't
actually have.

**C — Revert the split.** Drop the charset declarations; treat the keyboard as
one indivisible unit delivered whole at F5. Cost: ar/fa/ur content stays blocked
on input-handling work it does not depend on, and the small, tested groundwork is
thrown away.

I recommend **A**, gated on the native audit landing first.

## What is still blocked regardless of this decision

- **RTL input handling (F5, the other half).** Cursor/backspace direction, ZWNJ,
  hamza sequencing. Real new behaviour; not started.
- **Fonts (F6), and Urdu specifically.** Naskh is bundled for ar/fa. **Urdu
  Nastaliq remains a stop-and-ask** — the ink-level spike (SVG `getExtentOfChar`)
  I proposed was never run, so Urdu has an open rendering question the other two
  don't.
- **The `RTL_SUPPORTED` flip (end of F6).** The last step; needs every mode green
  on real content. Nothing before it makes ar/fa/ur playable.

---

*This doc lives on `feature/rtl-feedback` because that's where the code it
concerns lives. It should be folded into `docs/DECISIONS-PENDING.md` (currently
on `build-54`) when those branches converge.*
