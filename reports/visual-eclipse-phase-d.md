# CC-VISUAL-ECLIPSE Phase D: icons, celebrations, cards

All three features ship behind their own flags, default off on main and killable from the signed kill file: `visual.icons`, `visual.celebrate`, `visual.cards`. With a flag off its code path is a no-op.

## F8: icons (`visual.icons`)

- Eric approved the sheet on 2026-10-23. There are 46 code-drawn icons in `design/icons/` (V-D7: code-authored, swappable). Each is on a 24 px grid with rounded strokes, coloured with `currentColor`, and its width comes from the `--icon-stroke` token.
- `scripts/icons-build.mjs` writes the inline sprite in `index.html` and `src/icons_data.rs`. `src/icons.rs::html(name)` is the one way an icon reaches the page: a slot holding the drawn icon and the emoji it replaced, with the flag choosing which shows.
- Every mode's `icon` in `config/modes.json` is now an icon name. 21 strings in 15 locales lost their baked-in emoji prefix.
- AT-20: `scripts/icons-check.mjs` runs in pre-push. Every exemption is named with a reason:
  - share text
  - Board Game and Randospell scene art
  - achievement badges
  - the dev door
  - typography
  - picture content
- E-Q11: Spell Picture's fan, hamsa and stork moved to emoji that iOS 15.0 can draw.

## F7: celebrations (`visual.celebrate`)

- V-D18 triggers:
  - **Big:**
    - a Daily day-streak tier (7/30/100/365)
    - a Climb rank-up
    - a perfect run (10 in a row; a 10/10 Daily counts)
    - a new personal best, once per run, against the best the run started from
    - a theme unlock (from Phase E)
  - **Small:** an achievement.
- `src/celebrate.rs::Planner` holds the AT-18 rules:
  - never during an active word
  - at most one big per 60 s (the extra plays small)
  - big lasts 2.5 s at most, and a tap skips it
  - Reduce Motion shows a calm card with no sparks
- A big celebration holds the next word's auto-advance until it ends.
- E-Q9's local records are display-only: the perfect-run count and the best Climb rank per difficulty.

## F11: result cards (`visual.cards`)

- `src/result_card.rs` holds a pure layout (a list of draw operations) and a canvas renderer at 1080×1350. Cards use the player's live colours, the orb, the score, the mode, the language accent, and script faces with right-to-left layout for Arabic.
- Cards cover the chain, Daily, Practice, Board Game and Randospell. The image is shared with the text on the native app; the web still shares text.
- AT-27: a card has no name, avatar or free text (`result_card::tests`). Kid Mode cannot share at all.
- AT-26: `cards.spec` checks 16 goldens in `tests/e2e/visual/cards/` (5 modes × 3 palettes plus an Arabic card), plus determinism and size.

## Open

| Item | Why it is open |
|---|---|
| AT-19 during big celebrations | Needs a phone and the frame report (the same open item as Phase A). |
| Eric's review of the icons in the app and the celebrations | The Phase D gate. |
| Histograms on cards | No card mode keeps one today. F11 shows one only "where the mode already has one". |
| Web image sharing | Native shares the image; the web keeps text, as before. |
| Card and celebration strings | Drafts listed in `needs_audit`. |
