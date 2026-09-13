# CC-TRANSLATE-SCREEN — The Spell Translate screen

## Signed decisions (Eric, 2026-09-12)

Recorded before any code. Where a decision resolves a conflict with an earlier
signed file, the earlier ruling is named.

| # | Ruling |
|---|---|
| **D1** | **SIGNED.** Native script primary; transliteration beneath, **on by default** for non-Latin targets when the UI language is Latin-script, off elsewhere; player-togglable. Reverses the mockup, which showed `kot` alone. |
| **D2** | **SIGNED.** Bank search, committed only by selecting a suggestion. **Amends CC-TRANSLATE-TOOLS I4** from "no free-text input on any kid-facing surface" to "no free-text *submit*": the player may type to search, but nothing typed is ever committed. |
| **D3** | **SIGNED.** Languages the player does not own are listed. Word, audio and Save render; *Spell it* routes to that language's PREVIEW tier. |
| **D4** | **SIGNED.** Spell Jr gets Translate: Jr bank filter, Easy + Medium through the CC-ONBOARD-JR resolver, no camera entry. **Resolves a contradiction** left by CC-TRANSLATE-TOOLS D2 (Kid Mode: full core; Little Speller: no translator) once CC-ONBOARD-JR D7 made those one experience. |
| **D5** | **SIGNED: app-only.** Keeps the 2026-08-02 whole-suite app-only ruling (CC-BUY-DRIVERS); this file's web recommendation is declined. |
| **D6** | **SIGNED.** Three actions — *Spell it* (primary), *Save to My Words*, *Clear*. "Add to plan" leaves this screen. |
| **D9** | **SIGNED** as recommended: inline to 4 senses, then "more senses"; never a modal. |
| **D10** | **SIGNED** as recommended: Save stores the target word only. |
| **D12** | **SIGNED** as recommended: text renders offline; audio shows `unavailable`; never blocks on network. |
| **F14** | **CUT.** Instrumentation conflicts with the global zero-telemetry posture (CC-BUY-DRIVERS S1, from CC-LEARNING-ENGINE D5: everything on-device, zero telemetry). |
| Surface | **Replace the existing Translate pop-up in place** — one Translate surface, not two. |
| §8 Done | **Map to repo equivalents.** The commands below name a `spell-core` crate, `src/screens/`, `.spec.ts` Playwright files and `scripts/symbol-scan.mjs`; none exist. Use the `spell_wasm` crate, `tests/e2e/specs/*.mjs`, and the gate's existing machine-translation scan. The existing audio router (`api::play_word`) is "the one resolver", since CC-PLAYER-CONTRACT and CC-BUILD219-FIXES are not in the repo. |
| Branch | **Built on top of `cc-onboard-jr`**, because D4's Jr gating consumes that branch's resolver. |

Still open: D7 and D8 (Phase B), D11 (deferred), and **which languages get a
native-speaker gloss audit first**. Today zero of the fourteen gloss files in
`config/gloss/` are `audited`, so Phase A builds a screen that can answer
nothing in production until the first audit lands (Phase C blocker).

## Phase A build notes (2026-09-12)

Where the repo and this file disagree, what was built and why. Each is a
finding to review, not a silent near-match.

- **Nothing answers in production yet.** All 14 gloss files are `audited:
  false`, so the pickers list English alone and no pair can form. Every test
  marks a language's REAL rows audited through a test-build-only switch
  (`translate::seam_set_audited`, compiled out of production). The switch
  flips a flag; it cannot author a row, so I7 holds under test too.
- **Where it lives.** Rules in `src/translate_screen.rs` (no DOM); the screen
  in `src/translate_ui.rs`, replacing the pop-up in place; markup and CSS in
  `index.html` (`#trScreen`). App-only via the existing `cfg(not(feature =
  "web"))` wall (D5).
- **I3 is enforced, not assumed.** The target lookup was a `HashMap` scan that
  would pick arbitrarily if a language ever had two words for one concept.
  It is now an index, and `scripts/gloss-check.mjs` fails the build on a
  duplicate concept. There are zero today.
- **Spell it serves in the target's language.** `game::request_word` now
  carries a language and serves the word in it for that one turn, the way the
  Daily Challenge serves its locale. The player's study language never
  changes. Its only prior caller was the pop-up being replaced.
- **Senses (F3).** Today's gloss data holds one concept per word, so no word
  has two senses and the sense list never expands in practice. The path is
  built, keyed by word and sense, with D9's four-then-more rule.
- **Transliteration (D1).** The default rule is implemented from data: a
  language's script is read from its own name in the registry. But a
  romanization scheme ships for Mandarin only, so today transliteration
  appears under Chinese targets and nowhere else.
- **Audio "loading" (F6).** The router reports failure but not success. The
  control shows loading on press, turns unavailable if the router fails
  (which is also D12's offline case), and returns to ready once it has had
  time to fail. Mandarin passes its reading, as the game does.
- **No My Words word cap exists.** The free limit is two custom lists, and My
  Words is one list. F9's cap-reached state cannot occur, so the
  `translate-cap-reached` fixture has nothing to exercise.
- **Fan-out link (F11).** Its line is reserved and held hidden: the Tool 1
  all-languages card it opens does not exist yet, and a link to nothing would
  be a dead end. Decline #2's live affordance is the language picker instead.
- **Swap from a decline (F7).** Languages always exchange. The new target is
  the sense looked up in the new target language; when there is no source word
  to carry across, the source card is empty. It never refuses.
- **Keyboard (F2).** The search field uses the system keyboard. The
  per-language script keyboard lives in CC-PLAYER-CONTRACT, which is not in
  the repo, and this file forbids new keyboard code.
- **Done §8, mapped.** #1 and #7 are `cargo test --lib translate_` (in the
  gate). #2, #3, #8, #9 and #11 are `tests/e2e/specs/translate-screen.mjs`.
  #4 is the gate's existing machine-translation scan. #5 and #6 are new gate
  greps over the two screen files. #10 (Maestro device flow) and the
  screenshot review packet are not produced here.
- **Spell Jr (D4).** The Jr resolver filters both suggestions and answers to
  Easy and Medium and kid-safe words, so the `translate-jr-band-decline` state
  is not reachable through the screen; it is covered by the Spell it verdict's
  unit test.

---

**Status:** REVIEW-GATED. Phase A executable on sign-off of D1–D5; Phases B–C blocked (see §Phases).

**Position in the stack.** This file owns **one screen and its states**. It owns nothing else.

| Concern | Authoritative file | This file's relationship |
|---|---|---|
| Gloss pivot, ingest validation, decoys, `glossAudited` flag | **CC-BANK-TRANSLATE** Phase A | Consumes. Never redefines. Phase A stays deadline-critical and untouchable. |
| Bidirectional lookup, My Words import path, offline operation, per-language keyboards | **CC-BANK-TRANSLATE** Phase B | This file is the **UI surface** of Phase B. Phase B keeps the data contract; this file keeps the layout, states, and affordances. |
| The 13-tool suite (fan-out, spell-the-translation, transliteration toggle, camera, Passport, …) | **CC-TRANSLATE-TOOLS** | This file specifies only the **entry points** to Tools 1, 2, 3 and 10. It does not implement any tool. |
| Audio resolver (one resolver, no system-voice fallback) | **CC-BUILD219-FIXES** F4 | Consumes. Adds no second resolver. |
| Per-language audio preconditions, script keyboard, graceful degradation | **CC-PLAYER-CONTRACT** P1–P7 | Conforms. This screen is one row in the servability scorecard. |
| Jr difficulty resolution, under-13 routing | **CC-ONBOARD-JR** v1.1 | Consumes the resolver. Does not fork it. |

If any instruction here implies a reversal of a signed decision in those files, **stop and ask.** Do not infer.

---

## 1. Intent — read this before touching layout

Eric's mockup states the goal in four words: *an easy translation experience.* Two stacked rows, a language on the left, a word on the right, audio on each, clear and save at the bottom. That shape is correct and the implementation must preserve its calm: **two cards, one vertical spine, nothing that moves.**

But the shape hides the one thing that decides whether this screen feels finished or broken, and it is not a visual question:

> **Spell Translate can only answer for words that exist in an audited bank with an audited gloss. Google Translate accepts anything. If our field looks and behaves like Google's field, then every word outside the bank — most words a curious player will try first — returns a dead end, and the player concludes the feature is broken rather than bounded.**

So the field does not accept-then-fail. **The field is a search over the answerable set, and it steers the player onto a word we can answer before they finish typing it.** A closed output space is only safe if the input space is *visibly* closed too. Every other decision in this file follows from that sentence.

Second thesis, inherited from CC-TRANSLATE-TOOLS and non-negotiable here: **the result card is a door, not a destination.** A player who learns `кот` and leaves has gotten a worse product than one who taps *Spell it* and owns it. Translation is the on-ramp; spelling is the app.

**Judgment rule for gaps.** When this file is silent, choose the option that (a) keeps a rendered string traceable to an audited bank row, and (b) keeps the player one tap from a spelling session. If those two conflict, (a) wins and you stop and ask.

---

## 2. Scope

One screen — `TranslateScreen` — plus its states, its two pickers, its four actions, and its declines. It ships behind the existing feature flag until Phase C.

---

## 3. Features

### F1 — Screen skeleton: the two-card spine

**Intent.** The mockup's two-row geometry is the product's face. It must read identically in every one of the 15 languages, in RTL, at the largest accessibility text size, and while empty. Layout that jumps as content arrives destroys the calm that is the entire point.

**Mechanism.**
- Vertical order, fixed: title → source card → swap control → target card → fan-out link → action row.
- Both cards are the **same component** in two configurations (`role: source | target`), not two components. One layout bug, one fix.
- Each card contains: language chip (top-left), direction label (top-right, `from` / `to`), the word (large, primary), an audio control (trailing), and a secondary line (gloss on source, transliteration on target — F5).
- **Card height is reserved, not computed.** Both cards render at their full populated height when empty, with placeholder text. Filling, swapping, or clearing must not change the vertical position of any element below them.
- The swap control sits on the seam between the cards and is the only element allowed to overlap card bounds.

**Invariant I1 — Geometric stability.** For any sequence of input, swap, clear, and language-change events, the `y` offset of the action row is constant. Asserted by snapshot comparison across the state matrix, not by eye.

---

### F2 — Source entry is a bank search, not a text field

**Intent.** Stated in §1. The player must discover the bounds of the feature through suggestion, not through rejection. A player who types `c-a-t-e-r-p` and sees the suggestion list go empty at `cater` has learned the bound painlessly; a player who types `caterpillar`, hits enter, and gets an error has learned that the app is flaky.

**Mechanism.**
- Tapping the source word opens an inline search over the source language's bank, filtered to rows where `glossAudited == true` **and** at least one target-language row shares the gloss.
- Suggestions appear from the first character, ranked: exact prefix → prefix of any inflected surface form → substring. Cap at 8.
- The entered value is **only ever committed by selecting a suggestion.** There is no free-text submit path, therefore there is no free-text failure path.
- As the suggestion list empties, the list area shows the decline copy from F12 in place — the bound is announced where the player is already looking, before they commit anything.
- Input normalizes to NFC on every keystroke (app-wide baseline), and uses the per-language script keyboard from CC-PLAYER-CONTRACT. No new keyboard code.

**Invariant I2 — No committable miss.** It is impossible to reach a result-pending state for a word that has no renderable answer. A miss is a pre-commit state, never a post-commit error.

---

### F3 — Sense selection: the pivot is a gloss, not a word

**Intent.** The bank pivots on a sense-locked English gloss. `bat` is two words; `light` is three. If the player types `bat` and we silently pick one, we will eventually show a Spanish `murciélago` to someone thinking about baseball, and the feature will have lied.

**Mechanism.**
- When a selected source word maps to more than one gloss, the suggestion row expands into one row per sense, each labelled with its gloss (`bat — flying mammal`, `bat — sports implement`). The player picks the sense, not the string.
- The chosen gloss renders as the source card's secondary line, permanently, so the player can see what they asked for.
- Single-sense words show the gloss on the same line with no selection step. No extra tap for the common case.

**Invariant I3 — Sense traceability.** Every rendered target string is reachable by a single `(glossId, targetLang)` lookup. No target string is ever selected by ranking, recency, frequency, or heuristic.

---

### F4 — Language chips: what is listable

**Intent.** The picker is the discovery funnel — "what's cat in Korean?" is how a player meets a language they haven't bought. But CC-TRANSLATE-TOOLS Tool 1 set the precedent that unavailable rows are **absent, not locked**, because a padlock in a fan-out card reads as a paywall pressed against a child's face. The picker and the fan-out must not disagree about which languages exist.

**Mechanism.**
- Both chips open the same picker component, listing languages where `glossAudited == true`, sorted: current pair → device/home language → remaining alphabetically by endonym.
- Entitlement does **not** filter the list (see D3) — but an unentitled target language's result card renders the word, the audio, and Save to My Words, while *Spell it* routes to the language's PREVIEW tier rather than a purchase wall.
- A language that is gloss-dark is absent with no explanation, exactly as in the fan-out card.
- Source and target can never be set to the same language; selecting the target's language in the source picker performs a swap (F7) instead of an invalid state.

---

### F5 — Result rendering: script first, transliteration second

**Intent.** This is a spelling app. The string the player will one day be asked to type is the native-script string. It must be visually primary, always, in every language — otherwise Translate teaches a habit that the game then penalizes.

**Mechanism.**
- Target word renders in native script at primary size, using the per-language typography stack already established for the game surfaces. No romanization substitutes for it, ever.
- Transliteration renders as the card's secondary line, muted, at ~12–13px, for non-Latin target scripts only, governed by the existing Tool 3 transliteration toggle. Default per D1.
- RTL targets (Arabic, when RTL ships) mirror the card's internal layout; the vertical spine does not mirror.
- Stress marks, tone marks, and ё follow the display/match separation already signed in CC-PLAYER-CONTRACT and CC-STRESS-LAYER. This screen displays; it never grades. If those files change the display rule, this screen inherits it — it does not carry its own copy.

**Invariant I4 — Script primacy.** For every language, the native-script element's rendered font size is strictly greater than the transliteration element's, and the transliteration element is never the sole rendered form.

---

### F6 — Audio: three states, one resolver

**Intent.** Build 219 shipped a Daily Challenge word in a barely audible system voice. On a translation screen the audio *is* half the answer — a player learning `кот` needs to hear it. A wrong-sounding voice here is worse than no voice, because the player will repeat what they heard.

**Mechanism.**
- Both audio controls call the single resolver from CC-BUILD219-FIXES F4. No fallback to any platform TTS voice. No second code path on this screen.
- Three visual states, all reserved in layout: `ready` (play), `loading` (indeterminate, control stays same size), `unavailable` (control dimmed with the audited "audio unavailable" copy on press — never silent, never a fake play).
- Per-language audio preconditions come from CC-PLAYER-CONTRACT P4. A language failing its precondition is not muted quietly; it renders the unavailable state.

**Invariant I5 — No synthetic fallback.** A symbol scan proves this screen references no platform speech API directly.

---

### F7 — Swap

**Intent.** A heritage-language household asks in both directions within one sitting — "what's *cat*?" then "what's *собака*?". Making them reopen two pickers to reverse is the difference between a tool they use daily and one they use once.

**Mechanism.**
- Swap exchanges both the languages and the words in a single atomic state update; if a result is displayed, the reversed pair is displayed immediately without a re-lookup (the gloss is symmetric).
- If the reversed direction has no answer for the current gloss — possible when one language's bank covers the sense and the other's does not — the languages swap, the source word is retained, and the target enters the F12 decline state. The swap never silently refuses.
- Swap is disabled-in-appearance only when both cards are empty; it is never hidden.

---

### F8 — Clear

**Intent.** Clear means "new question," not "reset the app." A player who cleared and lost their language pair would have to re-pick two languages to ask a second question — the most common action on the screen made expensive.

**Mechanism.** Clear empties both word slots, the gloss, and the transliteration. It **preserves** both language selections, scroll position, and the transliteration toggle. Focus returns to the source search.

---

### F9 — Save to My Words

**Intent.** This is the loop that no translator app can close and the reason the feature exists: curiosity becomes practice material in two taps.

**Mechanism.**
- Saves through the **existing** My Words import path. No new entitlement logic, no new storage schema, no new validation. (CC-BANK-TRANSLATE Phase B, unchanged.)
- Saves the target-language word with its gloss and language tag — not the source word.
- At the free-tier My Words cap, the control shows the existing audited cap copy and the existing upgrade route. It does not invent new copy here.
- On success: inline confirmation in the action row. No navigation, no toast that moves layout, no modal.

---

### F10 — Spell it

**Intent.** The door. Tool 2, on this screen.

**Mechanism.**
- Hands the target word to the standard spelling session path. Scored normally, counted normally, feeding Misses and FSRS normally. There is no "translate practice" session type and no parallel scoring.
- The word's difficulty band is resolved by the existing resolver; in Jr, by the Jr resolver (CC-ONBOARD-JR D7 — Jr is Easy + Medium only). A target word outside the player's legal band gets the planner's honest audited decline, not a silent drop.
- Returning from the session restores this screen with the pair intact.

---

### F11 — Fan-out entry

**Intent.** One concept across fifteen writing systems is the card no competitor can render. It belongs one tap from the pair view, not behind a menu.

**Mechanism.** A single link below the target card opens Tool 1's all-languages card for the current `glossId`. This file specifies the entry point and its copy only; the card itself is CC-TRANSLATE-TOOLS.

---

### F12 — Declines: the screen says what it can't do

**Intent.** Every bound in this feature is a *designed* bound, not a failure. The copy must sound like a word list with edges, not like an error.

**Mechanism.** Four decline states, all audited strings, all rendered inline where the player is looking — never as an alert:
1. **Not in the bank** (search returns nothing): the closed-space copy, in the suggestion area, pre-commit.
2. **No answer in this target** (gloss exists in source, absent in target): decline in the target card, with the fan-out link offering languages that *do* have it.
3. **Audio unavailable** (F6).
4. **Gloss-dark language** — never reached, because the language is absent from the picker (F4). If this state is ever entered, it is a bug and the build fails.

**Invariant I6 — No dead ends.** Every decline state renders at least one live affordance (a different language, a different word, or the fan-out).

---

### F13 — Jr and web gating

**Intent.** Two surfaces could quietly inherit this screen without anyone deciding they should: Spell Jr and spellgame.net. Both need an explicit answer before code ships, because both carry compliance weight — Jr is COPPA-adjacent, and the web build has no App Store review shielding it.

**Mechanism.** Governed by D4 and D5. Implementation is a single registry gate per surface, following the defs-dark pattern — one flag, no scattered checks, per the single-source-of-truth doctrine.

---

### F14 — Instrumentation  *(CUT, 2026-09-12 — zero-telemetry posture)*

**Intent.** Two numbers decide whether this screen worked, and neither is "translations performed." The screen is a funnel, so measure the funnel.

**Mechanism.** Log, with no free-text and no word-content PII beyond existing telemetry policy: `search_committed`, `result_rendered`, `door_taken(spell | save | fanout | none)`, `decline_shown(type)`, `pair(sourceLang, targetLang)`. **Primary metric: door-taken rate.** Secondary: decline rate by type — a rising type-1 rate is a bank-coverage signal, not a UI signal.

---

## 4. Invariants (consolidated — each is a build failure, not a warning)

| # | Invariant |
|---|---|
| I1 | Action-row `y` offset constant across the entire state matrix. |
| I2 | No committable miss — misses are pre-commit only. |
| I3 | Every target string is a single `(glossId, targetLang)` lookup; never ranked or inferred. |
| I4 | Native script strictly larger than transliteration; transliteration never sole form. |
| I5 | No direct platform-speech API reference on this screen. |
| I6 | Every decline renders a live affordance. |
| I7 | **Closed space.** Every string rendered in a target-language slot traces to a bank row with `glossAudited == true`. No network translation client is reachable from this screen's dependency graph. |
| I8 | No locked/padlocked rows in either picker — absent or present, never teased. |
| I9 | NFC normalization on every input and every comparison. |

---

## 5. Decisions

Signed items are binding. Open items block the phases noted. **If you disagree with any signed item, stop and ask — do not implement the alternative.** (Rulings are recorded in the table at the top of this file.)

| # | Decision | Recommendation | Blocks |
|---|---|---|---|
| **D1** | **Transliteration default.** The mockup shows `kot` alone; §F5 shows `кот` with `kot` beneath. | **Native primary, transliteration on by default** for non-Latin targets when `uiLang` is Latin-script, off elsewhere; player-togglable via Tool 3. Rationale: transliteration-alone teaches a string the game will later mark wrong. **This reverses the mockup — it needs your signature.** | Phase A |
| **D2** | **Free typing vs. bank search** (§F2). | **Bank search, suggestion-commit only.** The alternative — free text with post-commit errors — makes a designed bound read as a defect. | Phase A |
| **D3** | **Unentitled target languages listable?** | **Yes, listable** — word + audio + Save render; *Spell it* routes to that language's PREVIEW tier. Discovery is the funnel. Note this is a deliberate extension of Tool 1's absent-not-locked rule, not a contradiction of it: absent means *unanswerable*, not *unpurchased*. | Phase A |
| **D4** | **Does Translate exist in Spell Jr?** | **Yes, with the Jr bank filter and Jr difficulty resolver**, no fan-out to unentitled languages, no camera entry ever. If you'd rather keep Jr free of it entirely, say so — it is one registry flag either way. | Phase C |
| **D5** | **Does Translate ship on spellgame.net?** | **Yes.** Spell Picture is app-only because it is the app's exclusive; Translate is a discovery funnel and works better where sharing is cheap. Contradict me if the web build's audit posture makes you want it app-only. | Phase C |
| **D6** | **Three actions or two?** The mockup has Clear + Save; §F10 adds *Spell it*. | **Three**, with *Spell it* as the primary. Tool 2 requires both doors. | Phase A |
| **D7** | **Screen name.** "Spell Translate" in the hub, or just "Translate"? Mode names take per-locale equivalents, not literal translations (standing policy). | **"Spell Translate"** in English; per-locale equivalents authored in the same round as the other mode names. | Phase B |
| **D8** | **Where does it live in the hub** — mode tile, or a tab alongside Reports? | **Tab.** It is a tool, not a mode; a mode tile implies a scored session. | Phase B |
| **D9** | **Sense-picker UI** when a word has >4 senses. | Expand inline to 4, then "more senses" — never a modal. | Phase A |
| **D10** | **Does Save to My Words also save the source word?** | **No** — target only. Saving both doubles the cap burn for one lookup. | Phase A |
| **D11** | **Search scope** — dictionary forms only, or inflected surface forms too? Depends on the same bank census that blocks CC-RU-ORTHO §0. | **Defer.** Ship dictionary-form search in Phase A; revisit once that census lands. | Phase B |
| **D12** | **Offline behavior when audio cache is cold.** | Text result renders; audio shows `unavailable` (F6). The screen never blocks on network. | Phase A |

---

## 6. Constraints and non-goals

**Do not:**
- Add, call, or vendor any machine-translation client, API, or model. Not as a fallback, not behind a flag, not for unbanked words. This is the feature's entire App Store safety story.
- Touch scoring, difficulty banding, FSRS, shields, or leaderboards. *Spell it* hands off to the existing session path and nothing more.
- Touch the CC-BANK-TRANSLATE Phase A spreadsheet schema, ingest validation, or decoy gate. That round is deadline-critical.
- Implement any of Tools 3–13. Entry points only, and only the four named in F11/F5.
- Introduce a second audio resolver, a second My Words write path, a second entitlement check, or any `if (lang === …)` branch. Single-source-of-truth doctrine.
- Add definitions to any surface here beyond the pivot gloss itself, in any language that is definitions-dark.
- Introduce a modal, an alert, or a toast that reflows layout. Every state is inline.

**Non-goals for this file:** sentence or phrase translation; camera/OCR lookup; conjugation tables; history/favorites (a separate decision); any offline-pack download UI.

---

## 7. Phases

- **Phase A — the screen, behind the flag.** F1–F3, F5–F12, F14. Executable on sign-off of D1, D2, D3, D6, D9, D10, D12. Builds against whatever `glossAudited` coverage exists today; a language with zero coverage is simply absent from the pickers, which exercises F4 honestly.
- **Phase B — hub integration and naming.** F13 gating decisions, D7/D8. Blocked on Phase A acceptance.
- **Phase C — ship.** Blocked on CC-BANK-TRANSLATE Phase A gloss coverage reaching the audited threshold for at least the launch language set, plus D4/D5.

---

## 8. Done — checkable, not adjectival

Phase A is complete when all of the following pass on a clean checkout. If a path or harness name below does not match the repo, **stop and ask** rather than inventing a near-match. *(Signed 2026-09-12: map each to its repo equivalent.)*

```bash
# 1. Unit + core
cargo test -p spell-core translate_

# 2. Screen behavior, full state matrix
npx playwright test tests/translate/screen.spec.ts

# 3. Geometric stability (I1) — action-row offset constant across states
npx playwright test tests/translate/layout-stability.spec.ts

# 4. Closed-space proof (I7) — no MT client anywhere in this screen's graph
node scripts/symbol-scan.mjs --entry src/screens/translate --deny-list scripts/deny/mt-clients.json

# 5. No direct platform speech API (I5)
! grep -rnE "SpeechSynthesis|AVSpeechSynthesizer|TextToSpeech" src/screens/translate

# 6. No per-language branching (doctrine)
! grep -rnE "lang\s*===\s*['\"]" src/screens/translate

# 7. Every rendered target string traces to an audited row (I3, I7) — property test over the real bank
cargo test -p spell-core translate_traceability -- --include-ignored

# 8. Script primacy (I4) across all 15 languages
npx playwright test tests/translate/script-primacy.spec.ts

# 9. Decline states all render a live affordance (I6)
npx playwright test tests/translate/declines.spec.ts

# 10. Device flow — lookup, swap, clear, save, spell-it, back
maestro test .maestro/translate-loop.yaml

# 11. RTL and largest-text-size snapshots
npx playwright test tests/translate/a11y-rtl.spec.ts
```

**Required fixtures** (named so review can find them):
`translate-empty`, `translate-single-sense`, `translate-multi-sense`, `translate-no-target-answer`, `translate-audio-unavailable`, `translate-unentitled-target`, `translate-rtl-ar`, `translate-jr-band-decline`, `translate-cap-reached`.

**Review packet for sign-off:** screenshots of all nine fixtures in light and dark, at default and largest text size, plus the door-taken instrumentation schema. No code merges to the release branch before that packet is reviewed.
