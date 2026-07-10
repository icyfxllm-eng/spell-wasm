# UI translation — native-speaker review checklist

The UI string tables live in `src/i18n/locales/{code}.json`. `en.json` is the
canonical key set; `scripts/i18n-check.mjs` (run in `npm run build`) enforces
key parity across all locales.

Brand names kept **untranslated** on purpose: **SPELL** (top-of-app mark) and
**The Climb** (leaderboard). Emoji glyphs in button labels are intentional and
carried across locales.

## Confidence by locale

| Code | Language   | Status | Notes |
|------|------------|--------|-------|
| en   | English    | canonical | reference — matches shipped wording verbatim |
| es   | Español    | good | reviewed against in-app Spanish gameplay |
| fr   | Français   | good | AZERTY audience; verify "Dust"→ H2H term choice |
| de   | Deutsch    | good | watch string length (layout audit, Phase 1.4) |
| pt   | Português  | good | pt-PT neutral; confirm pt-BR reads naturally |
| it   | Italiano   | good | — |
| nl   | Nederlands | **needs native review** | machine-assisted draft |
| pl   | Polski     | **needs native review** | machine-assisted draft; longest strings (layout audit) |
| sv   | Svenska    | **needs native review** | machine-assisted draft |
| nb   | Norsk Bokmål | **needs native review** | machine-assisted draft |
| tr   | Türkçe     | **needs native review** | machine-assisted draft; agglutinative length |

## What a reviewer should check per locale
1. Natural phrasing of buttons/labels (not literal calques).
2. Terminology for game concepts: **misses / chain / streak / kid mode**.
3. Settings descriptions read as helpful microcopy, not translationese.
4. No clipped/overflowing strings at 375 pt width + Big Text (see Phase 1.4).
5. Diacritics render correctly (compare against the on-screen keyboard, Phase 2).

Add reviewer initials + date here as each locale is signed off.
