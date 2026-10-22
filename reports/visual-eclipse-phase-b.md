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
