# CC-VISUAL-ECLIPSE Phase B: carry-ins

Work queued for Phase B (orb and feedback) besides F3 and F4 themselves. Each item has a done condition.

## B-C1: the birthday month is clipped on small phones

**What:** On an iPhone SE (2nd gen), the first-launch birthday prompt shows "Januar" instead of "January". Found on the simulator while preparing build 287, 2026-10-21. Eric asked for the fix in Phase B.

**Cause:** the three date pickers share the row equally, with no minimum width:
- `.dob-row select{flex:1; min-width:0}` (index.html, `.dob-row`)
- `#ageMonth`, `#ageDay` and `#ageYear` each get a third of a ~280 pt modal.

A month name needs more room than a day number. The longest names are worse than January: "September", and translations such as "Septiembre" and "Сентябрь".

**Fix direction:** give the month picker the room it needs. For example, weight it (`#ageMonth{flex:2}`) and size the day and year pickers to their content. Don't shorten month names: an abbreviation is harder for a child to read.

**Done when:** an e2e check at 320×568 and 375×667 confirms that every month name, in every one of the 15 locales, fits its picker without clipping (the select's `scrollWidth` is no wider than its `clientWidth`, or measured text width fits the box). Also needs a simulator screenshot on the SE, and the birthday prompt must still fit at 320 wide.
