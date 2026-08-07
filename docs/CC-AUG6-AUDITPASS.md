# CC-AUG6-AUDITPASS — Aug 6 device audit fix file

Eric's on-device audit (handwritten notes + annotated screenshots) found
three classes: a broken core loop, a class of settings that render but do
nothing, and two modes worth cutting. This file records the diagnoses and
the fixes. REVIEW-GATED: nothing enters submission until D1–D10 are signed.

Authority order: trust > layout law > fidelity > speed.

## Decisions

* D1 login — SIGNED (Eric, 2026-08-06): never required for play. Guest
  first; account only for leaderboard sync and purchase restore,
  prompted at the moment the feature needs it.
* D2 cut depth — SIGNED: UI removal + remote-flag OFF this release, code
  deleted next release if no reversal. Spell Racing reverses shipped
  Ghost Racing (build 55), so the flag is the safety rope. Spell It
  (say the letters) is NOT Say It and stays.
* D3 Climb tab slot — per Eric's notes; Climb stays reachable from home.
* D4 Spell Jr tiers — SIGNED: ONE Jr preset (friendly bank + no prices +
  big text) inheriting in-mode easy/medium. A third "hard Jr" is
  standard mode relabelled and blurs the Kid Mode boundary.
* D5 In Progress duplicates — CONTRADICTED BY THE CODE, see below. Not
  implemented; awaiting a replacement decision.
* D6 Spell It letters-only — per Eric's notes; letter definition comes
  from the registry, no per-language branches.
* D7 smashed-word imports — propose-split with user confirm, never
  silent. PENDING.
* D8 extra-attempt single source — Tools & Features wins. PENDING.
* D9 family voice / offline packs — SIGNED: hide behind a remote flag
  until the Voice Studio pipeline lands. Dead Download buttons violate
  F8.
* D10 settings-truth CI is permanent. PROPOSED STANDING RULE.

## F0 step-0 diagnoses

**F1 orb dead-tap — CLASS (c), read but the effect was a trap.** The tap
handler and TTS were both innocent; `tts=true` in the native debug was
telling the truth. `#plcCard` was authored inside
`<div class="wp-screen" id="wpPicker">`, and `.wp-screen` is
`display:none` until the picker opens. On the game screen
`maybe_offer_placement` found the card via `dom::exists` (true — it IS
in the document), added `.show`, and returned true, so `next_word()`
returned WITHOUT SERVING. A `display:none` ancestor hides the whole
subtree, so the card never appeared and Try/Skip were unreachable;
`should_offer_placement` stays true until one of those buttons sets
`placed`, so the orb was dead FOREVER, once per language. Regression
introduced when `learner_surfaces` defaulted ON.

**F2 mode exit — CLASS (c), a CSS cascade bug.** Practice always had a
44x44 ✕ wired to `close()`. Line 333 sets `.pr-screen` padding-top with
the safe-area inset; line 342 then set `padding:10px 14px` — a later
shorthand at equal specificity, which threw the inset away. On a notched
iPhone the header rendered under the Dynamic Island. `.dm-screen` is
declared BEFORE the safe-area rule so it escaped; `.wp-screen` sets the
inset inline. Practice was the only casualty.

**F4 scroll — a layout law the codebase never had.** `.wp-grid` declares
`overflow-y:auto` but is a flex child without `min-height:0`, whose
default `min-height:auto` refuses to shrink below content — so the
scroller never engages and content spills off a fixed parent. FOUR
scrollers had this (`.pchips`, `.wp-grid`, `.dm-missed`, `.climb-list`)
and TWO surfaces had no scroller at all (`.pr-screen`, and `#wpPlay`,
one of `.wp-screen`'s two instances). Big Text at 1.5x is what tips a
surface over, which is why it showed on device and never in a desktop
viewport. Spell Picture was where Eric hit it first, not the only place.

**F5 duplicate In Progress tiles — CONTRADICTS D5, NOT IMPLEMENTED.**
D5 assumes one saved instance per difficulty tier. `Run` has no tier
field, and creation is guarded on `(pic, lang)`, so a second run for the
same picture in the same language cannot exist. Resuming "the most
recent tier" would therefore fix nothing. Leading hypothesis is that one
subject renders in three PLACES (resume strip, a category list, the main
grid) via cross-listing. Needs a device repro before a replacement
decision.

## Fixes shipped (138)

F1: `#plcCard` moved to body level, and `maybe_offer_placement` now
refuses to consume a turn it cannot show — if `dom::rendered` says the
card is not really on screen, it takes `.show` back off and serves the
word. `dom::rendered` checks `offsetParent` plus a measured box (our
modals are `position:fixed`, where offsetParent is null by design).
F2: `.pr-screen` safe-area padding restored inline, and it scrolls.
F4: `min-height:0` on all four scrollers; `.wp-stage` becomes the play
screen's scroll region (the screen itself must not scroll — the lent
keyboard has to stay put).

## Invariants added

`scripts/scroll-check.mjs`, in the gate as "reachability laws":
1. every `overflow-y:auto|scroll` rule declares `min-height:0`;
2. every fixed flex-column surface scrolls or has a descendant that
   does, checked against the real element tree (an earlier draft matched
   selector names by substring, passed vacuously, and would have shipped
   the bug it exists to catch);
3. no `.wp-modal` nested inside a mode screen. This caught `#wpHk` as
   well as `#plcCard` — `#wpHk` was never a live bug because it only
   opens while the picker is up, but the law does not exempt modals that
   happen to be lucky.

## Test-infrastructure faults found on the way

**placement.mjs had NEVER RUN.** `run.mjs` declared
`['playhub', playHub, placement]` but the loop destructured
`[name, mod]`, silently dropping every module after the first. The spec
written to protect the placement card never executed once — a dead spec
is a dead switch for tests, the same failure shape this whole file is
about. Fixed to `[name, ...mods]`; the app suite went 80 -> 82.

**The e2e report was being overwritten.** App and site runs both wrote
`tests/e2e/TEST-REPORT.md`, so the site's 11 results clobbered the app's
80 and no record survived of WHICH app specs ran. Split into
`TEST-REPORT-app.md` and `TEST-REPORT-site.md`.
