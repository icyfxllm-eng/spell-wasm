use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement, Window};

pub fn window() -> Window {
    web_sys::window().expect("no window")
}

pub fn doc() -> Document {
    window().document().expect("no document")
}

/// True when the element exists in THIS build's document — the app and
/// the site ship different HTML, so cross-build wiring must probe before
/// it grabs (the picker/learner surfaces exist only in the app shell).
pub fn exists(id: &str) -> bool {
    doc().get_element_by_id(id).is_some()
}

/// True when the element is actually RENDERED — not merely present in
/// the document. `exists` is not enough for anything that gates play:
/// an element inside a `display:none` ancestor is in the DOM, answers
/// `exists`, accepts classes, and shows the user nothing. That gap is
/// what killed the orb on Aug 6, so any surface that can swallow a turn
/// must ask this instead.
///
/// `offsetParent` is null exactly when the element or an ancestor is
/// `display:none` (and for `position:fixed`, which our modals are — so
/// fall back to measuring a box, which a hidden subtree cannot produce).
pub fn rendered(id: &str) -> bool {
    let Some(e) = doc().get_element_by_id(id) else { return false };
    let Ok(h) = e.dyn_into::<web_sys::HtmlElement>() else { return false };
    if h.offset_parent().is_some() {
        return true;
    }
    let r = h.get_bounding_client_rect();
    r.width() > 0.0 && r.height() > 0.0
}

pub fn el(id: &str) -> Element {
    doc().get_element_by_id(id).unwrap_or_else(|| panic!("missing element #{id}"))
}

/// Attach a click handler to an element that is REPLACED each time it is
/// rendered — candidate chips, for instance. `on_click` leaks a handler per
/// call for long-lived controls; this is for markup that is thrown away and
/// rebuilt, where the element carrying the old handler no longer exists.
pub fn on_click_once(id: &str, f: impl Fn() + 'static) -> bool {
    let Some(el) = doc().get_element_by_id(id) else { return false };
    let cb = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || f());
    let ok = el
        .add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())
        .is_ok();
    cb.forget();
    ok
}

/// `on_click_once` for an element found by selector rather than id — candidate
/// chips are generated and have no stable ids of their own.
pub fn on_click_once_sel(sel: &str, f: impl Fn() + 'static) -> bool {
    let Ok(Some(el)) = doc().query_selector(sel) else { return false };
    let cb = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || f());
    let ok = el
        .add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())
        .is_ok();
    cb.forget();
    ok
}

/// The drawing pad's canvas. Removed when drawing was retired and restored for
/// CC-CJK-INK; the pad is the only caller, and it panics on absence for the
/// same reason `el` does -- a missing canvas is a build mistake, not a state.
pub fn canvas(id: &str) -> web_sys::HtmlCanvasElement {
    el(id)
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap_or_else(|_| panic!("#{id} is not a canvas"))
}

pub fn set_css_var(name: &str, value: &str) {
    if let Some(de) = doc().document_element().and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
        let _ = de.style().set_property(name, value);
    }
}

pub fn input(id: &str) -> HtmlInputElement {
    el(id).dyn_into::<HtmlInputElement>().unwrap_or_else(|_| panic!("#{id} is not an input"))
}

pub fn select(id: &str) -> HtmlSelectElement {
    el(id).dyn_into::<HtmlSelectElement>().unwrap_or_else(|_| panic!("#{id} is not a select"))
}

pub fn textarea(id: &str) -> HtmlTextAreaElement {
    el(id).dyn_into::<HtmlTextAreaElement>().unwrap_or_else(|_| panic!("#{id} is not a textarea"))
}

pub fn set_text(id: &str, text: &str) {
    el(id).set_text_content(Some(text));
}

/// Set text on an element that MAY not be rendered, without panicking. For
/// diagnostics that ride along with whatever screen happens to be up — `el`
/// panics on a missing id, which is right for real controls and wrong for a
/// readout nobody's looking at yet.
pub fn set_text_if(id: &str, text: &str) {
    if let Some(e) = doc().get_element_by_id(id) {
        e.set_text_content(Some(text));
    }
}

/// What an element currently reads. Used where a writer must not stomp a
/// message it did not write — the audio router clears its own failure banner
/// on recovery, but `voiceNote` is shared with the missing-browser-voice
/// notice, which is not the router's to erase.
pub fn text(id: &str) -> String {
    el(id).text_content().unwrap_or_default()
}

pub fn set_html(id: &str, html: &str) {
    el(id).set_inner_html(html);
}

/// Append to an element's markup without disturbing what is already
/// there (the insight line rides under a rendered meaning).
pub fn append_html(id: &str, html: &str) {
    let e = el(id);
    let cur = e.inner_html();
    e.set_inner_html(&format!("{cur}{html}"));
}

/// Programmatically click an element, if it exists.
///
/// Lets one surface route to another's EXISTING entry point instead of copying
/// its handler — the Play hub taps `sayItBtn` rather than reimplementing Say It's
/// open flow, so there is still exactly one place that knows how a mode starts.
/// Silently does nothing when the target is absent (unlike [`el`], which panics):
/// a hub tile may outlive a hidden or unwired button, and a missing destination
/// should be inert, not fatal.
pub fn click(id: &str) {
    if let Some(e) = doc()
        .get_element_by_id(id)
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
    {
        e.click();
    }
}

pub fn add_class(id: &str, class: &str) {
    let _ = el(id).class_list().add_1(class);
}

pub fn remove_class(id: &str, class: &str) {
    let _ = el(id).class_list().remove_1(class);
}

pub fn toggle_class(id: &str, class: &str, on: bool) {
    let _ = el(id).class_list().toggle_with_force(class, on);
}

/// Works for any form control (input/select/textarea/button) since the
/// `disabled` boolean attribute reflects to the `.disabled` IDL property
/// on all of them.
pub fn set_disabled(id: &str, disabled: bool) {
    let e = el(id);
    if disabled {
        let _ = e.set_attribute("disabled", "");
    } else {
        let _ = e.remove_attribute("disabled");
    }
}

pub fn escape_html(s: &str) -> String {
    s.chars().fold(String::with_capacity(s.len()), |mut acc, c| {
        match c {
            '&' => acc.push_str("&amp;"),
            '<' => acc.push_str("&lt;"),
            '>' => acc.push_str("&gt;"),
            '"' => acc.push_str("&quot;"),
            _ => acc.push(c),
        }
        acc
    })
}

pub fn on_click<F: FnMut() + 'static>(id: &str, f: F) {
    let cb = Closure::<dyn FnMut()>::new(f);
    let _ = el(id).add_event_listener_with_callback("click", cb.as_ref().unchecked_ref());
    cb.forget();
}

/// Wires an event listener that hands the closure a specific typed event
/// (KeyboardEvent, PointerEvent, ...) rather than the generic `Event`.
pub fn on<E, F>(id: &str, kind: &str, mut f: F)
where
    E: JsCast + 'static,
    F: FnMut(E) + 'static,
{
    let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| {
        if let Ok(typed) = e.dyn_into::<E>() {
            f(typed);
        }
    });
    let _ = el(id).add_event_listener_with_callback(kind, cb.as_ref().unchecked_ref());
    cb.forget();
}

pub fn on_window<E, F>(kind: &str, mut f: F)
where
    E: JsCast + 'static,
    F: FnMut(E) + 'static,
{
    let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| {
        if let Ok(typed) = e.dyn_into::<E>() {
            f(typed);
        }
    });
    let _ = window().add_event_listener_with_callback(kind, cb.as_ref().unchecked_ref());
    cb.forget();
}

/// True when the event's target *is* the element with `id` (not a child of
/// it) — used for "click on the scrim backdrop closes the modal" handlers.
pub fn is_self_target(e: &web_sys::Event, id: &str) -> bool {
    e.target().and_then(|t| t.dyn_into::<Element>().ok()).map(|el| el.id() == id).unwrap_or(false)
}

thread_local! {
    static TOAST_TIMEOUT: std::cell::Cell<Option<i32>> = std::cell::Cell::new(None);
}

pub fn show_toast(msg: &str) {
    let html = format!("<span class=\"ti\">\u{1F3C6}</span>Achievement \u{2014} {}", escape_html(msg));
    set_html("toast", &html);
    add_class("toast", "show");
    let win = window();
    TOAST_TIMEOUT.with(|cell| {
        if let Some(id) = cell.take() {
            win.clear_timeout_with_handle(id);
        }
    });
    let closure = wasm_bindgen::closure::Closure::once(move || {
        remove_class("toast", "show");
    });
    let handle = win
        .set_timeout_with_callback_and_timeout_and_arguments_0(closure.as_ref().unchecked_ref(), 2600)
        .unwrap_or(0);
    TOAST_TIMEOUT.with(|cell| cell.set(Some(handle)));
    closure.forget();
}

/// v7 F7 — beforeinput listener carrying (inputType, data length) so the
/// submit path can tell typing from dictation (input provenance, D8).
pub fn on_before_input<F: Fn(String, u32) + 'static>(id: &str, f: F) {
    use wasm_bindgen::JsCast;
    let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(id))
    else {
        return;
    };
    let cb = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::Event)>::new(
        move |e: web_sys::Event| {
            let (ty, len) = match e.dyn_ref::<web_sys::InputEvent>() {
                Some(ie) => (
                    ie.input_type(),
                    ie.data().map(|d| d.chars().count() as u32).unwrap_or(0),
                ),
                None => ("insertText".to_string(), 1),
            };
            f(ty, len);
        },
    );
    let _ = el.add_event_listener_with_callback("beforeinput", cb.as_ref().unchecked_ref());
    cb.forget();
}
