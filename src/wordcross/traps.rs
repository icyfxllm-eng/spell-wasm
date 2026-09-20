//! F-C2 trap positions and F-C3 answerability, both read-only.

use std::collections::BTreeSet;

use super::super::wordsearch::confusion;
use super::super::wordsearch::lexicon::graphemes;

/// F-C2: the positions of a word that its language's confusion list edits --
/// the letters a player is likely to get wrong. Russian's `ambiguous_positions`
/// (CC-RU-ORTHO) does not exist yet, and the spec says not to wait for it: a
/// language with no confusion list simply has no trap positions, and its
/// crossings are unweighted.
pub fn trap_positions(lang: &str, word: &str) -> Vec<usize> {
    if !confusion::has_decoys(lang) {
        return Vec::new();
    }
    let g = graphemes(word);
    let mut out: BTreeSet<usize> = BTreeSet::new();
    for e in confusion::edits(lang, word) {
        let eg = graphemes(&e);
        let at = (0..g.len().max(eg.len())).find(|&i| g.get(i) != eg.get(i)).unwrap_or(0);
        out.insert(at.min(g.len().saturating_sub(1)));
    }
    out.into_iter().collect()
}

/// F-C3: the positions at which a crossing tells this word apart from every
/// other spelling that sounds like it. None when the word is in no collision
/// group (nothing to tell apart); an empty list when no single position can.
pub fn distinguishing(lang: &str, word: &str) -> Option<Vec<usize>> {
    let members = crate::homophones::group_members(lang, word);
    if members.is_empty() {
        return None;
    }
    let g = graphemes(word);
    let others: Vec<Vec<String>> = members.iter().map(|m| graphemes(m)).collect();
    Some((0..g.len()).filter(|&i| others.iter().all(|o| o.get(i) != g.get(i))).collect())
}
