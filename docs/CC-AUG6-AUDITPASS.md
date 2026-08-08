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
* D7 smashed-word imports — SIGNED (Eric, 2026-08-08). Propose-split
  with user confirm, never silent. Retroactive: F14 shipped in 142 built
  to this recommendation, so the signature confirms what is running.
* D8 extra-attempt single source — DELEGATED to Claude and resolved: the
  two controls are not duplicates (flag vs preference), Eric dropped the
  removal plan, and the preference is now authoritative.
* D9 family voice / offline packs — SIGNED: hide behind a remote flag
  until the Voice Studio pipeline lands. Dead Download buttons violate
  F8.
* D10 settings-truth CI is permanent — SIGNED (Eric, 2026-08-08) as a
  STANDING RULE. Every future toggle needs a manifest entry with an
  observable effect and a live effect test, or the build fails. In one
  day it found a genuinely dead switch (toolSayItToggle), forced the
  suppressed-row rendering, shrank correctly when F7 removed two
  controls, and proved its own selftest still bites on every run.

## THE REVIEW GATE IS CLOSED (2026-08-08)

The file opened "REVIEW-GATED: nothing enters the submission pipeline
until Eric signs D1–D10." All ten are now resolved — D1/D2/D4/D9 signed
2026-08-06, D3/D6 per Eric's notes, D5 and D8 delegated to Claude and
implemented, D7 and D10 signed 2026-08-08. D3 needed amending on the way:
"replace the Climb tile" would have deleted the mode's only entrance, so
Spell Picture became a fourth tile instead.

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

## Ship 139 — the queue

**Slower Voice 0.55** (Eric, 2026-08-06). Not a wiring fix: the setting
reached both paths already. 0.9 -> 0.7 was a 1.29x duration change,
over the 1.25x bar and still imperceptible, which is why it read as
dead. 0.55 gives ~1.64x. SpeechRate.swift's doc comment updated too —
it documented 0.7 as "slow" and would otherwise have become a lie.

**D8 (delegated to Claude).** The two controls are NOT duplicates:
`toolShieldsToggle` is the feature flag, `extraAttemptsToggle` is the
preference, and `extra_attempt_ctx` needs both. Eric dropped the
removal plan. The real defect was `s.extra_attempts || s.kid` — in
Spell Jr the preference was overridden, so moving the switch did
nothing. The preference is now authoritative and Kid Mode DEFAULTS it
on instead. A migration flag (`extraAttemptsSeen`) carries devices that
were already in Kid Mode: they experienced the second try while their
stored preference stayed false, so a naive read would have silently
taken it away from a young speller — and an age-locked device cannot
reach the toggle to put it back without the parent gate.

**D5 (delegated to Claude) — NOT the fix the file asked for.** The
premise fails twice. `Run` has no tier and creation is keyed on
`(pic, lang)`, so tiers cannot duplicate anything. Tracing every
emitter: one resume card (strip if in the first four, gallery head if
after — index-disjoint, never both) plus one browse tile. The gallery
proper skips unfinished runs; Category and Folder views each render one
flat list and a picture has exactly one folder. That is TWO
appearances, both legitimate. Three is not reproducible from the code,
so no fix was invented for it. What shipped is the provable half:
`resume_split` extracted pure, and a test that a subject offers at most
one resume affordance, the strip fills before it spills, and re-opening
a subject never clones its record. If a future hub section adds a
second, CI catches it. OPEN FOR ERIC: on device, are the three in the
Continue strip, a folder, or the hub — and do they show the SAME
progress number or different ones? Different numbers would mean three
real Run records, which the code says cannot exist, and that would be a
different hunt.

**F15 offline packs hidden (D9).** Class (d), which F0's taxonomy does
not have: not unpersisted, not unread, not unimplemented — implemented
and UNSERVED. The client verifies signatures and streams shards; there
is no `/packs` route in backend/ at all. The section (not just the
buttons) is behind `flags::offline_packs`, default OFF, and the wire
path returns before attaching listeners or firing the auto-suggest
toast. A heading over an empty box still promises a feature.

**config/settings-effects.json** lands as F8's data, with the
`suppressed_by` column Eric asked for. Writing it surfaced the real
shape of the audit: ALL 17 rendered controls are wired — there is no
class (a) or (b) anywhere in settings — and FIVE are suppressed by Kid
Mode (extra attempts, reminders, Say It, photo import, Ghost Racing).
Four of those are deliberate; one was the D8 bug. Spell Jr silently
overrides nearly a third of the settings surface and none of it renders
as overridden, which is very likely what most of the "dead switches"
in the audit actually were. The CI half of F8 is not in this ship.

## Ship 140 — F8, the settings-truth invariant

The gate is `scripts/settings-truth-check.mjs`. It deliberately does NOT
check wiring, because diagnosing all seventeen controls produced the
finding that shapes this feature: EVERY control was already wired. Not
one was disconnected. A connectivity check would have gone green on all
seventeen while every symptom Eric found stayed broken. They failed
three other ways — SUPPRESSED by a mode, TOO SMALL to perceive, or
UNSERVED by a backend that does not exist.

So the gate checks that each control names an observable effect and a
test that EXISTS, and that a mode-overridden control declares its
suppressor. Five laws: rendered ⊆ manifest, manifest ⊆ rendered, an
effect stated, a live effect test named, `suppressed_by` present (null
is a claim too). `--selftest` plants a dead toggle and must fail the
build — the gate runs BOTH itself and its selftest, so a gate that
stops biting is itself a gate failure.

FOUND BY WRITING THE TESTS — one genuinely dead switch. `toolSayItToggle`
has exactly one consumer, gating `wire()` for a mode the code calls
dormant with no entry points; the Spell It launcher beside it is gated
by `spell_aloud`. It resolves into the F7 cut rather than needing a fix.

SUPPRESSED ROWS NOW SAY SO. Five of seventeen controls are overridden by
Kid Mode (extra attempts, reminders, Say It, photo import, Ghost
Racing). Four are deliberate and NONE of them told the player — a live
looking switch that ignores you is most of what "dead switch" meant in
the audit. `set_suppressed` disables the input and marks the row.

BIG TEXT REACHES THE PLAY SURFACE (Eric, 2026-08-06). All 32 existing
rules scaled chrome — labels, notes, captions, footer — and nothing
matched the word being spelled. A child turns this on because reading
is hard, and the one thing that stayed small was the thing they were
reading. `.ltr` 24->32, hint 18->23, feedback 15->19, meaning 13.5->17.

HARNESS SPLIT. Four tools are Avail::Native/Server; in headless
Chromium `native_lang::available()` is false, so flipping those flags
changes nothing a web test could see and an assertion would pass for the
WRONG REASON. They declare `harness:"device"` and a `maestro:` id, owed
a device pass. Calling them covered would have been the same lie the
gate exists to catch.

THREE OF MY OWN MISTAKES, all caught by the tests rather than review:
the prefs key was `spell_prefs` (real: `byear_prefs_v1`), which would
have compared undefined === false and passed vacuously; the big-text
assertion measured `body`, which no rule touches (16 -> 16); then it
scoped to `#setupScrim`, but the document's first `.set-row .lbl2 small`
lives in `#accountScrim`. A test asserting `classList.contains` would
have been green through all three.

App suite 82 -> 91. The count was checked, not the registration:
placement.mjs was registered for weeks and never ran.

## Ship 141 — F16, the platform-wrong copy

TWO strings, not one. The audit named the explainer; the worse offender
was the failure path. `help.howItWorks` told an iPhone user their words
are "spoken with your browser's voices — smoothest in Chrome or Edge on
a computer" and that progress is "saved in this browser only" — three
false claims in one sentence on iOS. `speech.noVoice` said "This browser
can't speak — try Chrome or Edge", and it fires exactly when audio has
already failed, so a parent mid-problem was told to install a different
browser.

MECHANISM. `i18n::t_platform(key)` prefers `<key>.native` IN THE CURRENT
LOCALE on the wrapped build, reading `window.isWrappedPlatform()` — the
shell already declares that as THE single source of truth for "am I
wrapped", so this asks it rather than inventing a second answer that
could drift. `translate_page`'s `apply()` now routes EVERY `[data-i18n]`
key through it, so any future key gains platform-awareness for free;
F16 asked for one source, not a check per surface.

IT DOES NOT FALL BACK TO ENGLISH `.native`. An untranslated locale keeps
its own ordinary string. Wrong-language is a worse failure than
platform-wrong, and silently serving English to a Spanish child to fix a
platform detail would trade a small lie for a big one.

CONTENT: 30 strings, both keys across all fifteen locales, parity intact
at 581 keys. UNAUDITED for the fourteen non-English locales (Eric:
"draft all fourteen unaudited", 2026-08-06) — same posture as the gloss.
Each reuses that locale's OWN existing bold terms verbatim (↻ Fallos,
↻ 間違い, ＋ Maneno yangu) so the nav labels still match the screen
instead of being re-coined. iOS copy is three sentences, no browser.

GUARD: i18n-check gained a law — a key with a `.native` sibling must not
be reached through plain `t()`. Proven by regressing the call site and
watching it fail. Without it the next edit silently puts browser copy
back on a phone, which is precisely how this survived to a device audit.

CAUGHT IN DRAFTING: the Japanese string contained the English word
"answer" (`answerのたびに`), found by scanning non-Latin drafts for Latin
words before they reached a locale file. The first version of the guard
itself referenced `fs`/`ROOT`, which that script does not define — it
crashed rather than passing vacuously, the right failure mode.

STILL OPEN in F16: the stray white tail element Eric circled. Not
located; needs the screenshot or the screen name.

## Ship 142 — F14 smashed-word segmentation

`propose_split` (photo_import.rs): fewest-pieces DP over a folded bank
set, 2-letter minimum piece, 24-char cap. "Thisisanexample" comes apart
into this·is·an·example; a word already in the bank is never offered a
split, so "notebook" stays whole.

IT ONLY PROPOSES, AND HERE IS WHY. A prototype over the real EN bank
mis-split 1 in 10 plausible custom words: "Sundeep" -> sun · deep. A
silent auto-split would rename somebody's child. That case is pinned as
a TEST — the behaviour is correct (both pieces really are bank words),
and the safeguard is the human. D7's confirm step is not ceremony.

THE PIECES ARE SHOWN, NOT A COUNT (Eric, 2026-08-06). "Split: this · is
· an · example", full width under its chip. Reading the pieces is the
only way a parent can catch a Sundeep, so a count would have removed
the very thing that makes the confirm meaningful.

CEILING = THE BANK. "thecatsatonthemat" proposes nothing because `sat`
and `mat` are missing from the EN bank (BD-G3 again). No proposal is
the right failure; a partial or wrong one is not.

VERIFIED IN A BROWSER, not reasoned about. The photo flow is
native-gated so e2e cannot reach the review sheet through the camera —
`__spelltest.photoReview(words)` fabricates the recognizer RESULT and
lets the real classifier and renderer run (it saves nothing and skips
no gate). The spec asserts the pieces render with separators, that a
bank word gets no proposal, and — the one that matters — that NOTHING
has split before the tap. App suite 92 -> 93.

## FOUND: Calendar, Translate and Reports cannot tile (pre-existing)

`play_hub` builds `ctx.enabled` as every mode id filtered through
`flags::is_on(id)`, and `is_on` is a match with `_ => false`. There is
no arm for calendar, translate or reports, so all three read as OFF,
`permitted()` refuses them, and their tiles never render. Their only
entry points are the hidden buttons the tile would have clicked
(calOpenBtn / trOpenBtn / repOpenBtn), so THREE SHIPPED MODES ARE
UNREACHABLE.

This is what Eric was asking when he said "is the calendar and translate
installed on the app yet" — they are built, wired, and invisible.

scripts/modes-check.mjs has been reporting exactly this and is NOT in
gate.sh, so it has failed unread. Fixing the flags is a one-line-each
change; fixing the silence is adding the check to the gate. Both belong
in the next ship, not this one — 142 is F14 and I am not bundling a
three-mode visibility change into it unannounced.

## Ship 143 — three shipped modes could not be reached

Calendar, Translate and Reports were built, wired, tested and INVISIBLE.
`play_hub` builds `ctx.enabled` from `flags::is_on(id)`; `is_on` is a
match ending in `_ => false`; those three had no arm. So they read as
OFF, `permitted()` refused them, and their tiles never rendered. Their
only other entry points are the hidden buttons the tile would have
clicked. Eric asked hours earlier whether Calendar and Translate were
"installed on the app yet" — they were, and nothing could open them.

Three flag fns (default ON — all three are live in the registry and were
never meant to be switchable off) plus their three `is_on` arms.

WHY NOBODY SAW IT. Two reasons, both worth keeping in mind:
  * scripts/modes-check.mjs had been saying it in plain words — "a
    registry entry with no implementation is a tile leading nowhere" —
    and was NOT in gate.sh. It is now. A check nobody runs does not
    exist.
  * A browser cannot show this. All three are platforms:["ios"], so the
    web build omits them for a legitimate reason and looks identical
    either way. The bug was only visible on device, which is exactly
    where Eric found it and where CI never looks.

MY FIRST TEST FOR THIS WAS VACUOUS. It built ctx.enabled straight from
the live list and passed with the bug still in place — it proved
`permitted()` works, which was never in doubt. Retargeted at the seam
that actually failed (`flags::is_on` answers yes for every live mode)
and verified by removing an arm and watching it fail. That makes three
times today a test had to be checked against the bug it claims to catch:
the wp-grid collapse, the F8 selftest, and this.

## Ship 144 — F3 SOLVED, and it was none of the suspects

Instrumented before optimising, and the measurement changed the fix.
`render_picker_body` timing, published to window.__pickerMs and shown on
the #nativeStatus debug row (the test seam is dev-only and this bug lives
on device, so a seam counter would have been unreadable where it matters).

  query   tiles   before -> after
  zzzz        0    154ms
  mona        1     37ms
  "e"       368   3503ms -> 65ms
  "a"       378   3727ms -> 190ms cold, 65ms warm

~10ms PER TILE, and not one of them was DOM. `subject_total()` runs once
per tile, and for a scan-locked picture it reloaded the entire saved
State from storage AND ran `spellpic::plan` — the scan-stack capacity
planner, the same computation whose full sweep takes ~18 minutes over 382
subjects — just to count words. A one-letter search planned 378 pictures
on the keystroke.

WINDOWING WOULD HAVE BEEN THE WRONG FIX, and it is the one I proposed to
Eric before measuring. It would have hidden the cost behind fewer visible
tiles while leaving a planner call on every tile that did render.

Fixed by memoizing on (pic, lang, seed) — deterministic, so it cannot go
stale — and threading the caller's already-loaded state through so no
tile re-reads storage. Then `warm_totals` fills the memo in 12-picture
chunks between timer ticks when the picker opens, so the first broad
search finds it warm rather than paying ~2s mid-keystroke.

ALSO: .wp-screen respected the TOP safe-area inset and not the bottom
(flat 10px), so on any iPhone with a home indicator the last row of tiles
sat under it. .pr-screen already did this correctly — Spell Picture was
the one surface that did not.

## OPEN: Spell Picture cannot be scrolled on device (build 150)

Eric: "cant scroll, nothing moves at all". NOT the .wp-grid collapse —
that shipped fixed in 141/build 148, and he is on 150. A 375x667 Chromium
viewport does not reproduce: the grid overflows (654 in a 505 window) and
scrolls its full range. Ruled out so far: no global touchmove/touchstart
preventDefault in the shell, no touch-action:none on the grid or an
ancestor, the long-press handler does not preventDefault, and the
Capacitor config sets no scroll override with no Swift touching the
scrollView.

So the geometry is published too — `window.__pickerGeom` as
"grid clientH/scrollH screen clientH/scrollH" on the same debug row. It
separates the live hypotheses: scrollH == clientH means nothing to
scroll; screen scrollH > its clientH means the flex chain is not
constraining on device and the SCREEN is the overflowing box; grid
overflowing but immovable means touch is blocked. Guessing at a device I
cannot see has been the expensive way to work today.

## Maestro device flows (the four harness:"device" manifest entries)

tool-photo-effect, tool-spellaloud-effect, photo-replace-effect,
tool-spelloff-effect. Each header states WHY it cannot be an e2e spec:
`native_lang::available()` is false in headless Chromium, so a web
assertion would pass whether the flag works or not. Verifying the ids and
labels against the shell first caught three wrong row names I had
guessed — the real ones are "Photo → word list", "Spell It" and
"Online Spell-Off". The spell-aloud header also records the naming trap:
`sayItBtn` is governed by the SPELL_ALOUD flag, not `say_it`.

## Ship 145 — F12 "no prices" made true, and a false claim removed

F12's label promises "Bigger text, friendly words, no prices". Big text
was real; friendly words were real (kid_filter on selection); NO PRICES
was not. Calendar is kidSafe:true, and its planner rendered
"Unlocks with Complete" for any week past the current one — an upsell on
a surface a child reaches, on the very setting that says there are none.
modes.rs states the Little Speller zero-purchase-surface doctrine and a
playhub spec enforces it FOR TILES; nothing extended it to copy rendered
INSIDE a surface, which is exactly how this survived. Kid Mode now shows
absence.

THE FREE CLAIM (Eric, 2026-08-08: "the app is 12.99 no matter what").
help.howItWorks said "Everything is free" in all fifteen locales — and I
had carried that sentence into the iOS variants I wrote for F16 without
questioning it. With a paid download AND net.spellgame.complete as a
product, it was false on two counts, on the screen App Review reads.
Eric then confirmed to strip it from the site copy too. Removed from
both keys in all fifteen locales; deleting a sentence needs no
translation, so nothing new is unaudited.

pt/pl/ja needed a second pass — their ORIGINAL copy words it differently
from the variants I authored (Tudo é grátis vs gratuito, Wszystko jest
darmowe vs za darmo, すべて無料です vs だよ). A blanket sweep would have
reported success while the claim stood in three languages, so the check
is a regex over every free-word in every locale, not a replace count.

MY TEST WAS WRONG FIRST. It banned the word "Complete" outright and
flagged the goal card "Complete 3 Daily Challenges", where it is a verb.
A test that cannot tell an upsell from ordinary copy would force the
product to avoid a common English word. Retargeted at the actual upsell
— the "Unlocks with" phrase and the .gd-chip element. Fourth test today
that needed correcting; every one failed loudly rather than passing
empty. 94/94.

## Ship 146 — F13 rejection wiring (tutorial deferred)

F13 was far more built than the sheet implied. Saying the TARGET word
already voided the utterance and re-prompted, even with letters mixed in;
letters already came from a per-language lexicon with no per-language
branches (D6 satisfied); and the re-prompt copy "Spell it letter by
letter!" already existed in all fifteen locales.

THE HOLE: `interpret()` returns SpellOutcome::WholeWord for ANY word-like
utterance, and nothing outside the module ever called it. So the rule
lived on paper — say any word that is not the target and the parse
scraped whatever letters it could out of the noise. Now wired in
`on_final`, with two guards that are the whole design:

  * AFTER the edit-command check. "undo" and "clear" are single WORDS and
    a test confirms they classify as WholeWord — wiring the rejection
    ahead of that would have swallowed voice editing entirely.
  * ONLY when the segment accumulated nothing. Letters spelled during a
    segment are already on screen; a child who spells c-a-t then trails
    off into a word must keep them. `says_target` may discard visible
    letters (D3: saying the answer voids the attempt) but a stray word
    must not carry that penalty.

Threshold checked before wiring rather than after: WHOLE_WORD_YIELD is
0.5 and the comparison is `<`, so "um see ay uh tee" (3/5) passes and a
lone stray word beside one letter (1/2) still keeps the letter. It errs
toward accepting, which is the right direction for a child spelling
slowly.

DEFERRED: the Spell It tutorial (orb → listen → say letters → confirm).
New UI and new copy, so fourteen unaudited translations — Eric asked for
it separately, after seeing the rejection behave on device.

## Ship 147 — F13 the Spell It guide (the audit's last feature)

spell_aloud.rs had carried this note for months: "Routes to the
CC-SPELLIT-GUIDE guide screen once that spec lands; direct entry until
then (deviation flagged in the review notes)." This is that screen, and
the deviation closes.

ONCE, WITH A WAY BACK (Eric, 2026-08-08). Four steps — hear the word,
listening, spell it letter by letter, tap ✓ — on first entry, remembered
in storage, then straight to play. "How this works" on the mic surface
brings it back, because showing it once must not make it unfindable:
Eric's yellow-circled mic annotation was about discoverability, and a
guide you can tap past once and never find again fixes nothing.

Replaying does NOT clear the seen flag. Asking for a reminder is not the
same as never having read it, and clearing would re-show it unbidden on
the next entry.

THREE OF THE FOUR STEPS REUSE COPY THAT ALREADY SHIPPED — top.spellIt,
voiceSpell.listening, voiceSpell.spellItOut — so the guide teaches the
same words the live mic uses, and only four new keys were needed (60
unaudited strings rather than ~105). Deliberately NOT reused:
sayit.begin ("Continue"). It reads perfectly and it is in the CUT mode's
namespace — D2 deletes Say It's code next release, and a guide depending
on sayit.* would break then. Borrowing from something scheduled for
deletion is how a mystery failure arrives two releases later.

Body level, like every modal since F1: the reachability law would reject
it inside a mode screen, and that law exists because #plcCard nested in
a hidden screen is what killed the orb.

## Ship 147 — THE SCROLL FIX (and my three wasted rounds)

Eric, build 150: "cant scroll, nothing moves at all". I asked for the
geom readout three times instead of eliminating the obvious pattern.

THE CAUSE: a nested scroller inside a position:fixed container —
`.wp-grid { overflow-y:auto }` inside `.wp-screen { position:fixed }`.
That is the classic WKWebView failure, and it scrolls PERFECTLY in
Chromium. Which is exactly why three rounds of desktop measurement told
me "654 in a 505 window, scrolls its full range" while Eric could not
move it at all. Every measurement I took was true and none of them was
about his device.

THE FIX: one scroller — the screen itself — plus
-webkit-overflow-scrolling:touch and an explicit touch-action:pan-y.
Verified with 378 tiles: 11,205px of scroll range, last tile reachable.
Scoped to #wpPicker because #wpPlay is also .wp-screen and must NOT
scroll, or the lent keyboard slides away mid-word.

MY LINT BLOCKED THE FIX TWICE. It demanded min-height:0 on any scroller,
then rejected it for having nothing to hold space — unsatisfiable for
#wpPicker, which is not a flex child at all but viewport-pinned via
.wp-screen's inset:0. It judged one rule at a time and could not see
across them. Now it resolves the element's full token set against the
DOM, because a law that cannot be satisfied is a law someone disables.

LESSON, recorded because it cost the most today: when a symptom is
device-only and the desktop says fine, stop measuring the desktop. The
pattern (nested scroller in a fixed container) was diagnosable from the
CSS alone on the first pass.

## The translation queue is now reviewable

Two sheets under docs/review/, so the language pass is a task someone
can pick up rather than a hunt:

  * gloss-es-review.md — 258 Spanish rows, sorted by English concept,
    with what the reviewer is and is NOT checking (the keys are already
    live bank words; only the sense pairing is in question) and the two
    fields to set when done. Spanish is the cheapest language to light:
    the tables, loader, CI and resolver are all finished, and ONE
    signature turns the translator from dark to usable.
  * ui-strings-unaudited.md — the 98 UI strings drafted during this
    audit (F16 platform copy, F14's split label, F13's guide), grouped
    by key with English above each locale, so a reviewer needs only
    their own language.

### The pinned header (Eric: "pin the search bar")

`.wp-searchrow` had ALREADY declared position:sticky — it was inert,
because the scroll container used to be `.wp-grid` and the search row is
a SIBLING of the grid, not inside it. Making the screen the scroller
activated it for the first time.

Pinning the search bar alone would have shipped a regression: `.wp-top`
holds the ✕, so the exit would have scrolled away and needed a scroll
back up to find — the F2 complaint in a new costume, introduced by the
fix for F4. Both rows pin as ONE sticky wrapper instead. Not two stacked
offsets: those need a hard-coded header height and Big Text changes it.

Background is explicit rather than `inherit`, because a transparent
sticky header lets the tiles show through it — a bug that could only
appear now, since the rule had never actually stuck to anything.

Verified 4000px into a 378-tile list: exit at y=22, search at y=86, both
on screen, header opaque.

### The site-boot break (caught by the gate, not by me)

Placing the Spell It guide next to #plcCard put it INSIDE a Spell
Picture strip region — pure proximity, nothing to do with the picker. On
the site build those elements vanish, `dom::on_click` calls `el()`, which
PANICS, and boot dies. All eleven site tests timed out waiting for wasm
that never started.

My own e2e run before arming had passed because I only read the APP
suite. The gate runs both, which is the only reason this did not ship.

Fixed twice over: the markup moved out of the strip region, and the
wiring guarded with `dom::exists` (the idiom wire_placement already uses)
so a future strip-region edit cannot take the site down again.

And I wrote the guard as an early `return` first, which would have
skipped the `spellAloudEnter` wiring below it — breaking Spell It entry
on the site while "fixing" the panic. Scoped `if` now. Checking a fix
against what it skips is as necessary as checking what it catches.

## CONFIRMED ON DEVICE — build 154 (Eric, 2026-08-08: "it scrolls")

The nested-scroller diagnosis was right. `.wp-grid { overflow-y:auto }`
inside a `position:fixed` screen scrolls in every desktop browser and can
refuse touch entirely under WKWebView. One scroller — the screen itself —
fixes it.

WHAT THIS COST, recorded so the next device-only bug goes faster: three
rounds of desktop measurement, each accurate and each irrelevant, plus
three requests for a readout Eric could not easily produce. The pattern
was visible in the CSS on day one. RULE: when a symptom is device-only
and the desktop reports healthy, the desktop has stopped being evidence —
stop measuring it and start eliminating patterns known to differ.

## THE AUDIT IS COMPLETE

Seventeen features done or explicitly closed. D1–D10 signed. Twelve ships
(136–147, builds 143–154).

Found beyond the sheet: three shipped modes nobody could reach
(calendar/translate/reports had no flag arm), a search running the
scan-stack planner 378 times per keystroke, a picker I broke and
repaired, a share button and shield HUD rendering while "hidden", a spec
that had never once executed, a checker reporting a real bug to nobody,
an upsell shown to children in Spell Jr, and "Everything is free" in
fifteen languages on a $12.99 app.

FALLBACK NOT SHIPPED: picker pagination was drafted (8 tiles/page, no
scrolling required, no new strings) as insurance against the scroll fix
failing. It is not needed and is NOT in the tree — dead code behind a
dark flag is a maintenance cost with no user. The draft stays in the
session scratchpad if the class of bug ever returns.

CONFIG NOT SHIPPED: `ios.scrollEnabled` was queued as belt-and-braces.
It defaults to true in Capacitor, so with scrolling proven working it
would be a no-op committed for reassurance — and `contentInset` was
deliberately never touched because it moves safe-area layout and would
confound the fix just verified.
