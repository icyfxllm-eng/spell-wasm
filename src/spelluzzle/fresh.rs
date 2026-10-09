//! F11 freshness: a board nobody has seen lately, sharing few words with the
//! last one. A pure function over the history so the host can run it 500 times.

use super::gen::{generate_after, GenError};
use super::lex::Lexicon;
use super::store::History;
use super::types::{Board, Tier};

/// Draw fresh seeds until a board's hash is not in `history`. Returns the board
/// and the seed that made it, which resuming needs. Never fails for want of a fresh board unless the
/// pool genuinely cannot make one (the caller then shows the unavailable state).
pub fn fresh_board(
    lex: &Lexicon,
    tier: Tier,
    history: &History,
    previous: &[String],
    mut next_seed: impl FnMut() -> u64,
) -> Result<(Board, u64), GenError> {
    let mut last = None;
    for _ in 0..64 {
        let seed = next_seed();
        match generate_after(seed, tier, lex, previous) {
            Ok(g) if !history.contains(g.board.hash()) => return Ok((g.board, seed)),
            Ok(_) => {}
            Err(e) => last = Some(e),
        }
    }
    Err(last.unwrap_or(GenError::EmptyPool))
}

/// A board for a tier that needs verified seeds (Medium to Expert): draw from the
/// seeds the build step verified against the large validity list. The board for a
/// seed is fixed, so the overlap rule is met by choosing among seeds, not by steering
/// the generator. Prefers a board sharing at most `MAX_SHARED` words with `previous`;
/// after enough tries it takes the one sharing fewest, and never fails for that.
pub fn fresh_verified(
    lex: &Lexicon,
    tier: Tier,
    seeds: &[u32],
    history: &History,
    previous: &[String],
    mut pick: impl FnMut(usize) -> usize,
) -> Option<(Board, u64)> {
    if seeds.is_empty() {
        return None;
    }
    let mut best: Option<(usize, Board, u64)> = None;
    for _ in 0..200 {
        let seed = seeds[pick(seeds.len()) % seeds.len()] as u64;
        let Ok(g) = super::gen::generate(seed, tier, lex) else { continue };
        if history.contains(g.board.hash()) {
            continue;
        }
        let shared = g.board.words().iter().filter(|w| previous.contains(w)).count();
        if shared <= super::gen::MAX_SHARED {
            return Some((g.board, seed));
        }
        if best.as_ref().map_or(true, |(s, _, _)| shared < *s) {
            best = Some((shared, g.board, seed));
        }
    }
    best.map(|(_, b, s)| (b, s))
}
