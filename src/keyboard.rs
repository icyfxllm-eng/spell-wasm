//! The custom on-screen keyboard — the only way to type an answer on touch
//! devices. There is no `<input>` anywhere in the answer flow, so the iOS
//! system keyboard (and its dictation key + autocorrect / QuickType bar) never
//! opens during a round: the anti-cheat property this module exists to
//! guarantee. Physical keyboards still work on desktop via a window-level
//! keydown that routes into the same answer state (game::type_char/backspace).
//!
//! The layout follows the active word language (§2.2): AZERTY for French,
//! QWERTZ + ä ö ü for German, extra ñ / ç / å ä ö keys, and long-press accent
//! popovers so every diacritic in a locale's word list is reachable in ≤2
//! gestures. My Words falls back to the English layout + apostrophe/hyphen keys.

use std::cell::RefCell;

use wasm_bindgen::JsCast;

use crate::consts::MINE;
use crate::App;
use crate::{dom, game};

/// A keyboard layout: three base rows (lowercase, extra letter keys already
/// placed) plus long-press accent sets keyed by base character.
struct Layout {
    rows: &'static [&'static str],
    long_press: &'static [(char, &'static str)],
}

// Accent popovers per §2.2. The base key is prepended automatically, so these
// list only the accented alternatives.
const EN: Layout = Layout { rows: &["qwertyuiop", "asdfghjkl", "zxcvbnm"], long_press: &[] };
// Swahili (Kiswahili) — Latin, standard QWERTY. Native orthography is the Latin
// alphabet; digraphs (ch, sh, ng') are typed as sequences, and ng' words carry an
// apostrophe that the all-alphabetic word gate filters out, so no extra keys are
// needed. CC-MASTER-PARITY Track S.
const SW: Layout = Layout { rows: &["qwertyuiop", "asdfghjkl", "zxcvbnm"], long_press: &[] };
const ES: Layout = Layout {
    rows: &["qwertyuiop", "asdfghjklñ", "zxcvbnm"],
    long_press: &[('a', "á"), ('e', "é"), ('i', "í"), ('o', "ó"), ('u', "úü")],
};
const FR: Layout = Layout {
    rows: &["azertyuiop", "qsdfghjklm", "wxcvbn"],
    long_press: &[('e', "éèêë"), ('a', "àâ"), ('c', "ç"), ('i', "îï"), ('o', "ôœ"), ('u', "ùûü"), ('y', "ÿ")],
};
const DE: Layout = Layout { rows: &["qwertzuiopü", "asdfghjklöä", "yxcvbnm"], long_press: &[('s', "ß")] };
const PT: Layout = Layout {
    rows: &["qwertyuiop", "asdfghjklç", "zxcvbnm"],
    long_press: &[('a', "áàâã"), ('e', "éê"), ('i', "í"), ('o', "óôõ"), ('u', "ú")],
};
const PL: Layout = Layout {
    rows: &["qwertyuiop", "asdfghjkl", "zxcvbnm"],
    long_press: &[('a', "ą"), ('c', "ć"), ('e', "ę"), ('l', "ł"), ('n', "ń"), ('o', "ó"), ('s', "ś"), ('z', "źż")],
};
// Russian: the standard ЙЦУКЕН layout. `ё` is a long-press on `е` rather than a
// key of its own — on a physical ЙЦУКЕН it lives off in the number row, and
// giving it a top-level key would push the rows past every other layout's width.
//
// This pairs with CC-LINEUP-SWAP D4: canonical word forms store `ё` where
// dictionaries write it, and typing `е` in a `ё` position is ACCEPTED, so the
// long-press is an affordance for players who want the dictionary spelling, not
// a hoop anyone must jump through. The charset gate still needs `ё` reachable —
// canonical forms contain it — which is exactly what the long-press provides.
const RU: Layout = Layout {
    rows: &["йцукенгшщзхъ", "фывапролджэ", "ячсмитьбю"],
    long_press: &[('е', "ё")],
};
// Vietnamese: QWERTY + long-press for the letter-modified vowels (ă â ê ô ơ ư)
// and đ; the five tones are applied post-fix via a dedicated tone row (see
// build_keys / crate::viet). Every vowel form is thus reachable in ≤2 taps.
// The trailing SPACE in the last row is deliberate and load-bearing: a
// Vietnamese word is space-separated syllables (học sinh, thành phố), so without
// a space key a player cannot type what they are shown. Placed in the row the
// same way fil places its hyphen. `a_vietnamese_player_can_type_a_space` guards
// it, because an invisible character at the end of a string literal is exactly
// the kind of thing a tidy-up deletes.
const VI: Layout = Layout {
    rows: &["qwertyuiop", "asdfghjkl", "zxcvbnm "],
    long_press: &[('a', "ăâ"), ('e', "ê"), ('o', "ôơ"), ('u', "ư"), ('d', "đ")],
};
// Korean Dubeolsik (2-set): consonants left, vowels right. Keys emit jamo; the
// Hangul automaton (crate::hangul, via game::type_jamo) composes syllable blocks
// live. Tense consonants + ㅒ/ㅖ are on long-press instead of a shift layer.
const KO: Layout = Layout {
    rows: &["ㅂㅈㄷㄱㅅㅛㅕㅑㅐㅔ", "ㅁㄴㅇㄹㅎㅗㅓㅏㅣ", "ㅋㅌㅊㅍㅠㅜㅡ"],
    long_press: &[('ㅂ', "ㅃ"), ('ㅈ', "ㅉ"), ('ㄷ', "ㄸ"), ('ㄱ', "ㄲ"), ('ㅅ', "ㅆ"), ('ㅐ', "ㅒ"), ('ㅔ', "ㅖ")],
};
// Japanese: 50-on hiragana grid (5 rows). Voiced (dakuten), semi-voiced
// (handakuten) and small kana are on long-press, producing the precomposed kana
// directly (か→が, は→ばぱ, つ→づっ, や→ゃ …). Every kana is reachable in ≤2 taps.
const JA: Layout = Layout {
    rows: &["あいうえお", "かきくけこさしすせそ", "たちつてとなにぬねの", "はひふへほまみむめも", "やゆよらりるれろわをん"],
    long_press: &[
        ('あ', "ぁ"), ('い', "ぃ"), ('う', "ぅゔ"), ('え', "ぇ"), ('お', "ぉ"),
        ('か', "が"), ('き', "ぎ"), ('く', "ぐ"), ('け', "げ"), ('こ', "ご"),
        ('さ', "ざ"), ('し', "じ"), ('す', "ず"), ('せ', "ぜ"), ('そ', "ぞ"),
        ('た', "だ"), ('ち', "ぢ"), ('つ', "づっ"), ('て', "で"), ('と', "ど"),
        ('は', "ばぱ"), ('ひ', "びぴ"), ('ふ', "ぶぷ"), ('へ', "べぺ"), ('ほ', "ぼぽ"),
        ('や', "ゃ"), ('ゆ', "ゅ"), ('よ', "ょ"), ('わ', "ゎ"),
    ],
};
// Filipino (Tagalog): QWERTY + a visible ñ key (right of l, Spanish convention)
// and a hyphen key on the primary layer (orthographic in mag-aaral, pag-ibig).
// No composition engine — plain Latin spelling + ñ + hyphen.
const FIL: Layout = Layout { rows: &["qwertyuiop", "asdfghjklñ", "zxcvbnm-"], long_press: &[('n', "ñ")] };
// Mandarin: QWERTY for pinyin letters + a tone-number row (1-4, 5=neutral) and
// ü on long-press v (v→ü is the standard pinyin-input convention, also folded in
// the normalizer). The typed answer is pinyin; audio + reveal are the hanzi.
const ZH: Layout = Layout { rows: &["qwertyuiop", "asdfghjkl", "zxcvbnm", "12345"], long_press: &[('v', "ü")] };

// --- RTL scripts (CC-RTL F5, charset half only) ------------------------------
//
// This is a CHARSET DECLARATION, not a finished keyboard. It exists so the
// word-list pipeline's charset gate (build-wordlists.py + every_word_char_is_
// typeable) has an inventory to validate Arabic content against — the same thing
// that unblocks a language for content, as ru already is. The RTL INPUT behaviour
// (cursor/backspace direction, ZWNJ, hamza sequencing) is the other half of F5 and
// is deliberately NOT here.
//
// It is unreachable at runtime today: ar is rtl_blocked, so play never selects it
// and this layout never renders. If it ever does render (e.g. a My Words list
// tagged Arabic), these correct-charset keys still beat the &EN fallback's Latin
// keys — it would just lack RTL input polish.
//
// The inventory follows the standard national layout, but a native/expert pass is
// owed before any Arabic bank is built — there is no verified_by here and this
// comment is not one.
//
// Diacritics (harakat) are omitted: standard Arabic text is written undiacritized,
// so a spelling bank carries none.
const AR: Layout = Layout {
    rows: &["ضصثقفغعهخحج", "شسيبلاتنمكط", "ذرزوظدء"],
    long_press: &[('ا', "أإآ"), ('و', "ؤ"), ('ي', "ئى"), ('ت', "ة")],
};
// Persian (CC-PERSIAN-FOUNDATION F1). Arabic-script but NOT Arabic: ک U+06A9
// and ی U+06CC where Arabic has ك and ي, plus the four Persian-only letters
// پ چ ژ گ. All 32 letters sit on the BASE layer — iOS hides ژ on long-press of
// ز, but hunting a base letter behind a hold is exactly the friction Eric's
// 2026-08-09 ruling rules out, and unlike آ it has no lenient form the match
// key could forgive. No row exceeds 11 keys (the de/ar maximum), so it cannot
// overflow at 320px. Long-press carries آ ء ؤ ئ. No ZWNJ key: F2 makes it
// orthography the game owns, never a required keystroke.
const FA: Layout = Layout {
    rows: &["ضصثقفغعهخحج", "شسیبلاتنمکگ", "ظطزژرذدپوچ"],
    long_press: &[('ا', "آء"), ('و', "ؤ"), ('ی', "ئ")],
};
// Devanagari for Hindi (CC-HINDI-PHASE0 groundwork — the input-side companion to
// akshara.rs; a CHARSET declaration, not a registered language). Covers the full
// Hindi Devanagari inventory: independent vowels, consonants, matras, virama,
// nuqta and the anusvara/candrabindu/visarga signs — so every NFC codepoint of a
// Hindi word is typeable. Aspirates sit on long-press of their unaspirated base
// (क->ख), the Devanagari-phonetic convention.
//
// D4: keys are base codepoints only; the precomposed nuqta letters U+0958–095F
// (which devanagari-check.mjs rejects) never appear — क़ is typed क + ़ (U+093C).
// Unregistered like ru.json's era: Hindi is not in BUILTIN_LANGS, so this renders
// nowhere yet; it exists so the word-list gate has an inventory when content lands.
// LAYOUT ERGONOMICS (which char on which long-press) want a native/expert pass;
// the CHARSET (coverage) is asserted complete by rtl-adjacent tests below.
const HI: Layout = Layout {
    rows: &["अआइईउऊएऐओऔ", "कगङचजञटडण", "तदनपबमयरलव", "शषसहळ", "ािीुूेैोौ", "ं़्ःँ"],
    long_press: &[('क', "ख"), ('ग', "घ"), ('च', "छ"), ('ज', "झ"), ('ट', "ठ"), ('ड', "ढ"), ('त', "थ"), ('द', "ध"), ('प', "फ"), ('ब', "भ"), ('इ', "ऋ"), ('ु', "ृ"), ('अ', "ऽ")],
};


/// The typing-unit rows for a locale — the registry's own layout, exposed so a
/// second surface can offer real units without re-deriving them.
///
/// CC-WORD-CHAINS F1 says "input via existing typing-unit input". The keyboard
/// proper is not mountable elsewhere: build_keys renders into the play screen's
/// letters container and wire_actions binds the game's submit path, so reusing
/// it would mean refactoring the core play surface. This hands over the DATA
/// instead, which is the part that must not be duplicated — a chain screen
/// inventing its own kana grid is a second place for the alphabet to be wrong.
pub fn unit_rows(locale: &str) -> &'static [&'static str] {
    layout_for(locale).rows
}

/// The long-press alternates the main keyboard offers on `c` in `locale` (empty
/// when it offers none): the shared keyboard gives every surface that
/// long-presses the same alternates, from this one registry.
pub fn alternates(locale: &str, c: char) -> &'static str {
    layout_for(locale).long_press.iter().find(|(k, _)| *k == c).map(|(_, a)| *a).unwrap_or("")
}

/// The shared-keyboard rows for a surface that types the layout's own units:
/// every row key with its alternates, then, for Vietnamese, the tone row the
/// main keyboard has. Bee and Word Chains had neither, so an accented, voiced or
/// toned word could not be typed there at all.
pub fn shared_rows(locale: &str) -> Vec<Vec<crate::spellkb::Cap>> {
    use crate::spellkb::{Cap, Key};
    let mut rows: Vec<Vec<Cap>> = layout_for(locale)
        .rows
        .iter()
        .map(|r| r.chars().map(|c| Cap { key: Key::Unit(c.to_string()), accents: alternates(locale, c).to_string(), class: "" }).collect())
        .collect();
    if locale == "vi" {
        rows.push(crate::viet::TONE_MARKS.iter().map(|&m| Cap::key(Key::Tone(m))).collect());
    }
    rows
}

fn layout_for(locale: &str) -> &'static Layout {
    match locale {
        "es" => &ES,
        "fr" => &FR,
        "de" => &DE,
        "pt" => &PT,
        "pl" => &PL,
        "ru" => &RU,
        "vi" => &VI,
        "ko" => &KO,
        "ja" => &JA,
        "fil" => &FIL,
        "zh" => &ZH,
        "sw" => &SW,
        // CC-RTL F5 charset half. Returning the right layout (not the &EN
        // fallback) keeps the Rust charset gate consistent with the JSON one the
        // pipeline reads; RTL input behaviour is still unimplemented, but these
        // are unreachable in play while rtl_blocked.
        "ar" => &AR,
        // Unreachable until fa enters the registry (that reversal is Eric's),
        // wired the same way ar is while rtl_blocked: the charset must exist
        // for the pipeline to validate content against.
        "fa" => &FA,
        "hi" => &HI,
        _ => &EN,
    }
}

/// Which keyboard to show: My Words always uses the English layout (its words
/// are English + apostrophe/hyphen); built-in languages use their own.
fn keyboard_locale(app: &App) -> String {
    let s = app.borrow();
    if s.lang != MINE {
        // Impossible Mix (and a Mix miss replayed from Misses) serves words in languages other than
        // the study language: the keyboard follows the word, or a German word has no ä/ö/ü/ß and no
        // Shift for its capitals.
        if s.impossible_word && s.cur_lang != s.lang && crate::consts::is_builtin_lang(&s.cur_lang) {
            return s.cur_lang.clone();
        }
        return s.lang.clone();
    }
    // My Words: match the imported list's "Speak in" language so the right
    // keyboard shows (Korean Dubeolsik, kana grid, …). Mandarin
    // is the exception — its keyboard types pinyin, not the hanzi a user imports,
    // so My Words Chinese stays on the English layout. Unsupported/browser-only
    // picks also fall back to English.
    match game::mine_lang(&s) {
        Some("zh") | None => "en".to_string(),
        Some(code) => code.to_string(),
    }
}

/// True when the active keyboard composes Hangul (Korean) — key taps and
/// backspace route through the composition engine (crate::hangul) instead of a
/// plain append/pop.
pub fn active_is_korean(app: &App) -> bool {
    keyboard_locale(app) == "ko"
}

pub fn setup(app: &App) {
    BUILT_LOCALE.with(|b| *b.borrow_mut() = keyboard_locale(app));
    build_keys(app);
    wire_actions(app);
    wire_physical(app);
    wire_paste_guard();
    game::sync_keyboard(app);
}

/// Rebuild the letter keys for the active language (called on a language
/// change). Only the key buttons are re-created; the window-level handlers
/// (physical keyboard, paste guard) stay wired from `setup`, and long-press
/// lives in the shared keyboard.
pub fn rebuild(app: &App) {
    crate::spellkb::close_popover();
    BUILT_LOCALE.with(|b| *b.borrow_mut() = keyboard_locale(app));
    build_keys(app);
    game::sync_keyboard(app);
}

thread_local! {
    /// The locale the keyboard on screen was built for.
    static BUILT_LOCALE: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Rebuild the keyboard if the word just served wants a different one (an Impossible Mix word in
/// another language, or the first ordinary word after one). A no-op otherwise.
pub fn follow_word(app: &App) {
    if BUILT_LOCALE.with(|b| *b.borrow() != keyboard_locale(app)) {
        rebuild(app);
    }
}

/// The main keyboard, drawn and pressed through the shared keyboard (`spellkb`):
/// the layout's rows with their long-press alternates, the German Shift, the My
/// Words punctuation and the Vietnamese tone row. Keys type through
/// `game::emit_key`. Labels are lowercase and go uppercase only while Shift is
/// armed (K-D19).
fn build_keys(app: &App) {
    use crate::spellkb::{Attr, Cap, Edges, Key, Surface};
    let locale = keyboard_locale(app);
    let layout = layout_for(&locale);
    let accents_for = |c: char| -> &'static str { layout.long_press.iter().find(|(k, _)| *k == c).map(|(_, a)| *a).unwrap_or("") };
    let last = layout.rows.len().saturating_sub(1);
    let rows: Vec<Vec<Cap>> = layout
        .rows
        .iter()
        .enumerate()
        .map(|(ri, letters)| {
            let mut row: Vec<Cap> = letters.chars().map(|c| Cap { key: Key::Unit(c.to_string()), accents: accents_for(c).to_string(), class: "" }).collect();
            if ri == last {
                for p in ['\'', '-'] {
                    row.push(Cap { key: Key::Unit(p.to_string()), accents: String::new(), class: "kb-punct" });
                }
            }
            row
        })
        .collect();
    // German words can be typed with capitals in every mode. Hidden (inert)
    // unless the word on screen is German; see game::toggle_shift.
    let shift = if locale == "de" {
        "<button type=\"button\" class=\"kb-key kb-shift btn-hide\" id=\"kbShift\" aria-label=\"shift\">\u{21e7}</button>"
    } else {
        ""
    };
    let letters = Surface { host: "kbLetters", row_class: "kb-row", key_class: "kb-key", attr: Attr::Raw("data-k"), long_press: true, field: "spellbox" };
    let a = app.clone();
    crate::spellkb::mount(letters, &rows, Edges { lead: shift, tail: "" }, move |k| {
        if let Key::Unit(u) = k {
            if let Some(ch) = u.chars().next() {
                game::emit_key(&a, ch);
            }
        }
    });
    if locale == "de" {
        let a = app.clone();
        dom::on_click("kbShift", move || game::toggle_shift(&a));
    }
    let base_cols = layout.rows.iter().map(|r| r.chars().count()).max().unwrap_or(10);
    let cols = base_cols.max(layout.rows[last].chars().count() + 2);
    if let Some(el) = dom::doc().get_element_by_id("gameKeyboard").and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
        let _ = el.style().set_property("--kb-cols", &cols.to_string());
    }
    // The Vietnamese tone row, now with a name per tone (KF10).
    if locale == "vi" {
        let tones: Vec<Cap> = crate::viet::TONE_MARKS.iter().map(|&m| Cap::key(Key::Tone(m))).collect();
        let row = Surface { host: "kbToneRow", row_class: "", key_class: "kb-key kb-tone", attr: Attr::ToneHex("data-tone"), long_press: false, field: "" };
        let a = app.clone();
        crate::spellkb::mount(row, &[tones], Edges::default(), move |k| {
            if let Key::Tone(m) = k {
                game::apply_vi_tone(&a, *m);
            }
        });
        dom::remove_class("kbToneRow", "btn-hide");
    } else {
        dom::set_html("kbToneRow", "");
        dom::add_class("kbToneRow", "btn-hide");
    }
}

fn wire_actions(app: &App) {
    {
        let a = app.clone();
        dom::on_click("kbBackspace", move || game::backspace(&a));
    }
    {
        let a = app.clone();
        dom::on_click("kbSubmit", move || game::submit_guess(&a));
    }
}

/// Physical keyboard (desktop): route letters / Backspace / Enter into the
/// answer state — unless focus is in a real text field (the name / import
/// boxes), so those keep working normally. Accepts any single alphabetic
/// character (including accents typed on a native locale keyboard).
fn wire_physical(app: &App) {
    let a = app.clone();
    dom::on_window::<web_sys::KeyboardEvent, _>("keydown", move |e| {
        if target_is_text_field(e.as_ref()) {
            return;
        }
        let key = e.key();
        if key == "Enter" {
            game::submit_guess(&a);
            return;
        }
        if key == "Backspace" {
            e.prevent_default();
            game::backspace(&a);
            return;
        }
        if e.ctrl_key() || e.meta_key() || e.alt_key() {
            return;
        }
        // Exactly one printable char, and one we accept.
        let mut chars = key.chars();
        if let (Some(c), None) = (chars.next(), chars.next()) {
            // Digits are accepted for Mandarin pinyin tone numbers; letters/ñ/etc.
            // for every language; apostrophe/hyphen for My Words / Filipino.
            if c.is_alphabetic() || c.is_ascii_digit() || c == '\'' || c == '-' {
                e.prevent_default();
                // A German word keeps a physical Shift's capital (Impossible grades it; every other
                // mode folds case); every other language lowercases the key exactly as before.
                let lower = if e.shift_key() && game::word_takes_capitals(&a) { c } else { c.to_lowercase().next().unwrap_or(c) };
                game::emit_key(&a, lower);
            }
        }
    });
}

/// Defense-in-depth: block paste on the game screen (dictation/paste insert a
/// whole word at once). A real text field like the import box still accepts it.
fn wire_paste_guard() {
    dom::on_window::<web_sys::Event, _>("paste", |e| {
        if !target_is_text_field(&e) {
            e.prevent_default();
        }
    });
}

fn target_is_text_field(e: &web_sys::Event) -> bool {
    let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else {
        return false;
    };
    matches!(el.tag_name().to_uppercase().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn reachable(layout: &Layout) -> HashSet<char> {
        let mut s = HashSet::new();
        for row in layout.rows {
            s.extend(row.chars());
        }
        for (base, acc) in layout.long_press {
            s.insert(*base);
            s.extend(acc.chars());
        }
        s
    }

    /// The keyboard layout SSOT (assets/keyboards/{code}.json, consumed by the
    /// word-list pipeline's charset gate) must match the Rust layouts, so the
    /// gate and the runtime keyboard can never diverge.
    #[test]
    fn json_layouts_match_rust() {
        // (code, json) pairs — embedded at compile time from the SSOT files.
        let jsons: &[(&str, &str)] = &[
            ("en", include_str!("../assets/keyboards/en.json")),
            ("es", include_str!("../assets/keyboards/es.json")),
            ("fr", include_str!("../assets/keyboards/fr.json")),
            ("de", include_str!("../assets/keyboards/de.json")),
            ("pt", include_str!("../assets/keyboards/pt.json")),
            ("pl", include_str!("../assets/keyboards/pl.json")),
            ("ru", include_str!("../assets/keyboards/ru.json")),
            ("vi", include_str!("../assets/keyboards/vi.json")),
            ("ko", include_str!("../assets/keyboards/ko.json")),
            ("ja", include_str!("../assets/keyboards/ja.json")),
            ("fil", include_str!("../assets/keyboards/fil.json")),
            ("zh", include_str!("../assets/keyboards/zh.json")),
            ("sw", include_str!("../assets/keyboards/sw.json")),
            ("ar", include_str!("../assets/keyboards/ar.json")),
            ("fa", include_str!("../assets/keyboards/fa.json")),
            ("hi", include_str!("../assets/keyboards/hi.json")),
        ];
        for (code, json) in jsons {
            let v: serde_json::Value = serde_json::from_str(json).unwrap();
            let layout = layout_for(code);
            let rows: Vec<String> = v["rows"].as_array().unwrap().iter().map(|r| r.as_str().unwrap().to_string()).collect();
            assert_eq!(rows, layout.rows.to_vec(), "{code} rows differ from JSON");
            let lp = &v["longPress"];
            assert_eq!(
                lp.as_object().map(|o| o.len()).unwrap_or(0),
                layout.long_press.len(),
                "{code} longPress count differs from JSON"
            );
            for (base, acc) in layout.long_press {
                let got = lp[base.to_string()].as_str().unwrap_or("");
                assert_eq!(got, *acc, "{code} longPress[{base}] differs from JSON");
            }
        }
    }

    /// CC-RTL F5 charset half: the Arabic declaration must cover the whole
    /// alphabet, so a future word bank in correct orthography is fully typeable.
    /// A dropped letter would silently block every word that uses it.
    #[test]
    fn rtl_charsets_cover_their_alphabet() {
        // The full modern Arabic alphabet (letters only; harakat and ligatures
        // excluded by design — see the layout comments).
        let ar = reachable(&AR);
        for c in "ابتثجحخدذرزسشصضطظعغفقكلمنهوي".chars() {
            assert!(ar.contains(&c), "Arabic letter {c:?} (U+{:04X}) not reachable", c as u32);
        }

        // Persian. fa is not in BUILTIN_LANGS yet, so every_word_char_is_typeable
        // does not reach it — without this, nothing would pin the coverage.
        let fa = reachable(&FA);
        for c in "ابپتثجچحخدذرزژسشصضطظعغفقکگلمنوهی".chars() {
            assert!(fa.contains(&c), "Persian letter {c:?} (U+{:04X}) not reachable", c as u32);
        }
        // The extras Eric's 2026-08-09 ruling made legal must be reachable too,
        // or the legal set admits words the player cannot type.
        for c in "آءؤئ".chars() {
            assert!(fa.contains(&c), "legal fa char {c:?} (U+{:04X}) not reachable", c as u32);
        }
        // Every base key is a Persian letter, not its Arabic lookalike: a bank
        // word is stored canonical, so ك/ي on the keyboard would only ever
        // produce a form the canonicalizer has to undo.
        let base: String = FA.rows.concat();
        assert!(!base.contains('\u{643}'), "Arabic kaf on the Persian base layer");
        assert!(!base.contains('\u{64a}'), "Arabic yeh on the Persian base layer");
        // And every legal codepoint round-trips: reachable => legal.
        for c in fa.iter() {
            assert!(crate::fa_canon::is_fa_legal_char(*c),
                    "{c:?} is typeable but not legal in a fa word");
        }
    }

    /// §3.4 gate 1: every character in every built-in word (after the strict
    /// fold the player must reproduce) is reachable on that locale's keyboard.
    #[test]
    fn every_word_char_is_typeable() {
        for (code, _, _, _) in crate::consts::BUILTIN_LANGS {
            let mut reach = reachable(layout_for(code));
            if code == "vi" {
                // The tone row makes every tone of every reachable vowel typeable.
                for v in reach.iter().copied().collect::<Vec<_>>() {
                    for t in crate::viet::TONE_MARKS {
                        if let Some(s) = crate::viet::retone(v, t) {
                            reach.extend(s.chars());
                        }
                    }
                }
            }
            if code == "ko" {
                // The Dubeolsik keyboard + Hangul automaton compose every syllable.
                for u in 0xAC00u32..=0xD7A3 {
                    if let Some(c) = char::from_u32(u) {
                        reach.insert(c);
                    }
                }
            }
            for tier in ["easy", "medium", "hard", "expert"] {
                for w in crate::words::tier_for(code, tier) {
                    // Mandarin entries are "pinyin|hanzi"; only the pinyin (the
                    // typed answer) has to be reachable on the keyboard.
                    let answer = if code == "zh" { w.split('|').next().unwrap_or(w) } else { w };
                    for c in crate::norm::fold_strict(answer).chars() {
                        assert!(reach.contains(&c), "locale {code}: char {c:?} in {w:?} not reachable on keyboard");
                    }
                }
            }
        }
    }
    /// Vietnamese needs a space key, and the character that provides it is
    /// invisible at the end of a string literal.
    ///
    /// A Vietnamese word is space-separated syllables -- học sinh, thành phố,
    /// cà phê. Without this key a player is shown a word they cannot type.
    /// 86% of the Vietnamese vocabulary in the corpus is multi-syllable, and the
    /// single syllables that remain are 2-6 characters, so hard and expert
    /// cannot be filled from any source without it.
    ///
    /// Grading does not require the space -- `fold_strict` drops whitespace, so
    /// `họcsinh` already matches `học sinh` -- which is what makes the key an
    /// affordance rather than a new way to be wrong. It still has to EXIST.
    #[test]
    fn a_vietnamese_player_can_type_a_space() {
        let vi = layout_for(crate::consts::VI);
        assert!(
            vi.rows.iter().any(|r| r.contains(' ')),
            "the vi layout lost its space key; a Vietnamese word cannot be typed without one"
        );
        // And nobody else grew one by accident: a space on an English keyboard
        // would let a player submit two words as one answer.
        for code in [crate::consts::EN, crate::consts::ES, crate::consts::FR, crate::consts::DE] {
            assert!(
                !layout_for(code).rows.iter().any(|r| r.contains(' ')),
                "{code} has a space key and should not"
            );
        }
    }

}
