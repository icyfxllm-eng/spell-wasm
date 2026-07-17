use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement, Window};

pub fn window() -> Window {
    web_sys::window().expect("no window")
}

pub fn doc() -> Document {
    window().document().expect("no document")
}

pub fn el(id: &str) -> Element {
    doc().get_element_by_id(id).unwrap_or_else(|| panic!("missing element #{id}"))
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

pub fn set_html(id: &str, html: &str) {
    el(id).set_inner_html(html);
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
