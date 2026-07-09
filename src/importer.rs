use std::collections::HashSet;
use wasm_bindgen::JsValue;

use crate::model::{AppState, CustomSet, CUSTOM_KEY};
use crate::profanity;
use crate::storage;
use regex::Regex;

fn word_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[\p{L}][\p{L}'\u{2019}\-]*").unwrap())
}

pub fn extract_words(text: &str) -> Vec<String> {
    let re = word_regex();
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<String> = Vec::new();
    for m in re.find_iter(text) {
        let trimmed = m.as_str().trim_matches(|c| c == '\'' || c == '\u{2019}' || c == '-');
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_lowercase();
        if seen.insert(key) {
            out.push(trimmed.to_string());
            if out.len() >= 2000 {
                break;
            }
        }
    }
    out
}

pub async fn fetch_words_from_url(url: &str) -> Result<Vec<String>, JsValue> {
    let text = storage::fetch_text(url).await?;
    let stripped = storage::strip_html_if_needed(&text);
    Ok(extract_words(&stripped))
}

pub fn load_custom(state: &mut AppState) {
    if let Some(mut c) = storage::get_json::<CustomSet>(CUSTOM_KEY) {
        // Screen already-stored words too: a list saved before a term entered
        // the blocklist (or carried over from an older build) still gets
        // cleaned on load. Re-persist only if something was actually removed.
        let before = c.words.len();
        let (clean, blocked) = profanity::filter_allowed(c.words);
        c.words = clean;
        state.custom = c;
        if blocked > 0 && before != state.custom.words.len() {
            save_custom(state);
        }
    }
}

fn save_custom(state: &AppState) {
    storage::set_json(CUSTOM_KEY, &state.custom);
}

pub fn save_words(state: &mut AppState, words: Vec<String>, speak_lang: String) {
    state.custom = CustomSet { words, speak_lang };
    save_custom(state);
}

pub fn clear_words(state: &mut AppState) {
    let speak_lang = state.custom.speak_lang.clone();
    state.custom = CustomSet { words: Vec::new(), speak_lang };
    save_custom(state);
}
