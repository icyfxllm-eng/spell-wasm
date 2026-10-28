# CC-THEMES-COSMOS Phase A: the Theme Lab and the twelve looks

Scope (spec Phase A):
- F1 Theme Lab;
- F3 token sets and drawn art for all 12 themes, with Jr variants and keycap tokens;
- F4 astro module;
- contrast and ΔE suites.

No unlock wiring, and nothing reaches players: the new themes are absent from Appearance, and the Lab exists only in dev and Lab builds.

Gate to Phase B: Eric reviews every theme in the Lab on his phone and approves or adjusts the palettes (AT-T18).

## How Eric reviews it

1. Get the Lab build:
   - On a phone: the internal-only Lab TestFlight build. Building it needs his go:

     ```
     SPELL_THEME_LAB=1 bundle exec fastlane ship
     ```

     That one command builds the app with `--features theme_lab` and uploads it. It also records the build number in `fastlane/lab-builds.txt` and `~/.spell-lab-builds`. From then on, `submit_review`, `submit_localized`, `submit_external_review` and `beta_external` all refuse that number.
   - In a browser: `bash scripts/build-web-test.sh` or `bash scripts/build-web-dev.sh`.
2. Tap the SPELL logo five times, then **Theme Lab**.
3. The Lab shows a grid of all 17 themes: live tiles with each one's family, wave, unlock rule and current counter value.
4. Tap a tile to wear that theme app-wide. A "LAB · {theme}" chip sits in the top corner on every screen; tap it to clear. The preview survives a relaunch.
5. Controls for the applied theme:
   - Sky band, pinned or following the clock;
   - Lunar phase, any of 8, or following the date;
   - Solar System planet, any of 8;
   - Jr register on or off;
   - Simulate Reduce Motion;
   - Play unlock moment (writes nothing);
   - a date and time override that reaches only the Sky band and the moon.

## What changed

| Piece | Where |
|---|---|
| 12 token sets, 4 Sky bands, 8 planets, Jr blocks, `theme.*` tokens (page ground, scrim, hint pill, orb face, keycaps), `art.*` colours | design/tokens.json |
| Theme enum +12; `ALL` stays the Classic five (Appearance, ledger); `REGISTRY` holds 17; Lab preview hook; `data-cosmos` / `data-band` / `data-phase` / `data-planet`; word-boundary refresh | src/themes.rs |
| Registry rows, waves, pinned names and facts, planets, look resolver | src/themes_cosmos.rs |
| SVG art from tokens: brushed grain, pavé field, sky bands, beach, phlox, aurora, wall ridge, moon terminator, planets, stone | src/theme_art.rs |
| Theme Lab (theme_lab feature only; it makes its own DOM and style) | src/theme_lab.rs |
| Moon phase (Meeus ch. 49), sky band, event table 2026–2040, cross-checked against NASA | src/astro.rs, src/astro_events.rs, tools/build_astro_events.py |
| Art layer, ground, scrims, glint, orb face layer, hint pill (tokens only, no box moves) | index.html |
| `[data-theme-scope]` token scoping for the Lab's live tiles | scripts/tokens-build.mjs |
| Lab build switch | scripts/build-web.sh (`SPELL_THEME_LAB=1`), Cargo.toml (`theme_lab`, implied by `dev_preview`) |
| Submit refusals | fastlane/Fastfile, fastlane/lab-builds.txt |

## Accents retuned for I-T3

ΔE2000 is now at least 20 from all eight verdict colours (Eric, 2026-10-28). Each hue moved the least it could, and the colour of the world itself lives in the art (T-D18).

| Accent | Spec start | Built |
|---|---|---|
| Bronze | #5A2F16 | #4d2814 |
| Gold | #5E440C | #564106 |
| Sky · Sunrise | #F6C08A | #f7deae |
| Sky · Sunset | #F2A07A | #ffc8c2 |
| Beach | #1E8FA3 | #2891ab (button #1d6e83, for white text) |
| Pink Moon | #F2A6C8 | #f7afd7 |
| Great Wall | #C9B79C | #c4b89f |
| Venus | #E3C98F | #e2cd92 |
| Mars | #D2693F | #a28377 (Mars stays red in its art) |
| Jupiter | #D8A877 | #b3a375 |
| Uranus | #8ED6D8 | #9bdff3 |
| Blood Moon | tuned | #c9cfe0 (cool silver; the red is the moon) |

Other changes:
- **Sky Day's Check button** is darker (#23649a) so its white label reads.
- **Neptune's button** is darker (#3a58c4) so its white label reads.
- **Bright themes take the light verdict pair:** Bronze, Silver, Gold, Beach and Sky Day (I-T2, per lightness).
- **Muted text on the page** (Hint, field labels) uses each look's panel-muted colour, as 59 existing theme blocks already do. The default 62% mix fell to 3.0–4.4:1 on the metal sheens, Beach's sand, and Sky's sunrise, day and sunset. The cosmos gate now checks this token on every ground stop.

## Suites

| Gate | What |
|---|---|
| scripts/themes-cosmos-check.mjs (pre-push, with selftest) | I-T3 ΔE across 44 looks; I-T2 per lightness, with no Jr block touching a verdict; ink and muted text on every stop of a look's ground; I-T1 no theme id where things are sold; I-T11 only themes.rs reads the Lab |
| scripts/check-contrast.mjs | every Cosmos look and variant, standard and Jr |
| scripts/astro-check.mjs (pre-push, with selftest) | AT-T14: no network, location or analytics in astro, and no NSLocation key; AT-T8: the expiry warning 2 years before 2040 |
| scripts/seam-absence-check.mjs --selftest (pre-push) | AT-T2: the Lab's markers in a store dist fail the build; a planted dist proves it bites |
| cargo tests themes_cosmos, theme_art, theme_lab, astro | registry, waves, pinned facts (F6), looks, terminator, determinism, AT-T7 against NASA's table |
| tests/e2e/specs/themes-cosmos.mjs | AT-T1, T3, T4, T5 (axe, every look, standard and Jr), T5b (worst-pixel contrast over art, plus the planted label), T11/T12/T19 (proxy: no animation or repaint of the art during a word; glint only between words, never under Reduce Motion), T13, T16, T17 |

### Not covered by automation

- **AT-T19's 60 fps** on the oldest device is a device trace. The e2e proves only that the field layer does not change or animate during a word.
- **AT-T18** is Eric's pass.

## Still open

- **C4 counters** for waves 1–3: Impossible correct, days played, sessions. They are needed for Phase B onward and await Eric's approval.
- **C9:** result cards take the theme's colours, but not its art. Drawing the art needs an F11 renderer change.
- **C7:** does Diamond's visibility follow `flags::impossible()`?
- **O1 / O2:** Great Wall. It is built for the Lab only.
- **The Test entitlements switch** in the store binary: next, per Eric's order.
