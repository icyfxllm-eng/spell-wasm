# CC-VISUAL-ECLIPSE A5: frame-time baseline on the iPhone SE (2nd gen)

AT-19 asks for **p95 frame time ≤ 16.7 ms and no frame over 50 ms** on the AT-19 device during:
- a spoken word
- typing
- the result
- BoardGame hops
- a Randospell roll
- big celebrations (once F7 exists in Phase D)

The device is the iPhone SE (2nd gen), signed 2026-10-20.

A headless browser cannot measure this, so the app measures itself. Only you can take this reading, on the device, from a TestFlight build that contains A5 (commit after `5da52dd9`).

## How to take the baseline (about 5 minutes)

1. Install the Phase A TestFlight build on the iPhone SE (2nd gen). Use normal settings: Reduce Motion off, Low Power Mode off, and the phone not hot from charging.
2. **Open the dev door:** five quick taps on the **SPELL** logo.
3. Switch on **⏱ Frame logger (AT-19)**, then **Close**. The switch stays on across relaunches until you turn it off.
4. Play normally:
   - **Spoken word, typing, result:** play 10 words in the main game. Let each word play and type each answer on the keyboard. Get some right and some wrong.
   - **BoardGame hops:** open BoardGame and play 3 or 4 turns, so pieces hop along several tiles.
   - **Randospell roll:** open Randospell and roll 5 or 6 times.
5. Open the dev door again (five taps on the logo) and tap **📊 Frame report**.
6. Tap **Copy** and paste the report into chat. If Copy doesn't work, long-press the report text to select and copy it.
7. Switch the logger **off** before handing the phone to anyone else.

## What the report says

```
moment            frames    p50     p95     max   >50ms  AT-19
boardgame hop        412   16.7    16.8    33.4      0  PASS
idle                9310   16.7    16.7    50.1      1  -
randospell roll      590   16.7    33.3    66.8      2  FAIL
...
```

- A moment's window starts when the code begins it:
  - a word starting to play: 2 s
  - a keystroke: 0.5 s
  - Check: 1.5 s
  - a hop: 1.2 s
  - a roll: 2 s
- Frames outside every window count as **idle**. Idle has no verdict, because AT-19 doesn't cover it.
- **Verdict:** PASS means p95 ≤ 16.7 ms, plus 0.5 ms for vsync jitter, and no frame over 50 ms. A smooth 60 Hz frame reads 16.6–16.8 ms.
- **Not counted:**
  - gaps over 1 s, which are the app being backgrounded or paused
  - frames while the page is hidden

## If a moment fails

That's stop-and-ask trigger 9: "the frame budget can't be met on the oldest supported device without cutting a feature". I bring you the numbers and the cause, and you decide. I won't cut or slow a feature without your call.

## Where it lives

| Piece | Location |
|---|---|
| Logger | `src/frame_log.rs` (pure statistics plus unit tests; the rAF sampler) |
| Moment marks | `api::play_word_with`, `game::emit_key`, `game::submit_guess`, BoardGame's hop step, Randospell's die landing |
| Switch and report | the dev door (`src/dev.rs`, `#devMenu`); never visible to players |
| End-to-end proof that the instrument works | `tests/e2e/specs/frame-log.mjs`; headless numbers, not AT-19 data |
