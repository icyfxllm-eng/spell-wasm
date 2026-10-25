//! A1.6: one million generated frames through the decoder, no panics.
//!
//! Half the cases are arbitrary text; half are valid frames with one
//! mutation (a field dropped, renamed, retyped, truncated or duplicated),
//! which is where a decoder actually breaks.

use proptest::prelude::*;
use spell_server::protocol::{decode, encode_client, samples, MAX_FRAME};
use uuid::Uuid;

fn valid_frames() -> Vec<String> {
    samples::client().iter().map(|m| encode_client(Uuid::new_v4(), m)).collect()
}

fn mutate(frame: &str, kind: u8, at: usize, junk: &str) -> String {
    let at = if frame.is_empty() { 0 } else { at % frame.len() };
    let at = (0..=at).rev().find(|i| frame.is_char_boundary(*i)).unwrap_or(0);
    match kind % 6 {
        0 => frame[..at].to_string(),                              // truncate
        1 => format!("{}{}{}", &frame[..at], junk, &frame[at..]),  // insert
        2 => frame.replacen("\"body\"", "\"bod\"", 1),             // rename
        3 => frame.replacen(":1,", ":\"1\",", 1),                  // retype
        4 => frame.replacen('{', "{\"v\":1,", 1),                  // duplicate key
        _ => frame.repeat(1 + at % 3),                             // concatenate
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 500_000, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn arbitrary_text_never_panics(s in any::<String>()) {
        let _ = decode(&s);
    }

    #[test]
    fn mutated_valid_frames_never_panic(pick in 0usize..11, kind in any::<u8>(), at in any::<usize>(), junk in ".{0,8}") {
        let frames = valid_frames();
        let f = mutate(&frames[pick % frames.len()], kind, at, &junk);
        let r = decode(&f);
        if f.len() > MAX_FRAME {
            prop_assert!(r.is_err());
        }
    }
}
