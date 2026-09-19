//! Locale tables are baked in with `include_str!`, so CC-PICTURE-PLATFORM I1
//! ("no byte of Spell Picture ... string exists in any web deploy artifact")
//! cannot be met by a cfg alone -- the strings are data, not code, and the
//! compiler keeps every one of them.
//!
//! So the tables are staged through OUT_DIR. The app build copies them
//! verbatim; the site build (`--features web`) drops every Spell Picture key
//! on the way through. i18n.rs always includes from OUT_DIR, so there is one
//! include path and no cfg to get wrong at the use site.
//!
//! Filtering is by KEY, not by value: a key is unambiguous, whereas matching
//! translated prose would be guesswork in fifteen languages.

use std::{env, fs, path::Path};

/// Keys belonging to the mode. Everything under `wordpic.` / `finale.`, plus
/// its hub-tile entries, which live under the `tools.` namespace.
fn is_picture_key(k: &str) -> bool {
    k.starts_with("wordpic.") || k.starts_with("finale.") || k.starts_with("tools.wordpic.")
}

fn main() {
    let src = Path::new("src/i18n/locales");
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("locales");
    fs::create_dir_all(&out).unwrap();
    let web = env::var("CARGO_FEATURE_WEB").is_ok();

    println!("cargo:rerun-if-changed=src/i18n/locales");
    println!("cargo:rerun-if-changed=build.rs");

    let mut dropped = 0usize;
    for entry in fs::read_dir(src).expect("locales dir") {
        let p = entry.unwrap().path();
        if p.extension().map(|e| e != "json").unwrap_or(true) {
            continue;
        }
        println!("cargo:rerun-if-changed={}", p.display());
        let body = fs::read_to_string(&p).unwrap();
        let staged = if web {
            // Deliberately a line filter rather than a JSON round-trip: it
            // preserves the file's own formatting and key order, so a diff of
            // app-vs-site tables shows only the removals.
            let kept: Vec<&str> = body
                .lines()
                .filter(|l| {
                    let k = l.trim_start().trim_start_matches('"');
                    let k = &k[..k.find('"').unwrap_or(0)];
                    let drop = is_picture_key(k);
                    if drop {
                        dropped += 1;
                    }
                    !drop
                })
                .collect();
            // The last surviving entry must not keep a trailing comma.
            let mut s = kept.join("\n");
            if let Some(i) = s.rfind(",\n}") {
                s.replace_range(i..i + 1, "");
            }
            s
        } else {
            body
        };
        fs::write(out.join(p.file_name().unwrap()), staged).unwrap();
    }

    // The mode registry, same treatment. modes.rs already filters by platform
    // at RUNTIME, which D1 rejects outright: a filtered entry is still in the
    // bundle, still names the mode, and is one DevTools session from being
    // un-filtered. On the site build the entry is deleted before it is
    // compiled in.
    let modes = fs::read_to_string("config/modes.json").expect("modes.json");
    println!("cargo:rerun-if-changed=config/modes.json");
    let staged_modes = if web { strip_app_only_modes(&modes) } else { modes };
    fs::write(Path::new(&env::var("OUT_DIR").unwrap()).join("modes.json"), staged_modes).unwrap();

    // CC-HUMAN-AUDIO e2e fixture. The testseam build compiles in a manifest
    // that maps every English bank word to one test clip, so the browser suite
    // can exercise the human provider for whatever word it is served. It is
    // inert until a spec arms it (human_audio.rs), and a production build never
    // sees it: it only exists under --features testseam.
    if env::var("CARGO_FEATURE_TESTSEAM").is_ok() {
        println!("cargo:rerun-if-changed=assets/words/en");
        let mut words: Vec<String> = Vec::new();
        for tier in ["easy", "medium", "hard", "expert"] {
            let p = format!("assets/words/en/{tier}.txt");
            if let Ok(txt) = fs::read_to_string(&p) {
                words.extend(txt.lines().map(str::trim).filter(|w| !w.is_empty() && !w.starts_with('#')).map(String::from));
            }
        }
        let clips: String = words
            .iter()
            .map(|w| format!("{}:\"fixture.m4a\"", serde_json_string(w)))
            .collect::<Vec<_>>()
            .join(",");
        let json = format!("{{\"version\":1,\"langs\":{{\"en\":{{\"base\":\"human-audio/en/\",\"clips\":{{{clips}}}}}}}}}");
        fs::write(Path::new(&env::var("OUT_DIR").unwrap()).join("human-audio-fixture.json"), json).unwrap();
    }

    if web {
        println!("cargo:warning=site build: dropped {dropped} Spell Picture locale strings");
    }
}

/// Remove every registry entry that does not declare the `web` platform.
///
/// Rebuilds the `modes` array from the entries that survive rather than
/// splicing text around the ones that go. The first attempt did comma
/// surgery -- delete the entry, drop the comma before it AND the one after
/// it -- which removed two separators for one entry and emitted JSON that
/// would not parse. The registry tests caught it immediately, which is the
/// argument for keeping data this load-bearing under test.
fn strip_app_only_modes(src: &str) -> String {
    let Some(arr) = src.find("\"modes\"").and_then(|i| src[i..].find('[').map(|j| i + j)) else {
        return src.to_string();
    };
    let bytes = src.as_bytes();
    let mut entries: Vec<&str> = Vec::new();
    let (mut i, mut depth, mut start) = (arr + 1, 0i32, 0usize);
    let mut arr_end = src.len();
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                if depth == 0 {
                    start = i;
                }
                depth += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    entries.push(&src[start..=i]);
                }
            }
            b']' if depth == 0 => {
                arr_end = i;
                break;
            }
            _ => {}
        }
        i += 1;
    }
    let kept: Vec<&str> = entries
        .into_iter()
        .filter(|e| {
            e.find("\"platforms\"")
                .and_then(|i| e[i..].find(']').map(|j| &e[i..i + j]))
                .map(|block| block.contains("\"web\""))
                .unwrap_or(true)
        })
        .collect();
    format!("{}\n    {}\n  {}", &src[..=arr], kept.join(",\n    "), &src[arr_end..])
}

/// Minimal JSON string quoting for the fixture (build scripts avoid a serde dep).
fn serde_json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
