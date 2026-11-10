//! CC-LAUNCH-EN v1.1 D-DEF12 / I4: THE one gate for showing a definition or an
//! example sentence. Every render path reaches row text only through it:
//!
//! - `/api/meaning` text is wrapped in [`crate::api::Meaning`], whose fields are
//!   private and whose only accessors (`definition()`, `sentence()`) ask
//!   [`can_render`]. Sentence audio plays only from a `Meaning` whose sentence
//!   passes (`api::play_sentence_audio`).
//! - Definition Match drops a pool row whose definition does not pass before
//!   the row is stored (`defmatch_screen::ensure_pool`), so the dealer, the
//!   cards, the reverse prompt and the recap only ever hold passing rows.
//!
//! `scripts/def-render-gate-check.mjs` fails the build if a new path reads that
//! text another way.
//!
//! Review state comes from the server row. A row with NO review fields is
//! legacy content (today's Wiktionary pool and dictionary cache): it renders
//! while [`RELEASE_REVIEWED_ONLY`] is false, which is every build before the
//! App Store submission (D-DEF11: current builds stay live, with attribution).
//! A row that HAS review fields renders a field only when that field is
//! reviewed (D-DEF9: definition and sentence separately). So a row the ingest
//! rejected stays hidden in every build, and the submission build shows
//! reviewed rows only.

use serde_json::Value;

/// D-DEF11: false until the App Store submission build, which must set it true
/// so only reviewed rows render (the release has to carry no BY-SA text).
/// A release check pins it, as `COMPLETE_ON_SALE` is pinned.
///
/// CC-LAUNCH-EN rev 4, Eric 2026-11-09 ("Cut it"): ON. English text comes from
/// Spell's own reviewed rows (api::DEF_SRC); legacy rows (Wiktionary pools and
/// dictionary text for the languages this release hides) no longer render.
pub const RELEASE_REVIEWED_ONLY: bool = true;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Definition,
    Sentence,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Review {
    /// No review fields on the row: pre-review content.
    #[default]
    Legacy,
    /// The row went through the definitions sheet; each field has its verdict.
    Rated { definition: bool, sentence: bool },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Meta {
    pub review: Review,
    pub source: Option<String>,
    pub source_license: Option<String>,
}

/// Read review state from a server row. Accepts `reviewed: {definition,
/// sentence}` or flat `reviewed_definition` / `reviewed_sentence`. A row with
/// either form is Rated; a field the row leaves out counts as not reviewed.
pub fn meta_from_json(v: &Value) -> Meta {
    let flag = |k: &str| v.get(k).and_then(Value::as_bool);
    let nested = v.get("reviewed").filter(|r| r.is_object());
    let def = nested.and_then(|r| r.get("definition")).and_then(Value::as_bool).or_else(|| flag("reviewed_definition"));
    let sen = nested.and_then(|r| r.get("sentence")).and_then(Value::as_bool).or_else(|| flag("reviewed_sentence"));
    let rated = nested.is_some() || def.is_some() || sen.is_some();
    let s = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
    Meta {
        review: if rated { Review::Rated { definition: def.unwrap_or(false), sentence: sen.unwrap_or(false) } } else { Review::Legacy },
        source: s("source"),
        source_license: s("source_license"),
    }
}

/// D-DEF12: may this field of this row be shown (or, for a sentence, spoken)?
pub fn can_render(meta: &Meta, field: Field) -> bool {
    can_render_in(meta, field, RELEASE_REVIEWED_ONLY)
}

/// [`can_render`] with the release switch passed in, so tests cover both builds.
pub fn can_render_in(meta: &Meta, field: Field, reviewed_only: bool) -> bool {
    match meta.review {
        Review::Legacy => !reviewed_only,
        Review::Rated { definition, sentence } => match field {
            Field::Definition => definition,
            Field::Sentence => sentence,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The submission build renders reviewed rows only, and asks the server for them.
    #[test]
    fn release_renders_reviewed_rows_only() {
        assert!(RELEASE_REVIEWED_ONLY, "the English release renders reviewed definitions and sentences only");
        assert_eq!(crate::api::DEF_SRC, "gen1");
        assert!(!can_render(&meta_from_json(&json!({"definition": "legacy"})), Field::Definition));
    }

    /// A15.1: one reviewed and one unreviewed row: exactly one renders, in the
    /// submission build; the unreviewed (rated, rejected) row never renders.
    #[test]
    fn a15_1_only_the_reviewed_row_renders() {
        let good = meta_from_json(&json!({"reviewed": {"definition": true, "sentence": true}}));
        let bad = meta_from_json(&json!({"reviewed": {"definition": false, "sentence": false}}));
        for strict in [false, true] {
            assert!(can_render_in(&good, Field::Definition, strict));
            assert!(!can_render_in(&bad, Field::Definition, strict));
        }
    }

    /// A15.1a: the fields are gated separately, both ways round.
    #[test]
    fn a15_1a_definition_and_sentence_are_gated_separately() {
        let def_only = meta_from_json(&json!({"reviewed_definition": true, "reviewed_sentence": false}));
        assert!(can_render_in(&def_only, Field::Definition, true));
        assert!(!can_render_in(&def_only, Field::Sentence, true));
        let sen_only = meta_from_json(&json!({"reviewed": {"definition": false, "sentence": true}}));
        assert!(!can_render_in(&sen_only, Field::Definition, true));
        assert!(can_render_in(&sen_only, Field::Sentence, true));
        // A rated row that names only one field leaves the other unreviewed.
        let partial = meta_from_json(&json!({"reviewed_definition": true}));
        assert!(!can_render_in(&partial, Field::Sentence, false));
    }

    /// D-DEF11: legacy rows (no review fields) stay live in current builds and
    /// render nothing in the submission build.
    #[test]
    fn legacy_rows_render_only_before_the_submission_build() {
        let legacy = meta_from_json(&json!({"pos": "noun", "definition": "x"}));
        assert_eq!(legacy.review, Review::Legacy);
        assert!(can_render_in(&legacy, Field::Definition, false));
        assert!(!can_render_in(&legacy, Field::Definition, true));
        assert!(!can_render_in(&legacy, Field::Sentence, true));
    }

    #[test]
    fn source_tags_are_read() {
        let m = meta_from_json(&json!({"source": "spell-gen-v1", "source_license": "original"}));
        assert_eq!(m.source.as_deref(), Some("spell-gen-v1"));
        assert_eq!(m.source_license.as_deref(), Some("original"));
    }
}
