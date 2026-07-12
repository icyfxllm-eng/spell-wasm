# REVIEW — CC-HOME-REGROUP (branch `feat/home-regroup`)

**Status: REVIEW-GATED. Not merged, not pushed to the submission pipeline.**
Two commits on the branch: `869d60d` (Feature 1 rename) + `b4d173b` (F2–F7 layout).

## What shipped

Eight look-alike pills → four visual families = four functional classes, ordered
top-to-bottom **header → my-data → modes → setup → play**:

| Family | Treatment | Controls |
|--|--|--|
| Meta (F2) | icon-only, ≥44pt, header corner | account (👤, state via aria+`.signed-in`), settings (⚙) |
| Data chips (F5) | outlined, muted, badge-capable | My words, Misses (live count badge) |
| Launchers (F4) | filled equal-width iconed cards | Spell Off · The Climb · Daily |
| Config (F3) | muted chevroned chip → setup sheet | Language · Difficulty · Timing |

## Decisions made (delegated ones)

- **Setup strip = option (a) summary chip → sheet**, not the 3-segment strip.
  Reason: German "Deutsch · Fortgeschritten · Ohne Zeit" cannot fit one line at
  375px as three live segments. The chip shows the value summary and opens a
  sheet holding the (unchanged) pickers. `game::update_setup_chip` recomputes the
  summary from state on every picker change, so chip and round never disagree.
- **Setup sheet uses icon rows (🌐/🎚/⏱), not text labels** → zero new i18n keys
  (parity stays 188). Done button reuses the existing `settings.done`.
- **"Spell-Off" → "Spell Off"**: only English used the stylistic hyphen. The other
  15 locales have no hyphen. **German "Buchstaben-Duell" is kept** — that hyphen
  is a correct compound-noun hyphen, not the "-Off" construction. *(Flagged for
  the native auditor to confirm — see below.)* Internal keys (`top.headToHead`,
  `vs.title`) untouched.

## ⚠ COPPA decision (approved by Eric mid-task)

The old `.pill` had **no `btn-hide` CSS rule**, so `climb.rs`'s gating of The Climb
+ Sign-in out of Kid Mode never took visual effect — **children could see/tap them**
(before screenshots confirm). The new launcher/meta families include `btn-hide`
rules, so they are now correctly hidden in Little Speller, matching `climb.rs`'s
documented intent. Eric chose **keep the fix** (hide from kids). No change to
`agegate.rs` or `climb.rs` gating logic — only the CSS now honors it.

## Acceptance (spec "Done when")

| # | Check | Result |
|--|--|--|
| 1 | Zero user-visible "Spell-Off"; internal keys intact | ✅ (`grep` clean in src/index; keys present) |
| 2 | Meta icons ≥44pt, existing flows, no body pills | ✅ |
| 3 | Setup chip one line @375px, updates live, round matches | ✅ en/de/th/ja verified (de = worst case) |
| 4 | 3 equal launchers, navigate, labels wrap not truncate | ✅ (de "Buchstaben-Duell" wraps to uniform height) |
| 5 | Misses badge live count + launch; My words modal; distinct | ✅ |
| 6 | Breakpoints in one file; no width forks in JS/Rust | ✅ (all CSS in index.html; no Rust width logic) |
| 7 | Playwright suite green; full round each mode | ✅ **34/34**; ⚠ Maestro iOS smoke NOT run here (needs iOS build) |
| 8 | Locale key-completeness 17 langs | ✅ 188 keys parity |
| 9 | Little Speller parity; no "Spell-Off"; age gate unchanged | ✅ (same families/order; Climb+Sign-in omitted; agegate byte-unchanged) |
| 10 | Cross-mode round-trip preserves selections | ✅ by construction (pickers/prefs untouched); no dedicated test added |

## Deliverables on the branch

- Before/after screenshots: `tests/e2e/shots/{before,after}/` — en·de·th·ja ×
  se(375)·large(430)·tablet(820) × std·kid (24 after, 16 before).
- Test output: `npm run e2e` → 34/34; `npm run build` gates all green.

## Open items for the auditor queue

1. **German "Buchstaben-Duell"** — confirm keeping the compound hyphen (not the
   "-Off" hyphen) is right; no other locale needed a rename.
2. **Maestro iOS smoke** — run on the Mac against an iOS build before this merges.
3. Internal review docs (`REVIEW_*.md`, `rename_audit.md`) still contain the
   historical "Spell-Off" string by design (audit trail; not user-visible).
