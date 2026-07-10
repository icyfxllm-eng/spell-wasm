# SpellGame — UI String Audit (Discovery Pass)

Deliverable for the Full UI Localization work order §1. **Read the architecture
finding first — it changes the recommended approach vs. the work order's
assumptions.**

## ⚠️ Critical architecture finding (report before implementing)

The work order assumes a **"JS/HTML frontend"** where strings live in JS and Rust
returns keys to a JS translation layer. **That is not this app.**

SpellGame's UI is **Rust-driven DOM**: the WebAssembly core writes essentially
all UI text directly (`dom::set_text` / `dom::set_html`, ~126 call sites), and
the rest is **static text in `index.html`**. There is **no JS UI layer** — only
the wasm bootloader (`index.html` `<script>`), `audio-native.js`, `ocr-shim.js`.

Consequence: the work order's plan ("`t()` in `src/i18n/i18n.js`, translate in
JS") would require piping ~126 render sites' keys+params across the wasm↔JS
boundary on every DOM write — awkward, slow, and invasive.

**Recommended adaptation (needs your OK):** put i18n **in Rust**.
- A `t(key, &params)` in a new `src/i18n.rs`, backed by locale tables.
- Locale data as JSON in `src/i18n/locales/{lang}.json`, embedded at build time
  via `include_str!` (they're tiny) — keeps the "one JSON per language, CI key
  parity" model from the work order intact.
- Static `index.html` labels get `data-i18n="key"` attributes + a Rust
  `translate_page()` walker (Rust already owns the DOM).
- Current language = the existing word-list selector (`state.lang`), exactly as
  the work order requires (one selector). Re-translate on language change.
- Everything else in the work order (key namespaces, `{param}` placeholders,
  `.one`/`.other` plurals, fallback chain, `check-i18n.js` CI, REVIEW_NEEDED.md)
  applies unchanged — only the *host language* of the i18n module moves from JS
  to Rust.

This keeps game logic byte-identical (translation happens at the display edge,
same as today's literals) and avoids a boundary refactor.

## 1. Languages (the single selector already offers these 11)

`en` English · `es` Español · `fr` Français · `de` Deutsch · `pt` Português ·
`it` Italiano · `nl` Nederlands · `pl` Polski · `sv` Svenska · `nb` Norsk ·
`tr` Türkçe  — UI translations must ship for exactly these.

## 2. String inventory (~250 user-facing strings)

### 2a. Rust-originated (convert to keys returned/translated in Rust)
| Module | ~count | Examples |
|---|---|---|
| `game.rs` | ~69 | orb glyph ("tap to hear a word", "listen…"), feedback ("Not quite — {n} tries left", "It was", "Time's up — it was"), praise pool ("Clean.", "Locked in.", …), tries/hint lines, versus ("{name} — chain of {n}", "{name} wins!"), meaning labels |
| `lib.rs` | ~42 | import/My-Words notes, drawing statuses ("Reading your writing…", "Couldn't read that…"), save messages ("Saved {n} of your words…") |
| `climb.rs` | ~18 | "Sign in", "The Climb", auth/error toasts, "Log in to post your chain…", "New record! Posted to The Climb — #{n} on {difficulty}." |
| `share.rs` | ~7 | share-sheet result text |
| `agegate.rs` | ~5 | parent-gate math ("What is {a} times {b}?"), verdict copy |
| `stats.rs`/`board.rs`/`achievements.rs` | ~11 | stats labels, empty states, achievement names+descs (7) |

### 2b. `index.html` static labels (~112) — `data-i18n` attributes
Top bar (Misses/⚔ Head-to-head/The Climb/Sign in/＋ My words/⚙), Replay/Slow,
"your spelling", "type what you heard", Check spelling / Enter / Hint /
Definition / Sentence / Give up, drawpad (Pen/Line/Eraser/Guide/Undo/Clear/Read
my writing), all modal titles/bodies/buttons (settings toggles + smalls, import,
head-to-head setup/result/quit, age gate, parent gate, auth, account, The Climb
tabs), footer, chain labels ("current chain"/"your best"), stats/achievements/
board card headings.

## 3. Dynamic / interpolated strings (placeholder-aware)
`{n} tries left` (+ singular "try"), `chain of {n}`, `{name}'s turn`, `Saved {n}
of your words ({m} skipped)`, `What is {a} times {b}?`, `#{rank} on {difficulty}`,
`{n} in a row.`, streak/best numbers, `{n} words spelled in a row`, share text,
misses "{n} due now". → need `{param}` placeholders; a few need `.one/.other`.

## 4. Do NOT translate
- **"SPELL"** brand header (`.brand .mark`) — explicit owner instruction, leave as-is.
- **Usernames** and **My Words** content (user data).
- **The target word** being spelled (it's already in the selected language).
- **"The Climb"** — DECISION NEEDED: brand name (keep English) vs localize. Recommend keep as a product/brand name; confirm.

## 5. Layout risk list (Big Text + long languages, esp. de/es/nl/pl)
- **On-screen keyboard action keys**: "Check spelling" / "Enter" / "⌫" (Delete) — tight; German/Dutch equivalents are long. Verify at 320px + Big Text.
- **Top-bar pills**: "⚔ Head-to-head", "The Climb", "＋ My words", "Sign in", "↻ Misses" — flex-wrap helps but watch overflow.
- **Settings toggle labels** (`.lbl2` + `<small>`), **hint/definition/sentence/give-up** row, **The Climb tabs**.
- Cross-check against the no-horizontal-scroll work already in place.

## 6. Scope estimate
~250 keys × 11 languages ≈ **2,750 translations**, plus converting ~250 Rust/HTML
string sites to `t()`/`data-i18n`. Per the work order's commit sequence, step 2
(key-driven, English-only, visually identical) is the safety anchor.
