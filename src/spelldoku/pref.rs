//! CC-SPELLDOKU-RULES v1 F5 — which symbols a difficulty uses, as the player
//! chose it. Pure: the screen owns the storage and hands the stored value in.
//!
//! D-R3 (Eric, 2026-09-22): Numbers on Easy and Medium, so a newcomer starts
//! with the Sudoku they know; Letters on Hard and Expert, which is what testers
//! see today and is where vocabulary progression belongs. Every tier offers all
//! three, including Mix (D-R12).

use super::gen::Tier;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pref {
    Numbers,
    Letters,
    /// Each new board takes one kind or the other, never three the same in a
    /// row. The generator never sees this: it is resolved first (D-R12).
    Mix,
}

impl Pref {
    pub fn id(self) -> &'static str {
        match self {
            Pref::Numbers => "numbers",
            Pref::Letters => "letters",
            Pref::Mix => "mix",
        }
    }

    pub fn parse(s: &str) -> Option<Pref> {
        match s {
            "numbers" => Some(Pref::Numbers),
            "letters" => Some(Pref::Letters),
            "mix" => Some(Pref::Mix),
            _ => None,
        }
    }

    /// The i18n key for this option in the picker.
    pub fn key(self) -> &'static str {
        match self {
            Pref::Numbers => "sd.tier.off",
            Pref::Letters => "sd.tier.letters",
            Pref::Mix => "sd.sym.mix",
        }
    }
}

/// D-R3's defaults, for a profile that has never chosen.
pub fn default_for(tier: Tier) -> Pref {
    match tier {
        Tier::Easy | Tier::Medium => Pref::Numbers,
        Tier::Hard | Tier::Expert => Pref::Letters,
    }
}

/// What the NEXT board should be: true for letters.
///
/// `recent` is the kinds of this tier's last boards, newest first, as `true`
/// for letters. `letters_ok` is F13 eligibility — when letters cannot be drawn
/// for this language, size and tier, Mix resolves to numbers and a stored
/// Letters preference is honoured as numbers WITHOUT being overwritten, so it
/// comes back if eligibility does.
pub fn resolve(pref: Pref, letters_ok: bool, recent: &[bool], seed: u64) -> bool {
    if !letters_ok {
        return false;
    }
    match pref {
        Pref::Numbers => false,
        Pref::Letters => true,
        Pref::Mix => {
            // Never a third of the same kind in a row.
            match recent {
                [a, b, ..] if a == b => !*a,
                _ => seed & 1 == 1,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_follow_d_r3() {
        assert_eq!(default_for(Tier::Easy), Pref::Numbers);
        assert_eq!(default_for(Tier::Medium), Pref::Numbers);
        assert_eq!(default_for(Tier::Hard), Pref::Letters);
        assert_eq!(default_for(Tier::Expert), Pref::Letters);
    }

    #[test]
    fn a_plain_preference_is_obeyed() {
        for seed in 0..8 {
            assert!(!resolve(Pref::Numbers, true, &[], seed));
            assert!(resolve(Pref::Letters, true, &[], seed));
        }
    }

    /// Done #16a: over a long run of Mix boards both kinds appear and no run of
    /// three of a kind ever occurs.
    #[test]
    fn mix_alternates_without_ever_running_three_deep() {
        let mut recent: Vec<bool> = Vec::new();
        let mut letters = 0;
        let mut numbers = 0;
        for seed in 0..1_000u64 {
            let k = resolve(Pref::Mix, true, &recent, seed.wrapping_mul(0x9E37_79B9));
            if k { letters += 1 } else { numbers += 1 }
            recent.insert(0, k);
            recent.truncate(2);
            assert!(
                !(recent.len() == 2 && recent[0] == recent[1] && seen_three(&recent, k)),
                "three of a kind in a row"
            );
        }
        assert!(letters > 100 && numbers > 100, "both kinds appear ({letters} letters, {numbers} numbers)");
    }

    fn seen_three(recent: &[bool], next: bool) -> bool {
        recent.len() == 2 && recent[0] == recent[1] && recent[0] == next
    }

    /// The cap, stated directly: after two of a kind the next one must flip.
    #[test]
    fn two_of_a_kind_forces_the_other() {
        for seed in 0..64u64 {
            assert!(!resolve(Pref::Mix, true, &[true, true], seed), "two letters force numbers");
            assert!(resolve(Pref::Mix, true, &[false, false], seed), "two numbers force letters");
        }
    }

    /// F5: with letters ineligible every preference serves numbers, and the
    /// stored preference is the caller's to keep -- this function never edits it.
    #[test]
    fn ineligible_letters_fall_back_without_forgetting_the_choice() {
        for pref in [Pref::Numbers, Pref::Letters, Pref::Mix] {
            for seed in 0..8 {
                assert!(!resolve(pref, false, &[], seed), "{pref:?} must serve numbers");
            }
        }
    }
}
