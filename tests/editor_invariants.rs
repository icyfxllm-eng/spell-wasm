//! Property tests for the grapheme-cluster editor (Feature 1), INV-1..INV-8.
//!
//! Strategy: drive random operation sequences (insert / backspace / set_cursor)
//! against BOTH the real `EditorBuffer` and an obviously-correct reference model
//! (a plain `Vec<String>` of graphemes + cursor with the same NFC-at-boundary
//! policy), asserting the invariants after every operation. Insert inputs are
//! drawn from a corpus of COMPLETE grapheme clusters — random ASCII plus the
//! per-language fixtures — so the tests exercise real multi-codepoint clusters
//! (Vietnamese ế, Korean 학, Thai กำ, ZWJ emoji) rather than orphan combining
//! marks, which the frontend never emits.

use proptest::prelude::*;
use spell_wasm::editor::EditorBuffer;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

/// Reference model: the "obviously correct" version the buffer must match.
#[derive(Clone, Default)]
struct Model {
    g: Vec<String>,
    cursor: usize,
    cap: Option<usize>,
    inserted: std::collections::HashSet<String>, // for INV-6
}
impl Model {
    fn insert(&mut self, s: &str) {
        let nfc: String = s.nfc().collect();
        for cl in nfc.graphemes(true) {
            if self.cap.is_some_and(|c| self.g.len() >= c) {
                return;
            }
            self.inserted.insert(cl.to_string());
            self.g.insert(self.cursor, cl.to_string());
            self.cursor += 1;
        }
    }
    fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.g.remove(self.cursor);
        }
    }
    fn set_cursor(&mut self, i: usize) {
        self.cursor = i.min(self.g.len());
    }
    fn text(&self) -> String {
        self.g.concat()
    }
}

/// Complete grapheme clusters an insert may carry (the frontend only ever emits
/// whole graphemes). Includes the spec's per-language fixtures + a ZWJ emoji +
/// a decomposed Vietnamese sequence (must fold to one NFC cluster).
const CLUSTERS: &[&str] = &[
    "a", "b", "c", "z", "s", "t", "r", "i", // en / "straight"
    "trường", "đặc", "biệt", "ế", "e\u{0302}\u{0301}", // vi (incl. decomposed ế)
    "학", "교", "없", "었", "다", // ko (incl. double-final 없)
    "し", "ん", "か", "漢", "字", // ja (kana + kanji)
    "你", "好", "吗", // zh
    "โรงเรียน", "น้ำ", "เรียน", // th (whole words — no orphan combining marks)
    "👍", "👨\u{200d}👩\u{200d}👧\u{200d}👦", // emoji + ZWJ family
];

#[derive(Clone, Debug)]
enum Op {
    Insert(String),
    Backspace,
    SetCursor(usize),
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0..CLUSTERS.len()).prop_map(|i| Op::Insert(CLUSTERS[i].to_string())),
        Just(Op::Backspace),
        (0usize..12).prop_map(Op::SetCursor),
    ]
}

fn apply(buf: &mut EditorBuffer, model: &mut Model, op: &Op) {
    match op {
        Op::Insert(s) => {
            buf.insert(s);
            model.insert(s);
        }
        Op::Backspace => {
            buf.backspace();
            model.backspace();
        }
        Op::SetCursor(i) => {
            buf.set_cursor(*i);
            model.set_cursor(*i);
        }
    }
}

fn check_invariants(buf: &EditorBuffer, model: &Model) {
    // INV-1: model equivalence (text + cursor).
    assert_eq!(buf.text(), model.text(), "INV-1 text");
    assert_eq!(buf.cursor(), model.cursor, "INV-1 cursor");
    // INV-2: cursor bounds.
    assert!(buf.cursor() <= buf.grapheme_count(), "INV-2 bounds");
    // INV-3: grapheme integrity — re-segmenting text() yields the same count.
    assert_eq!(
        buf.text().graphemes(true).count(),
        buf.grapheme_count(),
        "INV-3 segmentation stable for {:?}",
        buf.text()
    );
    // INV-6: every grapheme in the buffer was inserted (no fragments).
    for i in 0..buf.grapheme_count() {
        let g = buf.grapheme_at(i).unwrap();
        assert!(model.inserted.contains(g), "INV-6 fragment {:?}", g);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    // INV-1/2/3/6 over random operation sequences (no cap).
    #[test]
    fn model_equivalence(ops in proptest::collection::vec(op_strategy(), 0..40)) {
        let mut buf = EditorBuffer::new();
        let mut model = Model::default();
        for op in &ops {
            apply(&mut buf, &mut model, op);
            check_invariants(&buf, &model);
        }
    }

    // INV-4: backspace at 0 changes nothing.
    #[test]
    fn backspace_at_zero_noop(seed in 0..CLUSTERS.len()) {
        let mut buf = EditorBuffer::from_text(CLUSTERS[seed]);
        buf.set_cursor(0);
        let before = buf.text();
        buf.backspace();
        prop_assert_eq!(buf.text(), before);
        prop_assert_eq!(buf.cursor(), 0);
    }

    // INV-5: insert(g) then backspace() restores exact prior text+cursor.
    #[test]
    fn insert_backspace_inverse(
        ops in proptest::collection::vec(op_strategy(), 0..20),
        g in (0..CLUSTERS.len()),
    ) {
        let mut buf = EditorBuffer::new();
        let mut model = Model::default();
        for op in &ops { apply(&mut buf, &mut model, op); }
        let (t0, c0) = (buf.text(), buf.cursor());
        // insert then backspace one *cluster* (a single-grapheme insert)
        let cl = CLUSTERS[g];
        if cl.nfc().collect::<String>().graphemes(true).count() == 1 {
            buf.insert(cl);
            buf.backspace();
            prop_assert_eq!(buf.text(), t0);
            prop_assert_eq!(buf.cursor(), c0);
        }
    }

    // INV-7: cap is never exceeded and insert-at-cap is a no-op.
    #[test]
    fn length_cap_respected(cap in 1usize..8, ops in proptest::collection::vec(op_strategy(), 0..40)) {
        let mut buf = EditorBuffer::with_cap(cap);
        let mut model = Model::default();
        model.cap = Some(cap);
        for op in &ops {
            apply(&mut buf, &mut model, op);
            prop_assert!(buf.grapheme_count() <= cap, "INV-7 over cap");
            prop_assert_eq!(buf.text(), model.text());
        }
    }

    // INV-8: decomposed (NFD) and precomposed (NFC) inserts behave identically.
    #[test]
    fn nfc_nfd_equivalence(prefix in proptest::collection::vec(op_strategy(), 0..10)) {
        let mut a = EditorBuffer::new();
        let mut b = EditorBuffer::new();
        for op in &prefix {
            match op {
                Op::Insert(s) => { a.insert(s); b.insert(s); }
                Op::Backspace => { a.backspace(); b.backspace(); }
                Op::SetCursor(i) => { a.set_cursor(*i); b.set_cursor(*i); }
            }
        }
        a.insert("\u{1ebf}");          // precomposed ế
        b.insert("e\u{0302}\u{0301}"); // decomposed ế
        prop_assert_eq!(a.text(), b.text(), "INV-8");
        prop_assert_eq!(a.grapheme_count(), b.grapheme_count());
    }
}
