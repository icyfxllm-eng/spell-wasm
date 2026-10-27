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
    k.starts_with("wordpic.") || k.starts_with("finale.") || k.starts_with("tools.wordpic.") || is_boardgame_key(k) || is_spelluzzle_key(k) || is_randospell_key(k) || is_quickbee_draw_key(k) || is_online_key(k)
}

/// CC-BOARD-GAME I13: every string of the app-only Board Game mode lives under
/// `bg.`, so the site build can drop them by prefix, exactly as above.
/// CC-GAME-SERVER O1: online play is app only, so the site carries none of its strings.
fn is_online_key(k: &str) -> bool {
    k.starts_with("ol.")
}

fn is_boardgame_key(k: &str) -> bool {
    k.starts_with("bg.")
}

/// CC-RANDOSPELL O3: every string of the app-only Randospell mode lives under `rs.`.
fn is_randospell_key(k: &str) -> bool {
    k.starts_with("rs.")
}

/// CC-QUICKBEE-DRAW: every string of the app-only drawn Quick Bee lives under `qbd.`.
fn is_quickbee_draw_key(k: &str) -> bool {
    k.starts_with("qbd.")
}

/// CC-SPELLUZZLE O1: every string of the app-only Spelluzzle mode lives under `spz.`.
fn is_spelluzzle_key(k: &str) -> bool {
    k.starts_with("spz.")
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
    println!("cargo:rerun-if-changed=src/modes_strip.rs");
    let staged_modes = if web { strip_modes(&modes, keeps_on_web) } else { modes };
    // CC-HUB-NAV AE-1 F2: a row the Education edition does not ship is deleted
    // before it is compiled in -- absent, not filtered, like the site build.
    let edu = env::var("CARGO_FEATURE_EDUCATION").is_ok();
    let staged_modes = if edu { strip_modes(&staged_modes, keeps_in_education) } else { staged_modes };
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
        println!("cargo:warning=site build: dropped {dropped} Spell Picture and Board Game locale strings");
    }
}

// The registry strips live in one file the lib tests compile too.
include!("src/modes_strip.rs");

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
