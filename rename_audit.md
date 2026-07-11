# Mode-rename extraction audit (Step 1)

## Bucket (a) — localization files (renamed in Step 2, value-migrated across 17 locales)
| key | old value | new mode |
|---|---|---|
| top.headToHead | "⚔ Head-to-head" | **Spell-Off** (⚔ removed, I3) |
| settings.kid / settings.kidSmall | "Kid mode" / subtitle | **Little Spellers** |
| mode.timed | "Timed ⏱" | **Quick Bee** (+ new mode.timed.subtitle "Classic timed round") |
| vs.title | "Head-to-head" | **Spell-Off** |
| vs.leaveBlurb | "...head-to-head match..." | **Spell-Off** |
| error.offline | "...leaderboard and head-to-head..." | **Spell-Off** |
| (new) mode.tricky_words.title / .subtitle | — | **Tricky Words** (F6 not built; keys land now) |

## Bucket (b) — hardcoded user-visible (fallback text in index.html; data-i18n already wired so localized value wins — fallbacks updated for cleanliness)
- index.html:454 vsBtn fallback "⚔ Head-to-head"; :471 vsExit aria "Exit head-to-head"; :647 vs.title fallback; :674 vs.leaveBlurb fallback.

## Bucket (c) — INTERNAL identifiers (LEFT UNCHANGED per D3 — display ≠ identity)
- `kid: bool` / `kidToggle` / `s.kid` (Kid Mode storage + toggle id) — persisted, never rendered raw.
- `versus` / `vsBtn` / `top.headToHead` KEY name / `mode.timed` KEY name — i18n key names are internal ids (D3); only their VALUES change. (Deliberate reading of D2↔D3: a key kept with a new value is NOT an alias — no old name can resurrect. Flagged for your call if you want a full semantic key-rename.)
- HTML comments mentioning "head-to-head" (non-user-visible).

## D3/D10 STOP conditions: none. No storage key renders to screen; achievement identity is keyed on `id` (e.g. "timed10"), not display text — only ach.*.desc VALUES that name a mode change.
