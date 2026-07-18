# CC-HINDI-PHASE0 — status: COMPLETE (foundation proven, nothing registered)

Phase 0 asked one question — **can Hindi (Devanagari) work in this engine at
all?** — and required it be answered *before* any commitment to the language. The
answer, demonstrated across F1–F4 and not merely asserted, is **yes**. Phase 0
deliberately registers nothing, ships no content, and renders nothing in the app:
it is the foundation, and it holds.

## What was proven

| Item | Where | Result |
|---|---|---|
| **F1 — akshara segmentation** | `src/akshara.rs` (15 tests) | The extended grapheme cluster (UAX #29 GB9c) **is** the akshara. क्ष (क+्+ष) segments as ONE unit; so does स्त्री. No bespoke Devanagari segmenter needed. One source of truth in the Rust core (D3); no JS-side segmentation. |
| **F2 — render prototype** | `spike/hindi-akshara/` (both engines) | One tile per akshara renders correctly — conjuncts stay whole, the reordering i-matra कि renders ि before क, nuqta and candrabindu attach. `render-webkit.png`. |
| **F3 — nuqta gate** | `scripts/devanagari-check.mjs` (`--selftest`) | Word-list files carrying a composition-excluded precomposed nuqta (U+0958–095F) fail the build, naming file/line/codepoint. Enforced before content, not remembered. |
| **F4 — normalization stability** | `akshara.rs::segmentation_is_normalization_stable` + F2 spike | The same word precomposed vs decomposed segments identically (Rust) and renders identically (NFC width == NFD, both engines). D4's NFC baseline holds end to end. |
| **Keyboard charset** | `assets/keyboards/hi.json` + `keyboard.rs` | The full Hindi Devanagari inventory (61 codepoints) is declared and coverage-asserted against the akshara segmenter's own F2 word set, so *segment* and *type* agree. Charset only — unregistered. |

## The finding that shapes everything downstream

An akshara is one grapheme cluster, so all conjunct-forming and matra-reordering
happens **inside** one tile, and aksharas do not shape across tile boundaries.
**Devanagari can therefore use the English-style per-unit tile model** — one tile
per akshara, with per-tile styling (colour feedback, pop, whatever) safe. This is
the **opposite** of cursive Arabic, where per-letter boxes shatter the join
(`spike/urdu-nastaliq`). It means Hindi's render and feedback paths look like
English's, not like Arabic's — the cheaper of the two.

## What is next, and what gates it

None of this is a phase-0 concern; all of it is gated exactly like every other new
language, and none of it is blocked by a *rendering* unknown any more:

- **Registry entry** (`BUILTIN_LANGS`) — needs the same authority/audit sign-off
  as any language; phase 0 explicitly granted none (D8).
- **A bundled Devanagari font** (e.g. Noto Sans Devanagari), the way Nastaliq was
  bundled for Urdu — the F2 prototype used the system face. Buildable, but best
  done once this branch's font infrastructure converges with `feature/rtl-feedback`
  (which carries the lazy `unicode-range` font-gate logic).
- **Keyboard input handling + ergonomics** — the charset declares *what* is
  typeable; *how* the Devanagari keyboard is arranged (which char on which
  long-press) wants a native/expert pass, like the ar/fa/ur charsets.
- **Native-audited word content** — with the F3 nuqta gate and the charset gate
  already in place to validate it when it lands.

## Housekeeping

This branch (`feature/hindi-akshara`) predates the RTL work on
`feature/rtl-feedback`; the two share the endonym-test fix (aligned deliberately)
but diverge on the language registry and font infrastructure. This status should
fold into `docs/DECISIONS-PENDING.md` (on `build-54`) when the branches converge —
the same note the RTL docs carry.
