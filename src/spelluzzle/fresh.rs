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
