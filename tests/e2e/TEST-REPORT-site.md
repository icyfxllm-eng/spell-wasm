# TEST-REPORT — Web E2E (site build)

**15/15 passed** across 4 areas.

## coming — 5/5
- ✅ coming[std]: selecting Korean gates play + shows Notify Me
- ✅ coming[kid]: selecting Korean gates play + shows Notify Me
- ✅ coming: Spanish is now gated (English-only launch)
- ✅ coming: English is playable (not gated)
- ✅ coming: Notify Me tap confirms and survives reload

## human-audio — 4/4
- ✅ human audio: a verified clip heads the router (D2)
- ✅ human audio: switch off plays no human clip (F6)
- ✅ human audio: a clip that fails to load falls through to TTS (I6)
- ✅ human audio: slow replay is the same clip at 0.7x, pitch kept (D8)

## platform — 3/3
- ✅ platform[site]: English always plays
- ✅ platform[site]: non-English follows the platform
- ✅ platform[site]: every language is still LISTED

## picture-wall — 3/3
- ✅ wall: no picture DOM, tile, or asset request in any of 15 languages
- ✅ wall: direct picture routes are 404 by absence, not by guard
- ✅ wall: the hub registry itself carries no app-only mode
