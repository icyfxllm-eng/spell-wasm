//! CC-VISUAL-ECLIPSE F4 -- the reveal that teaches, behind `visual.feedback`.
//!
//! Under a wrong answer, two rows: "You wrote" (the attempt) and "Correct" (the
//! spelling the mode was already about to reveal), with the letters marked from
//! `display_diff`: extra letters struck, missing letters underlined, wrong
//! letters highlighted, swapped letters overlined. Every mark is a text
//! decoration or a background on an inline element -- never a box, a margin or
//! a transform -- so a cursive word stays one shaped run and still joins (the
//! CC-RTL F4 finding `render_reveal_colored` already lives by).
//!
//! It reveals nothing new (V-D15, I6): `may_show` requires the mode's policy to
//! allow a diff, which `modes-check` only permits where the spelling is shown
//! anyway, and the attempt row holds only what the player typed.
//!
//! Pure string building; the caller writes the result into `#feedback`.

use crate::display_diff::{display_diff, Op};
use crate::dom::escape_html;
use crate::modes::Policy;

/// Whether the two rows may be drawn: the policy allows a diff and there is an
/// attempt to compare (an empty answer -- a timeout, a give-up -- keeps the
/// plain reveal, so nothing ever renders as all-missing).
pub fn may_show(policy: &Policy, typed: &str) -> bool {
    policy.may_draw_diff() && !typed.trim().is_empty()
}

/// The two rows as HTML. `lang` is the study language: it sets the words'
/// `lang` and direction from the registry, so an Arabic row runs right to left
/// and right-aligned while the label, in the interface language, is isolated
/// in its own `<bdi>`.
pub fn rows_html(target: &str, attempt: &str, lang: &str, you_label: &str, ok_label: &str) -> String {
    let cells = display_diff(target, attempt);
    let mut you = String::new();
    let mut ok = String::new();
    for c in &cells {
        if let Some(a) = &c.attempt {
            you.push_str(&mark(c.op, a, true));
        }
        if let Some(t) = &c.target {
            ok.push_str(&mark(c.op, t, false));
        }
    }
    let dir = crate::consts::dir_attr(lang);
    let lang = escape_html(lang);
    let row = |cls: &str, label: &str, word_cls: &str, word: &str, plain: &str| {
        format!(
            "<span class=\"rd-row {cls}\" dir=\"{dir}\" role=\"group\" aria-label=\"{aria}\">\
             <bdi class=\"rd-label\">{label}</bdi> \
             <bdi class=\"rd-word {word_cls}\" lang=\"{lang}\" dir=\"{dir}\">{word}</bdi></span>",
            aria = escape_html(&format!("{label}: {plain}")),
            label = escape_html(label),
        )
    };
    format!(
        "<span class=\"reveal-diff\">{}{}</span>",
        // Both words are `.reveal`, so every per-script face, spacing and
        // isolation rule the reveal already has applies to them unchanged.
        row("rd-you", you_label, "reveal reveal-attempt", &you, attempt.trim()),
        row("rd-ok", ok_label, "reveal", &ok, target),
    )
}

/// The play screen's layout (AT-4): the answer box already shows what the
/// player typed, so the attempt's marks go there, captioned "You wrote", and the
/// result line carries the Correct row alone -- the same height as the reveal
/// it replaces. Returns (answer box, result line).
pub fn boxed(target: &str, attempt: &str, lang: &str, you_label: &str, ok_label: &str) -> (String, String) {
    let cells = display_diff(target, attempt);
    let (mut you, mut ok) = (String::new(), String::new());
    for c in &cells {
        if let Some(a) = &c.attempt {
            you.push_str(&mark(c.op, a, true));
        }
        if let Some(t) = &c.target {
            ok.push_str(&mark(c.op, t, false));
        }
    }
    let dir = crate::consts::dir_attr(lang);
    let lang_e = escape_html(lang);
    let boxed = format!(
        "<span class=\"rd-caption\" aria-hidden=\"true\">{you_l}</span>\
         <bdi class=\"ltr rd-word reveal-attempt\" lang=\"{lang_e}\" dir=\"{dir}\" aria-label=\"{aria}\">{you}</bdi>",
        you_l = escape_html(you_label),
        aria = escape_html(&format!("{you_label}: {}", attempt.trim())),
    );
    let line = format!(
        "<span class=\"reveal-diff\"><span class=\"rd-row rd-ok\" dir=\"{dir}\" role=\"group\" aria-label=\"{aria}\">\
         <bdi class=\"rd-label\">{ok_l}</bdi> <bdi class=\"rd-word reveal\" lang=\"{lang_e}\" dir=\"{dir}\">{ok}</bdi></span></span>",
        ok_l = escape_html(ok_label),
        aria = escape_html(&format!("{ok_label}: {target}")),
    );
    (boxed, line)
}

/// One letter in one row. A letter that is the same in both is bare text, so
/// a fully right stretch is one run with nothing around it.
fn mark(op: Op, letter: &str, attempt_row: bool) -> String {
    let l = escape_html(letter);
    match (op, attempt_row) {
        (Op::Same, _) => l,
        (Op::Sub, _) => format!("<mark class=\"rd-sub\">{l}</mark>"),
        (Op::Swap, _) => format!("<span class=\"rd-swap\">{l}</span>"),
        (Op::Extra, true) => format!("<s class=\"rd-extra\">{l}</s>"),
        (Op::Missing, false) => format!("<u class=\"rd-missing\">{l}</u>"),
        // An extra letter has no target and a missing one no attempt; the
        // caller never asks for those rows.
        (Op::Extra, false) | (Op::Missing, true) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(mode: &str) -> Policy {
        crate::modes::feedback_policy(mode, None)
    }

    #[test]
    fn only_a_mode_whose_policy_allows_a_diff_shows_the_rows() {
        assert!(may_show(&policy("standard"), "cat"));
        assert!(!may_show(&Policy::CLOSED, "cat"), "an unknown mode fails closed");
        assert!(!may_show(&policy("bee_sim"), "cat"), "Bee reveals but never diffs");
        assert!(!may_show(&policy("standard"), "  "), "no attempt, plain reveal");
    }

    #[test]
    fn every_mark_is_present_and_the_correct_word_carries_reveal() {
        let h = rows_html("receive", "recieve", "en", "You wrote", "Correct");
        assert_eq!(h.matches("rd-swap").count(), 4, "two swapped letters on each row: {h}");
        assert_eq!(h.matches("class=\"rd-word reveal\"").count(), 1);
        let h = rows_html("cat", "cats", "en", "You wrote", "Correct");
        assert!(h.contains("<s class=\"rd-extra\">s</s>"), "{h}");
        assert_eq!(h.matches("class=\"rd-word reveal\"").count(), 1);
        let h = rows_html("cat", "ct", "en", "You wrote", "Correct");
        assert!(h.contains("<u class=\"rd-missing\">a</u>"), "{h}");
        let h = rows_html("cat", "cot", "en", "You wrote", "Correct");
        assert_eq!(h.matches("<mark class=\"rd-sub\">").count(), 2, "highlighted on both rows");
    }

    #[test]
    fn boxed_puts_the_attempt_in_the_box_and_only_correct_on_the_line() {
        let (b, l) = boxed("cat", "cats", "en", "You wrote", "Correct");
        assert!(b.contains("rd-caption") && b.contains("<s class=\"rd-extra\">s</s>"), "{b}");
        assert!(!l.contains("You wrote") && l.contains("Correct") && l.matches("rd-row").count() == 1, "{l}");
        let (b, _) = boxed("كتاب", "كتب", "ar", "You wrote", "Correct");
        assert!(b.contains("lang=\"ar\" dir=\"rtl\""), "{b}");
        let (b, _) = boxed("cat", "<b>", "en", "Y", "C");
        assert!(!b.contains("<b>"), "{b}");
    }

    #[test]
    fn the_attempt_is_escaped() {
        let h = rows_html("cat", "<b>", "en", "You wrote", "Correct");
        assert!(!h.contains("<b>"), "{h}");
        assert!(h.contains("&lt;"));
    }

    #[test]
    fn arabic_rows_run_right_to_left_and_isolate_the_label() {
        let h = rows_html("كتاب", "كتب", "ar", "You wrote", "Correct");
        assert!(h.contains("class=\"rd-row rd-you\" dir=\"rtl\""), "{h}");
        assert!(h.contains("lang=\"ar\" dir=\"rtl\""), "{h}");
        assert!(h.contains("<bdi class=\"rd-label\">You wrote</bdi>"));
    }

    #[test]
    fn a_right_stretch_is_bare_text_so_cursive_still_joins() {
        let h = rows_html("كتاب", "كتب", "ar", "a", "b");
        assert!(h.contains(">كت<u class=\"rd-missing\">ا</u>ب<"), "{h}");
    }

    #[test]
    fn nothing_beyond_the_attempt_and_the_target_appears() {
        // I6: strip the markup and each row holds exactly its own word.
        let h = rows_html("necessary", "neccesary", "en", "Y", "C");
        let text = |cls: &str| {
            let start = h.find(&format!("class=\"rd-word {cls}\"")).unwrap();
            let inner = &h[h[start..].find('>').unwrap() + start + 1..];
            let inner = &inner[..inner.find("</bdi>").unwrap()];
            let mut out = String::new();
            let mut tag = false;
            for ch in inner.chars() {
                match ch {
                    '<' => tag = true,
                    '>' => tag = false,
                    _ if !tag => out.push(ch),
                    _ => {}
                }
            }
            out
        };
        assert_eq!(text("reveal reveal-attempt"), "neccesary");
        assert_eq!(text("reveal"), "necessary");
    }
}
