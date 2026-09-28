# CC-HUB-NAV v1.3.2 §0 census — report only

Run 2026-09-28 against the v1.3.1 tree (gate in flight). Nothing changed.

**No C1–C4 HALT fires.** But three of this amendment's premises do not hold
against this codebase, and one of them makes a signed decision and an
acceptance test unexecutable. Those are below the census.

---

## C1 — one drawer, and it is not native

`src/drawer.rs` builds it; `index.html` styles it. That is the whole
implementation. There is **no second drawer**: no Spell Jr variant, no iPad
variant, and no Swift drawer at all — the only Swift-adjacent hit is
`ios/App/App/public/index.html`, which is the Capacitor copy `npm run sync`
generates from the same file. **No HALT.**

Worth stating plainly because the rest of the spec assumes otherwise: the
drawer is HTML in a webview, so `closeDrawer()`, safe-area *insets*, the
`DrawerCloseTests` **UI test target** and VoiceOver's announcement string are
all named in terms this app does not have. The equivalents it does have are
`drawer::close()`, `env(safe-area-inset-*)` in CSS, the Playwright suite in
`tests/e2e/specs/drawer.mjs`, and an `aria-label` read by VoiceOver through
the webview. I read the spec as naming the behaviour, not the API, and will
implement the behaviour — say if you meant something else.

## C2 — the top ✕ and everything that references it

Positioned by `.hub-x` (`index.html:1677`):
`position:absolute; top:10px; inset-inline-end:12px`, inside a `.nav-head`
div that `drawer.rs:474` renders as the panel's first child. It is
`position:absolute` against the panel, so it is already pinned and already
scroll-proof — the problem is purely *where* it is pinned.

References, all of which go with it:

| | |
|---|---|
| `src/drawer.rs:42` | `const CLOSE: &str = "navDrawerClose"` |
| `src/drawer.rs:474` | renders the `.nav-head` wrapper and the button |
| `src/drawer.rs:~544` | `dom::on_click(CLOSE, close)` |
| `src/drawer.rs` (open) | first focus lands on it — F4's focus rule has to move with it |
| `tests/e2e/specs/drawer.mjs:153` | clicks it to test the scroll lock |
| `tests/e2e/specs/front-door.mjs:130` | clicks it to leave the drawer |

No analytics reference it (the drawer emits no telemetry). **`.hub-x` itself
must stay**: four other screens use it (`ghostClose`, `srClose`, `inkClose`,
`devMenuClose`). Only the drawer's use of it goes.

## C3 — the dismiss paths, and the one that does not exist

Four paths, and **every one already calls `drawer::close()`** — the single
function I1 asks for. No path sets its own state, and `close()` is the only
writer of the drawer's closed state:

| Path | Wired at | Calls |
|---|---|---|
| top ✕ | `drawer.rs` render | `close()` |
| scrim tap (dimmed area) | `drawer.rs` wire | `close()` |
| row selection | `drawer.rs` render, every row | `close()` |
| Escape key | `drawer.rs` wire | `close()` |

**There is no swipe-to-close gesture.** Not in `drawer.rs`, not in
`index.html`, not anywhere — the only pointer-drag handlers in the app belong
to the keyboard's accent popover and the word-search grid. The drawer has
never had one.

This is not a C3 HALT by its own wording (the HALT is "the swipe path uses a
different close function"), but it empties four clauses of this amendment:

- **D-C3** says the existing swipe stays exactly as it is. There is nothing
  to leave alone.
- **F4** says the other paths are "aligned to swipe; swipe is not aligned to
  them". They are already aligned to each other; there is no swipe to align
  to.
- **I7** says the diff contains no swipe changes. Trivially true.
- **T3** asserts a right swipe still dismisses the drawer, "unchanged from
  build 219". **This test cannot pass**, because the behaviour it guards has
  never shipped.

Two ways forward, and this one is yours: (a) drop T3 and D-C3 as describing
something that does not exist, or (b) treat them as a request to ADD
swipe-to-close. (b) is a real feature — a horizontal pointer-drag on the
panel with a distance/velocity threshold that must not fight the panel's
vertical scroll — and it is outside this amendment's stated scope ("owns only
how the drawer is dismissed" arguably covers it; "do not touch the swipe
gesture" implies it already exists). I have assumed (a) and will not add a
gesture without your word.

## C4 — the nudge watermark is written on OPEN. No HALT.

`drawer::open()` calls `clear_nudge()` before anything else; `clear_nudge()`
writes `spell_nav_nudge_v1` from the last miss total the app reported. Nothing
in `close()` touches it. So I8 holds by construction for any number of close
paths, and T8 will pass trivially — which is the right answer, but worth
noting it is not evidence of anything, since no close path could have written
it.

---

## Beyond the census: two things to decide, one thing to note

**1. F1's heading says "Bottom-left close button" and D-C1 says bottom-right.**
D-C1 is rev 3, signed, with a reason (right-thumb reach, consistent with
D-N10) and an explicit note that rev 2's bottom-left is superseded. F1's title
looks like rev 2 wording left behind. **I am building bottom-right.**

**2. F3 is a real bug and the fix is a one-liner that is currently missing.**
The panel is `height:100%` with no safe-area padding at all:

```
width:min(320px,85vw);height:100%;overflow-y:auto;...
```

That is exactly why rows scroll under the clock. The rest of the app handles
this correctly — `.dm-screen`/`.pr-screen` use
`padding-top:calc(env(safe-area-inset-top,0px) + 8px)` — so the drawer is the
outlier rather than the pattern needing invention.

**3. D-C4, D-C5 and D-C6 defaults accepted as written**, and none of them
needs a new string: `aria.close` already exists in all 15 locales and is what
the four other ✕ buttons use, so the icon-only button gets its VoiceOver name
for free. I would have had to stop and ask for "✕ Close" as visible text,
which is the localized-string case D-C4 names.
