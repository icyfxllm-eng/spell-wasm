//! Includes the REAL engine sources by path (no copy), under the module names
//! they expect: `crate::consts`, `crate::spelldoku::rng`, `crate::boardgame`.
#![allow(dead_code)]

#[path = "../../../src/consts.rs"]
mod consts;

#[path = "../../../src/spelldoku/rng.rs"]
pub mod rng_impl;

mod spelldoku {
    pub use super::rng_impl as rng;
}

#[path = "../../../src/boardgame/mod.rs"]
mod boardgame;

#[no_mangle]
pub extern "C" fn golden_rng_1000() -> u64 {
    boardgame::golden::rng_1000()
}

#[no_mangle]
pub extern "C" fn golden_game() -> u64 {
    boardgame::golden::game()
}

#[no_mangle]
pub extern "C" fn golden_game_sprint() -> u64 {
    boardgame::golden::game_sprint()
}

#[no_mangle]
pub extern "C" fn golden_game_stretch() -> u64 {
    boardgame::golden::game_stretch()
}

#[no_mangle]
pub extern "C" fn golden_game_boosts() -> u64 {
    boardgame::golden::game_boosts()
}

#[no_mangle]
pub extern "C" fn golden_game_streak() -> u64 {
    boardgame::golden::game_streak()
}

#[no_mangle]
pub extern "C" fn golden_game_all() -> u64 {
    boardgame::golden::game_all()
}
