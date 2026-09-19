//! CC-MYWORDS-LISTS F3/F5 — the My Words screen.
//!
//! Before this, nothing in the app could show a player what they had saved: the
//! only door was a paste sheet, and the only way to start fresh was to erase
//! everything. This screen is the other half of that fix — lists by date, one
//! tap into any of them, and a delete that can be taken back.
//!
//! Every list here is live: soft-deleted lists appear only under Recently
//! deleted, and older lists fold away rather than inviting a delete (D4).

use std::cell::{Cell, RefCell};

use crate::word_lists::{self, ListEntry, Lists};
use crate::{dom, i18n, i18n::t, App};

/// D4 (signed): lists older than this fold into "Older lists".
const FOLD_AFTER_DAYS: f64 = 21.0;
/// F3.1: how many language chips a card shows before "+N".
const CHIP_CAP: usize = 4;
/// F5.2/F5.4: how long an Undo stays on screen.
const UNDO_MS: i32 = 8_000;
/// The All words view is addressed like a list, but is read-only.
const ALL: &str = "__all";

thread_local! {
    static OPEN_LIST: RefCell<Option<String>> = const { RefCell::new(None) };
    static EDITING: Cell<Option<usize>> = const { Cell::new(None) };
    static RENAMING: Cell<bool> = const { Cell::new(false) };
    static CONFIRMING: Cell<bool> = const { Cell::new(false) };
    static SHOW_OLDER: Cell<bool> = const { Cell::new(false) };
    static SHOW_TRASH: Cell<bool> = const { Cell::new(false) };
    static UNDO: RefCell<Option<Undo>> = const { RefCell::new(None) };
    static UNDO_SEQ: Cell<u32> = const { Cell::new(0) };
}

/// What the Undo bar will put back.
enum Undo {
    List(String),
    Entry { list: String, index: usize, entry: ListEntry },
}

fn now() -> f64 {
    js_sys::Date::now()
}

fn esc(s: &str) -> String {
    dom::escape_html(s)
}

fn count_label(n: usize) -> String {
    i18n::tp("lists.count", &[("n", &n.to_string())])
}

// ---------- rendering ----------

fn card(l: &word_lists::WordList) -> String {
    let (chips, more) = word_lists::language_chips(l, CHIP_CAP);
    let mut chip_html = String::new();
    for c in chips {
        chip_html.push_str(&format!("<span class=\"lists-chip\">{}</span>", esc(&c)));
    }
    if more > 0 {
        chip_html.push_str(&format!("<span class=\"lists-chip\">+{more}</span>"));
    }
    format!(
        "<button type=\"button\" class=\"lists-card\" data-l-open=\"{id}\">\
           <span class=\"lc-name\">{name}</span>\
           <span class=\"lc-meta\">{count}</span>\
           <span class=\"lists-chips\">{chips}</span>\
         </button>",
        id = esc(&l.id),
        name = esc(&l.name),
        count = esc(&count_label(l.entries.len())),
        chips = chip_html,
    )
}

fn home_html(lists: &Lists) -> String {
    let (recent, older) = word_lists::folded(lists, now(), FOLD_AFTER_DAYS);
    let trash = word_lists::recently_deleted(lists);
    let mut html = String::new();
    // F3.4: with no live lists the screen says so — even when Recently deleted
    // still holds something, since that is not a list the player can play.
    if recent.is_empty() && older.is_empty() {
        html.push_str(&format!("<div class=\"lists-empty\">{}</div>", esc(&t("lists.emptyTitle"))));
    }
    let all = word_lists::all_words(lists);
    if !all.is_empty() {
        html.push_str(&format!(
            "<button type=\"button\" class=\"lists-card\" data-l-open=\"{ALL}\">\
               <span class=\"lc-name\">{}</span><span class=\"lc-meta\">{}</span></button>",
            esc(&t("lists.allWords")),
            esc(&count_label(all.len())),
        ));
    }
    for l in &recent {
        html.push_str(&card(l));
    }
    if !older.is_empty() {
        html.push_str(&format!(
            "<button type=\"button\" class=\"ghost\" data-l-older>{} ({})</button>",
            esc(&t("lists.older")),
            older.len()
        ));
        if SHOW_OLDER.with(|c| c.get()) {
            for l in &older {
                html.push_str(&card(l));
            }
        }
    }
    // The two doors, always reachable: saving words is the point of the screen.
    html.push_str(&format!(
        "<div class=\"lists-actions\">\
           <button type=\"button\" class=\"btn btn-check\" data-l-snap>{}</button>\
           <button type=\"button\" class=\"ghost\" data-l-add>{}</button>\
         </div>",
        esc(&t("lists.snap")),
        esc(&t("lists.addWords")),
    ));
    if !trash.is_empty() {
        html.push_str(&format!(
            "<button type=\"button\" class=\"ghost\" data-l-trash>{} ({})</button>",
            esc(&t("lists.trash")),
            trash.len()
        ));
        if SHOW_TRASH.with(|c| c.get()) {
            for l in &trash {
                html.push_str(&format!(
                    "<div class=\"lists-card\"><span class=\"lc-name\">{}</span>\
                       <span class=\"lc-meta\">{}</span>\
                       <span class=\"lists-actions\">\
                         <button type=\"button\" class=\"ghost\" data-l-restore=\"{id}\">{}</button>\
                         <button type=\"button\" class=\"ghost danger\" data-l-purge=\"{id}\">{}</button>\
                       </span></div>",
                    esc(&l.name),
                    esc(&count_label(l.entries.len())),
                    esc(&t("lists.restore")),
                    esc(&t("lists.deleteNow")),
                    id = esc(&l.id),
                ));
            }
        }
    }
    html
}

fn entry_row(idx: usize, e: &ListEntry, editable: bool, ordered: bool, last: bool) -> String {
    if !editable {
        return format!(
            "<div class=\"lists-entry\"><span class=\"le-word\">{}</span>\
               <span class=\"lists-chip\">{}</span></div>",
            esc(&e.text),
            esc(&e.lang)
        );
    }
    if EDITING.with(|c| c.get()) == Some(idx) {
        return format!(
            "<div class=\"lists-entry\">\
               <input type=\"text\" id=\"listsEdit\" value=\"{}\" aria-label=\"{}\" />\
               <button type=\"button\" class=\"ghost\" data-e-save=\"{idx}\">{}</button>\
               <button type=\"button\" class=\"ghost\" data-e-cancel>{}</button>\
             </div>",
            esc(&e.text),
            esc(&t("lists.edit")),
            esc(&t("finale.save")),
            esc(&t("parent.cancel")),
        );
    }
    // D3: in the player's own order, each word can be starred to the front or
    // nudged up and down. In Mixed those controls would promise something the
    // adaptive draw does not honour, so they are absent.
    let ordering = if ordered {
        format!(
            "<button type=\"button\" class=\"ghost\" data-e-star=\"{idx}\" aria-pressed=\"{}\">{}</button>\
             <button type=\"button\" class=\"ghost\" data-e-up=\"{idx}\"{}>↑</button>\
             <button type=\"button\" class=\"ghost\" data-e-down=\"{idx}\"{}>↓</button>",
            if e.starred { "true" } else { "false" },
            if e.starred { "\u{2605}" } else { "\u{2606}" },
            if idx == 0 { " disabled" } else { "" },
            if last { " disabled" } else { "" },
        )
    } else {
        String::new()
    };
    format!(
        "<div class=\"lists-entry\"><span class=\"le-word\">{}</span>\
           <span class=\"lists-chip\">{}</span>{ordering}\
           <button type=\"button\" class=\"ghost\" data-e-edit=\"{idx}\">{}</button>\
           <button type=\"button\" class=\"ghost\" data-e-remove=\"{idx}\">{}</button>\
         </div>",
        esc(&e.text),
        esc(&e.lang),
        esc(&t("lists.edit")),
        esc(&t("lists.remove")),
    )
}

fn detail_html(lists: &Lists, id: &str, kid: bool) -> String {
    let back = format!(
        "<div class=\"lists-row\"><button type=\"button\" class=\"ghost\" data-l-back>{}</button></div>",
        esc(&t("fd.back"))
    );
    if id == ALL {
        let all = word_lists::all_words(lists);
        let rows: String =
            all.iter().enumerate().map(|(i, e)| entry_row(i, e, false, false, false)).collect();
        return format!(
            "{back}<div class=\"lists-head\">{} · {}</div>\
             <div class=\"lists-actions\">\
               <button type=\"button\" class=\"btn btn-check\" data-l-play-all>{}</button>\
             </div>{rows}",
            esc(&t("lists.allWords")),
            esc(&count_label(all.len())),
            esc(&t("lists.playAll")),
        );
    }
    let Some(l) = lists.lists.iter().find(|l| l.id == id && l.live()) else {
        return back;
    };
    let head = if RENAMING.with(|c| c.get()) {
        format!(
            "<div class=\"lists-row\">\
               <input type=\"text\" id=\"listsRename\" value=\"{}\" aria-label=\"{}\" />\
               <button type=\"button\" class=\"ghost\" data-l-rename-save>{}</button>\
               <button type=\"button\" class=\"ghost\" data-l-rename-cancel>{}</button>\
             </div>",
            esc(&l.name),
            esc(&t("lists.rename")),
            esc(&t("finale.save")),
            esc(&t("parent.cancel")),
        )
    } else {
        format!(
            "<div class=\"lists-row\"><span class=\"lc-name\">{}</span>\
               <span class=\"lc-meta\">{}</span>\
               <button type=\"button\" class=\"ghost\" data-l-rename style=\"margin-inline-start:auto\">{}</button>\
             </div>",
            esc(&l.name),
            esc(&count_label(l.entries.len())),
            esc(&t("lists.rename")),
        )
    };
    let mine = l.order == word_lists::Order::InMyOrder;
    let rows: String = l
        .entries
        .iter()
        .enumerate()
        .map(|(i, e)| entry_row(i, e, true, mine, i + 1 == l.entries.len()))
        .collect();
    // D3: how this list hands its words to play.
    let order = format!(
        "<div class=\"lists-row\">\
           <button type=\"button\" class=\"ghost{}\" data-l-order=\"mixed\">{}</button>\
           <button type=\"button\" class=\"ghost{}\" data-l-order=\"mine\">{}</button>\
         </div>",
        if mine { "" } else { " on" },
        esc(&t("lists.orderMixed")),
        if mine { " on" } else { "" },
        esc(&t("lists.orderMine")),
    );
    // F5.1/F5.2: delete lives here and nowhere else, and it asks first.
    let danger = if CONFIRMING.with(|c| c.get()) {
        format!(
            "<div class=\"lists-row\"><span>{}</span>\
               <button type=\"button\" class=\"ghost danger\" data-l-del=\"{id}\">{}</button>\
               <button type=\"button\" class=\"ghost\" data-l-del-cancel>{}</button></div>",
            esc(&i18n::tp(
                "lists.confirmDelete",
                &[("name", &l.name), ("n", &l.entries.len().to_string())]
            )),
            esc(&t("bee.delete")),
            esc(&t("parent.cancel")),
            id = esc(&l.id),
        )
    } else {
        format!(
            "<button type=\"button\" class=\"ghost danger\" data-l-del-ask>{}{}</button>",
            esc(&t("lists.deleteList")),
            // F5.5: in Spell Jr the grown-up gate stands in front of it.
            if kid { " \u{1F512}" } else { "" },
        )
    };
    format!(
        "{back}{head}\
         <div class=\"lists-actions\">\
           <button type=\"button\" class=\"btn btn-check\" data-l-play=\"{id}\">{}</button>\
           {puzzle}\
         </div>\
         {order}{rows}\
         <div class=\"lists-actions\">\
           <button type=\"button\" class=\"ghost\" data-l-add=\"{id}\">{}</button>\
           {danger}\
         </div>",
        esc(&t("lists.play")),
        esc(&t("lists.addWords")),
        id = esc(&l.id),
        // CC-WORDGRID F-X1: a dated list can become a Spell Search puzzle.
        puzzle = if crate::flags::spell_search() {
            format!("<button type=\"button\" class=\"ghost\" data-l-puzzle=\"{}\">{}</button>", esc(&l.id), esc(&t("ws.makePuzzle")))
        } else {
            String::new()
        },
    )
}

fn render(app: &App) {
    let lists = word_lists::load();
    let kid = app.borrow().kid;
    let html = match OPEN_LIST.with(|c| c.borrow().clone()) {
        Some(id) => detail_html(&lists, &id, kid),
        None => home_html(&lists),
    };
    dom::set_html("listsBody", &html);
}

// ---------- the undo bar ----------

fn offer_undo(app: &App, text: &str, what: Undo) {
    UNDO.with(|u| *u.borrow_mut() = Some(what));
    dom::set_text("listsUndoText", text);
    let _ = dom::el("listsUndo").remove_attribute("hidden");
    let seq = UNDO_SEQ.with(|c| {
        c.set(c.get().wrapping_add(1));
        c.get()
    });
    let a = app.clone();
    crate::game::schedule(&a, UNDO_MS, move |_| {
        // Only the offer this timer belongs to: a newer one owns the bar now.
        if UNDO_SEQ.with(|c| c.get()) == seq {
            clear_undo();
        }
    });
}

fn clear_undo() {
    UNDO.with(|u| *u.borrow_mut() = None);
    let _ = dom::el("listsUndo").set_attribute("hidden", "");
    dom::set_text("listsUndoText", "");
}

fn take_undo(app: &App) {
    let Some(what) = UNDO.with(|u| u.borrow_mut().take()) else { return };
    let mut lists = word_lists::load();
    match what {
        Undo::List(id) => {
            word_lists::restore(&mut lists, &id);
        }
        Undo::Entry { list, index, entry } => {
            word_lists::insert_entry(&mut lists, &list, index, entry, now());
        }
    }
    word_lists::store(&lists);
    crate::lists_ui::refresh_pool(app);
    clear_undo();
    render(app);
}

// ---------- actions ----------

fn delete_list(app: &App, id: &str) {
    let mut lists = word_lists::load();
    let name = lists.lists.iter().find(|l| l.id == id).map(|l| l.name.clone()).unwrap_or_default();
    if !word_lists::soft_delete(&mut lists, id, now()) {
        return;
    }
    word_lists::store(&lists);
    crate::lists_ui::refresh_pool(app);
    CONFIRMING.with(|c| c.set(false));
    OPEN_LIST.with(|c| *c.borrow_mut() = None);
    render(app);
    offer_undo(app, &i18n::tp("lists.listDeleted", &[("name", &name)]), Undo::List(id.to_string()));
}

pub fn open(app: &App) {
    OPEN_LIST.with(|c| *c.borrow_mut() = None);
    EDITING.with(|c| c.set(None));
    RENAMING.with(|c| c.set(false));
    CONFIRMING.with(|c| c.set(false));
    clear_undo();
    render(app);
    dom::add_class("listsScreen", "show");
}

/// Open straight into one list — the save sheet's "Open" lands here.
pub fn open_list(app: &App, id: &str) {
    open(app);
    OPEN_LIST.with(|c| *c.borrow_mut() = Some(id.to_string()));
    render(app);
}

fn close() {
    dom::remove_class("listsScreen", "show");
    clear_undo();
}

fn attr(el: &web_sys::Element, name: &str) -> Option<String> {
    el.get_attribute(name)
}

pub fn wire(app: &App) {
    {
        let a = app.clone();
        dom::on_click("listsClose", move || {
            let _ = &a;
            close();
        });
    }
    {
        let a = app.clone();
        dom::on_click("listsUndoBtn", move || take_undo(&a));
    }
    let a = app.clone();
    dom::on::<web_sys::Event, _>("listsScreen", "click", move |ev| {
        use wasm_bindgen::JsCast;
        let Some(target) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
        let Some(el) = target
            .closest(
                "[data-l-open],[data-l-back],[data-l-older],[data-l-trash],[data-l-restore],\
                 [data-l-purge],[data-l-rename],[data-l-rename-save],[data-l-rename-cancel],\
                 [data-l-del],[data-l-del-ask],[data-l-del-cancel],[data-l-add],[data-l-snap],\
                 [data-l-play],[data-l-play-all],[data-l-order],[data-l-puzzle],\
                 [data-e-edit],[data-e-save],[data-e-cancel],[data-e-remove],\
                 [data-e-star],[data-e-up],[data-e-down]",
            )
            .ok()
            .flatten()
        else {
            return;
        };

        if let Some(id) = attr(&el, "data-l-puzzle") {
            if crate::wordsearch_ui::open_list(&a, &id) {
                close();
            }
            return;
        } else if let Some(id) = attr(&el, "data-l-play") {
            // F4.1: this list becomes what play serves, and the screen gets out
            // of the way.
            close();
            crate::lists_ui::play(&a, &[id]);
            return;
        } else if attr(&el, "data-l-play-all").is_some() {
            let lists = word_lists::load();
            let every: Vec<String> = word_lists::visible(&lists).iter().map(|l| l.id.clone()).collect();
            close();
            crate::lists_ui::play(&a, &every);
            return;
        } else if let Some(kind) = attr(&el, "data-l-order") {
            if let Some(id) = OPEN_LIST.with(|c| c.borrow().clone()) {
                let order = if kind == "mine" {
                    word_lists::Order::InMyOrder
                } else {
                    word_lists::Order::Mixed
                };
                let mut lists = word_lists::load();
                word_lists::set_order(&mut lists, &id, order, now());
                word_lists::store(&lists);
                crate::lists_ui::refresh_pool(&a);
            }
            render(&a);
        } else if let Some(i) = attr(&el, "data-e-star").and_then(|v| v.parse::<usize>().ok()) {
            if let Some(id) = OPEN_LIST.with(|c| c.borrow().clone()) {
                let mut lists = word_lists::load();
                let on = lists
                    .lists
                    .iter()
                    .find(|l| l.id == id)
                    .and_then(|l| l.entries.get(i))
                    .map(|e| !e.starred)
                    .unwrap_or(false);
                word_lists::set_star(&mut lists, &id, i, on, now());
                word_lists::store(&lists);
                crate::lists_ui::refresh_pool(&a);
            }
            render(&a);
        } else if let Some(i) = attr(&el, "data-e-up")
            .or_else(|| attr(&el, "data-e-down"))
            .and_then(|v| v.parse::<usize>().ok())
        {
            let down = attr(&el, "data-e-down").is_some();
            if let Some(id) = OPEN_LIST.with(|c| c.borrow().clone()) {
                let to = if down { i + 1 } else { i.saturating_sub(1) };
                let mut lists = word_lists::load();
                word_lists::move_entry(&mut lists, &id, i, to, now());
                word_lists::store(&lists);
                crate::lists_ui::refresh_pool(&a);
            }
            render(&a);
        } else if let Some(id) = attr(&el, "data-l-open") {
            OPEN_LIST.with(|c| *c.borrow_mut() = Some(id));
            EDITING.with(|c| c.set(None));
            CONFIRMING.with(|c| c.set(false));
            render(&a);
        } else if attr(&el, "data-l-back").is_some() {
            OPEN_LIST.with(|c| *c.borrow_mut() = None);
            CONFIRMING.with(|c| c.set(false));
            render(&a);
        } else if attr(&el, "data-l-older").is_some() {
            SHOW_OLDER.with(|c| c.set(!c.get()));
            render(&a);
        } else if attr(&el, "data-l-trash").is_some() {
            SHOW_TRASH.with(|c| c.set(!c.get()));
            render(&a);
        } else if let Some(id) = attr(&el, "data-l-restore") {
            let mut lists = word_lists::load();
            word_lists::restore(&mut lists, &id);
            word_lists::store(&lists);
            render(&a);
        } else if let Some(id) = attr(&el, "data-l-purge") {
            let mut lists = word_lists::load();
            word_lists::delete_now(&mut lists, &id);
            word_lists::store(&lists);
            render(&a);
        } else if attr(&el, "data-l-rename").is_some() {
            RENAMING.with(|c| c.set(true));
            render(&a);
        } else if attr(&el, "data-l-rename-cancel").is_some() {
            RENAMING.with(|c| c.set(false));
            render(&a);
        } else if attr(&el, "data-l-rename-save").is_some() {
            if let Some(id) = OPEN_LIST.with(|c| c.borrow().clone()) {
                let typed = dom::input("listsRename").value();
                let mut lists = word_lists::load();
                if let Some(name) = word_lists::rename_to(&lists, &id, &typed) {
                    if let Some(l) = lists.lists.iter_mut().find(|l| l.id == id) {
                        l.name = name;
                        l.updated_at = now();
                    }
                    word_lists::store(&lists);
                }
            }
            RENAMING.with(|c| c.set(false));
            render(&a);
        } else if attr(&el, "data-l-del-ask").is_some() {
            let kid = a.borrow().kid;
            if kid {
                // F5.5: a child cannot delete a parent's list on their own.
                let a2 = a.clone();
                let id = OPEN_LIST.with(|c| c.borrow().clone());
                crate::parent_gate_then(Box::new(move || {
                    if let Some(id) = id {
                        delete_list(&a2, &id);
                    }
                }));
            } else {
                CONFIRMING.with(|c| c.set(true));
                render(&a);
            }
        } else if attr(&el, "data-l-del-cancel").is_some() {
            CONFIRMING.with(|c| c.set(false));
            render(&a);
        } else if let Some(id) = attr(&el, "data-l-del") {
            delete_list(&a, &id);
        } else if attr(&el, "data-l-snap").is_some() {
            close();
            crate::photo_list::open_camera();
        } else if attr(&el, "data-l-add").is_some() {
            close();
            crate::open_import_sheet(&a);
        } else if let Some(i) = attr(&el, "data-e-edit").and_then(|v| v.parse::<usize>().ok()) {
            EDITING.with(|c| c.set(Some(i)));
            render(&a);
        } else if attr(&el, "data-e-cancel").is_some() {
            EDITING.with(|c| c.set(None));
            render(&a);
        } else if let Some(i) = attr(&el, "data-e-save").and_then(|v| v.parse::<usize>().ok()) {
            if let Some(id) = OPEN_LIST.with(|c| c.borrow().clone()) {
                let typed = dom::input("listsEdit").value();
                let mut lists = word_lists::load();
                let lang = lists
                    .lists
                    .iter()
                    .find(|l| l.id == id)
                    .and_then(|l| l.entries.get(i))
                    .map(|e| e.lang.clone())
                    .unwrap_or_default();
                if word_lists::edit_entry(&mut lists, &id, i, &typed, &lang, now()) {
                    word_lists::store(&lists);
                    crate::lists_ui::refresh_pool(&a);
                }
            }
            EDITING.with(|c| c.set(None));
            render(&a);
        } else if let Some(i) = attr(&el, "data-e-remove").and_then(|v| v.parse::<usize>().ok()) {
            if let Some(id) = OPEN_LIST.with(|c| c.borrow().clone()) {
                let mut lists = word_lists::load();
                if let Some(gone) = word_lists::remove_entry(&mut lists, &id, i, now()) {
                    word_lists::store(&lists);
                    crate::lists_ui::refresh_pool(&a);
                    let word = gone.text.clone();
                    render(&a);
                    offer_undo(
                        &a,
                        &i18n::tp("lists.wordRemoved", &[("word", &word)]),
                        Undo::Entry { list: id, index: i, entry: gone },
                    );
                    return;
                }
            }
            render(&a);
        }
    });
}
