//! CC-AUDIO-CLARITY v1.1 F6a — what a void must never do.
//!
//! The census (C11) found eight places a round's outcome can be written and
//! warned that adding a void to each is how a leak happens. These tests hold
//! the structural claim that made eight edits unnecessary: `submit_guess` is
//! the single root, `answered` is the universal "round is over" flag, and the
//! two SINKS -- `on_correct` and `finalize_incorrect_ex` -- are the only places
//! an outcome is written. If a ninth path appears, or a sink stops consulting
//! the guard, these fail.

#[cfg(test)]
mod tests {
    const GAME: &str = include_str!("game.rs");

    fn body(src: &str, from: &str) -> String {
        let i = src.find(from).unwrap_or_else(|| panic!("{from} moved"));
        let rest = &src[i..];
        // Up to the next top-level item.
        let end = rest[1..].find("\n}\n").map(|e| e + 2).unwrap_or(rest.len());
        rest[..end].to_string()
    }

    /// I10's structural half: both sinks consult the guard, and they do it
    /// BEFORE they write anything.
    #[test]
    fn both_outcome_sinks_consult_the_guard() {
        for sink in ["fn on_correct(app: &App) {", "fn finalize_incorrect_ex("] {
            let b = body(GAME, sink);
            let guard = b.find("round_voided(app)").unwrap_or_else(|| panic!("{sink} does not consult round_voided"));
            // Nothing that records may precede the guard.
            for writer in ["stats::record", "record_miss_routed", "bump_streak", "shield_", "note_attempt"] {
                if let Some(at) = b.find(writer) {
                    assert!(guard < at, "{sink}: {writer} runs before the void guard");
                }
            }
        }
    }

    /// The reveal must not call an outcome path at all. A void is an absence
    /// of an outcome (§9), not a quieter version of one.
    #[test]
    fn the_reveal_never_calls_an_outcome_path() {
        let b = body(GAME, "fn void_round(app: &App, reveal: bool) {");
        for outcome in ["on_correct(", "finalize_incorrect", "on_wrong", "stats::record", "record_miss_routed", "bump_streak"] {
            assert!(!b.contains(outcome), "reveal_word calls {outcome} -- a void must record nothing");
        }
        // And it must end the round for every other path (submit, timeout,
        // give up all key off `answered`).
        assert!(b.contains("s.answered = true"), "reveal_word must close the round to the other paths");
        assert!(b.contains("s.voided = true"), "reveal_word must raise the guard");
    }

    /// I11: shown, never entered. The word reaches the feedback line; the
    /// answer box is emptied and the inputs are locked before it is shown.
    #[test]
    fn the_revealed_word_never_reaches_the_answer_field() {
        let b = body(GAME, "fn void_round(app: &App, reveal: bool) {");
        assert!(b.contains("lock_inputs()"), "the inputs must be locked");
        assert!(b.contains("set_answer(app, \"\")"), "the answer box must be emptied");
        assert!(!b.contains("set_answer(app, &word"), "the word must never be placed in the answer field");
        let shows = b.find("dom::set_text(\"feedback\"").expect("the word is shown on the feedback line");
        let locks = b.find("lock_inputs()").unwrap();
        assert!(locks < shows, "the input is locked BEFORE the word is shown");
    }

    /// I12: the rescue is local. Nothing in it reaches the network.
    #[test]
    fn the_rescue_sends_nothing() {
        for f in ["fn void_round(app: &App, reveal: bool) {", "pub fn open_rescue(app: &App) {", "pub fn rescue_reveal_or_skip(app: &App) {"] {
            let b = body(GAME, f);
            for net in ["fetch", "XmlHttpRequest", "telemetry::record", "http"] {
                assert!(!b.contains(net), "{f} touches {net} -- the rescue must work offline and send nothing");
            }
        }
    }

    /// D13's caps, and that practice is unlimited.
    #[test]
    fn the_caps_are_what_d13_says() {
        let b = body(GAME, "fn void_cap(s: &AppState) -> Option<u32> {");
        assert!(b.contains("Some(2)") && b.contains("daily"), "Daily Challenge allows 2 per play");
        assert!(b.contains("Some(1)") && b.contains("climb"), "Climb allows 1 per run");
        assert!(b.contains("None"), "practice is unlimited");
    }

    /// At the cap the fourth step becomes a skip, and a skip is that mode's
    /// own miss -- this file does not invent a scoring rule for it (§9).
    #[test]
    fn at_the_cap_the_step_becomes_the_modes_own_miss() {
        let b = body(GAME, "pub fn skip_word(app: &App) {");
        assert!(b.contains("give_up(app)"), "a skip must be the existing miss, not a new rule");
    }

    /// CLIP-REPORT: "Can't tell" is the same void as the reveal, not a new
    /// scoring rule. Pointing it at the miss (give up) is the mutation that
    /// costs a player their streak, and these are what catch it.
    #[test]
    fn both_rescue_entries_go_through_the_one_void() {
        let r = body(GAME, "pub fn reveal_word(app: &App) {");
        assert!(r.contains("void_round(app, true)"), "the reveal shows the word through the shared void");
        let c = body(GAME, "pub fn cant_tell(app: &App) {");
        assert!(c.contains("void_round(app, false)"), "Can't tell voids the round without showing the word");
        for outcome in ["give_up(", "skip_word(", "on_correct(", "finalize_incorrect", "bump_streak", "break_streak", "stats::record", "record_miss_routed", "streak = 0"] {
            assert!(!c.contains(outcome), "cant_tell calls {outcome} -- a skip must not be an outcome");
        }
    }

    /// The report is the ONLY thing Can't tell sends, and it only goes through
    /// the telemetry module's route (opt-in, kill switch, Jr and My Words rules).
    #[test]
    fn cant_tell_reports_only_through_the_telemetry_route() {
        let c = body(GAME, "pub fn cant_tell(app: &App) {");
        for net in ["fetch", "XmlHttpRequest", "http", "post("] {
            assert!(!c.contains(net), "cant_tell touches {net}");
        }
        assert!(c.contains("telemetry::record_clip("), "the report goes through record_clip");
        assert!(c.contains("cant_tell_offered(app)"), "the same gate as the button guards the handler");
    }
}
