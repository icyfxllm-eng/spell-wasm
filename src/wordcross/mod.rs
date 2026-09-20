//! CC-WORDGRID Phase B — Spell Cross: a criss-cross of a list's own words,
//! every clue heard and never read (D1, D7). App only, like Spell Search,
//! whose F-X engine it reuses: the same ledger and seeds (F-X2, F-X3), the
//! same hint filter (F-X4), the same confusion list (F-C2), and the base
//! game's own stats (F-X6).

pub mod layout;
pub mod serve;
pub mod traps;

#[cfg(test)]
mod tests;
