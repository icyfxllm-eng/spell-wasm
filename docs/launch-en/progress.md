# CC-LAUNCH-EN progress

- 2026-11-07, §0: recon.md pushed (7d2b7baa).
- 2026-11-07, Phase A, part 1 (branch launch-en/phase-a, NOT on main):
  - The entitlement core moves to one rule: every language at Preview, with Daily added to Preview (D19). There are three products, the English purchase never grants Complete, the regional grant never reaches English (D2), and the Complete offer follows the matrix in complete_offer.
  - SpellStore covers StoreKit 2: the Swift plugin, store-native.js, src/entitlements_adapter.rs and the SpellGame.storekit file. The device build succeeds. It has not been run, because the simulator is blocked by an out-of-date CoreSimulator on this Mac.
  - Release builds stop honoring localStorage flag overrides, except the six Tools-screen player flags. The seam check and scripts/flag-override-release-check.mjs both pass, the latter checked against a dev build as a positive control.
  - Five new CI pieces: scripts/flags-audit.mjs with config/flags.release.json, scripts/placeholder-grep.sh, scripts/price-grep.sh, cargo test --lib repetition_sim, and maestro/release/reviewer-walkthrough.yaml.
  - Expected reds:
    - flags-audit: 38 ship/off decisions are open (docs/launch-en/flags-recs.md).
    - placeholder-grep: 8 strings wait for the Phase B purge.
    - repetition_sim_daily_30_dates: a real cycle-boundary repeat in the Daily.
    - The reviewer walkthrough waits for the paywall and the English-only registry.
  - Not done: per-surface gating (the main picker, Climb and about ten side modes ignore entitlements), Little Speller exemptions, and the paywall UI with the adult gate and Restore.
