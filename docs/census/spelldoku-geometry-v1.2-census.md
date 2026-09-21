# CC-SPELLDOKU-GEOMETRY v1.2 — §0 census (read-only)

Ran 2026-09-21 against `origin/main` plus the in-flight `cc-wordgrid-c` branch.
Measurements are from the real build driven in a headless browser at the stated
CSS viewport, not from reading the stylesheet.

**Outcome: one HALT (C5) and one ruling that supersedes the amendment (12×12).**

## C1 — How the board sizes itself

Fit-to-width CSS grid, one call site.

- `src/spelldoku_ui.rs:488` — `dom::el("sdGrid").set_attribute("style", &format!("--sd-n:{n}"))`
- `.sd-grid{--sd-n:9;grid-template-columns:repeat(var(--sd-n),1fr);max-width:520px;margin:0 auto}`

Cell pitch is therefore `min(available width, 520px) / n`. Nothing measures the
viewport in Rust, and no size is special-cased. **The 520 px cap matters:** past
a 520 px window the board stops growing, so a tablet does not buy a larger cell
the way the amendment assumes.

## C2 — Horizontal overflow

`.sd-screen{overflow:auto}`. Measured `hScroll:false` at every width tested
(375, 390, 414, 744, 1024). The board never scrolls sideways; the *stack*
scrolls vertically (see C4).

## C3 — Safe areas

The header pads with `env(safe-area-inset-top)`. No bottom inset is applied to
the keyboard row; on a notched phone that row sits directly on the home
indicator.

## C4 — Measured geometry (current build)

| Device | Size | Cell pitch | Glyph | Stack height vs viewport |
|---|---|---|---|---|
| iPhone SE 375×667 | 9×9 | 39 px | ~23 px | 884 vs 667 — **217 px overrun** |
| iPhone SE 375×667 | 12×12 | 29 px | ~16.9 px | 925 vs 667 — **258 px overrun** |
| iPad mini 744 | 9×9 | 57 px | — | fits |
| iPad mini 744 | 12×12 | 43 px | — | fits |

The keyboard is a persistent 202 px on the SE; the legend is 117 px at 9×9 and
158 px at 12×12 (three rows of chips). 9×9 clears the amendment's 38 px pitch
floor by one pixel and still scrolls. 12×12 fails the 17 px glyph floor.

## C5 — HALT: the mode chip says "Numbers" on a Word Mode board

`index.html:2845` — `<option value="off" data-i18n="sd.tier.off">Numbers</option>`

The picker label is tied to the Tier Mode toggle, not to the board's symbol set,
so a 9×9 Word Mode board (bank words, D12) still reads "Numbers" in the chip.
This is the tester report, reproduced. It is a labelling bug, not a geometry
one, and it is independent of everything else in this census — **it needs a
verdict before any F-item that touches the picker.**

## C6 — Device idiom

None anywhere. No `isPad`, no user-agent branch, no breakpoint in Rust. Layout
follows window width through CSS only.

## C7 — Shipping targets

`TARGETED_DEVICE_FAMILY = "1,2"` (iPhone **and** iPad), and `android/` is
present. So a phone is a first-class target, not a fallback — which is the
premise of the 12×12 ruling below.

## C8 — Type scaling

The board glyph is a `clamp()` expression; it does not follow iOS Dynamic Type.
A `body.big-text` class exists and is applied from the app's own setting.

---

## Ruling — 12×12 is cut (Eric, 2026-09-21)

> "12x12 cut entirly changed my mind wont work on smart phones"

This supersedes the amendment's D-G14 width gate, which would have kept 12×12
behind a minimum-width test. The census numbers support it: 29 px cells, a
16.9 px glyph under the floor, a 258 px overrun, and a 520 px grid cap that
keeps a bigger screen from helping. Removed in the build after 231; see
`docs/CC-SPELLDOKU-v1.2.md` for what came out and what D17/D18/D19 now mean
(void).

## Still open for Eric

1. **C5** — what should the chip say on a Word Mode board? ("Words" / the
   board's own name / hide the chip when Tier Mode is off.)
2. **C4** — 9×9 on an SE overruns by 217 px even after the cut. Tighten the
   legend and keyboard, or accept the scroll?
