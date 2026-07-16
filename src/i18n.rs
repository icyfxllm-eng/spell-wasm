//! Runtime UI localization. Locale tables are embedded (`include_str!` of
//! src/i18n/locales/{code}.json — flat key -> string), the active locale
//! follows the word-list language selector (so word language and UI language
//! are one setting), and `t`/`tp` fill `{placeholder}`s. Fallback chain:
//! active locale -> en -> the key itself (never renders `undefined`; a missing
//! key logs a console warning). `translate_page()` fills any `[data-i18n]`
//! element in the static HTML and updates `<html lang>`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;

use wasm_bindgen::JsCast;

use crate::dom;

thread_local! {
    static LOCALE: RefCell<String> = const { RefCell::new(String::new()) };
}

type Table = HashMap<String, String>;

fn tables() -> &'static HashMap<&'static str, Table> {
    static T: OnceLock<HashMap<&'static str, Table>> = OnceLock::new();
    T.get_or_init(|| {
        let mut m: HashMap<&'static str, Table> = HashMap::new();
        m.insert("en", parse(include_str!("i18n/locales/en.json")));
        m.insert("es", parse(include_str!("i18n/locales/es.json")));
        m.insert("fr", parse(include_str!("i18n/locales/fr.json")));
        m.insert("de", parse(include_str!("i18n/locales/de.json")));
        m.insert("pt", parse(include_str!("i18n/locales/pt.json")));
        m.insert("it", parse(include_str!("i18n/locales/it.json")));
        m.insert("nl", parse(include_str!("i18n/locales/nl.json")));
        m.insert("pl", parse(include_str!("i18n/locales/pl.json")));
        m.insert("sv", parse(include_str!("i18n/locales/sv.json")));
        m.insert("nb", parse(include_str!("i18n/locales/nb.json")));
        m.insert("tr", parse(include_str!("i18n/locales/tr.json")));
        m.insert("vi", parse(include_str!("i18n/locales/vi.json")));
        m.insert("ko", parse(include_str!("i18n/locales/ko.json")));
        m.insert("ja", parse(include_str!("i18n/locales/ja.json")));
        m.insert("zh", parse(include_str!("i18n/locales/zh.json")));
        m.insert("th", parse(include_str!("i18n/locales/th.json")));
        m.insert("fil", parse(include_str!("i18n/locales/fil.json")));
        m
    })
}

fn parse(s: &str) -> Table {
    serde_json::from_str(s).unwrap_or_default()
}

pub fn current() -> String {
    LOCALE.with(|l| {
        let v = l.borrow();
        if v.is_empty() {
            "en".to_string()
        } else {
            v.clone()
        }
    })
}

/// Set the active UI locale (call with a supported built-in code; unknown codes
/// fall back to English at lookup time anyway). Does NOT re-render — the caller
/// runs `translate_page()` + re-renders any dynamic strings.
pub fn set_locale(code: &str) {
    LOCALE.with(|l| *l.borrow_mut() = code.to_string());
}

const LOCALE_KEY: &str = "spellgame.locale";

/// True if `code` is a locale we ship a table for.
pub fn is_supported(code: &str) -> bool {
    tables().contains_key(code)
}

/// Resolve and set the boot locale. Priority: persisted choice ->
/// `fallback_lang` (the word-list language, when it is a supported locale) ->
/// the device language (base subtag) -> English. Persists nothing; the first
/// explicit selector change records the choice via `set_and_persist`.
pub fn init(fallback_lang: &str) {
    let loc = crate::storage::get_raw(LOCALE_KEY)
        .filter(|s| is_supported(s))
        .or_else(|| Some(fallback_lang.to_string()).filter(|s| is_supported(s)))
        .or_else(device_locale)
        .unwrap_or_else(|| "en".to_string());
    set_locale(&loc);
}

/// Set the active locale and remember it across launches.
pub fn set_and_persist(code: &str) {
    set_locale(code);
    crate::storage::set_raw(LOCALE_KEY, code);
}

/// Base subtag of `navigator.language` (e.g. "pt-BR" -> "pt"), if supported.
fn device_locale() -> Option<String> {
    device_base().filter(|b| is_supported(b))
}

/// The device's base language subtag (e.g. "ko-KR" -> "ko"), lowercased.
fn device_base() -> Option<String> {
    let lang = web_sys::window()?.navigator().language()?;
    Some(lang.split(['-', '_']).next()?.to_lowercase())
}

/// The device's language if it's a built-in *study* language, so a fresh install
/// opens matched to the player's device (words + UI), not English.
pub fn device_lang() -> Option<String> {
    device_base().filter(|b| crate::consts::is_builtin_lang(b))
}

/// Translate a key. active locale -> en -> key.
pub fn t(key: &str) -> String {
    let tbls = tables();
    let loc = current();
    if let Some(s) = tbls.get(loc.as_str()).and_then(|m| m.get(key)) {
        return s.clone();
    }
    if let Some(s) = tbls.get("en").and_then(|m| m.get(key)) {
        return s.clone();
    }
    web_sys::console::warn_1(&format!("[i18n] missing key: {key}").into());
    key.to_string()
}

/// Translate a key and fill `{name}` placeholders.
#[allow(dead_code)]
pub fn tp(key: &str, params: &[(&str, &str)]) -> String {
    let mut s = t(key);
    for (k, v) in params {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

/// Fill every `[data-i18n]` (textContent), `[data-i18n-ph]` (placeholder) and
/// `[data-i18n-aria]` (aria-label) in the static HTML, and set `<html lang>`.
pub fn translate_page() {
    apply("[data-i18n]", "data-i18n", |el, s| el.set_text_content(Some(s)));
    // Rich strings (embedded <b>, symbols) — locale text is our own/trusted, so
    // innerHTML is acceptable here. Used for the "How it works" block.
    apply("[data-i18n-html]", "data-i18n-html", |el, s| el.set_inner_html(s));
    apply("[data-i18n-ph]", "data-i18n-ph", |el, s| {
        let _ = el.set_attribute("placeholder", s);
    });
    apply("[data-i18n-aria]", "data-i18n-aria", |el, s| {
        let _ = el.set_attribute("aria-label", s);
    });
    update_tagline();
    if let Some(de) = dom::doc().document_element() {
        let _ = de.set_attribute("lang", &current());
        // data-lang drives the per-language accent tokens (F3); only ja/zh have a
        // theme block, every other language falls through to the default palette.
        let _ = de.set_attribute("data-lang", &current());
        // Lift the i18n boot cloak (FIX 3): the head script hid the translatable
        // chrome on <html class="booting"> for non-English locales so the English
        // static strings never flash before this translation lands. Now that the
        // page is translated, reveal it in one paint. Idempotent (a no-op on
        // later language-switch calls and for English, which was never cloaked).
        let _ = de.class_list().remove_1("booting");
    }
}

/// Set the brand tagline, appending the localized "just for fun" segment in Kid
/// Mode. Reads the `kid` body class so it stays correct however it's triggered
/// (boot, language change, Kid toggle). Replaces the old hardcoded CSS `::after`
/// that pinned "just for fun" to English.
pub fn update_tagline() {
    let kid = dom::doc()
        .body()
        .map(|b| b.class_list().contains("kid"))
        .unwrap_or(false);
    let text = if kid {
        format!("{} \u{b7} {}", t("brand.tag"), t("brand.tagFun"))
    } else {
        t("brand.tag")
    };
    dom::set_text("brandTag", &text);
}

fn apply(selector: &str, attr: &str, set: impl Fn(&web_sys::Element, &str)) {
    if let Ok(list) = dom::doc().query_selector_all(selector) {
        for i in 0..list.length() {
            if let Some(el) = list.get(i).and_then(|n| n.dyn_into::<web_sys::Element>().ok()) {
                if let Some(key) = el.get_attribute(attr) {
                    set(&el, &t(&key));
                }
            }
        }
    }
}
