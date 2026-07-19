# SpellGame — Build 56 release note (FULL DELTA)

**Status: AWAITING ERIC'S APPROVAL.** Nothing here is cut or shipped.

> **Version.** The build number is set automatically by fastlane to
> `latest_testflight_build_number + 1` (`fastlane/Fastfile`). TestFlight is at 55,
> so this cuts as **56** on its own; the committed `CURRENT_PROJECT_VERSION = 47` is
> stale/ignored and `MARKETING_VERSION` stays 1.1. Running the cut is still Eric's
> call (freeze).
>
> **Scope.** This is the FULL delta of `feature/rtl-feedback` over `main` (build 47):
> ~104 commits, 59 feat/fix. It is a **major release**, not a point update. Builds
> 48–55 were cut from somewhere other than `main` (which never moved), so **some of
> this may already be on TestFlight** — whoever cut 48–55 should reconcile what is
> genuinely new in 56. Everything below is classified by its REAL runtime state
> (verified against `src/flags.rs`, not commit messages).

> **TestFlight vs App Store flags.** `src/flags.rs` defaults are set for the QA
> build: `say_it`, `photo_list`, `spell_aloud`, `ghost_racing`, `syllable_replay`,
> and `attempts_shields` default **ON** so testers exercise them; `word_stories` and
> `online_spelloff` stay **OFF**. The docstring notes these revert to OFF "per PR"
> for the App Store. So this list is the **TestFlight** experience; the App Store
> subset is a later, separate flag decision.

---

## 1. Core gameplay change (LIVE, flag ON) — the headline
- **One attempt per word.** The legacy 3-try mechanic is retired; the base game is
  now one shot per word (CC-ATTEMPTS-SHIELDS, build-54).
- **New safety net:** an extra-attempts toggle (normal mode) and **shields** in The
  Climb replace it. `attempts_shields` defaults ON for QA — testers must feel the
  new one-shot flow. This is the biggest behavioral change in the build.

## 2. New surfaces (LIVE)
- **Play mode hub** — a tile-based launcher for the game modes (CC-MODE-HUB;
  `config/modes.json` is the mode registry). Wired unconditionally.
- **Tools & Features hub in Settings** — one place to toggle the optional features,
  with per-level explainers.
- **Daily Challenge** — a single submit control per viewport + auto-advance (plus a
  fix for a stale auto-advance timer that could skip a word).
- **Native-status readout in Settings** — diagnostics for the on-device stack.

## 3. iOS-only features — ON for TestFlight QA (no-op on web/Android)
Backed by the new NativeLanguageKit Capacitor plugin + AVSpeech offline TTS:
- **Say It** — on-device pronunciation practice (`say_it` ON; hard-off in Kid Mode, COPPA).
- **Spell It Out Loud** — voice spelling *input*: speak letter names, the parser
  produces what a keyboard would (`spell_aloud` ON, per Eric 2026-07-15).
- **Photo-to-word-list** — VisionKit OCR of a handout into a word list (`photo_list`
  ON; hidden in Kid Mode).

## 4. The Climb
- **Ghost racing** — race your best local run (`ghost_racing` ON).
- **Climb shields** — part of the attempts-shields safety net above.

## 5. Languages & content (LIVE)
- **Lineup swap** — cut it/nl/sv/nb; added ru/ar/fa/ur as coming-soon; the menu
  reflects the new roster (CC-LINEUP-SWAP). *(Turkish was reinstated here, then cut
  again by CC-HINDI-PHASE0 — net: not in the lineup.)*
- **Russian Cyrillic keyboard** added; contentless languages **no longer fall
  through to English** words (an explicit empty bank instead).
- **English-only launch** — Spanish (and the rest) are Coming Soon; English is the
  only active study language. **Entitlements/regional grants do NOT override this** —
  they gate purchase/feature access and still consult `is_active_lang`.
- **Richer English word bank** — rebuilt from Leipzig to 800 words (200/tier). Other
  languages' banks were grown too, but ride along dormant until activated.

## 6. Privacy, performance, polish (LIVE)
- **Privacy:** the browser no longer calls `dictionaryapi.dev` directly (`def_lang` deleted).
- **i18n:** the hint's "(N letters)" count is translated; boot cloaks translatable
  chrome to kill the English-flash reflow.
- **Web perf:** brotli-precompressed wasm + streaming compile; self-hosted fonts
  (no FOUT/external stalls); GPU-composited orb glow (no mobile stutter).
- **Profanity:** diacritic-safe normalization in the per-language screen.

## 7. Present but DORMANT (flag OFF — ships dark, zero observable effect)
- **Word stories** (etymology cards) — OFF until the CC BY-SA attribution approach
  is approved.
- **Online "Spell Off"** (async 1v1) — OFF until the `/api/match` backend is deployed.

## 8. Present but GATED — not user-reachable in production
- **The RTL engine** (ar/fa/ur rendering, keyboards, cursive feedback, Naskh/Nastaliq
  fonts) — gated by `RTL_SUPPORTED`; lazy fonts are never even fetched.
- **Hindi + the entire audit-preview build** — `#[cfg(audit_preview)]`, compiled out
  of production; verified byte-absent.
- **The 10-language activation & RTL ship packages** — `docs/*-checklist.md`;
  documents only, nothing applied.

## 9. Infrastructure (not directly user-facing)
- Consumer/education **editions axis** (compile-time; this is the consumer build).
- **Entitlements** resolver + regional grants (CF-IPCountry) + country→language map.
- Wordlist **trap registry + tagger** (content-quality tooling); word_data regen.
- CI/test/build fixes (entitlement purity gate, endonym test, font-lazy gate).

---

## Optional in this same build: activate up to 10 languages
Independently of the above, the ten content-ready LTR/CJK languages
(es/fr/de/pt/pl/vi/ko/ja/fil/zh) can be flipped ComingSoon→Active — a verified,
per-language, reversible status flip. Full package + sign-off:
**`docs/ltr-activation-checklist.md`**. RTL (ar/fa/ur) is a separate later build:
**`docs/rtl-ship-checklist.md`**.

## Safety guarantees (hold regardless of the above)
- **No RTL ships** — `RTL_SUPPORTED` is false in production.
- **No audit content ships** — `audit_preview` is compiled out; draft banks verified
  byte-absent from the production wasm.
- **English-only active** unless Eric applies the activation package.
- Production `cargo test --lib` green (212/0).

## Approvals required before cut
```
Source branch confirmed for the cut:        ____________________
Reconciled vs what builds 48–55 shipped:     ____________________
QA flag set confirmed (ON: say_it, photo_list, spell_aloud,
  ghost_racing, syllable_replay, attempts_shields):  ____________________
Languages activated (none / list):           ____________________
Approved to cut & submit to TestFlight (Eric): __________  date: ______
```
