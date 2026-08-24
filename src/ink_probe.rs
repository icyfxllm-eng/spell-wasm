//! CC-CJK-INK F1 + F2 — the ink pad, and the gate that measured it.
//!
//! ONE pad surface, two modes. A second canvas and a second set of pointer
//! handlers would be the same mistake as a second recogniser path, only further
//! from the recogniser -- so the probe and a real practice round share the
//! markup, the strokes, the undo and the commit, and differ in what happens
//! after the read.
//!
//! F0 measured Vision against a proxy: printed glyphs thinned and wobbled
//! toward a pen trace, because no handwriting CJK font is installed. It scored
//! 9/10 on Chinese and 8/10 on Japanese, and that number is worth exactly what
//! a font is worth as a stand-in for a hand, which is not much. Failure there
//! would have been decisive; success was only suggestive.
//!
//! So this screen replaces the proxy with a finger. It prompts a character,
//! takes what is drawn, runs the SAME on-device recognizer the feature would
//! use, and records whether the expected character came back. After the set it
//! reports per language.
//!
//! It scores itself on purpose. The alternative — export the drawings, run the
//! probe by hand, tally the output — puts three manual steps between the ink
//! and the number, and every one of them is a place for the answer to get
//! massaged. Here the only human input is the drawing.
//!
//! **There is no pass threshold, deliberately.** What counts as good enough is
//! a product call, and inventing a bar before seeing real data is how a gate
//! becomes theatre. The gate is that the number exists and Eric has seen it.

use std::cell::RefCell;

use wasm_bindgen_futures::spawn_local;

use crate::{dom, App};

/// The set. Simple and dense in both languages, because F0 says complexity is
/// the ceiling — 語 failed there in BOTH conditions while 山 and 川 sailed
/// through. A set of easy characters would flatter the recognizer and tell us
/// nothing about the words the bank actually holds.
const SET: [(&str, &str); 50] = [
    // zh — ascending stroke count
    ("zh", "八"), ("zh", "人"), ("zh", "大"), ("zh", "小"), ("zh", "山"),
    ("zh", "口"), ("zh", "日"), ("zh", "月"), ("zh", "白"), ("zh", "六"),
    ("zh", "百"), ("zh", "杯"), ("zh", "果"), ("zh", "树"), ("zh", "帮"),
    ("zh", "宝"), ("zh", "爱"), ("zh", "搬"), ("zh", "谢"), ("zh", "餐"),
    ("zh", "楼"), ("zh", "熊"), ("zh", "颜"), ("zh", "警"), ("zh", "蓝"),
    // ja — kana first (D5), then kanji
    ("ja", "あ"), ("ja", "い"), ("ja", "う"), ("ja", "え"), ("ja", "お"),
    ("ja", "か"), ("ja", "き"), ("ja", "ア"), ("ja", "イ"), ("ja", "ウ"),
    ("ja", "日"), ("ja", "本"), ("ja", "山"), ("ja", "川"), ("ja", "水"),
    ("ja", "火"), ("ja", "花"), ("ja", "犬"), ("ja", "猫"), ("ja", "空"),
    ("ja", "語"), ("ja", "電"), ("ja", "曜"), ("ja", "験"), ("ja", "議"),
];

struct Run {
    idx: usize,
    /// (language, expected, what came back, hit)
    rows: Vec<(String, String, String, bool)>,
}

/// What the pad is being used for. There is ONE pad surface and one set of
/// handlers; a second canvas and a second wiring would be the same mistake as a
/// second recognizer path, just further from the recogniser.
#[derive(Clone, PartialEq)]
pub enum Mode {
    /// F1's gate: a fixed set, self-scored.
    Probe,
    /// F4: write the characters of the word in play, one at a time. PRACTICE,
    /// never a grade -- candidates are shown and a failed read costs a retry,
    /// because at 84% and 72% a judge would mark a working hand wrong once in
    /// five.
    Practice { lang: String, chars: Vec<String>, at: usize },
}

thread_local! {
    static RUN: RefCell<Option<Run>> = const { RefCell::new(None) };
    static MODE: RefCell<Mode> = const { RefCell::new(Mode::Probe) };
}

/// Invariant 5 — the pad is for CJK and nothing else.
///
/// A pure function rather than an inline condition so it can be TESTED. The
/// carve-out in D3 exists because for CJK the character IS the spelling, which
/// is the opposite of CC-WORDPICTURE D8's rule that saying a word is not
/// spelling it. That carve-out has to stay exactly as wide as CJK: a drawn
/// answer accepted for Spanish would turn "write the word" into a way to skip
/// spelling it.
pub fn ink_allowed(lang: &str) -> bool {
    lang == crate::consts::ZH || lang == crate::consts::JA
}

/// True while the pad is being used for a single CJK character, which the
/// renderer needs to know: a character wants the square frame F0 validated,
/// a Latin word wants the wide one.
pub fn wants_square() -> bool {
    !matches!(mode(), Mode::Practice { ref lang, .. } if !ink_allowed(lang))
}

fn mode() -> Mode {
    MODE.with(|m| m.borrow().clone())
}

/// Open the pad on one character, as practice. The caller owns what happens
/// next: this reports what the recogniser saw and scores nothing.
pub fn open_practice(app: &App, lang: &str, word: &str) {
    let _ = app;
    // One character at a time. A two-character word is two things to write, and
    // asking for both on one pad would make the recogniser read a phrase, which
    // is not what F1 measured.
    let chars: Vec<String> = word
        .chars()
        .filter(|c| !c.is_ascii() && !c.is_whitespace())
        .map(|c| c.to_string())
        .collect();
    if chars.is_empty() {
        return;
    }
    MODE.with(|m| {
        *m.borrow_mut() = Mode::Practice { lang: lang.to_string(), chars, at: 0 }
    });
    RUN.with(|r| *r.borrow_mut() = None);
    dom::add_class("inkPad", "show");
    crate::drawing::size_canvas();
    crate::drawing::clear_canvas();
    practice_prompt();
}

/// Show the character being written, and where we are in the word.
fn practice_prompt() {
    let Mode::Practice { chars, at, .. } = mode() else { return };
    if at >= chars.len() {
        dom::set_text("inkPrompt", "\u{2713}");
        dom::set_text("inkProgress", "done");
        dom::set_html("inkCands", "");
        dom::set_text("inkResult", "nothing recorded \u{2014} this was practice");
        return;
    }
    dom::set_text("inkPrompt", &chars[at]);
    dom::set_text(
        "inkProgress",
        &if chars.len() > 1 { format!("{} of {}", at + 1, chars.len()) } else { "write it".into() },
    );
    dom::set_html("inkCands", "");
    dom::set_text("inkResult", "");
}

/// The player says which character they meant. THE ONLY verdict a drawn round
/// produces, and it is theirs -- nothing here writes to stats, the misses queue
/// or the tone drill (F4, Invariant: practice that punishes a working hand is
/// worse than no practice).
fn practice_pick(chose_expected: bool) {
    MODE.with(|m| {
        if let Mode::Practice { at, .. } = &mut *m.borrow_mut() {
            *at += 1;
        }
    });
    let _ = chose_expected;
    crate::drawing::clear_canvas();
    practice_prompt();
}

/// F3 — put the candidates on screen as things to TAP.
///
/// The recogniser read the ink; only the player knows what they meant to write.
/// Picking for them turns a wrong answer into a mystery, and F1 showed correct
/// reads landing at 0.30 confidence often enough that a top-1 rule would throw
/// away right answers. "None of these" is a first-class option, not a fallback:
/// at 84% and 72% it is the honest response roughly one time in five.
fn show_candidates(cands: &[(String, f64)]) {
    if cands.is_empty() {
        dom::set_html(
            "inkCands",
            "<button type=\"button\" class=\"ink-cand none\" id=\"inkRetry\">Try again</button>",
        );
    } else {
        let mut html = String::new();
        for (i, (t, c)) in cands.iter().take(3).enumerate() {
            html.push_str(&format!(
                "<button type=\"button\" class=\"ink-cand\" data-cand=\"{}\">{}<small>{:.0}%</small></button>",
                i,
                dom::escape_html(t),
                c * 100.0
            ));
        }
        html.push_str(
            "<button type=\"button\" class=\"ink-cand none\" id=\"inkRetry\">None of these</button>",
        );
        dom::set_html("inkCands", &html);
    }
    let _ = dom::on_click_once("inkRetry", || {
        // "None of these" is a retry, never a miss. The read failed, not the
        // hand -- and at F1's rates that is the honest answer one time in five.
        crate::drawing::clear_canvas();
        dom::set_html("inkCands", "");
        dom::set_text("inkResult", "");
    });
    if matches!(mode(), Mode::Practice { .. }) {
        let expect = match mode() {
            Mode::Practice { chars, at, .. } => chars.get(at).cloned().unwrap_or_default(),
            _ => String::new(),
        };
        for (i, (t, _)) in cands.iter().take(3).enumerate() {
            let picked_right = *t == expect || t.contains(&expect);
            let _ = dom::on_click_once_sel(
                &format!("[data-cand=\"{i}\"]"),
                move || practice_pick(picked_right),
            );
        }
    }
}

fn prompt() {
    RUN.with(|r| {
        let b = r.borrow();
        let Some(run) = b.as_ref() else { return };
        if run.idx >= SET.len() {
            drop(b);
            report();
            return;
        }
        let (lang, ch) = SET[run.idx];
        dom::set_text("inkPrompt", ch);
        dom::set_text(
            "inkProgress",
            &format!("{} · {} of {}", lang, run.idx + 1, SET.len()),
        );
        dom::set_text("inkResult", "");
        dom::set_html("inkCands", "");
    });
}

/// Take what is on the pad, recognize it, score it, move on.
fn commit(app: &App) {
    let _ = app;
    if let Mode::Practice { lang, chars, at } = mode() {
        let Some(expect) = chars.get(at).cloned() else { return };
        spawn_local(async move {
            let cands = crate::drawing::recognize(&lang).await;
            let found = cands.iter().any(|(t, _)| t.contains(&expect));
            let shown = cands
                .iter()
                .take(3)
                .map(|(t, c)| format!("{t} {c:.2}"))
                .collect::<Vec<_>>()
                .join("  ");
            // A miss says the READ failed, not that the player did. Nothing is
            // recorded either way -- see F4: a correctly drawn character the
            // recogniser does not return must leave the record untouched.
            let _ = (found, shown);
            show_candidates(&cands);
            dom::set_text("inkResult", "");
        });
        return;
    }
    let (lang, want) = RUN.with(|r| {
        let b = r.borrow();
        b.as_ref().filter(|run| run.idx < SET.len()).map(|run| SET[run.idx])
    })
    .unwrap_or(("zh", ""));
    if want.is_empty() {
        return;
    }
    spawn_local(async move {
        let cands = crate::drawing::recognize(lang).await;
        // A HIT is the expected character appearing among the candidates at
        // all, not only as the top one. The pad will show candidates to the
        // player (F3), so "it was offered" is the honest measure of whether
        // the recogniser found it.
        let hit = cands.iter().any(|(t, _)| t.contains(want));
        let got = cands
            .iter()
            .take(3)
            .map(|(t, c)| format!("{t} {c:.2}"))
            .collect::<Vec<_>>()
            .join("  ");
        RUN.with(|r| {
            if let Some(run) = r.borrow_mut().as_mut() {
                run.rows.push((
                    lang.to_string(),
                    want.to_string(),
                    got.clone(),
                    hit,
                ));
                run.idx += 1;
            }
        });
        dom::set_text(
            "inkResult",
            &if got.is_empty() {
                "nothing legible".to_string()
            } else {
                format!("{} {}", if hit { "HIT" } else { "miss" }, got)
            },
        );
        crate::drawing::clear_canvas();
        prompt();
    });
}

fn report() {
    let (zh_hit, zh_n, ja_hit, ja_n, lines) = RUN.with(|r| {
        let b = r.borrow();
        let Some(run) = b.as_ref() else { return (0, 0, 0, 0, String::new()) };
        let mut zh = (0, 0);
        let mut ja = (0, 0);
        let mut lines = String::new();
        for (lang, want, got, hit) in &run.rows {
            let t = if lang == "zh" { &mut zh } else { &mut ja };
            t.1 += 1;
            if *hit {
                t.0 += 1;
            } else {
                lines.push_str(&format!("{want} → {}\n", if got.is_empty() { "—" } else { got }));
            }
        }
        (zh.0, zh.1, ja.0, ja.1, lines)
    });
    dom::set_text("inkPrompt", "✓");
    dom::set_text("inkProgress", "done");
    dom::set_html(
        "inkResult",
        &format!(
            "<b>zh {zh_hit}/{zh_n} · ja {ja_hit}/{ja_n}</b><br><small>misses:<br>{}</small>",
            dom::escape_html(&lines).replace('\n', "<br>")
        ),
    );
}

pub fn open(app: &App) {
    MODE.with(|m| *m.borrow_mut() = Mode::Probe);
    RUN.with(|r| *r.borrow_mut() = Some(Run { idx: 0, rows: Vec::new() }));
    dom::add_class("inkPad", "show");
    crate::drawing::size_canvas();
    crate::drawing::clear_canvas();
    let _ = app;
    prompt();
}

fn close() {
    dom::remove_class("inkPad", "show");
}

pub fn wire(app: &App) {
    let a = app.clone();
    dom::on_click("inkCommit", move || commit(&a));
    dom::on_click("inkUndo", crate::drawing::undo_stroke);
    dom::on_click("inkClear", crate::drawing::clear_canvas);
    dom::on_click("inkClose", close);
    // Skip records a miss rather than silently dropping the character: a set of
    // fifty that quietly became forty would report a better number than the
    // hand earned.
    let a = app.clone();
    dom::on_click("inkSkip", move || {
        let _ = &a;
        if mode() != Mode::Probe {
            return;
        }
        RUN.with(|r| {
            if let Some(run) = r.borrow_mut().as_mut() {
                if run.idx < SET.len() {
                    let (lang, want) = SET[run.idx];
                    run.rows.push((lang.into(), want.into(), "skipped".into(), false));
                    run.idx += 1;
                }
            }
        });
        crate::drawing::clear_canvas();
        prompt();
    });
    dom::on::<web_sys::PointerEvent, _>("inkCanvasWrap", "pointerdown", |e| {
        crate::drawing::start_stroke(&e)
    });
    dom::on::<web_sys::PointerEvent, _>("inkCanvasWrap", "pointermove", |e| {
        crate::drawing::move_stroke(&e)
    });
    dom::on::<web_sys::PointerEvent, _>("inkCanvasWrap", "pointerup", |e| {
        crate::drawing::end_stroke(&e)
    });
}

#[cfg(test)]
mod f4_tests {
    use super::*;

    /// Done 5's deliberate case, and the reason the amendment exists: a
    /// correctly drawn character the recogniser does not return must leave the
    /// player's record untouched. F1 measured that at roughly one time in five,
    /// so a drawn round that scored would mark a working hand wrong constantly.
    ///
    /// This is a SYMBOL scan rather than a behavioural test, because the pad
    /// runs on wasm and the queues it must not touch live behind DOM state.
    /// Naming the forbidden calls is what can be checked here, and it is the
    /// same shape of guard the zh grading law uses.
    #[test]
    fn a_drawn_round_writes_to_no_record() {
        let src = include_str!("ink_probe.rs");
        // Everything below the test module is this test naming the very symbols
        // it forbids, which is the ban working rather than breaking.
        let shipped = src.split("#[cfg(test)]").next().unwrap();
        for forbidden in [
            "misses::add_miss",
            "tone_drill::add",
            "stats::record",
            "wordstats::record",
            "record_miss_routed",
            "note_attempt",
        ] {
            assert!(
                !shipped.contains(forbidden),
                "the ink pad calls {forbidden} — a drawn round is practice and \
                 must not write to any record (F4)"
            );
        }
    }

    /// The pad must not become a second way to answer a typed round: nothing in
    /// it touches the answer buffer or the submission path.
    #[test]
    fn a_drawn_answer_cannot_satisfy_a_typed_round() {
        let src = include_str!("ink_probe.rs");
        let shipped = src.split("#[cfg(test)]").next().unwrap();
        for forbidden in ["submit_guess", "s.answer", "type_char", "emit_key"] {
            assert!(
                !shipped.contains(forbidden),
                "the ink pad reaches {forbidden} — drawing is a separate study, \
                 not a shortcut through the typed round (F4)"
            );
        }
    }

    /// Practice walks the word one character at a time and ends; it never loops
    /// back into scoring.
    #[test]
    fn practice_advances_and_stops() {
        let chars: Vec<String> = "帮助".chars().map(|c| c.to_string()).collect();
        assert_eq!(chars.len(), 2);
        let m = Mode::Practice { lang: "zh".into(), chars: chars.clone(), at: 0 };
        let Mode::Practice { at, chars: cs, .. } = m else { panic!() };
        assert_eq!(cs[at], "帮");
        // past the end is the done state, not a panic and not a wrap
        assert!(cs.get(2).is_none());
    }
}

#[cfg(test)]
mod done7_tests {
    use super::*;

    /// Done 7, second half: a drawn answer accepted for Spanish must fail CI.
    ///
    /// The carve-out is deliberate and narrow. CC-WORDPICTURE D8 forbids a
    /// spoken word as a spelling answer, because saying it is not spelling it.
    /// Drawing is the opposite case ONLY for CJK, where the character is the
    /// spelling — so the exception has to stay exactly that wide.
    #[test]
    fn a_drawn_answer_is_never_offered_for_a_non_cjk_language() {
        for lang in ["es", "en", "fr", "de", "pt", "pl", "ru", "ar", "hi", "sw", "vi", "ko", "fil"] {
            assert!(
                !ink_allowed(lang),
                "the ink pad is offered for {lang} — drawing is a CJK carve-out \
                 and nowhere else (Invariant 5)"
            );
        }
        assert!(ink_allowed(crate::consts::ZH));
        assert!(ink_allowed(crate::consts::JA));
    }

    /// The guard has to be the thing the offer actually consults. A test that
    /// only exercises the function while the caller re-implements the condition
    /// inline would pass while the rule leaked.
    #[test]
    fn the_offer_consults_the_guard() {
        let game = include_str!("game.rs");
        assert!(
            game.contains("ink_probe::ink_allowed"),
            "game.rs decides the ink offer without the guard — the rule and the \
             code that enforces it have drifted apart"
        );
    }

    /// Korean is the case worth naming: it is CJK-adjacent and has its own
    /// script, but hangul is composed from jamo the keyboard already provides,
    /// so there is nothing here that typing does not already teach.
    #[test]
    fn korean_is_excluded_on_purpose() {
        assert!(!ink_allowed(crate::consts::KO));
    }
}
