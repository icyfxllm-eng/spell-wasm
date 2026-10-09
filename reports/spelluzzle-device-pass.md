# Spelluzzle: your device pass (E1 to E4)

It is on by default in the TestFlight build (kill-switch: `localStorage['spell_flag_spelluzzle'] = 'off'`). Open **Spelluzzle** from the drawer (Word puzzles group). English only. The glyph sheet for E4 is `reports/spelluzzle-glyph-sheet.html`.

- **E1: spelling the first word makes letters appear in other words, and it reads as a reward.** Start an Easy board, tap a word, spell it. Dimmed letters should appear in the other rows wherever the same runes are.
- **E2: a deliberate misspelling shows amber on the shared rune in both words, and nothing says which word is wrong.** Spell one word correctly, then another with a wrong letter on a shared rune. Both words' shared rune turns amber. No red, no cross, no message.
- **E3: the secret word can be entered early, and the orb says it only after the board is solved.** Tapping the secret row says nothing. Typing it is allowed. Finish the board and it is spoken once.
- **E4: no rune looks like a letter, and no two get confused at arm's length.** Look at the glyph sheet at phone size and on a real board.
- **E5 (Medium and up): the silent word can be worked out without Listen, and Listen never shows a letter.** Pick Medium. Tap a silent row (nothing is said), work it out from the other words, then try Listen on another board: it plays the word and costs the Codebreaker star.
- **E6: ten boards in a row at one tier feel different from each other.** Use New board ten times.

Also worth a look: the "Exit" word top left under the status bar, the first-board explainer, a long Expert word (10 cells) on a small phone, and leaving mid-board then reopening (the same board waits).
