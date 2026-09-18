//! CC-MYWORDS-LISTS F1 — the save sheet's destination control.
//!
//! One control, shared by both save doors (Snap a word list and the paste
//! sheet), so neither can disagree about where a save lands. There is no
//! destructive option here and never will be: saving only ever adds (I1).
//!
//! D1 (signed): the default follows the day. A list created today is the
//! default destination, so a worksheet photographed in two pages lands in one
//! list; otherwise the default is a new list named for today.

use crate::i18n;
use crate::word_lists::{self, Destination, ListSource};
use crate::{dom, i18n::t};

/// The option value that means "a new dated list".
const NEW: &str = "__new";

fn now_ms() -> f64 {
    js_sys::Date::now()
}

/// Today's label, in the UI language, derived once (F2.3).
pub fn today_label() -> String {
    word_lists::today_label(now_ms(), &i18n::current())
}

/// Fill a sheet's destination select and label its save button to match.
pub fn populate(select_id: &str, save_btn_id: &str) {
    if !dom::exists(select_id) {
        return;
    }
    let lists = word_lists::load();
    let today = today_label();
    let default = word_lists::default_destination(&lists, &today);
    let mut html = format!(
        "<option value=\"{NEW}\">{}</option>",
        dom::escape_html(&i18n::tp("lists.destNew", &[("date", &today)])),
    );
    for l in word_lists::visible(&lists) {
        html.push_str(&format!(
            "<option value=\"{}\">{}</option>",
            dom::escape_html(&l.id),
            dom::escape_html(&i18n::tp("lists.addTo", &[("name", &l.name)])),
        ));
    }
    dom::set_html(select_id, &html);
    let value = match &default {
        Destination::New => NEW.to_string(),
        Destination::Add(id) => id.clone(),
    };
    dom::select(select_id).set_value(&value);
    relabel(select_id, save_btn_id);
}

/// F1.3: the button says what the choice does.
pub fn relabel(select_id: &str, save_btn_id: &str) {
    if !dom::exists(select_id) || !dom::exists(save_btn_id) {
        return;
    }
    let label = match chosen(select_id) {
        Destination::New => t("lists.saveNew"),
        Destination::Add(id) => {
            let lists = word_lists::load();
            match lists.lists.iter().find(|l| l.id == id) {
                Some(l) => i18n::tp("lists.addTo", &[("name", &l.name)]),
                None => t("lists.saveNew"),
            }
        }
    };
    dom::set_text(save_btn_id, &label);
}

/// What the sheet is pointing at right now.
pub fn chosen(select_id: &str) -> Destination {
    let v = dom::select(select_id).value();
    if v.is_empty() || v == NEW {
        Destination::New
    } else {
        Destination::Add(v)
    }
}

/// Record the save. Returns the list's name and how many words were new to it,
/// so the sheet can say where the words went. Never removes anything.
pub fn commit(dest: Destination, words: &[(String, String)], source: ListSource) -> (String, usize) {
    let now = now_ms();
    let mut lists = word_lists::load();
    let id = match dest {
        Destination::Add(id) if lists.lists.iter().any(|l| l.id == id && l.live()) => id,
        // A list deleted while the sheet was open falls back to a new one rather
        // than dropping the save.
        _ => word_lists::create(&mut lists, &today_label(), source, now),
    };
    let added = word_lists::add_entries(&mut lists, &id, words, now);
    let name = lists.lists.iter().find(|l| l.id == id).map(|l| l.name.clone()).unwrap_or_default();
    word_lists::store(&lists);
    offer_open(&id);
    (name, added)
}

thread_local! {
    /// F1.4: the list the last save landed in, so the confirmation can offer to
    /// open it. Cleared once taken.
    static LAST_SAVED: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// F1.4: show the Open action for the list a save just landed in.
pub fn offer_open(id: &str) {
    LAST_SAVED.with(|c| *c.borrow_mut() = Some(id.to_string()));
    dom::remove_class("openSavedList", "btn-hide");
}

pub fn hide_open() {
    LAST_SAVED.with(|c| *c.borrow_mut() = None);
    dom::add_class("openSavedList", "btn-hide");
}

pub fn last_saved() -> Option<String> {
    LAST_SAVED.with(|c| c.borrow().clone())
}

/// The line a sheet shows after saving: how many words, and which list.
pub fn saved_note(name: &str, n: usize) -> String {
    i18n::tp("lists.savedTo", &[("n", &n.to_string()), ("name", name)])
}

/// Keep both sheets' buttons honest as the player changes the destination.
pub fn wire() {
    for (select_id, btn_id) in [("photoDest", "photoConfirm"), ("importDest", "saveWords")] {
        dom::on::<web_sys::Event, _>(select_id, "change", move |_| relabel(select_id, btn_id));
    }
}
