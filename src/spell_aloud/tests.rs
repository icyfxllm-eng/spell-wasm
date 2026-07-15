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
    // Speaking the word itself (not its letters) must be rejected as a whole word.
    let words = [
        "cat", "dog", "house", "apple", "table", "water", "planet", "friend",
        "school", "green", "small", "brain", "number", "chair", "ocean", "rhythm",
        "jump", "quick", "yellow", "orange", "purple", "window",
    ];
    for w in words {
        assert_eq!(
            interpret(EN, w, w),
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
        assert_eq!(interpret(ES, w, w), SpellOutcome::WholeWord, "ES whole word {w:?}");
    }
}

#[test]
fn genuine_spelling_is_never_rejected_even_when_it_resembles_word() {
    // High yield ⇒ never a whole-word rejection, whatever the similarity.
    assert_eq!(interpret(EN, "see ay tee", "cat"), SpellOutcome::Insert("cat".into()));
    assert_eq!(interpret(ES, "ce a ese a", "casa"), SpellOutcome::Insert("casa".into()));
    // Diacritic spelling of the exact target still inserts, never rejects.
    assert_eq!(
        interpret(ES, "a con tilde erre be o ele", "árbol"),
        SpellOutcome::Insert("árbol".into())
    );
}

#[test]
fn noise_yields_nothing() {
    // No letters parsed and doesn't resemble the target → Nothing (not WholeWord).
    assert_eq!(interpret(EN, "um well hmm", "cat"), SpellOutcome::Nothing);
    assert_eq!(interpret(EN, "", "cat"), SpellOutcome::Nothing);
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
    assert_eq!(interpret("fr", "chat", "chat"), SpellOutcome::Nothing);
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
