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
