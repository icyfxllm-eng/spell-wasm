# SpellGame — Build 56 release note (reconciled against 48–55)

**Status: AWAITING ERIC'S APPROVAL.** Nothing here is cut or shipped.

> **Version.** fastlane sets the build number to `latest_testflight + 1`
> (`fastlane/Fastfile`); TestFlight is at 55, so this cuts as **56** automatically.
> The committed `CURRENT_PROJECT_VERSION = 47` is stale/ignored; `MARKETING_VERSION`
> stays 1.1. Cutting is Eric's call (freeze).

## Reconciliation — this is NOT a fresh major release
The branch is ~59 feat/fix commits over `main` (build 47), but `main` is not what
shipped. The cut points are the **`build-52` and `build-54` branches**. Comparing:

- **`build-52` is fully contained** in `feature/rtl-feedback`.
- **`build-54` is the last cut branch**, and **~31 of the 59 commits already shipped
  in it.** So most of what looked like "new" is **already on TestFlight**, including
  the entire headline gameplay change.

**Already shipped (by ~build 54) — NOT new in 56:**
- **One attempt per word** (3-try retired) + extra-attempts / Climb shields.
- All **iOS QA features**: Say It, Spell It Out Loud, Photo-to-word-list, the
  NativeLanguageKit plugin + AVSpeech offline TTS.
- **Ghost racing** in The Climb; **word stories** & **online Spell Off** (both dark).
- The **Tools & Features hub**, **entitlements** + regional grants, **Kid-Mode**
  extra-attempt default, **native-status readout**.
- All four **web perf/boot fixes** (brotli wasm, self-host fonts, GPU orb, i18n cloak).
- **English-only launch** (Spanish → Coming Soon); the **privacy fix** (no
  dictionaryapi.dev); profanity normalization; Daily auto-advance + submit control.

That list is what testers on 54/55 already have. **Do not re-announce it as new.**

## Genuinely NEW since build 54 — the real build-56 delta
**User-facing:**
- **Play mode hub** — a tile-based mode launcher (CC-MODE-HUB), wired live. The
  most notable new surface.
- **Lineup swap** — menu roster changes: cut it/nl/sv/nb, add ru/ar/fa/ur as
  coming-soon, plus the **Russian Cyrillic keyboard**; contentless languages no
  longer fall through to English.
- **Richer English word bank** — rebuilt from Leipzig to 800 words (200/tier).
- **Translated hint count** — "(N letters)" localized (minor; non-English UI only).

**New but INFRA (little/no direct user effect in the consumer build):**
- Consumer/education **editions axis** (compile-time; this is the consumer build).
- Wordlist **trap registry + tagger** (content tooling); word_data regen.

**New but GATED — absent from production, testers can't reach it:**
- The **RTL engine** (ar/fa/ur rendering, keyboards, cursive feedback, Naskh/Nastaliq
  fonts) — `RTL_SUPPORTED` false.
- **Hindi + the entire audit-preview build** + the flag harness — `#[cfg(audit_preview)]`,
  compiled out; byte-absent from production.

So the honest one-liner: **build 56's user-visible delta over 55 is the Play mode
hub, the language-roster swap + Russian keyboard, a bigger English bank, and a hint
i18n fix** — everything else new is gated or internal.

## ⚠ Regression to fix BEFORE cutting from this branch
- **Service-worker cache version went backwards.** Build 54 shipped `sw.js` at
  `v43`; this branch is still at `v42`. Cutting 56 at v42 would push an older cache
  version to clients that already have v43. **Bump `CACHE_VERSION` to `v44`+ before
  the cut.**
- **Web-only (not TestFlight):** build 54's `Caddyfile` added a CSP that isn't on
  this branch. It affects the `spellgame.net` web deploy, not the iOS bundle — but
  reconcile it before the next *web* deploy.
- **Decision docs diverged:** build-54 carries `§5.1/§5.2/§10` decision entries not
  on this branch (docs only, no build impact, but the record is split).

## ⚠ Build 55 is unaccounted
There is no `build-55` branch. Build 55 was cut from somewhere after 54; if it
advanced past 54, some "new since 54" items above may already be in 55. **Whoever
cut 55 must confirm its source** so 56's delta is exact.

## Flag state (TestFlight build)
`say_it, photo_list, spell_aloud, ghost_racing, syllable_replay, attempts_shields`
default **ON** (QA); `word_stories, online_spelloff` default **OFF**. Per
`flags.rs`, the ON set reverts to OFF "per PR" for the App Store — a separate later
decision.

## Optional in this build: activate up to 10 languages
Independently, es/fr/de/pt/pl/vi/ko/ja/fil/zh can be flipped ComingSoon→Active — a
verified, reversible, per-language status flip. See `docs/ltr-activation-checklist.md`.
RTL (ar/fa/ur) is a separate later build (`docs/rtl-ship-checklist.md`).

## Safety guarantees
No RTL ships (`RTL_SUPPORTED` false); no audit content ships (`audit_preview`
compiled out, byte-absent); English-only active unless the activation package is
applied; production `cargo test --lib` green (212/0).

## Approvals / actions before cut
```
Bump sw.js CACHE_VERSION v42 -> v44+:        ____________________
Confirm build 55's source branch:            ____________________
Reconcile Caddyfile CSP (web deploy):        ____________________
Source branch for the cut confirmed:         ____________________
QA flag set confirmed:                       ____________________
Languages activated (none / list):           ____________________
Approved to cut & submit to TestFlight (Eric): __________  date: ______
```
