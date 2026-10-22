# CC-VISUAL-ECLIPSE Phase B: carry-ins

Work queued for Phase B (orb and feedback) besides F3 and F4 themselves. Each item has a done condition.

## B-C1: the birthday month is clipped on small phones (DONE)

**What:** On an iPhone SE (2nd gen), the first-launch birthday prompt shows "Januar" instead of "January". Found on the simulator while preparing build 287, 2026-10-21. Eric asked for the fix in Phase B.

**Cause:** the three date pickers share the row equally, with no minimum width:
- `.dob-row select{flex:1; min-width:0}` (index.html, `.dob-row`)
- `#ageMonth`, `#ageDay` and `#ageYear` each get a third of a ~280 pt modal.

A month name needs more room than a day number. The longest names are worse than January: "September", and translations such as "Septiembre" and "Сентябрь".

**Measured** (Chromium, 2026-10-21): each picker is 90 pt wide at both 320 and 375, leaving 49 pt for the text.
- **Clipped (7 of 12):** January, February, August, September, October, November, December.
- **Fit:** March, April, May, June, July.

**Fix direction:** give the month picker the room it needs. For example, weight it (`#ageMonth{flex:2}`) and size the day and year pickers to their content. Don't shorten month names: an abbreviation is harder for a child to read.

**Done when:** an e2e check at 320×568 and 375×667 confirms that every month name, in every one of the 15 locales, fits its picker without clipping (the select's `scrollWidth` is no wider than its `clientWidth`, or measured text width fits the box). Also needs a simulator screenshot on the SE, and the birthday prompt must still fit at 320 wide.

## B-C2: the birthday months are always in English (DONE)

**What:** the twelve month names are a hard-coded English list (`src/agegate.rs`, line 99), so a Spanish, Russian or Vietnamese player sees "January". Found while measuring B-C1.

**Fix direction:** take the names from the platform for the player's interface language. `Intl.DateTimeFormat(locale, {month: 'long'})` is in WebKit, so it costs no new strings and no translations.
- The stored value stays the month number, so the age verdict cannot change.
- Translated names are often longer than the English ones ("Septiembre", "Сентябрь", "Września"). So B-C1's width fix must be measured against every locale's names, not just English.

**Done when:** the B-C1 check runs over the localized names in all 15 interface languages. A unit or e2e check also confirms the selected month number round-trips unchanged.

**B-C1 and B-C2 resolution:**
- Month names now come from `Intl.DateTimeFormat` in the interface language. If Intl gives nothing usable, the code falls back to English and logs a console warning.
- Day and year size to their content and the month takes the rest of the row. At 360pt wide or less, the month gets its own line.
- `tests/e2e/specs/birthday-months.mjs` checks all 15 languages at 320 and 375 wide, and that the month number round-trips. The previous build fails it.

---

# Phase B: F3 and F4 (built 2026-10-21)

Both features are behind their own flag, default off on main and killable from the signed kill file: `visual.feedback` (`visual_feedback`) and `visual.orb` (`visual_orb`). With a flag off, its code path returns at once and no class or attribute it styles ever appears.

## The feedback policy registry (F4.1)

Every mode in `config/modes.json` now has `input` and `feedback` (`verdict`, `revealCorrect`, `diff`, `lengthBeforeAnswer`). Impossible is a level, so it overrides the length in `feedbackLevels`. Values were copied from how each mode behaves today (R8). The base game's opt-in Hint is recorded as `lengthBeforeAnswer: on_request`.
- `modes-check.mjs` rejects a missing or malformed policy, and a `diff` without `revealCorrect`. So a letter comparison can only appear where the spelling is shown anyway.
- `modes::feedback_policy(mode, level)` fails closed for an unknown mode and logs an error.
- Spell Cross's diff is recorded as false: the mode reveals per cell, not a whole spelling.

## F4: answer feedback that teaches

- `src/display_diff.rs`: optimal string alignment over NFC grapheme clusters. It is exported to the web layer as `displayDiff`.
  - AT-11: 39 unit tests.
  - AT-12: a test walks `src` and fails if anything except the module and the reveal renderer reaches into it. A second test grades some 140 attempts in nine languages (right answers and near misses) through every base-game grader with all flags off, then on, and requires identical verdicts.
- `src/answer_reveal.rs` draws the "You wrote" / "Correct" rows for a base-game misspelling, only where the policy allows a diff.
  - Marks: extra letters struck, missing underlined, wrong highlighted, swapped overlined.
  - Every mark is a text decoration, so a cursive word still joins.
  - Arabic rows run right to left, with the label in its own `<bdi>`.
  - Mandarin (per-syllable tone verdict) and Russian (stress mark) keep their own reveals.
- A wrong answer is caution amber in the base game, DefMatch, Spelldoku, Spell Cross and Impostor (E-Q7). The orb's shake becomes a dim (E-Q7b).
- A correct answer lifts the letters 25 ms apart, compressed so the wave ends within 300 ms. A cursive word is one shaped run, so it does not lift.
- Two new strings, `fb.youWrote` and `fb.correctRow`, in 15 locales. These are drafts and are listed in `needs_audit`.
- Tests: `answer-feedback.spec` covers off vs on, the marks, amber, the stagger budget, Reduce Motion, AT-13 and AT-14, and the four modes' amber.

## F3: the orb as a living character

- `src/orb.rs` holds the transition table for the seven states. An undefined move panics in debug, and is refused and counted in release.
  - AT-10: all 29 defined moves are named in tests, and all 13 undefined ones are proven to panic.
- `src/orb_view.rs` applies the state as `#orbWrap[data-orb]`.
  - Fetching waits 120 ms before it is drawn.
  - Speaking drives `--orb-env` from a 60 Hz RMS envelope read against the playback clock.
  - The bytes come from a side read (E-Q2 a). On the web it is a fetch of the same immutable URL, and on iOS the cached file via the new read-only `SpellAudio.readCached`.
  - The envelope is decoded in an OfflineAudioContext, so the audio session is never touched. No envelope means a gentle pulse, plus a console warning.
- Router hooks in `api.rs` only observe; the source order, URLs and playback calls are unchanged.
- Rest: the idle breath pauses while hidden, or after 30 s without input.
- Tests: `orb-states.spec` covers off (no attribute, no side read), the state run with a decoded WAV, no flicker on a cached replay, flare and dim, AT-9 across all seven states, rest, and the AT-8 report line.

## Still open (needs a device or Eric)

| Item | Why it is open |
|---|---|
| AT-8 (≤ 50 ms p95 over 100 words) | Needs an iPhone. The frame report's new "orb sync (AT-8)" line counts the words and gives the p95. On iOS the clock starts when `play()` resolves (E-Q2): `NATIVE_START_OFFSET_MS` is 0 until a device run measures it. |
| Eric's review of the orb states and the diff on device | The Phase B exit gate. |
| Modes with their own orb (Randospell, BoardGame, Quick Bee) | Stop-and-ask: each one adopts F3 only after Eric approves a side-by-side comparison. |
| `fb.youWrote` / `fb.correctRow` translations | Drafts awaiting the auditor sheet. |
| Milestone | The state, its look and its moves exist. Nothing in Phase B enters it; F7 will. |
