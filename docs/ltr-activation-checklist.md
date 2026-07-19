# 10-language activation — ready to sign (build 56)

**Status: AWAITING ERIC.** This activates the content-ready LTR/CJK languages that
are held ComingSoon for the English-first launch. It is the maximum a legitimate
production build 56 can carry: a **pure status flip**, no RTL machinery, and no
audit-draft content — every one of these languages already has a real, in-binary
word bank, a keyboard, and a backend voice. Nothing here is applied on
`feature/rtl-feedback`; the branch stays byte-identical to today's English-only
production. This is the reviewed, reversible change waiting for a yes.

## What flips
`ComingSoon → Active` for the ten languages below (in `LANGS_BASE`, `src/consts.rs`).
Each is **independently selectable** — activate all ten, or any subset Eric is
confident in. Nothing else changes: banks, keyboards, and voices are already shipped.

| Lang | Endonym | Production bank | Keyboard | Voice |
|------|---------|----------------:|:--------:|:-----:|
| es | Español | 798 | ✅ | es-ES |
| fr | Français | 800 | ✅ | fr-FR |
| de | Deutsch | 796 | ✅ | de-DE |
| pt | Português | 800 | ✅ | pt-BR |
| pl | Polski | 799 | ✅ | pl-PL |
| vi | Tiếng Việt | 800 | ✅ | vi-VN |
| ko | 한국어 | 800 | ✅ | ko-KR |
| ja | 日本語 | 775 | ✅ | ja-JP |
| fil | Filipino | 800 | ✅ | fil-PH |
| zh | 中文 | 1200 | ✅ | cmn-CN |

## What is NOT in this (and why)
- **ru** — LTR, but its production bank is empty (content is an audit draft only).
  It needs the same promote-then-flip as RTL before it can activate.
- **ar / fa / ur** — RTL: need the `RTL_SUPPORTED` flip + native audit. See
  `docs/rtl-ship-checklist.md`.
- **hi** — audit-only (D8); not registered in production.

## Prerequisite — the one real gate
`ComingSoon` means "gated until it passes native-speaker audit." Activating a
language asserts it HAS been reviewed. For any of the ten not yet audited, run it
through the same harness the RTL work built — it is playable in the audit-preview
build today:
```sh
bash scripts/build-web-audit.sh && node scripts/serve-audit.mjs   # all languages unlocked
# reviewer plays, taps ⚑ on bad words, exports; then:
python3 scripts/ingest-audit-flags.py audit-flags.txt
python3 scripts/build-wordlists.py     # drops the flagged words from the production banks
```
Record who reviewed each language in the sign-off (a fork cannot supply `verified_by`).

## The change (apply to the approved subset)

**`src/consts.rs`, `LANGS_BASE`** — for each approved language:
```rust
-(ES, "Español", ComingSoon, Ltr),
+(ES, "Español", Active, Ltr),
```
…and the same for fr, de, pt, pl, vi, ko, ja, fil, zh.

**Two gating tests update to the new active set:**
- `consts::registry_tests::only_en_active` → `active_languages_are_english_and_the_ten_ltr`:
  assert the active list equals `en` + the approved subset; ru/ar/fa/ur stay gated.
- `daily::tests::inactive_locale_falls_back_to_en`: now that es is active, use a
  still-gated locale (ru) for the fallback assertion, and add that an active locale
  (es) draws its OWN pool (`assert_ne!`).

**Verify:** `cargo test --lib` (green) and the menu e2e (the approved languages now
render as active, selectable options).

## Already verified (dry run, then discarded)
Activating all ten was applied on a throwaway branch and is known good:
- **Tests:** production **212 passed / 0 failed**, audit-preview **207 / 0**.
- **Scope:** a 2-file, ~30-line change (`consts.rs` statuses + the two tests). No
  content, pipeline, or binary-layout change beyond the status bytes.
- **Byte-identity:** the branch was deleted; `feature/rtl-feedback` is unchanged
  (es still ComingSoon, English-only, 212 tests).

## Risk & revert
- Each language reverts by flipping its one line back to `ComingSoon` — instantly,
  independently. No data migration, no user-state impact (banks/keyboards were
  always present).
- The fallthrough-to-English failure mode is impossible: `tier_for` routes each of
  these to its own bank, and the keyboard charset test pins that a word is typeable
  on its own keyboard.
- The only real question is per-language audit confidence — which is exactly what
  the sign-off below captures.

## Relationship to build 56
This is the shippable content of a production build 56: an English-experience update
(more word variety, the dictionaryapi privacy fix, the translated hint count) PLUS
whichever of these ten Eric activates. RTL (ar/fa/ur) is a separate, later build.

## Sign-off
```
Activate (✓ / ✗) and name the reviewer per language:
  es ___  reviewed by ______________      ko ___  reviewed by ______________
  fr ___  reviewed by ______________      ja ___  reviewed by ______________
  de ___  reviewed by ______________      fil ___ reviewed by ______________
  pt ___  reviewed by ______________      zh ___  reviewed by ______________
  pl ___  reviewed by ______________
  vi ___  reviewed by ______________

approved to ship (Eric): ____________________   date: __________
```
