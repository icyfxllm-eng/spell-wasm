//! Feature F5 — "Word stories" (etymology cards).
//!
//! After a word is answered, surface a one-line origin story ("From Latin
//! *iudicare*, 'to judge'") to turn drill into curiosity. Pure data plumbing,
//! fully cross-platform (no native code).
//!
//! Data flow: the offline pipeline (`tools/lexicon-ingest`, kaikki/Wiktionary
//! parse) extracts a first-hop etymology per list word into the resolved,
//! shipped store `src/i18n/etymology/{lang}.json` (a `word -> story` map). At
//! runtime [`story`] looks the word up, re-compresses defensively to one
//! first-hop sentence (<=120 chars), and screens it through the shared
//! profanity filter like any other displayed content. It renders in Kid Mode
//! too (Decision D5 — educational).
//!
//! GATED (Decision D3 — attribution): Wiktionary text is CC BY-SA, so the whole
//! feature sits behind [`crate::flags::word_stories`], which is OFF until Eric
//! approves the attribution approach. With the flag off, [`story`] returns
//! `None` and nothing renders — zero behavioral difference.

use std::collections::HashMap;
use std::sync::OnceLock;

use unicode_normalization::UnicodeNormalization;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::dom;

/// Hard cap on a rendered story, in Unicode scalar values.
pub const MAX_LEN: usize = 120;

/// Per-language resolved etymology stores, keyed by NFC-lowercased word. Only
/// the active study languages (en/es) ship a store today; everything else
/// resolves to `None`. The pipeline extends these files as coverage grows.
fn stores() -> &'static HashMap<&'static str, HashMap<String, String>> {
    static S: OnceLock<HashMap<&'static str, HashMap<String, String>>> = OnceLock::new();
    S.get_or_init(|| {
        let mut m: HashMap<&'static str, HashMap<String, String>> = HashMap::new();
        m.insert("en", parse(include_str!("i18n/etymology/en.json")));
        m.insert("es", parse(include_str!("i18n/etymology/es.json")));
        m
    })
}

fn parse(s: &str) -> HashMap<String, String> {
    // The JSON keys are already lowercased at build time; normalize on read so a
    // stray-cased or NFD key still matches the NFC-lowercased lookup.
    serde_json::from_str::<HashMap<String, String>>(s)
        .unwrap_or_default()
        .into_iter()
        .map(|(k, v)| (norm_key(&k), v))
        .collect()
}

/// NFC + Unicode lowercase — the canonical lookup key for a word.
fn norm_key(word: &str) -> String {
    word.nfc().collect::<String>().to_lowercase()
}

/// Compress a raw etymology to a single first-hop sentence of at most
/// [`MAX_LEN`] characters. NFC-normalizes, collapses whitespace, truncates at
/// the first sentence/hop boundary (so no etymology *chains*: "from X, from Y,
/// from Z" keeps only "from X"), then hard-caps the length at a word boundary
/// with an ellipsis. Language-agnostic.
pub fn compress(raw: &str) -> String {
    let nfc: String = raw.nfc().collect();
    // Collapse all runs of whitespace to single spaces.
    let collapsed = nfc.split_whitespace().collect::<Vec<_>>().join(" ");

    // First hop = text up to the first sentence terminator or hop separator
    // (em dash / semicolon / colon). Keeps "From Dutch 'jacht' (a hunt)" out of
    // "… — a fast ship; the spelling stayed Dutch."
    let mut end = collapsed.len();
    for (i, ch) in collapsed.char_indices() {
        if matches!(ch, '\u{2014}' | '\u{2013}' | ';' | ':' | '.' | '!' | '?') {
            end = i;
            break;
        }
    }
    let first = collapsed[..end].trim();

    // Drop etymology chains: keep everything up to (not including) a *second*
    // "from" token, which is where a chain hops to the next ancestor language.
    let hop = truncate_chain(first);

    // Hard length cap at a word boundary.
    let capped = truncate_chars(hop.trim(), MAX_LEN);

    // Tidy dangling separators/punctuation left by a cut.
    capped
        .trim()
        .trim_end_matches([',', ';', ':', '-', '\u{2013}', '\u{2014}', ' '])
        .trim()
        .to_string()
}

/// Keep text up to (excluding) the second `from` token — the first-hop rule.
fn truncate_chain(s: &str) -> String {
    let mut out = String::new();
    let mut from_count = 0usize;
    for tok in s.split_whitespace() {
        let bare = tok
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase();
        if bare == "from" {
            from_count += 1;
            if from_count >= 2 {
                break;
            }
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(tok);
    }
    out
}

/// Truncate to at most `max` Unicode scalar values, backing off to the last
/// word boundary and appending an ellipsis. Result length is always <= `max`.
fn truncate_chars(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        return s.to_string();
    }
    // Reserve one slot for the ellipsis, then back off to the last space.
    let mut cut = max.saturating_sub(1);
    while cut > 0 && chars[cut] != ' ' {
        cut -= 1;
    }
    if cut == 0 {
        cut = max.saturating_sub(1); // no space found: hard cut
    }
    let mut t: String = chars[..cut].iter().collect();
    t = t.trim_end().to_string();
    t.push('\u{2026}');
    t
}

/// True when every token of `text` clears the profanity filter (the same screen
/// applied to user-imported words). A blocked token rejects the whole story.
fn is_clean(text: &str) -> bool {
    !text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .any(crate::profanity::is_blocked)
}

/// The display-ready story for a word, ignoring the feature flag. Split out so
/// the resolution + compression + screening logic is unit-testable without a
/// live DOM or the flag. `lang` may be a full tag ("en-US"); the base subtag is
/// used to pick the store.
fn resolve(lang: &str, word: &str) -> Option<String> {
    let base = lang.split(['-', '_']).next().unwrap_or(lang);
    let raw = stores().get(base)?.get(&norm_key(word))?;
    let story = compress(raw);
    if story.is_empty() || !is_clean(&story) {
        return None;
    }
    Some(story)
}

/// The etymology story to display for `word` in `lang`, or `None`. Returns
/// `None` whenever the feature flag is off (Decision D1/D3), so the caller's
/// gated path is a no-op in shipped builds.
pub fn story(lang: &str, word: &str) -> Option<String> {
    if !crate::flags::word_stories() {
        return None;
    }
    resolve(lang, word)
}

/// Clear any showing card. Called when a new word begins.
pub fn clear() {
    dom::set_html("didYouKnow", "");
    dom::remove_class("didYouKnow", "show");
}

/// Render the dismissible "did you know?" card for `word` beneath the result
/// surface, if a story exists and the flag is on. Never blocks the next word —
/// it is a passive element the game clears on the following word.
pub fn render(lang: &str, word: &str) {
    clear();
    let Some(story) = story(lang, word) else {
        return;
    };
    let heading = crate::i18n::t("story.heading");
    let dismiss = crate::i18n::t("story.dismiss");
    let html = format!(
        "<button class=\"dyk-x\" id=\"dykDismiss\" aria-label=\"{}\">\u{00d7}</button>\
         <div class=\"dyk-head\">{}</div><div class=\"dyk-body\">{}</div>",
        dom::escape_html(&dismiss),
        dom::escape_html(&heading),
        dom::escape_html(&story),
    );
    dom::set_html("didYouKnow", &html);
    dom::add_class("didYouKnow", "show");
    if let Ok(el) = dom::el("dykDismiss").dyn_into::<web_sys::HtmlElement>() {
        let cb = Closure::<dyn FnMut()>::new(clear);
        el.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())
            .ok();
        cb.forget();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_is_one_first_hop_sentence() {
        // Curated seed: cut at the em dash / semicolon — first hop only.
        assert_eq!(
            compress("From Dutch 'jacht' (a hunt) — a fast ship; the spelling stayed Dutch."),
            "From Dutch 'jacht' (a hunt)"
        );
        assert_eq!(
            compress("From Greek rhythmos — flow; famously written with no standard vowel."),
            "From Greek rhythmos"
        );
    }

    #[test]
    fn compress_drops_etymology_chains() {
        // A within-sentence chain (from … from … from …) keeps only the first hop.
        let chained = "From Latin iudicare from Proto-Italic from Proto-Indo-European deik";
        let out = compress(chained);
        assert_eq!(out, "From Latin iudicare");
        assert_eq!(out.to_lowercase().matches("from").count(), 1);
    }

    #[test]
    fn compress_caps_at_120_chars() {
        let long = "From Latin ".to_string() + &"verylongrootword ".repeat(20);
        let out = compress(&long);
        assert!(out.chars().count() <= MAX_LEN, "len {}", out.chars().count());
        assert!(out.ends_with('\u{2026}'), "expected ellipsis, got {out:?}");
    }

    #[test]
    fn compress_nfc_normalizes() {
        // Decomposed é (e + U+0301) must fold to precomposed é in the output.
        let decomposed = "From French e\u{0301}tude"; // "étude" as e + combining acute
        let out = compress(decomposed);
        assert!(out.contains('\u{00e9}'), "expected NFC é in {out:?}");
        assert!(!out.contains('\u{0301}'), "combining mark leaked: {out:?}");
    }

    #[test]
    fn profanity_screen_rejects_blocked_text() {
        // A clean etymology passes; one containing a blocked root does not.
        assert!(is_clean("From Latin iudicare, to judge"));
        assert!(!is_clean("From Old English scitte (shit) — an early form"));
    }

    #[test]
    fn flag_off_yields_no_story() {
        // The feature flag defaults off, so the public entry point never returns
        // a card even for a word that has a resolved story.
        assert!(!crate::flags::word_stories(), "flag must default off");
        assert_eq!(story("en", "yacht"), None);
        assert_eq!(story("es", "almohada"), None);
    }

    #[test]
    fn resolve_finds_seeded_story_independent_of_flag() {
        // The resolution/compression path itself works (this is what flips on
        // once the flag is enabled) — proves the store + compression are wired.
        let s = resolve("en", "yacht").expect("yacht is seeded");
        assert!(s.starts_with("From Dutch"));
        assert!(s.chars().count() <= MAX_LEN);
        // Full language tag resolves via the base subtag.
        assert!(resolve("en-US", "rhythm").is_some());
        // A word with no story resolves to None.
        assert_eq!(resolve("en", "notaword"), None);
        // A language with no store resolves to None.
        assert_eq!(resolve("fr", "yacht"), None);
    }
}
