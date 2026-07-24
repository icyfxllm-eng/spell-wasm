//! CLI bridge for the Spell-Aloud loopback harness (CC-SPELL-ALOUD Phase 5).
//!
//! Reads a language + a spoken transcript and prints the parser's verdict as JSON, so
//! the whisper.cpp loopback (`tools/spell-aloud-loopback/`) can score REAL ASR output
//! against the SAME parser the app uses — no duplicated letter logic (Invariant I4).
//!
//!   cargo run --example spell_aloud_parse -- en "see ay tee"
//!   echo "be de burro" | cargo run --example spell_aloud_parse -- es

use std::io::Read;

use spell_wasm::spell_aloud::{apply_events, events, interpret, parse, Slot, SpellOutcome};

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let lang = args.get(1).cloned().unwrap_or_default();
    let transcript = if args.len() > 2 {
        args[2..].join(" ")
    } else {
        let mut s = String::new();
        let _ = std::io::stdin().read_to_string(&mut s);
        s.trim().to_string()
    };

    // Input-method view (parse/interpret) + mode view (events/apply_events).
    let letters = parse(&lang, &transcript).letters;
    let outcome = match interpret(&lang, &transcript) {
        SpellOutcome::Insert(_) => "insert",
        SpellOutcome::WholeWord => "whole_word",
        SpellOutcome::Nothing => "nothing",
    };
    let mut buf: Vec<Slot> = Vec::new();
    let applied = apply_events(&mut buf, &events(&lang, &transcript));
    let mode_word: String = buf.iter().map(|s| s.letters.as_str()).collect();

    println!(
        "{{\"letters\":\"{}\",\"outcome\":\"{}\",\"mode_word\":\"{}\",\"done\":{},\"chip\":{}}}",
        esc(&letters),
        outcome,
        esc(&mode_word),
        applied.done,
        applied.chip.is_some(),
    );
}
