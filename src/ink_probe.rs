//! CC-CJK-INK F1 — the real-ink gate.
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

thread_local! {
    static RUN: RefCell<Option<Run>> = const { RefCell::new(None) };
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
    });
}

/// Take what is on the pad, recognize it, score it, move on.
fn commit(app: &App) {
    let _ = app;
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
