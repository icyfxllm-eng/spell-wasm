//! Parser tests — the core CI target for Spell It Out Loud. All pure: no DOM, no
//! audio, no wasm. Feeds token strings (what the recognizer would emit) and
//! asserts the exact keyboard-equivalent letter string (Invariant I1).

use super::*;

/// Parse and return just the letters (the common case).
fn spell(lang: &str, transcript: &str) -> String {
    parse(lang, transcript).letters
}

// ---------------------------------------------------------------------------
// English: letter sequences → exact strings (≥40 known-answer cases)
// ---------------------------------------------------------------------------

#[test]
fn en_known_answer_sequences() {
    // (spoken letter names, expected typed string). Mixed canonical names and
    // ASR homophones, across word tiers.
    let cases: &[(&str, &str)] = &[
        ("see ay tee", "cat"),
        ("dee oh gee", "dog"),
        ("bee ee ee", "bee"),
        ("es you en", "sun"),
        ("em ay pee", "map"),
        ("aitch ay tee", "hat"),
        ("pee ee en", "pen"),
        ("see you pee", "cup"),
        ("bee ay dee", "bad"),
        ("ar ee dee", "red"),
        ("bee ee dee", "bed"),
        ("bee oh ex", "box"),
        ("ef eye ex", "fix"),
        ("jay ay em", "jam"),
        ("kay eye tee", "kit"),
        ("el ee gee", "leg"),
        ("en ee tee", "net"),
        ("pea eye gee", "pig"),
        ("ar you en", "run"),
        ("tee ee en", "ten"),
        ("vee ay en", "van"),
        ("double u ee bee", "web"),
        ("why ee es", "yes"),
        ("zee eye pee", "zip"),
        ("es tea oh pee", "stop"),
        ("pea el ay why", "play"),
        ("ef ar oh gee", "frog"),
        ("gee ar ee ee en", "green"),
        ("es em ay el el", "small"),
        ("bee ar ay eye en", "brain"),
        ("es see aitch oh oh el", "school"),
        ("ef ar eye ee en dee", "friend"),
        ("double u ay tea ee ar", "water"),
        ("pea el ay en ee tee", "planet"),
        ("oh see ee ay en", "ocean"),
        ("aitch oh you es ee", "house"),
        ("tee ay bee el ee", "table"),
        ("ay pea pea el ee", "apple"),
        ("en you em bee ee ar", "number"),
        ("see aitch ay eye ar", "chair"),
        ("kay en oh double u", "know"),
        ("ar aitch why tea aitch em", "rhythm"),
        ("queue you eye see kay", "quick"),
        ("jay you em pea", "jump"),
    ];
    for (spoken, want) in cases {
        assert_eq!(spell(EN, spoken), *want, "EN spelling {spoken:?}");
    }
    assert!(cases.len() >= 40, "need ≥40 EN cases, have {}", cases.len());
}

#[test]
fn en_bare_single_letters_and_case_insensitive() {
    assert_eq!(spell(EN, "C A T"), "cat");
    assert_eq!(spell(EN, "SEE AY TEE"), "cat");
    assert_eq!(spell(EN, "  see   ay   tee  "), "cat");
}

#[test]
fn en_output_is_nfc_and_ascii_for_english() {
    let p = parse(EN, "see ay tee");
    assert!(p.letters.is_ascii());
    // Idempotent under NFC.
    let nfc: String = p.letters.nfc().collect();
    assert_eq!(nfc, p.letters);
}

// ---------------------------------------------------------------------------
// Spanish: diacritics, ñ, multigraphs — NFC byte-for-byte
// ---------------------------------------------------------------------------

#[test]
fn es_diacritic_word_arbol() {
    // "a con tilde, erre, be, o, ele" → árbol (NFC).
    assert_eq!(spell(ES, "a con tilde erre be o ele"), "árbol");
    // The alternate phrasing "a con acento" yields the identical bytes.
    assert_eq!(spell(ES, "a con acento erre be o ele"), "árbol");
    // Byte-for-byte NFC precomposed á (U+00E1).
    assert_eq!(spell(ES, "a con tilde").as_bytes(), "\u{e1}".as_bytes());
}

#[test]
fn es_enye_is_single_precomposed_letter() {
    assert_eq!(spell(ES, "eñe"), "ñ");
    assert_eq!(spell(ES, "eñe").as_bytes(), "\u{f1}".as_bytes());
    // niño: ene, i, eñe, o
    assert_eq!(spell(ES, "ene i eñe o"), "niño");
    // eñe must NOT collapse to ene (accent/ñ preserved in matching).
    assert_ne!(spell(ES, "eñe"), spell(ES, "ene"));
}

#[test]
fn es_multigraph_names_expand_to_two_letters() {
    assert_eq!(spell(ES, "doble ele"), "ll"); // → two letters l l
    assert_eq!(spell(ES, "doble erre"), "rr");
    assert_eq!(spell(ES, "doble ere"), "rr");
    assert_eq!(spell(ES, "elle"), "ll"); // letter name elle → ll
    assert_eq!(spell(ES, "che"), "ch"); // letter name che → ch
    // calle: ce, a, doble ele, e
    assert_eq!(spell(ES, "ce a doble ele e"), "calle");
    // perro: pe, e, ere, doble erre? no — perro = p e r r o via ere + doble erre
    assert_eq!(spell(ES, "pe e doble erre o"), "perro");
}

#[test]
fn es_known_answer_sequences() {
    let cases: &[(&str, &str)] = &[
        ("ce a ese a", "casa"),
        ("ge a te o", "gato"),
        ("eme e ese a", "mesa"),
        ("a ge u a", "agua"),
        ("ele i be ere o", "libro"),
        ("ese o ele", "sol"),
        ("pe a ene", "pan"),
        ("uve e ere de e", "verde"),
        ("hache o ele a", "hola"),
        ("jota u ge o", "jugo"),
        ("i con tilde a", "ía"),
        ("ele u ene a", "luna"),
    ];
    for (spoken, want) in cases {
        assert_eq!(spell(ES, spoken), *want, "ES spelling {spoken:?}");
    }
}

#[test]
fn es_diacritic_all_vowels() {
    assert_eq!(spell(ES, "a con tilde"), "á");
    assert_eq!(spell(ES, "e con tilde"), "é");
    assert_eq!(spell(ES, "i con tilde"), "í");
    assert_eq!(spell(ES, "o con tilde"), "ó");
    assert_eq!(spell(ES, "u con tilde"), "ú");
    assert_eq!(spell(ES, "u con diéresis"), "ü");
    // corazón: ce o ere a zeta o con tilde ene
    assert_eq!(spell(ES, "ce o ere a zeta o con tilde ene"), "corazón");
}

// ---------------------------------------------------------------------------
// EVERY lexicon entry has a fixture → its letter (both languages)
// ---------------------------------------------------------------------------

#[test]
fn every_lexicon_entry_parses_to_its_value() {
    for lang in [EN, ES] {
        let raw: RawLexicon = serde_json::from_str(source(lang).unwrap()).unwrap();
        let mut checked = 0;
        for group in [&raw.letter_names, &raw.homophones, &raw.multigraph, &raw.diacritics] {
            for (spoken, expected) in group {
                let want: String = expected.nfc().collect();
                assert_eq!(
                    spell(lang, spoken),
                    want,
                    "{lang}: lexicon entry {spoken:?} must parse to {expected:?}"
                );
                checked += 1;
            }
        }
        assert!(checked > 0, "{lang}: lexicon should be non-empty");
    }
}

// ---------------------------------------------------------------------------
// Whole-word rejection (Feature 7): ≥20 target words → nothing inserted
// ---------------------------------------------------------------------------

#[test]
fn en_whole_word_said_is_rejected() {
    // Speaking the word itself (not its letters) must be rejected as a whole word —
    // by LETTER YIELD ALONE (no target is passed; answer-leak invariant, G-A).
    let words = [
        "cat", "dog", "house", "apple", "table", "water", "planet", "friend",
        "school", "green", "small", "brain", "number", "chair", "ocean", "rhythm",
        "jump", "quick", "yellow", "orange", "purple", "window",
    ];
    for w in words {
        assert_eq!(
            interpret(EN, w),
            SpellOutcome::WholeWord,
            "EN whole word {w:?} must be rejected (nudge, insert nothing)"
        );
    }
    assert!(words.len() >= 20, "need ≥20 whole-word cases, have {}", words.len());
}

#[test]
fn es_whole_word_said_is_rejected() {
    let words = ["casa", "gato", "mesa", "agua", "libro", "verde", "árbol", "corazón"];
    for w in words {
        assert_eq!(interpret(ES, w), SpellOutcome::WholeWord, "ES whole word {w:?}");
    }
}

#[test]
fn genuine_high_yield_spelling_is_always_inserted() {
    // High yield ⇒ never a whole-word rejection — and the matcher never saw a target.
    assert_eq!(interpret(EN, "see ay tee"), SpellOutcome::Insert("cat".into()));
    assert_eq!(interpret(ES, "ce a ese a"), SpellOutcome::Insert("casa".into()));
    // Diacritic spelling still inserts, byte-for-byte NFC.
    assert_eq!(
        interpret(ES, "a con tilde erre be o ele"),
        SpellOutcome::Insert("árbol".into())
    );
}

#[test]
fn non_letter_speech_nudges_and_only_silence_is_nothing() {
    // Without the target, "spoke a word" and "made noise" are the same low-yield
    // signal: both nudge (insert nothing, never credited).
    assert_eq!(interpret(EN, "um well hmm"), SpellOutcome::WholeWord);
    // Only a truly empty utterance is Nothing ("didn't catch that").
    assert_eq!(interpret(EN, ""), SpellOutcome::Nothing);
}

// --- Phase 0 answer-leak invariant + slot model -----------------------------

#[test]
fn interpret_takes_no_target_and_rejects_whole_words_without_it() {
    // Compile-time signature guard: interpret is (lang, transcript) -> SpellOutcome,
    // with NO target parameter. If a target is ever re-added, this fails to compile.
    let _guard: fn(&str, &str) -> SpellOutcome = interpret;
    // A whole word is rejected with no answer anywhere in sight.
    assert_eq!(interpret(EN, "elephant"), SpellOutcome::WholeWord);
    // A genuine spelling of some OTHER word still inserts.
    assert_eq!(interpret(EN, "see ay tee"), SpellOutcome::Insert("cat".into()));
}

#[test]
fn whole_word_yields_zero_slots_and_spelling_yields_one_per_letter() {
    // A spoken whole word (or babble) parses to zero letter-name slots.
    assert!(parse(EN, "cat").slots.is_empty());
    assert!(parse(EN, "um well hmm").slots.is_empty());
    // Genuine spelling yields one slot per spoken letter, in order.
    let letters: Vec<String> =
        parse(EN, "see ay tee").slots.iter().map(|s| s.letters.clone()).collect();
    assert_eq!(letters, ["c", "a", "t"]);
    // A multigraph name is a SINGLE slot carrying both letters.
    let ll = parse(ES, "elle");
    assert_eq!(ll.slots.len(), 1);
    assert_eq!(ll.slots[0].letters, "ll");
    // Slot letters concatenate to the full parsed string (I1).
    let p = parse(ES, "ene i eñe o");
    assert_eq!(p.slots.iter().map(|s| s.letters.as_str()).collect::<String>(), p.letters);
    assert_eq!(p.letters, "niño");
}

// ---------------------------------------------------------------------------
// Phase 2 — spoken commands (delete / clear / done) as events
// ---------------------------------------------------------------------------

#[test]
fn commands_parse_to_command_events() {
    use Command::*;
    assert_eq!(events(EN, "delete"), vec![Event::Command(Delete)]);
    assert_eq!(events(EN, "clear"), vec![Event::Command(Clear)]);
    assert_eq!(events(EN, "done"), vec![Event::Command(Done)]);
    // Spanish variants.
    assert_eq!(events(ES, "borrar"), vec![Event::Command(Delete)]);
    assert_eq!(events(ES, "listo"), vec![Event::Command(Done)]);
    // Greedy: the two-word "borrar todo" (clear) wins over "borrar" (delete).
    assert_eq!(events(ES, "borrar todo"), vec![Event::Command(Clear)]);
}

#[test]
fn letters_and_a_command_interleave_in_one_utterance() {
    // "c a t done" → three letters then Done, in order.
    let evs = events(EN, "see ay tee done");
    assert_eq!(
        evs,
        vec![
            Event::Letter(Slot { letters: "c".into() }),
            Event::Letter(Slot { letters: "a".into() }),
            Event::Letter(Slot { letters: "t".into() }),
            Event::Command(Command::Done),
        ]
    );
}

#[test]
fn apply_events_edits_the_buffer_and_flags_done() {
    let mut buf: Vec<Slot> = Vec::new();
    // letters accumulate; echo is the newly added letters
    let a = apply_events(&mut buf, &events(EN, "see ay tee"));
    assert_eq!(a.echo, "cat");
    assert!(!a.done);
    assert_eq!(buf.iter().map(|s| s.letters.as_str()).collect::<String>(), "cat");
    // delete pops one; delete on empty is a harmless no-op
    apply_events(&mut buf, &events(EN, "delete"));
    assert_eq!(buf.len(), 2);
    // clear empties; a further delete does nothing
    apply_events(&mut buf, &events(EN, "clear"));
    apply_events(&mut buf, &events(EN, "delete"));
    assert!(buf.is_empty());
    // done flags submit without adding letters
    let d = apply_events(&mut buf, &events(EN, "done"));
    assert!(d.done && d.echo.is_empty());
}

#[test]
fn every_command_entry_parses_to_its_command() {
    for lang in [EN, ES] {
        let raw: RawLexicon = serde_json::from_str(source(lang).unwrap()).unwrap();
        assert!(!raw.commands.is_empty(), "{lang}: commands block should be non-empty");
        for (phrase, id) in &raw.commands {
            let want = Command::from_id(id).unwrap_or_else(|| panic!("{lang}: bad command id {id:?}"));
            assert_eq!(
                events(lang, phrase),
                vec![Event::Command(want)],
                "{lang}: command {phrase:?} must parse to {want:?}"
            );
        }
    }
}

#[test]
fn a_whole_word_yields_no_events() {
    // The reject half of acceptance #1 at the mode level: no letters, no command.
    assert!(events(EN, "cat").is_empty());
    assert!(events(EN, "elephant").is_empty());
    assert!(events(ES, "casa").is_empty());
}

#[test]
fn accept_into_still_folds_letters_ignoring_commands() {
    // The Phase-0/1 letters-only accumulator is unchanged (input-method path).
    let mut buf: Vec<Slot> = Vec::new();
    assert_eq!(accept_into(&mut buf, EN, "see ay tee"), "cat");
    assert_eq!(buf.iter().map(|s| s.letters.as_str()).collect::<String>(), "cat");
}

// ---------------------------------------------------------------------------
// Phase 3 — clarifiers ("X as in Y") + confusable classes / decide_letter
// ---------------------------------------------------------------------------

#[test]
fn clarifier_confirms_a_letter_and_drops_the_example() {
    // The example word is ignored — even when it is itself a letter homophone.
    assert_eq!(events(EN, "bee as in ball"), vec![Event::Letter(Slot { letters: "b".into() })]);
    // "you" is the letter U; without clarifier handling this would wrongly add U.
    assert_eq!(events(EN, "bee as in you"), vec![Event::Letter(Slot { letters: "b".into() })]);
    // Spanish connectors "de" / "como en".
    assert_eq!(events(ES, "be de burro"), vec![Event::Letter(Slot { letters: "b".into() })]);
    assert_eq!(events(ES, "be como en barco"), vec![Event::Letter(Slot { letters: "b".into() })]);
    // Two clarified letters in a row keep only the letters.
    assert_eq!(
        events(EN, "bee as in ball see as in cat"),
        vec![Event::Letter(Slot { letters: "b".into() }), Event::Letter(Slot { letters: "c".into() })]
    );
    // A bare letter with no connector is unaffected; a trailing connector with no
    // example is just dropped as noise.
    assert_eq!(events(EN, "bee"), vec![Event::Letter(Slot { letters: "b".into() })]);
    assert_eq!(events(EN, "bee as in"), vec![Event::Letter(Slot { letters: "b".into() })]);
}

#[test]
fn confusable_classes_are_symmetric_and_scoped() {
    // English E-set members are mutually confusable...
    assert!(are_confusable(EN, "b", "d"));
    assert!(are_confusable(EN, "d", "b"));
    assert!(are_confusable(EN, "p", "t"));
    assert!(are_confusable(EN, "m", "n"));
    // ...but across classes / with a vowel outside them, they are not.
    assert!(!are_confusable(EN, "b", "m")); // different classes
    assert!(!are_confusable(EN, "a", "b")); // 'a' is in no class
    assert!(!are_confusable(EN, "b", "b")); // not itself
    // Spanish b/v.
    assert!(are_confusable(ES, "b", "v"));
    assert!(!are_confusable(ES, "b", "d"));
}

#[test]
fn decide_letter_accepts_when_confident_and_chips_only_low_confidence_confusables() {
    use LetterDecision::*;
    // Confident → always accept, even a confusable letter with a rival alternative.
    assert_eq!(decide_letter(EN, "b", 0.9, Some("d")), Accept("b".into()));
    // Low confidence, confusable letter, same-class alternative → two-choice chip.
    assert_eq!(decide_letter(EN, "b", 0.3, Some("d")), Chip("b".into(), "d".into()));
    // Low confidence but the alternative is NOT same-class → take the best guess.
    assert_eq!(decide_letter(EN, "b", 0.3, Some("m")), Accept("b".into()));
    // Low confidence, no alternative offered → best guess.
    assert_eq!(decide_letter(EN, "b", 0.3, None), Accept("b".into()));
    // Low confidence on a non-confusable letter (a vowel) → accept.
    assert_eq!(decide_letter(EN, "a", 0.1, Some("e")), Accept("a".into()));
    // Spanish b/v chip.
    assert_eq!(decide_letter(ES, "b", 0.4, Some("v")), Chip("b".into(), "v".into()));
}

// ---------------------------------------------------------------------------
// Phase 4 — Spanish b/v: bare name always chips; qualified/clarified resolves
// ---------------------------------------------------------------------------

#[test]
fn bare_be_and_ve_always_chip() {
    // Acceptance #2: an unqualified b/v name is ALWAYS a chip — never auto-resolved
    // (auto-resolving would leak the answer). Same {b,v} pair for both.
    assert_eq!(events(ES, "be"), vec![Event::Chip("b".into(), "v".into())]);
    assert_eq!(events(ES, "ve"), vec![Event::Chip("b".into(), "v".into())]);
}

#[test]
fn qualified_and_clarified_bv_resolve_without_a_chip() {
    // Qualified names disambiguate to a plain letter.
    assert_eq!(events(ES, "be larga"), vec![Event::Letter(Slot { letters: "b".into() })]);
    assert_eq!(events(ES, "ve corta"), vec![Event::Letter(Slot { letters: "v".into() })]);
    assert_eq!(events(ES, "uve"), vec![Event::Letter(Slot { letters: "v".into() })]);
    // Multigraph "doble ve"/"uve doble" → w, never a chip.
    assert_eq!(events(ES, "doble ve"), vec![Event::Letter(Slot { letters: "w".into() })]);
    // Clarifier resolves by the example word's first letter (user speech, not target).
    assert_eq!(events(ES, "be de burro"), vec![Event::Letter(Slot { letters: "b".into() })]);
    assert_eq!(events(ES, "ve de vaca"), vec![Event::Letter(Slot { letters: "v".into() })]);
    // A clarifier whose example starts with neither b nor v can't disambiguate → chip.
    assert_eq!(events(ES, "be de casa"), vec![Event::Chip("b".into(), "v".into())]);
}

#[test]
fn nino_still_spells_with_first_class_enye() {
    // Acceptance #2: "niño" spelled n-i-ñ-o, ñ precomposed, no b/v involved.
    let evs = events(ES, "ene i eñe o");
    let letters: String = evs
        .iter()
        .filter_map(|e| if let Event::Letter(s) = e { Some(s.letters.as_str()) } else { None })
        .collect();
    assert_eq!(letters, "niño");
    assert!(evs.iter().all(|e| matches!(e, Event::Letter(_))), "no chips in niño");
}

#[test]
fn apply_events_surfaces_a_chip_and_holds_the_rest() {
    // "ce be a": push c, then a chip for b/v; the trailing 'a' waits (not applied).
    let mut buf: Vec<Slot> = Vec::new();
    let applied = apply_events(&mut buf, &events(ES, "ce be a"));
    assert_eq!(buf.iter().map(|s| s.letters.as_str()).collect::<String>(), "c");
    assert_eq!(applied.chip, Some(("b".into(), "v".into())));
    assert!(!applied.done);
    // The English path never chips (no ambiguous names there).
    let mut buf2: Vec<Slot> = Vec::new();
    let a2 = apply_events(&mut buf2, &events(EN, "bee ee ee"));
    assert_eq!(a2.chip, None);
    assert_eq!(buf2.iter().map(|s| s.letters.as_str()).collect::<String>(), "bee");
}

#[test]
fn input_method_parse_still_auto_resolves_bv() {
    // The shipped input method (parse/spell) is UNCHANGED: bare "be"/"ve" still map
    // to b/v so words like "libro" and "verde" spell correctly there.
    assert_eq!(parse(ES, "ele i be ere o").letters, "libro");
    assert_eq!(parse(ES, "be").letters, "b");
    assert_eq!(parse(ES, "ve").letters, "v");
}

// ---------------------------------------------------------------------------
// D3 whole-word rejection (CC-SPELL-ALOUD-INTEGRATION Feature 4 / A2, A3, A5)
// ---------------------------------------------------------------------------

/// A6: reserved voice commands (undo/clear/…) must NEVER collide with a letter name
/// in that language's lexicon — else "delete" could be swallowed as a letter. CI gate.
#[test]
fn a6_reserved_commands_never_collide_with_letter_names() {
    use std::collections::HashSet;
    for lang in [EN, ES] {
        let raw: RawLexicon = serde_json::from_str(source(lang).unwrap()).unwrap();
        assert!(!raw.commands.is_empty(), "{lang}: expected reserved commands");
        let mut letters: HashSet<String> = HashSet::new();
        for group in [&raw.letter_names, &raw.homophones, &raw.multigraph, &raw.diacritics] {
            for k in group.keys() {
                letters.insert(norm_phrase(k));
            }
        }
        for cmd in raw.commands.keys() {
            assert!(
                !letters.contains(&norm_phrase(cmd)),
                "{lang}: reserved command {cmd:?} collides with a letter name"
            );
        }
        // The spec's two required commands exist (undo-style + clear).
        let ids: HashSet<&str> = raw.commands.values().map(String::as_str).collect();
        assert!(ids.contains("delete"), "{lang}: needs an undo/delete command");
        assert!(ids.contains("clear"), "{lang}: needs a clear command");
    }
}

#[test]
fn d3_says_target_rejects_whole_and_embedded_only() {
    // A2: saying the word itself.
    assert!(says_target("cat", "cat"));
    assert!(says_target("Cat.", "cat")); // case + edge punctuation
    // A3: the target embedded alongside letters voids the whole utterance.
    assert!(says_target("cat see ay tee", "cat"));
    assert!(says_target("see ay tee cat", "cat"));
    // Genuine spelling is NOT the word — never a false trip.
    assert!(!says_target("see ay tee", "cat")); // homophone letter names → C A T
    assert!(!says_target("c a t", "cat")); // single-letter ASR must NOT join to "cat"
    // A5 / D2: only the TARGET rejects; another word is just an ignored token.
    assert!(!says_target("dog", "cat"));
    assert!(!says_target("elephant", "cat"));
    // Spanish, NFC.
    assert!(says_target("niño", "niño"));
    assert!(!says_target("ene i eñe o", "niño"));
    // D6: if the target IS a letter-homophone word, saying it rejects (Feature 4 wins
    // over the lexicon mapping) — but the homophone spelling it does not.
    assert!(says_target("sea", "sea")); // "sea" == target → rejected
    assert!(!says_target("see", "sea")); // "see" != "sea" → spells C, not rejected
    // Empty target never rejects.
    assert!(!says_target("cat", ""));
}

// ---------------------------------------------------------------------------
// Robustness / edge cases
// ---------------------------------------------------------------------------

#[test]
fn greedy_prefers_longest_phrase() {
    // "double u" must win over "u" alone; "a con tilde" over "a".
    assert_eq!(spell(EN, "double u"), "w");
    assert_eq!(spell(ES, "a con tilde"), "á");
    // A bare "a" still parses as the letter a.
    assert_eq!(spell(ES, "a"), "a");
}

#[test]
fn unknown_tokens_are_skipped_not_fatal() {
    // Interleaved noise words are dropped; real letter names still land.
    assert_eq!(spell(EN, "see um ay uh tee"), "cat");
}

#[test]
fn transcript_edge_punctuation_is_trimmed() {
    assert_eq!(spell(EN, "see, ay. tee!"), "cat");
}

#[test]
fn unsupported_language_parses_to_nothing() {
    assert_eq!(parse("fr", "be a").letters, "");
    assert!(contextual_strings("fr").is_empty());
    // No lexicon → zero tokens parsed → Nothing (never a crash, never a whole-word).
    assert_eq!(interpret("fr", "chat"), SpellOutcome::Nothing);
}

#[test]
fn contextual_strings_expose_the_spoken_forms() {
    let en = contextual_strings(EN);
    assert!(en.contains(&"double u".to_string()));
    assert!(en.iter().any(|s| s == "bee"));
    let es = contextual_strings(ES);
    assert!(es.contains(&"a con tilde".to_string()));
    assert!(es.contains(&"eñe".to_string()));
}
