//! Runtime feature flags — the one place a shipped-dark feature is switched on.
//!
//! Each flag is a `pub fn -> bool`, read from localStorage (`spell_flag_<name>`
//! = "on"/"off"/…); when absent it falls back to the compiled-in default. A flag
//! being OFF must be a true no-op: the feature registers nothing, wires nothing,
//! renders nothing — zero observable diff. Under `cargo test` the storage read is
//! replaced by a settable in-process override so gates are unit-testable.
//!
//! This is the UNIFIED Feature Pack 2 module (the sibling branches each authored
//! their own copy; this reconciles them, keeping F6's testable design). Defaults
//! below are set for the FP2 **integration / TestFlight build**: F1, F2, F6, F7
//! are ON so testers exercise them; F5 word-stories stays OFF (its CC BY-SA
//! attribution gate is unresolved). For the App Store these revert to OFF per PR.

#[cfg(not(test))]
use crate::storage;

/// Pure flag resolver (unit-tested). "on"/"1"/"true"/"yes" → true,
/// "off"/"0"/"false"/"no" → false, anything else (incl. absent) → `default`.
fn resolve(stored: Option<&str>, default: bool) -> bool {
    match stored {
        Some("on") | Some("1") | Some("true") | Some("yes") => true,
        Some("off") | Some("0") | Some("false") | Some("no") => false,
        _ => default,
    }
}

#[cfg(not(test))]
fn stored(name: &str) -> Option<String> {
    // F4: the signed kill-only file can switch a flag OFF and nothing else (src/killflags.rs).
    if crate::killflags::is_killed(name) {
        return Some("off".to_string());
    }
    storage::get_raw(&format!("spell_flag_{name}"))
}

#[cfg(test)]
thread_local! {
    static TEST_OVERRIDE: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
fn stored(name: &str) -> Option<String> {
    if crate::killflags::is_killed(name) {
        return Some("off".to_string());
    }
    TEST_OVERRIDE.with(|c| c.borrow().clone())
}

/// Test-only: force the next flag read (`Some("on")`/`Some("off")`) or clear
/// (`None`) so both branches of a flag-gated path are reachable in unit tests.
/// GLOBAL (one override for all flags) — fine for unit tests, which set it right
/// before the single flag read they exercise. E2E has no `cfg(test)` seam, so it
/// flips the real `localStorage['spell_flag_<name>']` instead.
#[cfg(test)]
pub fn set_test_override(v: Option<&str>) {
    TEST_OVERRIDE.with(|c| *c.borrow_mut() = v.map(|s| s.to_string()));
}

/// CC-EN-EXPERT-GROWTH F4/F8 — serve the re-tiered English bank
/// (`word_data_en_retier.rs`) instead of the current one. **Default OFF**: with
/// it off English plays exactly as before (I8). Spell Jr and Kid Mode never see
/// the re-tier whatever this says (I5; `words::en_tier`).
///
/// **TestFlight 1 (CC-IMPOSSIBLE-LAUNCH Feature 6, Eric 2026-10-19): default ON.**
/// The signed kill file (`killflags`) can still switch it off with the name `en_retier`.
pub fn en_retier() -> bool {
    resolve(stored("en_retier").as_deref(), true)
}

/// CC-IMPOSSIBLE v1 F12 -- the master Impossible flag. **Default OFF.** Nothing
/// Impossible is offered unless this is on. v1 F12 calls these remote flags; this
/// module is localStorage only, so turning one on for players needs a build (or a
/// remote-flag channel, which does not exist yet).
pub fn impossible() -> bool {
    resolve(stored("impossible").as_deref(), true)
}

/// CC-IMPOSSIBLE v1 F12 -- one flag per language (`spell_flag_impossible_<lang>`),
/// every one **default OFF**. The registry (`crate::impossible`) still applies
/// the pool, table, release and entitlement gates on top of it.
pub fn impossible_lang(lang: &str) -> bool {
    // RELEASE BRANCH ONLY (TestFlight 2): the ten launch languages on. hi/vi/ar/zh/ja stay locked by the registry.
    let launch = matches!(lang, "en" | "es" | "ru" | "fr" | "de" | "pt" | "pl" | "ko" | "fil" | "sw");
    resolve(stored(&format!("impossible_{lang}")).as_deref(), launch)
}

/// CC-IMPOSSIBLE v1.2 M8 -- Impossible Mix. **Default OFF.** It needs the master
/// flag and at least two qualifying languages as well (`impossible::mix_status`).
pub fn impossible_mix() -> bool {
    resolve(stored("impossible_mix").as_deref(), true)
}

/// F6 "Spell Racing in The Climb" (formerly Ghost racing) — race your best local run. Cross-platform.
pub fn ghost_racing() -> bool {
    resolve(stored("ghost_racing").as_deref(), true)
}

/// F7 "Syllable replay on misses" — the "hear it slowly" affordance on the
/// reveal / spaced-repetition surface (es-only).
pub fn syllable_replay() -> bool {
    resolve(stored("syllable_replay").as_deref(), true)
}

/// F1 "Photo-to-word-list" — VisionKit OCR of a handout; iOS-only, hidden in Kid Mode.
pub fn photo_list() -> bool {
    resolve(stored("photo_list").as_deref(), true)
}

/// CC-LETTER-FORGE — the honeycomb word-finding mode. **Default ON** since Eric
/// signed the strings and flipped it live (2026-08-13). He did so before Done
/// #4's Korean block-assembly capture was reviewed and before F4/F5 exist, so a
/// first-time player currently gets no how-to; that was his call, recorded here
/// so the gap is not mistaken for an oversight.
pub fn letter_forge() -> bool {
    resolve(stored("letter_forge").as_deref(), true)
}

/// CC-WORD-CHAINS — per-language chaining (shiritori done right). **Default ON**
/// since Eric signed the strings and flipped it live (2026-08-13). The D1
/// settings live in chains::Strictness, so a wrong call is a table edit rather
/// than a rebuild — which is what made flipping it before a native check of the
/// zh tone reading a reasonable risk.
pub fn word_chains() -> bool {
    resolve(stored("word_chains").as_deref(), true)
}

/// CC-IMPOSTOR — spot the real spelling. **Default ON** since Eric accepted the
/// D3 bank-versus-dictionary gap (2026-08-12) and signed the strings.
pub fn impostor() -> bool {
    resolve(stored("impostor").as_deref(), true)
}

/// CC-BEE-SIM — the staged spelling bee. **Default ON** since Eric signed the
/// strings and flipped it live (2026-08-13). The contestant table went live
/// without a line-by-line review; dropping the flags first is what made that
/// tolerable, since twenty-four given names claim nothing about anyone.
pub fn bee_sim() -> bool {
    resolve(stored("bee_sim").as_deref(), true)
}

/// CC-BOARD-GAME v1.2 -- the roll-and-spell board race. **Default ON** (Eric,
/// 2026-10-08: released to TestFlight). Kill-switch with
/// `localStorage['spell_flag_boardgame'] = 'off'`. Compiled out with the mode
/// on the site build (I13): a kill-switch for an absent mode is a string leak
/// and nothing else.
#[cfg(not(feature = "web"))]
pub fn boardgame() -> bool {
    resolve(stored("boardgame").as_deref(), true)
}

/// CC-BOARD-GAME-POLISH v1 Feature 1 (D-P16) -- Stretch words in the Board Game.
/// **Default ON**. TestFlight default ON (D-P25, Oct 9 2026); set back to false before a public release.
/// Kill-switch: `localStorage['spell_flag_boardStretch'] = 'off'`. Compiled out with the mode.
/// Stretch stays structurally off in Spell Jr whatever this says (O-P2).
#[cfg(not(feature = "web"))]
pub fn board_stretch() -> bool {
    // TestFlight default ON (D-P25, Oct 9 2026); set back to false before a public release.
    resolve(stored("boardStretch").as_deref(), true)
}

/// CC-SPELLUZZLE v1 F13 -- the rune cipher puzzle. **Default ON** (Eric, 2026-10-09:
/// released to TestFlight, for his device pass E1 to E6). Kill-switch with
/// `localStorage['spell_flag_spelluzzle'] = 'off'`. App only (O1), so it is compiled
/// out with the mode on the site build.
#[cfg(not(feature = "web"))]
pub fn spelluzzle() -> bool {
    resolve(stored("spelluzzle").as_deref(), true)
}

/// CC-SPELLUZZLE v1.1 F22 -- pencil runes (F16), the rune strip (F17) and the ripple (F18).
/// Each is **default OFF** and requires the `spelluzzle` flag (D25). With all three off the
/// mode is v1 byte for byte (I13). Switch one on with `localStorage['spell_flag_spelluzzle_pencil']
/// = 'on'` (and `_strip`, `_ripple`).
#[cfg(not(feature = "web"))]
pub fn spelluzzle_pencil() -> bool {
    spelluzzle() && resolve(stored("spelluzzle_pencil").as_deref(), true)
}
#[cfg(not(feature = "web"))]
pub fn spelluzzle_strip() -> bool {
    spelluzzle() && resolve(stored("spelluzzle_strip").as_deref(), true)
}
#[cfg(not(feature = "web"))]
pub fn spelluzzle_ripple() -> bool {
    spelluzzle() && resolve(stored("spelluzzle_ripple").as_deref(), true)
}

/// CC-SPELLUZZLE v1.1 F15 -- Par boards. **Default ON** (device pass, 2026-10-10), requires `spelluzzle` (D25).
/// `localStorage['spell_flag_spelluzzle_par'] = 'on'`.
#[cfg(not(feature = "web"))]
pub fn spelluzzle_par() -> bool {
    spelluzzle() && resolve(stored("spelluzzle_par").as_deref(), true)
}

/// CC-SPELLUZZLE v1.1 F19 -- Expeditions, and F21 -- My Words boards. **Default ON** (device pass, 2026-10-10), each
/// requires `spelluzzle` (D25). `localStorage['spell_flag_spelluzzle_expedition'] = 'on'` and
/// `...spelluzzle_mywords`.
#[cfg(not(feature = "web"))]
pub fn spelluzzle_expedition() -> bool {
    spelluzzle() && resolve(stored("spelluzzle_expedition").as_deref(), true)
}
#[cfg(not(feature = "web"))]
pub fn spelluzzle_mywords() -> bool {
    spelluzzle() && resolve(stored("spelluzzle_mywords").as_deref(), true)
}

/// CC-SPELLUZZLE v1.1 F20 -- Rune Duel. **Default ON** (device pass, 2026-10-10), requires `spelluzzle` (D25).
/// `localStorage['spell_flag_spelluzzle_duel'] = 'on'`.
#[cfg(not(feature = "web"))]
pub fn spelluzzle_duel() -> bool {
    spelluzzle() && resolve(stored("spelluzzle_duel").as_deref(), true)
}

/// F13: one flag per language, so a language can be pulled on its own. English is the
/// only one built; the others stay off, and F12 keeps ko zh ja ar hi out whatever this says.
#[cfg(not(feature = "web"))]
pub fn spelluzzle_lang(lang: &str) -> bool {
    resolve(stored(&format!("spelluzzle_{lang}")).as_deref(), lang == "en")
}

/// CC-BOARD-GAME-POLISH v1 Feature 2 (D-P16) -- boost tiles in the Board Game. **Default ON**.
/// TestFlight default ON (D-P25, Oct 9 2026); set back to false before a public release.
/// Kill-switch: `localStorage['spell_flag_boardBoosts'] = 'off'`. Compiled out with the mode.
#[cfg(not(feature = "web"))]
pub fn board_boosts() -> bool {
    // TestFlight default ON (D-P25, Oct 9 2026); set back to false before a public release.
    resolve(stored("boardBoosts").as_deref(), true)
}

/// CC-BOARD-GAME-POLISH v1 Feature 3 (D-P16) -- the hot streak. **Default ON**.
/// TestFlight default ON (D-P25, Oct 9 2026); set back to false before a public release.
/// Kill-switch: `localStorage['spell_flag_boardStreak'] = 'off'`. Compiled out with the mode.
#[cfg(not(feature = "web"))]
pub fn board_streak() -> bool {
    // TestFlight default ON (D-P25, Oct 9 2026); set back to false before a public release.
    resolve(stored("boardStreak").as_deref(), true)
}

/// CC-RANDOSPELL v1 F9 -- the dice-driven spelling mode. **Default ON** for the
/// first TestFlight cut (Eric, 2026-10-14: "good to go for shipping"), with
/// `randospell_marathon`. Daily, Head-to-Head and Jr stay default OFF until
/// their device pass. Kill it with
/// `localStorage['spell_flag_randospell'] = 'off'`. Compiled out with the mode on
/// the site build (O3). Per-language eligibility is `randospell::bank::eligible`
/// and the registry's language list, not this flag (D1).
#[cfg(not(feature = "web"))]
pub fn randospell() -> bool {
    resolve(stored("randospell").as_deref(), true)
}

/// CC-RANDOSPELL F9: Marathon (Phase B). Needs `randospell`.
#[cfg(not(feature = "web"))]
#[allow(dead_code)]
pub fn randospell_marathon() -> bool {
    randospell() && resolve(stored("randospell_marathon").as_deref(), true)
}

/// CC-RANDOSPELL F9: Daily (Phase C). Needs `randospell`.
#[cfg(not(feature = "web"))]
#[allow(dead_code)]
pub fn randospell_daily() -> bool {
    randospell() && resolve(stored("randospell_daily").as_deref(), true)
}

/// CC-RANDOSPELL F9: Head-to-Head (Phase C). Needs `randospell`.
#[cfg(not(feature = "web"))]
#[allow(dead_code)]
pub fn randospell_h2h() -> bool {
    randospell() && resolve(stored("randospell_h2h").as_deref(), true)
}

/// CC-RANDOSPELL F9: the Spell Jr variant (Phase D). Needs `randospell`.
#[cfg(not(feature = "web"))]
#[allow(dead_code)]
pub fn randospell_jr() -> bool {
    randospell() && resolve(stored("randospell_jr").as_deref(), true)
}

/// CC-RANDOSPELL v1.1 F19 -- the "over the top" mechanics. Each is **default OFF**
/// and needs `randospell`; v1.1 ships them together (D-R7), the individual flags
/// are for staging. With one off, that mechanic is exactly v1.
#[cfg(not(feature = "web"))]
pub fn randospell_heat() -> bool {
    randospell() && resolve(stored("randospell_heat").as_deref(), true)
}

#[cfg(not(feature = "web"))]
pub fn randospell_call() -> bool {
    randospell() && resolve(stored("randospell_call").as_deref(), true)
}

#[cfg(not(feature = "web"))]
pub fn randospell_stretch() -> bool {
    randospell() && resolve(stored("randospell_stretch").as_deref(), true)
}

#[cfg(not(feature = "web"))]
pub fn randospell_doubles() -> bool {
    randospell() && resolve(stored("randospell_doubles").as_deref(), true)
}

#[cfg(not(feature = "web"))]
pub fn randospell_ear() -> bool {
    randospell() && resolve(stored("randospell_ear").as_deref(), true)
}

/// CC-RANDOSPELL v1.1 F19 -- the NPC table (F10). **Default OFF**; needs `randospell`.
/// With it on, Jr's Sprint also gets its one racing NPC.
#[cfg(not(feature = "web"))]
pub fn randospell_npc() -> bool {
    randospell() && resolve(stored("randospell_npc").as_deref(), true)
}

/// CC-RANDOSPELL v1.1 F15 -- Steal, NPC table and Head-to-Head only. **Default OFF**.
#[cfg(not(feature = "web"))]
pub fn randospell_steal() -> bool {
    randospell() && resolve(stored("randospell_steal").as_deref(), true)
}

/// CC-RANDOSPELL v1.1 F16 -- the Final Round (was "Last Call"), NPC table and
/// Head-to-Head only. **Default OFF**.
#[cfg(not(feature = "web"))]
pub fn randospell_lastcall() -> bool {
    randospell() && resolve(stored("randospell_lastcall").as_deref(), true)
}

/// CC-RANDOSPELL v1.1 F18 -- the Eclipse visuals. **Default OFF** and independent:
/// with it off the mode plays with plain numerals and pips and every state is
/// still readable (I20).
#[cfg(not(feature = "web"))]
pub fn randospell_fx() -> bool {
    randospell() && resolve(stored("randospell_fx").as_deref(), true)
}

/// CC-QUICKBEE-DRAW F4 -- the drawn Quick Bee, one flag per language
/// (`spell_flag_quickbee_draw_<lang>`), every one **default OFF**. Which
/// languages exist, and which are measured enough to be offered at all, is the
/// `quickbee_draw::LANGS` table, not this file.
#[cfg(not(feature = "web"))]
pub fn quickbee_draw_lang(lang: &str) -> bool {
    crate::quickbee_draw::cfg(lang).is_some_and(|c| resolve(stored(c.flag_key).as_deref(), c.measured))
}

/// The registry's mode flag: any language's draw flag is on.
#[cfg(not(feature = "web"))]
pub fn quickbee_draw() -> bool {
    crate::quickbee_draw::LANGS.iter().any(|c| resolve(stored(c.flag_key).as_deref(), c.measured))
}

/// F5 "Word stories" — etymology cards. **Default OFF**: dark until the CC BY-SA
/// attribution approach is approved.
pub fn word_stories() -> bool {
    resolve(stored("word_stories").as_deref(), false)
}

/// Online "Spell Off" (async 1v1, server-owned seed — see online_spelloff.rs +
/// backend/matches.py). **Default OFF**: the /api/match backend must be deployed
/// to the Pi before this can function, so it ships dark until then.
pub fn online_spelloff() -> bool {
    resolve(stored("online_spelloff").as_deref(), false)
}

/// Feature "Spell It Out Loud" — voice spelling INPUT (a mic beside the answer
/// field; the player speaks letter names "C… A… T" and the parser produces the
/// same string a keyboard would). **Default ON for the v2 TestFlight QA pass**
/// (Eric, 2026-07-15) so the on-device mic is testable without a localStorage
/// flag. Still iOS-only at runtime and hidden until the per-language voiceSpell
/// flag + on-device speech availability both hold, so on the web/Android it stays
/// a no-op. Override off with `localStorage['spell_flag_spell_aloud'] = '0'`.
pub fn spell_aloud() -> bool {
    resolve(stored("spell_aloud").as_deref(), true)
}

/// CC-ATTEMPTS-SHIELDS — the extra-attempts toggle (normal mode) + shields (The
/// Climb). **Default ON for the build-54 QA pass** (Eric): build-54 retires the
/// legacy 3-try mechanic, so the base game is now one attempt per word and these
/// aids ARE the safety net — they must be live for testers to exercise the new
/// one-shot flow. Storage key `spell_flag_attempts_shields`. Override off with
/// `localStorage['spell_flag_attempts_shields'] = 'off'`. The extra-attempts
/// *toggle* still defaults OFF per-player (a separate pref); this flag only
/// reveals its settings row and enables Climb shields.
pub fn attempts_shields() -> bool {
    resolve(stored("attempts_shields").as_deref(), true)
}

/// AUDITPASS F15 / D9 (signed, Eric 2026-08-06) — the offline-packs
/// section. **Default OFF.**
///
/// The client is complete: rows render per active language, the click
/// dispatches, `download()` fetches `{api_base}/packs/{lang}/v1/
/// manifest.json` and verifies its signature against PACK_PUB_KEY. What
/// does not exist is the SERVER — there is no `/packs` route anywhere in
/// backend/. So every Download 404s into `fail()`, which is why the Aug
/// 6 audit read the buttons as doing nothing. No client work can fix a
/// missing endpoint, and a button that cannot succeed is the dead
/// switch F8 exists to forbid, so the whole section stays hidden until
/// the pack artifacts are built and deployed. Flip with
/// `localStorage['spell_flag_offline_packs'] = 'on'` to develop against
/// a local server.
pub fn offline_packs() -> bool {
    resolve(stored("offline_packs").as_deref(), false)
}

/// Effective value of a tool flag by its storage name (the `<name>` in
/// `spell_flag_<name>`), honoring each flag's compiled-in default. Used by the
/// Tools hub to set a row's initial switch state without duplicating defaults.
/// CC-DEF-MATCH — Definition Match (REVIEW-GATED). Default ON: the registry
/// entry is coming_soon (teaser only) until P3 lands the playable loop, so the
/// flag currently just lets the teaser tile render.
pub fn def_match() -> bool {
    resolve(stored("def_match").as_deref(), true)
}

/// CC-WORD-PICTURE v5 — calligram mode. Default ON: the registry gates
/// audiences (PREVIEW free trio, Kid computed eligibility); the flag is the
/// standard runtime kill-switch. Compiled out with the mode on the site
/// build (I1): a kill-switch for an absent mode is a string leak and
/// nothing else.
#[cfg(not(feature = "web"))]
pub fn word_picture() -> bool {
    resolve(stored("word_picture").as_deref(), true)
}

/// CC-REPORTS / CC-CALENDAR / CC-TRANSLATE — three SHIPPED modes that
/// could not be reached.
///
/// `play_hub` builds its enabled set as every mode id filtered through
/// `is_on`, and `is_on` is a match ending in `_ => false`. These three
/// had no arm, so they read as OFF, `permitted()` refused them, and
/// their tiles never rendered — their only other entry points are the
/// hidden buttons the tile would have clicked. Built, wired, tested,
/// invisible. Eric asked "is the calendar and translate installed on the
/// app yet"; this is the answer.
///
/// Default ON: all three are live in the registry and were never meant
/// to be switchable off. scripts/modes-check.mjs had been reporting the
/// gap in plain words the whole time and was not in the gate — it is now.
pub fn reports() -> bool {
    resolve(stored("reports").as_deref(), true)
}

pub fn calendar() -> bool {
    resolve(stored("calendar").as_deref(), true)
}

/// CC-TRANSLATE-SCREEN Phase A ships behind this flag. **Default OFF** (Eric,
/// 2026-09-14: "flag off"): until Phase B's hub placement and naming are signed
/// and a gloss file is audited, the screen answers nothing, and a tile that
/// answers nothing is a dead end. Develop with
/// `localStorage['spell_flag_translate'] = 'on'`.
pub fn translate() -> bool {
    resolve(stored("translate").as_deref(), false)
}

/// CC-SPELLDOKU v1 — the standard runtime kill-switch. Default ON: whether a
/// language actually plays is decided by its number-word table (D3), not here.
/// App only (D9).
#[cfg(not(feature = "web"))]
pub fn spelldoku() -> bool {
    resolve(stored("spelldoku").as_deref(), true)
}

/// CC-WORDGRID v1 Phase A — Spell Search's runtime kill-switch. Default ON:
/// whether a language plays is decided by the launch set in
/// `wordsearch::lexicon`, not here. App only.
#[cfg(not(feature = "web"))]
pub fn spell_search() -> bool {
    resolve(stored("spell_search").as_deref(), true)
}

/// CC-WORDGRID v1 Phase B — Spell Cross's runtime kill-switch. Default ON;
/// which languages play is the launch set in `wordsearch::lexicon`. App only.
#[cfg(not(feature = "web"))]
pub fn spell_cross() -> bool {
    resolve(stored("spell_cross").as_deref(), true)
}

/// CC-PRACTICE — the guided first-contact mode. Default ON (failure-proof,
/// writes nothing outside its own record); per-language gate is consts::practice.
pub fn practice() -> bool {
    resolve(stored("practice").as_deref(), true)
}

/// CC-LEARNING-ENGINE L1 — Learner Model word selection (reorders the next
/// few deck draws toward due/uncertain skills). **Default ON** since the
/// L1/L2 QA pass (2026-08-05: selection property tests 10k-seeded, the
/// placement evals, three guardian fixtures, and the placement e2e).
/// Override off with `localStorage['spell_flag_learner_select']='off'`.
pub fn learner_select() -> bool {
    resolve(stored("learner_select").as_deref(), true)
}

/// CC-TONAL-POLISH F4/D2 (signed) — the run's flawless-streak words feed
/// the focal stroke where available. **Default ON.** Word choice only:
/// every candidate faces the same solve/overlap/frame law, so OFF vs ON
/// differs only in WHICH legal word lands on the focal slot. Override
/// off with `localStorage['spell_flag_streak_focal'] = 'off'`.
pub fn streak_focal() -> bool {
    resolve(stored("streak_focal").as_deref(), true)
}

/// CC-LEARNING-ENGINE L2 feature 6 — the in-game pattern insight, one
/// line on the post-answer reveal. **Default OFF**: the spec makes the
/// Kid Mode copy variant Eric's review gate, so the strings ship in the
/// audited pool. **Default ON** since Eric read them (2026-08-05:
/// "insight copy passes") — the Kid-Mode register review the spec
/// required. Fires only when one skill is genuinely shaky and only
/// after an answer resolves; silence is the normal case.
pub fn learner_insight() -> bool {
    resolve(stored("learner_insight").as_deref(), true)
}

/// CC-LEARNING-ENGINE surfaces — the placement offer card and the Stats
/// guardian section. **Default ON** since the same QA pass (the in-game
/// insight line stays unbuilt, so nothing here needs the Kid-Mode copy
/// review — placement + guardian report only). Override off with
/// `localStorage['spell_flag_learner_surfaces']='off'`.
pub fn learner_surfaces() -> bool {
    resolve(stored("learner_surfaces").as_deref(), true)
}

/// CC-SPELLDOKU-RULES v1 F5: the header chip is the Numbers / Letters / Mix
/// picker now, and that picker does not expose Tier Mode. Tier Mode (v1.3) was
/// only ever reachable FROM that chip, so rather than stranding a shipped
/// feature it moves behind its own flag, which is where F5 already assumed it
/// lived. On, and the chip offers its two readings again below the three
/// symbol choices. Storage key `spell_flag_sd_tier_mode`. Default off.
pub fn sd_tier_mode() -> bool {
    resolve(stored("sd_tier_mode").as_deref(), false)
}

/// CC-PICK-QUEUE v1 Phase A -- tap words in Misses and My Words to line them up
/// and practice them first. **Default OFF** until Eric signs the strings (they
/// are machine drafts in `needs_audit`). App only (O-F): the site build is
/// permanently false, so the site cannot reach any of it. Develop with
/// `localStorage['spell_flag_pick_queue_a'] = 'on'`.
#[cfg(not(feature = "web"))]
pub fn pick_queue_a() -> bool {
    resolve(stored("pick_queue_a").as_deref(), false)
}

#[cfg(feature = "web")]
pub fn pick_queue_a() -> bool {
    false
}

/// CC-PICK-QUEUE v1 Phase B -- focus stars: a starred word leads every session
/// until it is learned (cap ten per language). Its OWN flag, **default OFF**,
/// and it only takes effect while `pick_queue_a` is also on, because the stars
/// live on Phase A's surfaces (see `pick_stars::enabled`). Permanently false in
/// the site build (O-F). Develop with `localStorage['spell_flag_pick_queue_b'] = 'on'`.
#[cfg(not(feature = "web"))]
pub fn pick_queue_b() -> bool {
    resolve(stored("pick_queue_b").as_deref(), false)
}

#[cfg(feature = "web")]
pub fn pick_queue_b() -> bool {
    false
}

/// CC-PICK-QUEUE v1 Phase C -- help choosing: sort, search and "Toughest 5" on
/// the lineup surfaces. Its OWN flag, **default OFF**, and it only takes effect
/// while `pick_queue_a` is also on, because the controls live on Phase A's
/// surfaces (see `pick_help::enabled`). Permanently false in the site build
/// (O-F). Develop with `localStorage['spell_flag_pick_queue_c'] = 'on'`.
#[cfg(not(feature = "web"))]
pub fn pick_queue_c() -> bool {
    resolve(stored("pick_queue_c").as_deref(), false)
}

#[cfg(feature = "web")]
pub fn pick_queue_c() -> bool {
    false
}

/// CC-PICK-QUEUE v1 Phase D -- F4: a lined-up or starred word spelled wrong
/// comes back in the same session, 3-5 words later. Its OWN flag, **default
/// OFF**, and it only takes effect while `pick_queue_a` is also on (the lineup
/// is what it re-queues). Permanently false in the site build (O-F). Develop
/// with `localStorage['spell_flag_pick_queue_d'] = 'on'`.
#[cfg(not(feature = "web"))]
pub fn pick_queue_d() -> bool {
    resolve(stored("pick_queue_d").as_deref(), false)
}

#[cfg(feature = "web")]
pub fn pick_queue_d() -> bool {
    false
}

/// CC-VISUAL-ECLIPSE `visual.type` (Phase A3): per-script typography beyond the
/// shared type roles -- taller line height on word surfaces for scripts with
/// stacked marks (vi, hi, ar, fa, ur) and explicit regional system faces for zh,
/// ja and ko. **Default OFF** (V-D22: the build turns it on, the signed kill file
/// can only switch it off, as `visual_type`). Off is a true no-op: the body never
/// gets the `visual-type` class, so none of its rules match.
pub fn visual_type() -> bool {
    resolve(stored("visual_type").as_deref(), false)
}

/// CC-VISUAL-ECLIPSE `visual.feedback` (Phase B, F4): answer feedback that
/// teaches -- the letters lift in a stagger on a correct answer, and a wrong one
/// is caution amber with "You wrote" / "Correct" rows marked from
/// `display_diff`, where the mode's feedback policy allows it. **Default OFF**
/// (V-D22), killable as `visual_feedback`. Off is a true no-op: no body class,
/// and the reveal takes today's path.
pub fn visual_feedback() -> bool {
    resolve(stored("visual_feedback").as_deref(), false)
}

/// CC-VISUAL-ECLIPSE `visual.orb` (Phase B, F3): the orb as a living character
/// -- the seven states, the speech envelope, the idle pause. **Default OFF**
/// (V-D22), killable as `visual_orb`. Off: no body class, no state machine
/// attribute, no envelope decode.
pub fn visual_orb() -> bool {
    resolve(stored("visual_orb").as_deref(), false)
}

/// CC-SPELL-KEYBOARD `keyboard.core` (Phase A: KF1, KF2, KF10): every on-screen
/// key grid renders and dispatches through the one shared keyboard
/// (`spellkb`). **Default OFF** (K-D28), killable as `keyboard_core`. Off is a
/// true no-op: each surface builds its keys exactly as before.
pub fn keyboard_core() -> bool {
    resolve(stored("keyboard_core").as_deref(), false)
}

pub fn is_on(name: &str) -> bool {
    match name {
        // The `core` surfaces have no flag because there is nothing to flip:
        // the base game, The Climb and the Daily cannot be "off". They had no
        // arm at all until CC-HUB-NAV Phase B, which did not matter while the
        // only caller was the hub sheet -- core never tiles there anyway -- but
        // the drawer gates on `enabled` too, and D-N6 puts all three at the top
        // of Spell it. my_words and misses joined them in Phase C for the same
        // reason: surfaces, not modes, with nothing to flip. Without these they
        // were on paper only, exactly as the note below describes for
        // calendar, translate and reports.
        "standard" | "climb" | "daily" | "my_words" | "misses" | "versus" => true,
        "ghost_racing" => ghost_racing(),
        "syllable_replay" => syllable_replay(),
        "photo_list" => photo_list(),
        "word_stories" => word_stories(),
        "online_spelloff" => online_spelloff(),
        "spell_aloud" => spell_aloud(),
        "attempts_shields" => attempts_shields(),
        "def_match" => def_match(),
        "practice" => practice(),
        "learner_select" => learner_select(),
        "learner_surfaces" => learner_surfaces(),
        "learner_insight" => learner_insight(),
        "streak_focal" => streak_focal(),
        // The four arcade modes. Registered here as well as above because
        // is_on is the dispatcher the hub asks — a mode LIVE in the registry
        // with no arm here is on paper only, which is exactly how calendar,
        // translate and reports stayed invisible for a release.
        #[cfg(not(feature = "web"))]
        "letter_forge" => letter_forge(),
        #[cfg(not(feature = "web"))]
        "word_chains" => word_chains(),
        #[cfg(not(feature = "web"))]
        "impostor" => impostor(),
        #[cfg(not(feature = "web"))]
        "bee_sim" => bee_sim(),
        #[cfg(not(feature = "web"))]
        "word_picture" => word_picture(),
        #[cfg(not(feature = "web"))]
        "reports" => reports(),
        #[cfg(not(feature = "web"))]
        "calendar" => calendar(),
        #[cfg(not(feature = "web"))]
        "translate" => translate(),
        #[cfg(not(feature = "web"))]
        "spelldoku" => spelldoku(),
        #[cfg(not(feature = "web"))]
        "spell_search" => spell_search(),
        #[cfg(not(feature = "web"))]
        "spell_cross" => spell_cross(),
        #[cfg(not(feature = "web"))]
        "boardgame" => boardgame(),
        #[cfg(not(feature = "web"))]
        "randospell" => randospell(),
        #[cfg(not(feature = "web"))]
        "quickbee_draw" => quickbee_draw(),
        #[cfg(not(feature = "web"))]
        "spelluzzle" => spelluzzle(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bug class this file's own comment warns about, made a test: a
    /// registry mode that falls through to `_ => false` is invisible to every
    /// surface gating on `enabled`, however live the registry says it is. It
    /// cost calendar, translate and reports a release, and it hid the three
    /// core surfaces from the drawer until CC-HUB-NAV Phase B.
    ///
    /// Asked of the SOURCE, not of behaviour: an arm may legitimately return a
    /// constant false (word_stories ships dark), which is indistinguishable at
    /// runtime from having no arm at all. Storage is also a no-op in a host
    /// test, so there is nothing to flip. The question is whether the dispatcher
    /// mentions the mode — falling through is the bug.
    #[test]
    fn every_registry_mode_has_an_arm() {
        let src = include_str!("flags.rs");
        let dispatcher = &src[src.find("pub fn is_on").expect("is_on exists")..];
        let dispatcher = &dispatcher[..dispatcher.find("\n}").expect("is_on closes")];
        for m in crate::modes::all() {
            assert!(dispatcher.contains(&format!("\"{}\"", m.id)),
                    "{} is in the registry but is_on has no arm for it, so it falls through to \
                     `_ => false` and is on paper only", m.id);
        }
    }

    #[test]
    fn visual_feedback_and_orb_ship_dark() {
        set_test_override(None);
        assert!(!visual_feedback() && !visual_orb(), "Phase B ships dark until a build turns it on (V-D22)");
        set_test_override(Some("on"));
        assert!(visual_feedback() && visual_orb());
        set_test_override(None);
    }

    #[test]
    fn visual_type_ships_dark() {
        set_test_override(None);
        assert!(!visual_type(), "visual.type is default off until a build turns it on (V-D22)");
        set_test_override(Some("on"));
        assert!(visual_type());
        set_test_override(None);
    }

    #[test]
    fn resolve_semantics() {
        assert!(resolve(Some("on"), false));
        assert!(resolve(Some("1"), false));
        assert!(!resolve(Some("off"), true));
        assert!(!resolve(Some("0"), true));
        assert!(resolve(None, true));
        assert!(!resolve(None, false));
        assert!(resolve(Some("garbage"), true));
    }

    /// CC-RANDOSPELL: the first cut ships Sprint and Marathon; Daily, Head-to-Head
    /// and Jr stay dark until their device pass, and every sub-flag needs the mode.
    #[cfg(not(feature = "web"))]
    #[test]
    fn randospell_defaults_are_sprint_and_marathon_only() {
        set_test_override(None);
        assert!(randospell() && randospell_marathon());
        assert!(!randospell_daily() && !randospell_h2h() && !randospell_jr());
        // v1.1 mechanics ship together (D-R7) and stay dark until their device pass.
        assert!(!randospell_npc(), "the NPC table stays dark until its device pass");
        assert!(!randospell_steal() && !randospell_lastcall() && !randospell_fx());
        assert!(!randospell_heat() && !randospell_call() && !randospell_stretch() && !randospell_doubles() && !randospell_ear());
        set_test_override(Some("off"));
        assert!(!randospell() && !randospell_marathon(), "the mode flag is the kill switch");
        set_test_override(None);
    }

    #[test]
    fn defaults_match_the_integration_build() {
        set_test_override(None);
        assert!(ghost_racing() && syllable_replay() && photo_list());
        assert!(!word_stories(), "F5 ships dark");
        assert!(!translate(), "Translate Phase A ships dark until Phase B");
        set_test_override(Some("on"));
        assert!(translate(), "and the override turns it on");
        set_test_override(Some("off"));
        assert!(!ghost_racing());
        set_test_override(None);
    }

    #[test]
    fn is_on_honors_each_flag_default() {
        set_test_override(None);
        // Same defaults the Tools hub renders each switch from.
        assert!(is_on("ghost_racing") && is_on("syllable_replay"));
        assert!(is_on("photo_list") && is_on("spell_aloud") && is_on("attempts_shields"));
        assert!(!is_on("word_stories") && !is_on("online_spelloff"));
        assert!(!is_on("unknown_flag"), "unknown names read as off");
        set_test_override(Some("off"));
        assert!(!is_on("ghost_racing"));
        set_test_override(None);
    }

    #[cfg(not(feature = "web"))]
    #[test]
    fn board_game_rule_flags_default_on_for_testflight() {
        // D-P25 (Oct 9 2026). Absent storage reads the default; "off" is the kill-switch.
        set_test_override(None);
        assert!(board_stretch() && board_boosts() && board_streak(), "TestFlight default ON; set back before a public release");
        assert!(!resolve(Some("off"), true), "spell_flag_<name>=off kills each rule");
    }

    #[test]
    fn attempts_shields_defaults_on_and_override_wins() {
        set_test_override(None);
        assert!(attempts_shields(), "attempts_shields defaults ON for the build-54 QA pass");
        set_test_override(Some("off"));
        assert!(!attempts_shields());
        set_test_override(Some("on"));
        assert!(attempts_shields());
        set_test_override(None);
    }
}
