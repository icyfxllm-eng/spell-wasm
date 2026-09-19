//! I14 — a board's N symbols as the core sees them.
//!
//! Symbol `v` (1..=N) is the list of its accepted forms, and each form is a
//! sequence of opaque glyph ids. Nothing here knows what a glyph is, which
//! language it belongs to, or whether the symbol is a number or a bank word:
//! the binding layer (`bind`) turns graphemes into ids before generation and
//! ids back into graphemes for the screen. Fragment clues (F2) are patterns
//! over these ids, so the generator, solver, grader and canonical hasher stay
//! the same whatever the symbols are.

pub struct Symbols {
    n: usize,
    /// Aim fragment search at glyphs another symbol shares (see `aimed`).
    aimed: bool,
    /// Index `v - 1`: the accepted forms of symbol `v`.
    forms: Vec<Vec<Vec<u32>>>,
}

impl Symbols {
    pub fn new(forms: Vec<Vec<Vec<u32>>>) -> Self {
        Symbols { n: forms.len(), aimed: false, forms }
    }

    /// Symbols whose forms rarely line up (N unrelated forms, not the number
    /// words 1..N): the generator cuts fragments only where another symbol of
    /// the same length shares a glyph at the same position, since a random
    /// reveal almost never lands there.
    pub fn aimed(forms: Vec<Vec<Vec<u32>>>) -> Self {
        Symbols { n: forms.len(), aimed: true, forms }
    }

    pub fn is_aimed(&self) -> bool {
        self.aimed
    }

    /// For symbol `v`: its first form, and for every other symbol whose first
    /// form has the same length, the positions where the two agree -- when they
    /// agree somewhere but not everywhere.
    pub fn agreements(&self, v: u8) -> Vec<Vec<usize>> {
        let Some(f) = self.first(v) else { return Vec::new() };
        (1..=self.n as u8)
            .filter(|&u| u != v)
            .filter_map(|u| self.first(u))
            .filter(|g| g.len() == f.len())
            .map(|g| (0..f.len()).filter(|&i| f[i] == g[i]).collect::<Vec<usize>>())
            .filter(|a| !a.is_empty() && a.len() < f.len())
            .collect()
    }

    pub fn n(&self) -> usize {
        self.n
    }

    /// The first accepted form of symbol `v`: the one a fragment is cut from.
    pub fn first(&self, v: u8) -> Option<&[u32]> {
        self.forms.get((v as usize).checked_sub(1)?).and_then(|f| f.first()).map(|f| f.as_slice())
    }

    /// F2: the symbols with an accepted form matching the pattern position by
    /// position (None = hidden), as a bit mask over 1..=N.
    pub fn fragment_set(&self, pattern: &[Option<u32>]) -> u16 {
        let mut m = 0u16;
        for (i, forms) in self.forms.iter().enumerate() {
            let fits = forms.iter().any(|f| {
                f.len() == pattern.len() && f.iter().zip(pattern).all(|(c, p)| p.map_or(true, |p| p == *c))
            });
            if fits {
                m |= 1u16 << (i + 1);
            }
        }
        m
    }

    /// Can these symbols make a fragment at all -- some partly hidden form that
    /// still fits two or more symbols? Hard and Expert need a NECESSARY
    /// fragment (F2), so a symbol set that cannot make one cannot serve them.
    pub fn fragments_possible(&self) -> bool {
        (1..=self.n as u8).any(|v| {
            let Some(form) = self.first(v) else { return false };
            let len = form.len();
            if len > 16 {
                return false;
            }
            (1..len).any(|hide| {
                (0..(1u32 << len)).filter(|m| m.count_ones() as usize == hide).any(|mask| {
                    let p: Vec<Option<u32>> =
                        form.iter().enumerate().map(|(i, c)| if mask & (1 << i) != 0 { None } else { Some(*c) }).collect();
                    self.fragment_set(&p).count_ones() >= 2
                })
            })
        })
    }
}
