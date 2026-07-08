use crate::consts::REVIEW;
use crate::dom::{self, escape_html};
use crate::model::{AppState, BoardEntry, LB_KEY};
use crate::storage;

pub fn name_for(state: &AppState, key: &str) -> String {
    crate::game::name_for(state, key)
}

fn get_board() -> Vec<BoardEntry> {
    storage::get_json(LB_KEY).unwrap_or_default()
}

fn set_board(board: &[BoardEntry]) {
    storage::set_json(LB_KEY, &board);
}

pub fn qualifies(streak: u32) -> bool {
    let board = get_board();
    if board.len() < 10 {
        return true;
    }
    streak > board.iter().map(|e| e.streak).min().unwrap_or(0)
}

pub fn render(state: &AppState) {
    dom::set_text("boardScope", "this device only");
    let board = get_board();
    if board.is_empty() {
        dom::set_html("boardList", "<li class=\"empty\">No chains yet \u{2014} be the first to start one.</li>");
        return;
    }
    let mut sorted = board;
    sorted.sort_by(|a, b| b.streak.cmp(&a.streak));
    let html: String = sorted
        .iter()
        .take(10)
        .enumerate()
        .map(|(i, e)| {
            format!(
                "<li class=\"{}\"><span class=\"rank\">{}</span><span class=\"who\">{}</span><span class=\"meta\">{}{} \u{b7} {}</span><span class=\"score\">{}</span></li>",
                if i == 0 { "top" } else { "" },
                i + 1,
                escape_html(if e.name.is_empty() { "anon" } else { &e.name }),
                if e.timed { "\u{23f1} " } else { "" },
                escape_html(&name_for(state, &e.lang)),
                escape_html(&e.level),
                e.streak
            )
        })
        .collect();
    dom::set_html("boardList", &html);
}

pub fn save_score(state: &mut AppState, name: &str, streak: u32) {
    let name = if name.trim().is_empty() { "anon".to_string() } else { name.trim().chars().take(14).collect() };
    state.saved_name = name.clone();
    let lang = if state.review { REVIEW.to_string() } else { state.lang.clone() };
    let level = if state.review { "review".to_string() } else { state.level.clone() };
    let mut board = get_board();
    board.push(BoardEntry {
        name,
        streak,
        lang,
        level,
        timed: state.timed,
        ts: js_sys::Date::now(),
    });
    board.sort_by(|a, b| b.streak.cmp(&a.streak));
    board.truncate(10);
    set_board(&board);
    render(state);
}
