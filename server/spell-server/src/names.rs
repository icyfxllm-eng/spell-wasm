//! G-D4 / F10 — display names.
//!
//! Adults type a name (14 characters, the local Head-to-Head limit) and it
//! must pass the app's profanity filter. Kids never choose: the server makes
//! `<Color> <Piece> <NN>` from fixed lists in the kid's UI language, with each
//! language's own word order and agreement (a red bee is "Abeja Roja", "Rote
//! Biene", "Красная Пчела").
//!
//! MACHINE-DRAFTED (O12): every non-English list below is a draft awaiting a
//! native speaker's audit, the same standing as the app's `needs_audit`
//! locales. The pieces are BoardGame's four (wizard hat, wand, bee, book),
//! which the app shows only as icons, so these words are new strings.

use crate::store::random_bytes;

pub const MAX_NAME_CHARS: usize = 14;

#[derive(Clone, Copy)]
enum G {
    M,
    F,
    N,
}

#[derive(Clone, Copy)]
enum Order {
    /// Color before piece ("Red Bee").
    AdjNoun,
    /// Piece before color ("Abeja Roja").
    NounAdj,
}

struct Lists {
    /// Eight colors, each as [masculine, feminine, neuter].
    colors: [[&'static str; 3]; 8],
    pieces: [(&'static str, G); 4],
    order: Order,
    sep: &'static str,
}

/// One form for every gender.
const fn same(s: &'static str) -> [&'static str; 3] {
    [s, s, s]
}

const fn mf(m: &'static str, f: &'static str) -> [&'static str; 3] {
    [m, f, m]
}

/// The app's UI locales (src/i18n/locales).
pub const UI_LANGS: [&str; 15] = ["ar", "de", "en", "es", "fil", "fr", "hi", "ja", "ko", "pl", "pt", "ru", "sw", "vi", "zh"];

pub fn ui_supported(lang: &str) -> bool {
    UI_LANGS.contains(&lang)
}

fn lists(lang: &str) -> Lists {
    use G::*;
    use Order::*;
    match lang {
        "es" => Lists {
            colors: [
                mf("Rojo", "Roja"),
                same("Azul"),
                same("Verde"),
                mf("Amarillo", "Amarilla"),
                mf("Morado", "Morada"),
                same("Naranja"),
                same("Rosa"),
                mf("Plateado", "Plateada"),
            ],
            pieces: [("Sombrero", M), ("Varita", F), ("Abeja", F), ("Libro", M)],
            order: NounAdj,
            sep: " ",
        },
        "fr" => Lists {
            colors: [
                same("Rouge"),
                mf("Bleu", "Bleue"),
                mf("Vert", "Verte"),
                same("Jaune"),
                mf("Violet", "Violette"),
                same("Orange"),
                same("Rose"),
                mf("Argenté", "Argentée"),
            ],
            pieces: [("Chapeau", M), ("Baguette", F), ("Abeille", F), ("Livre", M)],
            order: NounAdj,
            sep: " ",
        },
        "pt" => Lists {
            colors: [
                mf("Vermelho", "Vermelha"),
                same("Azul"),
                same("Verde"),
                mf("Amarelo", "Amarela"),
                mf("Roxo", "Roxa"),
                same("Laranja"),
                same("Rosa"),
                mf("Prateado", "Prateada"),
            ],
            pieces: [("Chapéu", M), ("Varinha", F), ("Abelha", F), ("Livro", M)],
            order: NounAdj,
            sep: " ",
        },
        "de" => Lists {
            // Strong nominative, no article: Roter Hut, Rote Biene, Rotes Buch.
            colors: [
                ["Roter", "Rote", "Rotes"],
                ["Blauer", "Blaue", "Blaues"],
                ["Grüner", "Grüne", "Grünes"],
                ["Gelber", "Gelbe", "Gelbes"],
                same("Lila"),
                same("Orange"),
                same("Rosa"),
                ["Silberner", "Silberne", "Silbernes"],
            ],
            pieces: [("Hut", M), ("Zauberstab", M), ("Biene", F), ("Buch", N)],
            order: AdjNoun,
            sep: " ",
        },
        "pl" => Lists {
            colors: [
                mf("Czerwony", "Czerwona"),
                mf("Niebieski", "Niebieska"),
                mf("Zielony", "Zielona"),
                mf("Żółty", "Żółta"),
                mf("Fioletowy", "Fioletowa"),
                mf("Pomarańczowy", "Pomarańczowa"),
                mf("Różowy", "Różowa"),
                mf("Srebrny", "Srebrna"),
            ],
            pieces: [("Kapelusz", M), ("Różdżka", F), ("Pszczoła", F), ("Książka", F)],
            order: AdjNoun,
            sep: " ",
        },
        "ru" => Lists {
            // All four pieces are feminine nouns.
            colors: [
                mf("Красный", "Красная"),
                mf("Синий", "Синяя"),
                mf("Зелёный", "Зелёная"),
                mf("Жёлтый", "Жёлтая"),
                mf("Фиолетовый", "Фиолетовая"),
                mf("Оранжевый", "Оранжевая"),
                mf("Розовый", "Розовая"),
                mf("Серебряный", "Серебряная"),
            ],
            pieces: [("Шляпа", F), ("Палочка", F), ("Пчела", F), ("Книга", F)],
            order: AdjNoun,
            sep: " ",
        },
        "ar" => Lists {
            colors: [
                mf("أحمر", "حمراء"),
                mf("أزرق", "زرقاء"),
                mf("أخضر", "خضراء"),
                mf("أصفر", "صفراء"),
                mf("بنفسجي", "بنفسجية"),
                mf("برتقالي", "برتقالية"),
                mf("وردي", "وردية"),
                mf("فضي", "فضية"),
            ],
            pieces: [("قبعة", F), ("عصا", F), ("نحلة", F), ("كتاب", M)],
            order: NounAdj,
            sep: " ",
        },
        "hi" => Lists {
            // All four pieces are feminine; the colors are their feminine forms.
            colors: [
                same("लाल"),
                same("नीली"),
                same("हरी"),
                same("पीली"),
                same("बैंगनी"),
                same("नारंगी"),
                same("गुलाबी"),
                same("सुनहरी"),
            ],
            pieces: [("टोपी", F), ("छड़ी", F), ("मधुमक्खी", F), ("किताब", F)],
            order: AdjNoun,
            sep: " ",
        },
        "sw" => Lists {
            // "<piece> <connector> <color>": the connector follows the noun
            // class (kofia ya, nyuki wa, kitabu cha), so it rides on the piece.
            colors: [
                same("bluu"),
                same("kijani"),
                same("njano"),
                same("zambarau"),
                same("pinki"),
                same("kijivu"),
                same("kahawia"),
                same("fedha"),
            ],
            pieces: [("Kofia ya", N), ("Fimbo ya", N), ("Nyuki wa", N), ("Kitabu cha", N)],
            order: NounAdj,
            sep: " ",
        },
        "fil" => Lists {
            // The linker is part of the color: -ng after a vowel, "na" after a consonant.
            colors: [
                same("Pulang"),
                same("Asul na"),
                same("Berdeng"),
                same("Dilaw na"),
                same("Lilang"),
                same("Kahel na"),
                same("Rosas na"),
                same("Pilak na"),
            ],
            pieces: [("Sumbrero", N), ("Baston", N), ("Bubuyog", N), ("Aklat", N)],
            order: AdjNoun,
            sep: " ",
        },
        "vi" => Lists {
            colors: [
                same("Đỏ"),
                same("Xanh Dương"),
                same("Xanh Lá"),
                same("Vàng"),
                same("Tím"),
                same("Cam"),
                same("Hồng"),
                same("Bạc"),
            ],
            pieces: [("Mũ", N), ("Đũa Phép", N), ("Ong", N), ("Sách", N)],
            order: NounAdj,
            sep: " ",
        },
        "ja" => Lists {
            colors: [
                same("赤い"),
                same("青い"),
                same("緑の"),
                same("黄色い"),
                same("紫の"),
                same("オレンジの"),
                same("ピンクの"),
                same("銀の"),
            ],
            pieces: [("ぼうし", N), ("つえ", N), ("ハチ", N), ("ほん", N)],
            order: AdjNoun,
            sep: "",
        },
        "ko" => Lists {
            colors: [
                same("빨간"),
                same("파란"),
                same("초록색"),
                same("노란"),
                same("보라색"),
                same("주황색"),
                same("분홍색"),
                same("은색"),
            ],
            pieces: [("모자", N), ("지팡이", N), ("꿀벌", N), ("책", N)],
            order: AdjNoun,
            sep: " ",
        },
        "zh" => Lists {
            colors: [
                same("红色"),
                same("蓝色"),
                same("绿色"),
                same("黄色"),
                same("紫色"),
                same("橙色"),
                same("粉色"),
                same("银色"),
            ],
            pieces: [("帽子", N), ("魔杖", N), ("蜜蜂", N), ("书本", N)],
            order: AdjNoun,
            sep: "",
        },
        _ => Lists {
            colors: [
                same("Red"),
                same("Blue"),
                same("Green"),
                same("Yellow"),
                same("Purple"),
                same("Orange"),
                same("Pink"),
                same("Silver"),
            ],
            pieces: [("Hat", N), ("Wand", N), ("Bee", N), ("Book", N)],
            order: AdjNoun,
            sep: " ",
        },
    }
}

fn compose(lang: &str, color: usize, piece: usize, nn: u8) -> String {
    let l = lists(lang);
    let (p, g) = l.pieces[piece % 4];
    let c = l.colors[color % 8][g as usize];
    match l.order {
        Order::AdjNoun => format!("{c}{}{p} {nn:02}", l.sep),
        Order::NounAdj => format!("{p}{}{c} {nn:02}", l.sep),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum NameError {
    Empty,
    TooLong,
    Rejected,
}

/// Validates an adult's typed name.
pub fn adult(raw: &str) -> Result<String, NameError> {
    let name: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        return Err(NameError::Empty);
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(NameError::TooLong);
    }
    if name.chars().any(|c| c.is_control()) {
        return Err(NameError::Rejected);
    }
    // Check the whole name and each word, so "nice <slur>" cannot hide a term.
    if spell_wasm::native_api::is_blocked(&name) || name.split(' ').any(spell_wasm::native_api::is_blocked) {
        return Err(NameError::Rejected);
    }
    Ok(name)
}

/// A generated kid name in the kid's UI language. Any `display_name` the
/// kid's client sent is ignored (A10.3). `ui_lang` must be a UI locale (the
/// connection refuses any other before a name is made).
pub fn kid(ui_lang: &str) -> String {
    loop {
        let b = random_bytes::<3>();
        let name = compose(ui_lang, b[0] as usize, b[1] as usize, b[2] % 100);
        // The filter's leet layer reads digits as letters ("kijani 99" folds
        // to "...nigg"), so a generated name is checked like a typed one.
        if !blocked(&name) {
            return name;
        }
    }
}

fn blocked(name: &str) -> bool {
    spell_wasm::native_api::is_blocked(name) || name.split(' ').any(spell_wasm::native_api::is_blocked)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adult_names() {
        assert_eq!(adult("  Grandma   Jo "), Ok("Grandma Jo".into()));
        assert_eq!(adult(""), Err(NameError::Empty));
        assert_eq!(adult("abcdefghijklmno"), Err(NameError::TooLong));
        assert_eq!(adult("a\u{7}b"), Err(NameError::Rejected));
        assert_eq!(adult("fuck"), Err(NameError::Rejected));
        assert_eq!(adult("hi fuck"), Err(NameError::Rejected));
    }

    #[test]
    fn every_locale_composes_every_name_with_agreement() {
        assert_eq!(compose("en", 0, 2, 7), "Red Bee 07");
        assert_eq!(compose("es", 0, 2, 7), "Abeja Roja 07");
        assert_eq!(compose("es", 0, 3, 7), "Libro Rojo 07");
        assert_eq!(compose("fr", 1, 1, 42), "Baguette Bleue 42");
        assert_eq!(compose("de", 0, 2, 1), "Rote Biene 01");
        assert_eq!(compose("de", 0, 3, 1), "Rotes Buch 01");
        assert_eq!(compose("de", 0, 0, 1), "Roter Hut 01");
        assert_eq!(compose("ru", 0, 2, 3), "Красная Пчела 03");
        assert_eq!(compose("ar", 0, 3, 3), "كتاب أحمر 03");
        assert_eq!(compose("ar", 0, 2, 3), "نحلة حمراء 03");
        assert_eq!(compose("ja", 0, 2, 5), "赤いハチ 05");
        assert_eq!(compose("sw", 0, 2, 5), "Nyuki wa bluu 05");
        assert_eq!(compose("fil", 1, 2, 5), "Asul na Bubuyog 05");
        let mut blocked_combos = Vec::new();
        for lang in UI_LANGS {
            for c in 0..8 {
                for p in 0..4 {
                    for nn in 0..100 {
                        let n = compose(lang, c, p, nn);
                        assert!(n.ends_with(&format!(" {nn:02}")) && n.chars().count() > 4, "{lang}: {n}");
                        if blocked(&n) {
                            blocked_combos.push(n);
                        }
                    }
                }
            }
        }
        // The filter rejects some numbers outright (hate codes) and reads
        // digits as letters, so a few combinations are redrawn by `kid()`.
        // They must stay rare enough that the redraw is instant.
        let total = UI_LANGS.len() * 8 * 4 * 100;
        let nums: std::collections::BTreeSet<&str> =
            blocked_combos.iter().filter_map(|n| n.rsplit(' ').next()).collect();
        println!("blocked: {} of {total}; numbers involved: {nums:?}", blocked_combos.len());
        assert!(blocked_combos.len() * 20 < total, "over 5% blocked: {nums:?}");
    }

    #[test]
    fn a_generated_kid_name_never_trips_the_filter() {
        for lang in UI_LANGS {
            for _ in 0..2000 {
                let n = kid(lang);
                assert!(!blocked(&n), "{lang}: {n}");
            }
        }
    }

    #[test]
    fn ui_languages_are_the_apps_locales() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../src/i18n/locales");
        let mut found: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok()?.file_name().to_str()?.strip_suffix(".json").map(str::to_string))
            .collect();
        found.sort();
        assert_eq!(found, UI_LANGS.to_vec(), "a UI locale was added or removed: update the kid-name lists");
    }
}
