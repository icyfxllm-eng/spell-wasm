# CC-RTL ship checklist — ready to sign

**Status: AWAITING ERIC.** This un-gates Arabic, Persian, and Urdu in production —
they become Active and playable. It is a deliberate governance decision (CC-LINEUP-SWAP
D2: `RTL_SUPPORTED` is the one switch that may ever un-gate an RTL language). Nothing
here is applied on `feature/rtl-feedback`; the branch stays byte-identical to today's
gated production. This document is the reviewed, reversible change waiting for a yes.

## What flips, and what does not
- **ar / fa / ur → Active + playable.** Their banks (Leipzig CC BY, native-reviewed)
  ship. Rendering, keyboards, cursive per-letter feedback, and fonts are already built
  and were gated only by `RTL_SUPPORTED`.
- **ru → banked, still ComingSoon.** Russian's content is promoted alongside (same
  mechanism) so its bank is production-ready, but it stays gated until its own audit —
  activating it is then a one-line status flip, no RTL dependency.
- **Hindi → unchanged.** Stays audit-only (CC-HINDI-PHASE0 D8): not registered in
  production, serves an empty bank there.

## Prerequisites (must all be true before applying)
1. **Blockers 1 & 2 — DONE.** Banks rebuilt from Leipzig CC BY (attribution-only, no
   §4 copyleft decision needed); content is ship-ready. (commit `24e15f6`)
2. **Blocker 3 — native audit complete.** A native speaker of each of ar/fa/ur has
   played the audit-preview build and flagged the cuts, and those flags are ingested:
   ```sh
   python3 scripts/ingest-audit-flags.py audit-flags.txt   # per language
   python3 scripts/build-draft-banks.py                    # drops the flagged words
   ```
   Record who reviewed each language (the sign-off below names a human — a fork cannot
   supply `verified_by`).
3. **Blocker 5 — backend voices confirmed.** `LANG_VOICES` (backend, separate repo)
   has ar / fa / ur voices. Verify **Persian first** — fa-IR neural voices are thinner
   across providers. Without a voice a word cannot be spoken, so the language cannot
   play. Definitions are NOT required (the meaning surface hides when empty).

## The change (apply in order)

**Step A — promote content** (mechanical, scripted):
```sh
python3 scripts/promote-rtl.py     # copies reviewed drafts -> assets/words/{ar,fa,ur,ru}/,
                                    # adds them to build-wordlists LANGS, regenerates word_data.rs
```

**Step B — un-gate** (`src/consts.rs`), two edits:
```rust
// 1. the D2 switch
-pub const RTL_SUPPORTED: bool = cfg!(feature = "audit_preview");
+pub const RTL_SUPPORTED: bool = true;   // CC-RTL shipped

// 2. LANGS_BASE — ar/fa/ur only (ru stays ComingSoon)
-(AR, "العربية", ComingSoon, Rtl),
+(AR, "العربية", Active, Rtl),
-(FA, "فارسی", ComingSoon, Rtl),
+(FA, "فارسی", Active, Rtl),
-(UR, "اردو", ComingSoon, Rtl),
+(UR, "اردو", Active, Rtl),
```

**Step C — route the banks** (`src/words.rs`, `tier_for`): replace the shared empty
arm with explicit per-language arms; Hindi keeps the audit-only route:
```rust
-RU | AR | FA | UR | HI => audit_draft_or_empty(lang, tier),
+AR => simple_tier(AR_EASY, AR_MEDIUM, AR_HARD, AR_EXPERT, tier),
+FA => simple_tier(FA_EASY, FA_MEDIUM, FA_HARD, FA_EXPERT, tier),
+UR => simple_tier(UR_EASY, UR_MEDIUM, UR_HARD, UR_EXPERT, tier),
+RU => simple_tier(RU_EASY, RU_MEDIUM, RU_HARD, RU_EXPERT, tier),
+HI => audit_draft_or_empty(lang, tier),
```

**Step D — update the gating tests** to the shipped invariants. These currently assert
the *gated* state and MUST change deliberately (they are the tripwire):
- `consts::registry_tests`: `only_en_active` → active set is `{en, ar, fa, ur}`;
  `rtl_languages_are_registered_but_hard_gated` → `…and_shipped` (not blocked, active);
  `rtl_gate_matches_the_build_config` → `rtl_gate_is_shipped` (RTL_SUPPORTED true);
  delete `rtl_gate_survives_an_active_status` (its premise — the gate blocking an Active
  RTL language — is exactly what shipping removes).
- `words::content_tests`: `registered_but_contentless_languages_serve_no_words` →
  `hindi_serves_no_words_in_production` (Hindi is the only audit-only one left); add
  ar/fa/ur/ru to `content_languages_still_have_banks`.

**Step E — verify:** `cargo test --lib` (green), `cargo test --lib --features audit_preview`
(green), and the e2e menu suite (ar/fa/ur now appear as active options).

## Already verified (dry run, then discarded)
Steps A–E were applied on a throwaway branch and are known to compile and pass:
- **Content:** ar / fa / ur / ru each promote to **800 words** through the full
  production pipeline (charset, exclusions, length, balance floor) — gates green,
  14 locales.
- **Tests:** production **211 passed / 0 failed**, audit-preview **208 / 0**.
- **Byte-identity:** the branch was deleted; production on `feature/rtl-feedback` is
  unchanged (RTL_SUPPORTED still gated, ar/fa/ur ComingSoon, 212 tests).

## Risk & revert
- Each flip is one line and reverts in one line; Step A reverts by `git checkout` of
  `assets/words/{ar,fa,ur,ru}`, `word_data.rs`, and `build-wordlists.py`.
- The failure mode the tests guard against — a language falling through to `en_tier`
  and spelling English words under an Arabic name — cannot happen: the arms are
  explicit and the keyboard charset test would catch it.
- Partial RTL rendering (the original reason for the gate) is not a risk: rendering,
  keyboards, and feedback were built and reviewed in the audit build before this.

## Not authorized by this change
- **ru activation** — separate, on its own audit.
- **Backend TTS** — a backend config change, not in this repo.
- **Definitions** — deferred; the game ships RTL without per-word meanings.

## Sign-off
```
verified_by:            _______________________   (names the human reviewer per language)
  ar reviewed by:       _______________________
  fa reviewed by:       _______________________
  ur reviewed by:       _______________________
backend voices confirmed: ____________________
approved to ship (Eric): ____________________   date: __________
```
