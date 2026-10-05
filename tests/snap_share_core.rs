//! CC-SNAP-SHARE 3.3 — the share payload's invariants.
//!
//! Level 3's own done-check is "QR round-trip reproduces the list
//! byte-for-byte on a second device". The device half needs a camera and a
//! renderer; the byte-for-byte half is here, and it is the half that can be
//! wrong silently.

use proptest::prelude::*;
use spell_wasm::snap_share::{crc32, decode, encode, encoded_len, DecodeError, EncodeError, SharePayload, VERSION};

fn p(lang: &str, day: Option<u32>, words: &[&str]) -> SharePayload {
    SharePayload { lang: lang.into(), test_day: day, words: words.iter().map(|w| w.to_string()).collect() }
}

/// The hand-rolled CRC-32 must BE CRC-32, not merely be stable. A checksum
/// that is self-consistent passes every round-trip test in this file and
/// then fails the first time anything else has to verify one of these
/// codes. The standard check value settles it.
#[test]
fn the_checksum_is_really_crc32() {
    assert_eq!(crc32(b"123456789"), 0xCBF4_3926, "not CRC-32/IEEE");
    assert_eq!(crc32(b""), 0, "the empty string's CRC-32 is 0");
    assert_eq!(crc32(b"a"), 0xE8B7_BE43);
}

#[test]
fn a_list_round_trips() {
    let orig = p("en", Some(19_450), &["because", "friend", "thought"]);
    let bytes = encode(&orig).unwrap();
    assert_eq!(decode(&bytes).unwrap(), orig);
}

#[test]
fn a_list_without_a_date_round_trips() {
    let orig = p("ru", None, &["кот", "собака"]);
    assert_eq!(decode(&encode(&orig).unwrap()).unwrap(), orig);
}

/// D7, signed: words only. No photo, no identity, no progress. Held by
/// arithmetic rather than by reading the struct: the encoded length is
/// exactly its declared parts, so a fourth field cannot ride along unseen.
#[test]
fn d7_payload_carries_nothing_else() {
    let pay = p("en", Some(19_450), &["cat", "dog", "fox"]);
    let bytes = encode(&pay).unwrap();
    let expected = 1                                   // version
        + pay.lang.len()                               // lang
        + 1                                            // SEP
        + "19450".len()                                // the date
        + pay.words.iter().map(|w| 1 + w.len()).sum::<usize>()  // SEP + word each
        + 4;                                           // checksum
    assert_eq!(bytes.len(), expected,
        "the payload carries {} bytes more than its declared parts", bytes.len() as i64 - expected as i64);
}

#[test]
fn an_empty_list_is_not_shareable() {
    assert_eq!(encode(&p("en", None, &[])), Err(EncodeError::NoWords));
}

#[test]
fn a_separator_in_a_word_is_refused_not_swallowed() {
    let bad = SharePayload { lang: "en".into(), test_day: None, words: vec!["ca\u{1f}t".into()] };
    assert_eq!(encode(&bad), Err(EncodeError::SeparatorInText),
        "silently splitting here would land a different list on the other phone");
}

#[test]
fn a_future_version_is_refused_not_guessed() {
    let mut bytes = encode(&p("en", None, &["cat"])).unwrap();
    bytes[0] = VERSION + 1;
    assert_eq!(decode(&bytes), Err(DecodeError::UnknownVersion(VERSION + 1)));
}

#[test]
fn truncation_and_emptiness_are_caught() {
    assert_eq!(decode(&[]), Err(DecodeError::Empty));
    assert_eq!(decode(&[VERSION, 0x61]), Err(DecodeError::Truncated));
}

#[test]
fn a_damaged_code_is_rejected_rather_than_imported() {
    let good = encode(&p("en", Some(5), &["because", "friend"])).unwrap();
    let mut bad = good.clone();
    let i = bad.len() / 2;
    bad[i] ^= 0x01; // one bit, as a mis-scan would
    assert_eq!(decode(&bad), Err(DecodeError::Checksum));
}

#[test]
fn encoding_is_deterministic() {
    let pay = p("de", Some(42), &["Haus", "Straße", "Mädchen"]);
    let first = encode(&pay).unwrap();
    for _ in 0..20 {
        assert_eq!(encode(&pay).unwrap(), first);
    }
}

/// The number 3.3 will be asked about: does a real list fit a code someone
/// can scan? Measured, not asserted against a capacity I would be inventing
/// -- but a regression that doubled the size should be visible.
#[test]
fn a_twenty_word_list_is_reported_at_a_sane_size() {
    let en: Vec<&str> = vec!["because", "friend", "thought", "through", "enough", "people",
        "beautiful", "different", "important", "together", "another", "between", "children",
        "something", "sometimes", "question", "remember", "surprise", "tomorrow", "yesterday"];
    let ru: Vec<&str> = vec!["кот", "собака", "дом", "школа", "книга", "город", "вода", "друг",
        "мама", "папа", "ночь", "день", "рука", "нога", "хлеб", "молоко", "дерево", "окно",
        "дверь", "стол"];
    let zh: Vec<&str> = vec!["ba1", "ban1", "bu4", "cha2", "ai1", "an4", "chi1", "dao4", "fan4",
        "gou3", "hao3", "jia1", "kan4", "lai2", "mao1", "ni3", "pao3", "qu4", "ren2", "shui3"];
    for (name, ws) in [("en", en), ("ru", ru), ("zh", zh)] {
        let pay = p(name, Some(19_450), &ws);
        let n = encoded_len(&pay).unwrap();
        println!("  {name}: 20 words -> {n} bytes");
        assert!(n < 600, "{name} 20-word list is {n} bytes, which no longer fits a modest QR");
        assert_eq!(decode(&encode(&pay).unwrap()).unwrap(), pay);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5_000))]

    /// Level 3's done-check, the half that does not need two devices:
    /// whatever goes in comes back identical.
    #[test]
    fn round_trip_is_exact(
        lang in "[a-z]{2}",
        day in prop::option::of(0u32..100_000),
        words in prop::collection::vec("[\\p{L}\\p{N}'-]{1,18}", 1..25),
    ) {
        let pay = SharePayload { lang, test_day: day, words };
        let bytes = encode(&pay).unwrap();
        prop_assert_eq!(decode(&bytes).unwrap(), pay);
    }

    /// Any single-byte corruption anywhere is caught. A scanner that
    /// imported a half-read code would hand a child the wrong spellings.
    #[test]
    fn any_single_byte_corruption_is_caught(
        words in prop::collection::vec("[a-z]{2,10}", 1..8),
        at in 0usize..40,
        xor in 1u8..=255,
    ) {
        let pay = SharePayload { lang: "en".into(), test_day: Some(7), words };
        let good = encode(&pay).unwrap();
        let i = at % good.len();
        let mut bad = good.clone();
        bad[i] ^= xor;
        if bad == good { return Ok(()); }
        prop_assert!(decode(&bad).is_err(), "a corrupted byte at {} decoded anyway", i);
    }
}
