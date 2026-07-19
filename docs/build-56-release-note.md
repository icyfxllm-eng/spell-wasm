# SpellGame — Build 56 release note

**Status: AWAITING ERIC'S APPROVAL.** Nothing here is cut or shipped. This is the
production-safe payload assembled on `feature/rtl-feedback`, ready for a version bump
and a TestFlight/App Store build once approved.

> **Version note.** The build number is NOT taken from the repo — fastlane sets it at
> cut time to `latest_testflight_build_number + 1` (`fastlane/Fastfile`). TestFlight
> is at build 55, so this cuts as **build 56** automatically; the committed
> `CURRENT_PROJECT_VERSION = 47` is stale and ignored, and `MARKETING_VERSION` stays
> 1.1. No manual bump is needed — but running fastlane / cutting the build is still
> Eric's call (the freeze).
>
> **Scope caveat — read before approving.** This note describes the *recent* payload
> on `feature/rtl-feedback`. That branch is **~104 commits ahead of `main`**, so a
> build cut from it carries substantially more than changes 1–3 below. Whoever cuts
> build 56 must (a) confirm which branch it is cut from, and (b) reconcile the FULL
> delta against what build 55 shipped — see "Open question: the 104-commit gap" at
> the end. The safety guarantees (no RTL, no audit content) hold regardless of branch.

## Headline
An **English-experience + privacy** update, optionally activating up to **ten more
languages**. No right-to-left languages and no audit content ship in this build —
that is guaranteed at the byte level, not just by policy.

## User-facing changes

1. **Richer English word variety.** The English bank was rebuilt from the Leipzig
   corpus to 800 words (200 per tier), up from a small curated set. English is the
   one language players actually draw from today, so this is real added variety in
   both normal play and the Daily Challenge.
2. **Privacy: no third-party dictionary calls.** The app no longer calls
   `dictionaryapi.dev` directly from the browser (the `def_lang` mapping was
   deleted). This closes an outbound third-party request for every user.
3. **Correct hint localization.** The hint's "(N letters)" count is now translated
   instead of hardcoded English — visible to anyone running a non-English interface
   locale.

## Language activation (optional — Eric selects)
Up to **ten content-ready languages** can be activated in this same build — a pure
status flip, since each already has an in-binary word bank, keyboard, and backend
voice:

> **es, fr, de, pt, pl, vi, ko, ja, fil, zh**

Each is independently selectable, and each requires a native-audit sign-off first
(that is what `ComingSoon → Active` asserts). The full package — the exact flip, the
per-language sign-off block, and how to audit any not-yet-reviewed language via the
audit-preview harness — is in **`docs/ltr-activation-checklist.md`**. Activating none
of them is a valid choice; the update still ships changes 1–3.

## Also included (no user-visible effect)
- **Thai fully removed** from the codebase (it was never offered; this is cleanup).
- **Grown banks for the coming-soon languages** ride along in the binary, dormant
  until each is activated.
- **The RTL engine** (logical-property CSS, cursive feedback, ar/fa/ur keyboards,
  Nastaliq/Devanagari fonts) is present but gated and inert — fonts are lazy-loaded,
  so they are never even fetched while the languages are gated.

## Deliberately excluded — the guarantee
- **ar / fa / ur (RTL)** — a separate, later build. Needs the `RTL_SUPPORTED` flip
  and native audit. See `docs/rtl-ship-checklist.md`.
- **ru** — its production bank is empty (content is an audit draft); not activatable
  yet.
- **hi (Hindi)** — audit-only (D8); not registered in production at all.
- **The entire `audit_preview` build** — draft banks, the flag widget, the unlocked
  preview. `#[cfg]`-compiled out of production; verified absent from the production
  wasm at the byte level.

## Verification & safety
- `cargo test --lib` green in both configurations. With all ten languages activated
  (the maximum): production **212 / 0**, audit-preview **207 / 0**.
- The activation is a ~30-line, 2-file change (statuses + two gating tests). No word
  a language serves can fall through to English — `tier_for` routes each to its own
  bank and the keyboard charset test pins typeability.
- Every activation reverts by flipping one line back to `ComingSoon`, independently,
  with no user-data impact.

## Approvals required before cut
1. **Per-language activation sign-off** — name a native reviewer per activated
   language (`docs/ltr-activation-checklist.md`).
2. **Eric authorizes** the version bump and the build cut (the freeze).
3. **Release considerations** for the activated languages: App Store metadata /
   screenshots may need to reflect the newly available languages. Backend voices for
   all ten already exist, so no backend change is required for this set.

## Open question: the 104-commit gap (RESOLVE BEFORE CUTTING)
`feature/rtl-feedback` is **~104 commits / 59 feat+fix commits ahead of `main`**
(both at build 47). A build cut from this branch is therefore a **major release**,
and changes 1–3 above are only its most recent slice. Ahead of `main`, at least:

- **Live / user-visible:** the Play mode hub (CC-MODE-HUB), the consumer/education
  editions axis (CC-EDITIONS), the lineup swap (cut it/nl/sv/nb; add ru/ar/fa/ur;
  Russian Cyrillic keyboard), entitlements + regional grants (CC-ENTITLEMENTS), the
  Settings tools hub, Kid-Mode extra-attempt default, Daily auto-advance + single
  submit control, and several web perf/boot fixes (brotli wasm, self-hosted fonts,
  GPU orb, English-flash cloak).
- **Present but behind OFF flags (ship dormant):** online "Spell Off", spell-aloud
  voice input, word stories, syllable replay, attempts-shields / Climb shields.

**This means:** the release note above cannot stand as the whole story if 56 is cut
from this branch. Two honest paths:
1. **Cut a focused build** from a narrower branch (just changes 1–3 + activation),
   if a small update is what's intended.
2. **Do a full-delta release note** for the major release: audit all ~59 commits,
   classify each as live vs flag-gated, and confirm what already shipped in 48–55.
   That is a real task, not a paragraph.

Either way, whoever cuts the build confirms the source branch first. The safety
guarantees (no RTL, no audit content) hold on any of these branches.

## Sign-off
```
Source branch confirmed for the cut:  ____________________
Full-delta reconciled vs build 55:     ____________________
User-facing changes 1–3 approved:      ____________________
Languages activated (list):            ____________________
Build number assigned:                 ____________________
Approved to cut & submit (Eric):       ____________________   date: __________
```
