# Translate Phase A — device run (Done §8 #10)

Eric, 2026-09-14: "use the Simulator" in place of a Maestro flow. Run by hand on
2026-09-14, iPhone 17e Simulator (iOS 26.5), Debug build of the App scheme.
Screenshots are halved to keep the repo light.

## What was installed, and why it is not the shipped app

The shipped app answers nothing yet: no gloss file is signed, and the test seam
that marks rows audited is compiled out of production. So the Simulator ran the
**test build** (`scripts/build-web-test.sh`) inside the real iOS shell, with one
script injected into the gitignored `ios/App/App/public` copy, never a repo file:

- launch 1 leaves the `translate` flag at its default;
- later launches turn it on;
- every launch marks Spanish and Japanese audited through the seam (the real
  rows, unedited).

Afterwards the production `public` copy was restored (seam markers: 0) and the
review build was uninstalled from the Simulator. Audio went to the live server,
as the app always does.

## The loop

| # | Step | Result |
|---|---|---|
| 01 | Fresh install, adult test birthday, play without an account, open the hub | **No Translate tile.** The flag is off by default on a real iOS build. |
| 02 | Relaunch (flag on), open the hub | **Spell Translator** tiles last. |
| 03 | Tap the tile | Opens English → Español, empty. |
| 04 | Type `cat` (iOS keyboard) | One suggestion, `cat CAT`. Nothing commits on typing. |
| 05 | Tap the suggestion | Target `gato`; the chosen sense shows under the source. |
| — | Tap the target's audio | No failure shown: the router reported none. I cannot hear the Simulator, so playback itself is unconfirmed. |
| 06 | Swap | Español → English, `gato` → `cat`. |
| 07 | Clear | Both languages kept, question emptied, field focused. |
| — | Swap back, type `dog`, pick it | `perro`. |
| 08 | Save to My Words | Inline "Saved to My Words"; the screen stays. |
| 09 | Now spell it | Screen closes into the game, which serves a word. Study language stays English. |
| 10–11 | Type `perro`, submit | Chain 1, best 1: the served word was `perro`, graded correct. |
| 12 | Reopen from the hub | The last lookup (`dog` → `perro`) is kept. |
| 13 | Close (×) | Back to home. |

## Seen on device, for review

- **The tile's subtitle says "every language you own"**, but D3 lists languages
  the player does not own. The line promises less than the screen does.
- **Closing left the home screen scrolled to the bottom** (13), not where it was
  before the hub opened. Scrolling the hub may be scrolling the page behind it,
  or closing Translate may be; this run cannot tell which.

Outside Translate, spotted in passing:

- **The front door's email and password fields render as plain white bars** on
  iOS, unlike the rest of the screen.
- **"Longest chains" wraps its empty-state line one word per line** on home (13).
