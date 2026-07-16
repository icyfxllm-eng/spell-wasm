//! Word-list data-source attributions on the Settings → About surface.
//!
//! The content is generated from the provenance registry (`sources/registry.json`)
//! into `credits.json` by `scripts/gen-credits.mjs`, and embedded here at build
//! time. Adding a registry entry (and regenerating `credits.json`) therefore
//! auto-adds its attribution to this screen — no code change required (D4:
//! attribution lives only in the generated credits file).

use serde::Deserialize;

use crate::dom;

#[derive(Deserialize, Default)]
struct Credits {
    #[serde(default)]
    sources: Vec<Source>,
}

#[derive(Deserialize)]
struct Source {
    name: String,
    url: String,
    license: String,
    attribution: String,
}

// Embedded at compile time from the repo-root generated file.
const CREDITS_JSON: &str = include_str!("../credits.json");

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Render the credits list into `#creditsSources` if that container exists.
/// Safe to call repeatedly (idempotent); a no-op when the element is absent or
/// the embedded JSON is empty/invalid.
pub fn render() {
    let container = match dom::doc().get_element_by_id("creditsSources") {
        Some(c) => c,
        None => return,
    };
    let credits: Credits = serde_json::from_str(CREDITS_JSON).unwrap_or_default();

    let mut html = String::new();
    for s in &credits.sources {
        html.push_str(&format!(
            "<div class=\"credit-row\" style=\"font-size:0.85em;line-height:1.4\">\
               <a href=\"{url}\" target=\"_blank\" rel=\"noopener noreferrer\" \
                  style=\"font-weight:600\">{name}</a>\
               <div style=\"opacity:0.7\">{license}</div>\
               <div style=\"opacity:0.7\">{attr}</div>\
             </div>",
            url = esc(&s.url),
            name = esc(&s.name),
            license = esc(&s.license),
            attr = esc(&s.attribution),
        ));
    }
    // Content is our own generated registry text (not user input) and is escaped
    // above; innerHTML is acceptable here (mirrors the trusted-string pattern in
    // i18n::translate_page for data-i18n-html).
    container.set_inner_html(&html);
}
