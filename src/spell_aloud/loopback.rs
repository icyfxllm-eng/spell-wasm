//! Offline loopback ORACLE (CC-SPELL-ALOUD Phase 5, acceptance #3 + #1).
//!
//! Runs in `cargo test` / on every PR. It feeds a 50-word suite per language of
//! realistic spoken-letter transcripts — the ASR homophone variants the recognizer
//! actually emits ("see/sea"→c, "why"→y, "double u"→w) — through the SAME parser the
//! app uses, and asserts high letter accuracy plus the accept/reject/chip behaviour.
//!
//! It **stands in for whisper on every PR** (no audio, deterministic, fast). The REAL
//! TTS→whisper→parser loop (3 synthetic voices × 3 speeds, the ≥95%-under-ASR-noise
//! bar) lives in `tools/spell-aloud-loopback/` and runs offline where the binaries
//! exist — mirroring `tools/audio-verify/`.

use super::*;
use crate::consts::{EN, ES};

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// Letter accuracy of a parsed spelling vs the expected word (1.0 = identical).
fn letter_accuracy(expected: &str, got: &str) -> f64 {
    let m = expected.chars().count().max(got.chars().count()).max(1);
    1.0 - levenshtein(expected, got) as f64 / m as f64
}

fn score_suite(lang: &str, suite: &[(&str, &str)]) -> f64 {
    let total: f64 = suite.iter().map(|(t, w)| letter_accuracy(w, &parse(lang, t).letters)).sum();
    total / suite.len() as f64
}

/// The worst-scoring entry (for a helpful failure message).
fn worst(lang: &str, suite: &[(&str, &str)]) -> (String, String, f64) {
    suite
        .iter()
        .map(|(t, w)| (t.to_string(), parse(lang, t).letters, letter_accuracy(w, &parse(lang, t).letters)))
        .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
        .unwrap()
}

// 50 English words, spelled with a mix of canonical letter names and ASR homophones.
#[rustfmt::skip]
const EN_SUITE: &[(&str, &str)] = &[
    ("see ay tee","cat"), ("dee oh gee","dog"), ("es you en","sun"), ("em ay pee","map"),
    ("aitch ay tee","hat"), ("pee ee en","pen"), ("see you pee","cup"), ("bee ay dee","bad"),
    ("ar ee dee","red"), ("bee ee dee","bed"), ("bee oh ex","box"), ("ef eye ex","fix"),
    ("jay ay em","jam"), ("kay eye tee","kit"), ("el ee gee","leg"), ("en ee tee","net"),
    ("pea eye gee","pig"), ("ar you en","run"), ("tee ee en","ten"), ("vee ay en","van"),
    ("double u ee bee","web"), ("why ee es","yes"), ("zee eye pee","zip"), ("es tea oh pee","stop"),
    ("pea el ay why","play"), ("ef ar oh gee","frog"), ("gee ar ee ee en","green"),
    ("es em ay el el","small"), ("bee ar ay eye en","brain"), ("es see aitch oh oh el","school"),
    ("ef ar eye ee en dee","friend"), ("double u ay tea ee ar","water"), ("pea el ay en ee tee","planet"),
    ("oh see ee ay en","ocean"), ("aitch oh you es ee","house"), ("tee ay bee el ee","table"),
    ("ay pea pea el ee","apple"), ("en you em bee ee ar","number"), ("see aitch ay eye ar","chair"),
    ("kay en oh double u","know"), ("queue you eye see kay","quick"), ("jay you em pea","jump"),
    ("dee ay why","day"), ("es tea ay ar","star"), ("em oh oh en","moon"), ("bee ee ay ar","bear"),
    ("el eye oh en","lion"), ("tee ar ee ee","tree"), ("es en ay kay ee","snake"), ("see el oh you dee","cloud"),
];

// 50 Spanish words, spelled with canonical letter names (be→b, uve→v, ce→c, ere→r…).
#[rustfmt::skip]
const ES_SUITE: &[(&str, &str)] = &[
    ("ce a ese a","casa"), ("ge a te o","gato"), ("eme e ese a","mesa"), ("a ge u a","agua"),
    ("ele i be ere o","libro"), ("ese o ele","sol"), ("pe a ene","pan"), ("hache o ele a","hola"),
    ("jota u ge o","jugo"), ("ele u ene a","luna"), ("eme a ene o","mano"), ("de e de o","dedo"),
    ("ce a ese o","caso"), ("uve i de a","vida"), ("de a de o","dado"), ("ge o te a","gota"),
    ("ele u pe a","lupa"), ("ere o ese a","rosa"), ("te a zeta a","taza"), ("efe o ce a","foca"),
    ("ene u be e","nube"), ("pe e ere a","pera"), ("ele a te a","lata"), ("eme o ene o","mono"),
    ("pe a te o","pato"), ("ere a ene a","rana"), ("ese o pe a","sopa"), ("be o ce a","boca"),
    ("uve a ce a","vaca"), ("de a eme a","dama"), ("ce a eme a","cama"), ("ere a eme a","rama"),
    ("ge o eme a","goma"), ("ele o eme a","loma"), ("te o eme o","tomo"), ("ce o de o","codo"),
    ("ele o de o","lodo"), ("eme o de o","modo"), ("ene i de o","nido"), ("ele i eme a","lima"),
    ("ere i eme a","rima"), ("ce i eme a","cima"), ("ge a ele a","gala"), ("be a ele a","bala"),
    ("ese a ele a","sala"), ("eme a ele a","mala"), ("pe a ele a","pala"), ("te a ele a","tala"),
    ("uve e ele a","vela"), ("ese e de e","sede"),
];

#[test]
fn en_loopback_suite_is_50_words_and_high_accuracy() {
    assert!(EN_SUITE.len() >= 50, "acceptance #3 wants a 50-word suite, have {}", EN_SUITE.len());
    let avg = score_suite(EN, EN_SUITE);
    assert!(avg >= 0.97, "EN loopback accuracy {avg:.3} < 0.97; worst = {:?}", worst(EN, EN_SUITE));
}

#[test]
fn es_loopback_suite_is_50_words_and_high_accuracy() {
    assert!(ES_SUITE.len() >= 50, "acceptance #3 wants a 50-word suite, have {}", ES_SUITE.len());
    let avg = score_suite(ES, ES_SUITE);
    assert!(avg >= 0.97, "ES loopback accuracy {avg:.3} < 0.97; worst = {:?}", worst(ES, ES_SUITE));
}

/// Acceptance #1 in loopback form: "c, a, t, done" accepts CAT; "cat" is rejected and
/// consumes nothing; and the es b/v ambiguity always chips (never auto-picked).
/// A1 (en): serve "cat"; the separate utterances "see","ay","tee" build C,A,T, and
/// submitting the assembled word scores correct. Parser layer: each utterance inserts
/// exactly its letter (as the input method appends), the sequence assembles to the
/// target, and neither a letter nor the whole spelling false-rejects; the cheat "cat"
/// contributes nothing (A2 companion). End-to-end scoring is guaranteed by A10.
#[test]
fn a1_cat_en_letter_by_letter() {
    let target = "cat";
    let mut buf = String::new();
    for u in ["see", "ay", "tee"] {
        match interpret("en", u) {
            SpellOutcome::Insert(l) => buf.push_str(&l),
            other => panic!("utterance {u:?} should insert a letter, got {other:?}"),
        }
        assert!(!says_target(u, target), "letter {u:?} must not be a whole-word reject");
    }
    assert_eq!(buf, target, "C,A,T assembles to the target → submit grades correct");
    assert!(!says_target("see ay tee", target));
    assert_eq!(interpret("en", "see ay tee"), SpellOutcome::Insert("cat".into()));
    assert!(says_target("cat", target), "the cheat (saying the word) is rejected");
}

/// A4 (es): serve "niño"; utterances "ene","i","eñe","o" build n,i,ñ,o, NFC-exact
/// (ñ precomposed U+00F1); submit scores correct. The cheat "niño" contributes nothing.
#[test]
fn a4_nino_es_letter_by_letter_nfc() {
    let target = "niño";
    let mut buf = String::new();
    for u in ["ene", "i", "eñe", "o"] {
        match interpret("es", u) {
            SpellOutcome::Insert(l) => buf.push_str(&l),
            other => panic!("utterance {u:?} should insert a letter, got {other:?}"),
        }
        assert!(!says_target(u, target));
    }
    assert_eq!(buf, target);
    assert_eq!(buf.as_bytes(), "ni\u{f1}o".as_bytes(), "ñ is precomposed NFC (U+00F1)");
    assert!(!says_target("ene i eñe o", target));
    assert!(says_target("niño", target), "the cheat is rejected");
}

#[test]
fn loopback_accepts_spelling_rejects_whole_word_and_chips_bv() {
    // accept: letters + done → the word, submit flagged
    let mut buf: Vec<Slot> = Vec::new();
    let a = apply_events(&mut buf, &events(EN, "see ay tee done"));
    assert_eq!(buf.iter().map(|s| s.letters.as_str()).collect::<String>(), "cat");
    assert!(a.done, "\"done\" completes the turn");
    // reject: a spoken whole word yields no events and interprets as WholeWord
    assert!(events(EN, "cat").is_empty());
    assert_eq!(interpret(EN, "cat"), SpellOutcome::WholeWord);
    // chip: an unqualified es b/v name always disambiguates via a chip
    let mut b2: Vec<Slot> = Vec::new();
    assert!(apply_events(&mut b2, &events(ES, "be")).chip.is_some());
}
