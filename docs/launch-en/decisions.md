# CC-LAUNCH-EN v1: decisions recorded after §0 review

Signed by Eric 2026-11-07, in answer to the §0 recon (recon.md) and the fix list that followed it.

- **D19, what stays free in English.** Easy tier in standard mode, Little Speller and Daily Challenge are free. The Climb follows the tier rule and is paid, because it climbs past Easy. This reverses the July 2026 "Daily and Climb free forever" ruling for the Climb only.
- **D20, release base.** The release cuts from main. release/online-testflight is excluded under the no-online decision.
- **Audio ops (item 6).** Eric creates the Cloudflare R2 bucket on audio.spellgame.net, re-enables Google TTS billing, and approves the warm run. Claude writes the sync and warm scripts and the client fallback.
- **Start.** Phase A: the StoreKit 2 plugin SpellStore, the single entitlement rule, the cut to the release flag reader's localStorage, and the five CI scripts.
- **One entitlement rule.** For every language, Easy tier in standard mode is free and every tier above it needs that language's entitlement. English's entitlement is english, the others ride complete or a grant. D2 excludes English from the grant.
- **Product ids.** net.spellgame.english, net.spellgame.complete and net.spellgame.complete_upgrade.
- **D4.** Still decided by R7. As recon found, Complete is not created until StoreKit is wired and photo lists and reports are verified on device. O4 is open.

## Gating rulings, signed by Eric 2026-11-07 after the Preview-leak audit

- **D21, side modes.** Every mode clamps to Easy at Preview, the way Def Match, Randospell and Quick Bee Draw already do. Medium and above open the paywall. The modes this covers are Letter Forge, Word Chains, Impostor, Bee, Spelldoku, Spell Search, Spell Cross, Spelluzzle, Board Game, Practice and Versus.
- **D22, Daily arc.** The free Daily serves its full Easy-to-Expert arc. There is no Preview clamp on the Daily.
- **D23, Daily cycle boundary.** Fix it now, in Phase C, by widening the boundary guard so a fresh cycle cannot re-deal a word from the previous cycle's last 30 days. Only the deals in each boundary window change. After the fix, A7.1 passes.
- **D24, Little Speller.** Kids never see a paywall. Little Speller is free at every difficulty, so no surface is locked for a kid profile. The paywall appears only on standard profiles, behind the adult gate.

## Flag table and Climb rule, signed by Eric 2026-11-07

- The recommendations in flags-recs.md are accepted, with three rows set by Eric. `en_retier` is off, because 1,210 words have no settled regrading; they return with EN-EXPERT-GROWTH. `spelluzzle` is off: no device-pass verdict was ever recorded, and if Eric writes one line saying E1–E6 passed, it can go back on. `def_match` is off, because it works online only. The result is 18 ship and 20 off, recorded in config/flags.release.json, and the defaults in src/flags.rs match it.
- **One Climb rule.** Anything that climbs past Easy is paid, so the Climb variant of Def Match follows the main Climb's gate.
- **Push gate.** Nothing pushes until A4.1–A4.9 pass on a real iPhone, with the scheme's StoreKit Configuration set to SpellGame.storekit. In App Store Connect, Eric creates net.spellgame.english ($4.99) and net.spellgame.complete_upgrade ($7.99). net.spellgame.complete stays unavailable until R7.
