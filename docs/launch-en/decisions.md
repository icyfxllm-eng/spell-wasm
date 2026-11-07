# CC-LAUNCH-EN v1: decisions recorded after §0 review

Signed by Eric 2026-11-07, in answer to the §0 recon (recon.md) and the fix list that followed it.

- **D19, what stays free in English.** Easy tier in standard mode, Little Speller and Daily Challenge are free. The Climb follows the tier rule and is paid, because it climbs past Easy. This reverses the July 2026 "Daily and Climb free forever" ruling for the Climb only.
- **D20, release base.** The release cuts from main. release/online-testflight is excluded under the no-online decision.
- **Audio ops (item 6).** Eric creates the Cloudflare R2 bucket on audio.spellgame.net, re-enables Google TTS billing, and approves the warm run. Claude writes the sync and warm scripts and the client fallback.
- **Start.** Phase A: the StoreKit 2 plugin SpellStore, the single entitlement rule, the cut to the release flag reader's localStorage, and the five CI scripts.
- **One entitlement rule.** For every language, Easy tier in standard mode is free and every tier above it needs that language's entitlement. English's entitlement is english, the others ride complete or a grant. D2 excludes English from the grant.
- **Product ids.** net.spellgame.english, net.spellgame.complete and net.spellgame.complete_upgrade.
- **D4.** Still decided by R7. As recon found, Complete is not created until StoreKit is wired and photo lists and reports are verified on device. O4 is open.
