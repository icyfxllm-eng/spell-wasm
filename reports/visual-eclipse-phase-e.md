# CC-VISUAL-ECLIPSE Phase E: themes and accents

Both features ship behind flags that are off by default and killable: `visual.themes` and `visual.accents`.

## F9: earnable themes (`visual.themes`)

- A theme is a token set and nothing else: `:root[data-theme=<id>]` overrides in `design/tokens.json`, plus a brighter Jr variant under `body.kid` (V-D6).
- **The themes:**
  - Eclipse (free, dark)
  - Daybreak (free, light)
  - Chalkboard (earned; chalk-dust texture)
  - Typewriter (earned; paper and ink, a typewriter face for Latin-script interfaces, a keystrike sound)
  - Spelling Bee Stage (earned; spotlight and podium)
- Randospell's table and Board Game's ring keep their own art.
- **Appearance in Settings:**
  - System (the default, following the phone's light or dark setting), then each theme.
  - A locked theme shows how it is earned, never a price or a store link (V-D5).
- **Unlocks (V-D19, E-Q8):**
  - Chalkboard: a 7-day Daily streak.
  - Typewriter: 500 lifetime correct words.
  - Spelling Bee Stage: 100 correct words at Hard or above.
- **How unlocks are stored:** in `spell_themes_v1`, never revoked, and never wiped by the stats reset.
  - Retroactive: a baseline frozen from the base-game stats the first time, plus the Daily best streak.
  - Forward: a counter of every correct answer in every mode, and of base-game Hard+ answers.
  - An unlock is a big celebration (F7).
- **The colour pickers step aside:** Background, Orb, Glow, Controls and Buttons no longer pin their defaults inline, so a theme's own colours show unless the player picked one.
- **Changes on main regardless of the flags:**
  - Chinese buttons now use the zh gold. The red accent with dark text measured 3.65:1, below AA.
  - The Daybreak and Typewriter verdict colours are darker variants of the same teal and amber, so they reach 3:1 on light panels.

## F10: per-language accents (`visual.accents`)

- There are 15 `lang.accent.<code>` tokens. `:root[data-study=<code>]` points `--lang-accent` at the language on screen, so Misses follows each word's language.
- Accents appear on the language chip (a dot), progress fills, and a 12% orb tint (the spec allows up to 15%). They are never used as a text colour.
- No flags anywhere (AT-25, `scripts/flags-check.mjs`). The finish-line icon is renamed `finish`.

## Tests

- **AT-21** (theme contrast): `scripts/check-contrast.mjs` now covers every theme × standard and Jr, resolves `color-mix`, and adds muted text, ink on the raised panel, and button text. `themes.spec` also runs a small axe-like contrast pass over the rendered play screen in all ten variants.
  - There is no axe-core: adding it would mean a network dependency, so this is the substitute.
- **AT-22** (theme isolation): grading reads no theme. `display_diff`'s all-flags-on regression test covers it.
- **AT-23** (unlocks):
  - unit tests: retroactive grant, baseline plus counter, never revoked
  - `themes.spec`: the rows, no price or store link, a qualified player unlocks on first launch and keeps it after a stats wipe, and a locked theme can't be chosen
- **AT-24** (accent contrast): the contrast gate requires 4.5:1 for every accent if any rule ever colours text with one. None does today.
- **AT-25** (accent separation): CIEDE2000 checked against a published reference pair. Every accent is ΔE00 ≥ 20 from both verdict colours and their light-theme variants. zh and sw were moved to pass.

Full suites: the app suite passes except two known flakes; web-config cargo has the 4 failures that predate Eclipse.

## Open

| Item | Why it is open |
|---|---|
| Eric approves all 5 themes and the 15 accents | The Phase E gate. |
| The texture art | Code-drawn CSS. A painted texture can replace it later. |
| Theme strings | Drafts listed in `needs_audit`. |
