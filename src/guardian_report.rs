//! CC-GUARDIAN-REPORT v1 Phase A — word types (F2), the practice list (F3)
//! and the Monday ping's payload (F7), on top of the CC-REPORTS-SHARE composer
//! (the F1 window, the buckets, the closed lexicon), which this extends and
//! does not fork. Pure: the clock, the zone and the bank view are injected, so
//! the same inputs give the same bytes (I10). Read-only over `LearnerQuery`
//! (I1); no storage, DOM or network (I2). The screen is `reports_share_ui.rs`
//! and the scheduler is `notifications.rs`.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::rc::Rc;

use crate::learner_query::{AttemptDetail, ProfileId};
use crate::reports_share::{self as rs, ComposeInput, Report, TypeGroup, Which, Zone};

const DAY_S: i64 = 86_400;
/// G-D18: at most 10 practice words per language.
pub const PRACTICE_MAX: usize = 10;
/// T-LONG: at least 8 units in the bank's stored unit.
pub const LONG_MIN: usize = 8;
/// G-D17: groups under 3 words are hidden, at most 4 are shown.
const MIN_GROUP: usize = 3;
const MAX_GROUPS: usize = 4;
const GROUP_WORDS: usize = 5;
/// F3.3: Spell Jr practises Easy and Medium only.
const JR_CAP: usize = 1;
/// F7: the ping fires Monday 08:00 local (O3 default).
pub const PING_HOUR: u32 = 8;

// --------------------------------------------------------------- registry

/// F2's closed type registry (v1). No other types exist (I13), and the
/// declaration order IS the registry order: TRAP → SOUND → LONG → TIER (G-D17).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WordType {
    Trap(String),
    Sound,
    Long,
    Tier(usize),
}

impl WordType {
    pub fn label_key(&self) -> String {
        match self {
            WordType::Trap(rule) => format!("greport.type.trap.{rule}"),
            WordType::Sound => "greport.type.sound_alike".into(),
            WordType::Long => "greport.type.long_words".into(),
            WordType::Tier(t) => format!("greport.type.tier.{}", crate::consts::TIER_ORDER[*t]),
        }
    }
}

/// One bank row: the answer, the row as the bank stores it (the PURITY ledger's
/// key), and its tier (0..=3, easy..expert).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BankRow {
    pub word: String,
    pub row: String,
    pub tier: usize,
}

/// What the report may know about the bank. Injected (I10): the live view is
/// [`LiveBank`], tests name every column.
pub trait BankView {
    /// The lowest tier holding `word`; `None` = not a bank row.
    fn tier(&self, lang: &str, word: &str) -> Option<usize>;
    /// Length in the bank's stored unit, the one Randospell D12 reads.
    fn length(&self, word: &str) -> usize;
    /// A member of a set in a `verified` SENSE-CUE table.
    fn sound_alike(&self, lang: &str, word: &str) -> bool;
    /// `TRAP_MISS(rule_id)`. Nothing records a rule id yet (census C5).
    fn trap_rule(&self, lang: &str, word: &str) -> Option<String>;
    /// Not a proper noun and profanity-clean; for Spell Jr, on the kid list.
    fn clean(&self, lang: &str, word: &str, junior: bool) -> bool;
    /// The language has a PURITY ledger. Without one there is no top-up.
    fn has_ledger(&self, lang: &str) -> bool;
    /// The row's PURITY verdict passes.
    fn purity_pass(&self, lang: &str, row: &str) -> bool;
    /// A named auditor covers the row.
    fn audited(&self, lang: &str, word: &str) -> bool;
    /// Every bank row, in bank order.
    fn rows(&self, lang: &str) -> Vec<BankRow>;
    /// The learner store's identity for a word (`word_id0`).
    fn word_id(&self, lang: &str, word: &str) -> String;
}

/// F2: the one primary type, by TRAP → SOUND → LONG → TIER. Only a bank row
/// has a stored length, a tier or a sound-alike set, so a word from a player's
/// own list gets a type only from a trap rule. `None` = in no group (I12).
pub fn primary_type(b: &dyn BankView, lang: &str, word: &str) -> Option<WordType> {
    if let Some(rule) = b.trap_rule(lang, word) {
        return Some(WordType::Trap(rule));
    }
    let tier = b.tier(lang, word)?;
    if b.sound_alike(lang, word) {
        Some(WordType::Sound)
    } else if b.length(word) >= LONG_MIN {
        Some(WordType::Long)
    } else {
        Some(WordType::Tier(tier))
    }
}

// ----------------------------------------------------------- missed set W

/// A word of W: at least one miss-class outcome in the window.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Missed {
    word: String,
    id: String,
    misses: usize,
    last: i64,
}

/// W, in practice order: miss count descending, then most recent miss, then
/// word id (F3.1). Excluded (G-D16 as amended): Speech, Impossible, attempts
/// without their CR-1 instant or class, and every correct class (Correct and
/// OtherSense). There is no close class (L-D2) and voided rounds never log.
fn missed_set(log: &[AttemptDetail], in_window: impl Fn(i64) -> bool) -> Vec<Missed> {
    let mut by: BTreeMap<&str, Missed> = BTreeMap::new();
    for a in log.iter().filter(|a| rs::counts(a)) {
        let (Some(ts), Some(o)) = (a.ts, a.outcome) else { continue };
        let ts = ts as i64;
        if !rs::is_miss(o) || !in_window(ts) {
            continue;
        }
        let m = by.entry(a.word.as_str()).or_insert_with(|| Missed { word: a.word.clone(), id: a.word_id.clone(), misses: 0, last: ts });
        m.misses += 1;
        m.last = m.last.max(ts);
    }
    let mut w: Vec<Missed> = by.into_values().collect();
    w.sort_by(|a, b| b.misses.cmp(&a.misses).then(b.last.cmp(&a.last)).then_with(|| a.id.cmp(&b.id)));
    w
}

/// F3.3: Spell Jr is capped at Medium; a standard profile at the highest tier
/// it played in this language in the window. `None` = no tiered play.
fn tier_cap(b: &dyn BankView, lang: &str, log: &[AttemptDetail], in_window: impl Fn(i64) -> bool, junior: bool) -> Option<usize> {
    if junior {
        return Some(JR_CAP);
    }
    log.iter()
        .filter(|a| rs::counts(a) && a.outcome.is_some() && a.ts.is_some_and(|t| in_window(t as i64)))
        .filter_map(|a| b.tier(lang, &a.word))
        .max()
}

// ------------------------------------------------------------ determinism

/// FNV-1a: stable across builds and platforms, unlike the std hasher.
fn fnv(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// F3.4: the selection seed, `hash(profile_id, week_start, language)`.
pub fn seed(profile: ProfileId, monday: i64, lang: &str) -> u64 {
    fnv(&format!("{}|{}|{}", profile.0, iso(monday), lang))
}

fn shuffle<T>(v: &mut [T], mut state: u64) {
    for i in (1..v.len()).rev() {
        let j = (splitmix(&mut state) % (i as u64 + 1)) as usize;
        v.swap(i, j);
    }
}

// ---------------------------------------------------------------- compose

pub struct GuardianInput<'a> {
    pub base: ComposeInput<'a>,
    pub bank: &'a dyn BankView,
    /// The profile is Spell Jr (the tier cap and the kid list).
    pub junior: bool,
}

/// The REPORTS-SHARE report for the window, with each language's word types
/// (F2) and practice list (F3) filled in. Pure.
pub fn compose(input: &GuardianInput) -> Report {
    let mut r = rs::compose(&input.base);
    let (zone, monday, b) = (input.base.zone, r.monday, input.bank);
    let in_window = |t: i64| rs::week_monday(t, zone) == monday;
    for s in &mut r.sections {
        let lang = s.lang.clone();
        let log = input.base.q.attempt_detail_log(input.base.profile, &lang);
        let w = missed_set(&log, in_window);

        // F2: group W by primary type; hide small groups; biggest first.
        let mut by: BTreeMap<WordType, Vec<&Missed>> = BTreeMap::new();
        for m in &w {
            if let Some(t) = primary_type(b, &lang, &m.word) {
                by.entry(t).or_default().push(m);
            }
        }
        let mut shown: Vec<(WordType, Vec<&Missed>)> = by.into_iter().filter(|(_, v)| v.len() >= MIN_GROUP).collect();
        shown.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));
        shown.truncate(MAX_GROUPS);
        s.types = shown
            .iter()
            .map(|(t, v)| TypeGroup { label_key: t.label_key(), count: v.len(), words: v.iter().take(GROUP_WORDS).map(|m| m.word.clone()).collect() })
            .collect();

        // F3.1: the missed words, in order, that are bank rows inside the cap
        // or words from the player's own lists. An own-list word has no tier,
        // so no cap binds it (signed 2026-10-28); `clean` still does.
        let cap = tier_cap(b, &lang, &log, in_window, input.junior);
        let within = |word: &str| match b.tier(&lang, word) {
            Some(t) => cap.is_some_and(|c| t <= c),
            None => true,
        };
        let mut list: Vec<String> =
            w.iter().filter(|m| within(&m.word) && b.clean(&lang, &m.word, input.junior)).take(PRACTICE_MAX).map(|m| m.word.clone()).collect();

        // F3.2: top-up, same primary type, round-robin over the shown groups.
        // Every filter is binding; running out means a shorter list (F3.5).
        if let (true, true, Some(c)) = (list.len() < PRACTICE_MAX, b.has_ledger(&lang), cap) {
            let seen = input.base.q.seen_set(input.base.profile, &lang);
            let in_w: BTreeSet<&str> = w.iter().map(|m| m.word.as_str()).collect();
            let eligible: Vec<(WordType, String)> = b
                .rows(&lang)
                .into_iter()
                .filter(|r| r.tier <= c && b.audited(&lang, &r.word) && b.purity_pass(&lang, &r.row))
                .filter(|r| b.clean(&lang, &r.word, input.junior) && !in_w.contains(r.word.as_str()))
                .filter(|r| !seen.contains(&b.word_id(&lang, &r.word)))
                .filter_map(|r| primary_type(b, &lang, &r.word).map(|t| (t, r.word)))
                .collect();
            let base = seed(input.base.profile, monday, &lang);
            let mut pools: Vec<Vec<String>> = shown
                .iter()
                .enumerate()
                .map(|(i, (t, _))| {
                    let mut once = BTreeSet::new();
                    let mut pool: Vec<String> = eligible.iter().filter(|(et, w)| et == t && once.insert(w.clone())).map(|(_, w)| w.clone()).collect();
                    shuffle(&mut pool, base ^ (i as u64 + 1));
                    pool
                })
                .collect();
            'fill: loop {
                let mut took = false;
                for pool in &mut pools {
                    while let Some(word) = pool.pop() {
                        if list.contains(&word) {
                            continue;
                        }
                        list.push(word);
                        took = true;
                        if list.len() >= PRACTICE_MAX {
                            break 'fill;
                        }
                        break;
                    }
                }
                if !took {
                    break;
                }
            }
        }
        s.practice = list;
    }
    r
}

// ------------------------------------------------------------------- ping

/// The local Monday (day count) of the next 08:00 Monday strictly after now.
pub fn next_ping_monday(now_s: i64, zone: &dyn Zone) -> i64 {
    let this_monday = rs::week_monday(now_s, zone);
    let local = now_s + zone.offset_s(now_s);
    if local < this_monday * DAY_S + i64::from(PING_HOUR) * 3600 {
        this_monday
    } else {
        this_monday + 7
    }
}

/// What one profile's closing week holds, for the ping. Counts only (I7).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProfileWeek {
    pub sessions: bool,
    pub practice: usize,
    pub mastered: usize,
}

/// One profile's week starting `monday`: any logged attempt with an instant
/// in it is a session; the counts are the Guardian Report's own.
pub fn profile_week(input: &GuardianInput, monday: i64) -> ProfileWeek {
    let base = ComposeInput { which: Which::Week(monday), ..input.base };
    let r = compose(&GuardianInput { base, bank: input.bank, junior: input.junior });
    let sessions = input.base.langs.iter().any(|lang| {
        input
            .base
            .q
            .attempt_detail_log(input.base.profile, lang)
            .iter()
            .any(|a| a.ts.is_some_and(|t| rs::week_monday(t as i64, input.base.zone) == monday))
    });
    ProfileWeek {
        sessions,
        practice: r.sections.iter().map(|s| s.practice.len()).sum(),
        mastered: r.sections.iter().map(|s| s.mastered.len()).sum(),
    }
}

/// Which copy the one weekly notification carries (I11). `Guardian` where the
/// report row is reachable; `Digest` is CC-GUARDIAN-DASH F2's count line for
/// a device where it is not, through the same scheduler (Phase D merge).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PingCopy {
    Guardian,
    Digest,
}

/// A notification to schedule. Never a word, never a profile name (I7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ping {
    /// (year, month, day) of the Monday, local; fires at [`PING_HOUR`].
    pub date: (i64, u32, u32),
    pub title: String,
    pub body: String,
    /// The in-app route the action opens, `#guardian-report?week=…`.
    pub route: Option<String>,
}

/// F7: the one notification for the Monday `ping_monday`, from each profile's
/// week before it. No profile with a session = no ping (G-D12).
pub fn ping(copy: PingCopy, ping_monday: i64, weeks: &[ProfileWeek], t: &dyn Fn(&str) -> String) -> Option<Ping> {
    let active: Vec<&ProfileWeek> = weeks.iter().filter(|w| w.sessions).collect();
    if active.is_empty() {
        return None;
    }
    let fill = |key: &str, n: usize| t(key).replace("{n}", &n.to_string());
    let body = if active.len() > 1 {
        fill("greport.pingProfiles", active.len())
    } else {
        match (copy, active[0].practice) {
            (PingCopy::Guardian, 1) => t("greport.pingWord1"),
            (PingCopy::Guardian, n) => fill("greport.pingWords", n),
            (PingCopy::Digest, _) => fill("gdash.digestBody", active[0].mastered),
        }
    };
    let (title, route) = match copy {
        PingCopy::Guardian => (t("greport.name"), Some(route_for(ping_monday - 7))),
        PingCopy::Digest => (t("gdash.title"), None),
    };
    Some(Ping { date: rs::civil(ping_monday), title, body, route })
}

// ------------------------------------------------------------ identifiers

/// A local day count as `YYYY-MM-DD`.
pub fn iso(day: i64) -> String {
    let (y, m, d) = rs::civil(day);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The Monday a `YYYY-MM-DD` names, or `None` if it is not a Monday.
pub fn parse_monday(s: &str) -> Option<i64> {
    let mut p = s.split('-');
    let (y, m, d) = (p.next()?.parse().ok()?, p.next()?.parse().ok()?, p.next()?.parse().ok()?);
    if p.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let day = rs::days_from_civil(y, m, d);
    (rs::civil(day) == (y, m, d) && (day + 3).rem_euclid(7) == 0).then_some(day)
}

/// The deep link's in-app form (the router's hash).
pub fn route_for(monday: i64) -> String {
    format!("#guardian-report?week={}", iso(monday))
}

/// The week a route's `?week=` names, if it names a Monday.
pub fn week_of_route(hash: &str) -> Option<i64> {
    let q = hash.split_once('?')?.1;
    q.split('&').find_map(|kv| kv.strip_prefix("week=")).and_then(parse_monday)
}

/// I9 / KC-1: the My Words key, `(profile_id, week_start, language)`.
pub fn list_key(profile: ProfileId, monday: i64, lang: &str) -> String {
    format!("guardian:{}:{}:{}", profile.0, iso(monday), lang)
}

// -------------------------------------------------------------- live bank

/// The PURITY verdicts the app needs: per language with a ledger, the rows
/// that do not pass. Generated from `assets/words/purity/*.tsv` and checked
/// against them by `live_bank_tests`.
const PURITY: &str = include_str!("../config/guardian-purity.json");
/// Top-up's audited claim: a named auditor per language. Like SENSE-CUE D9,
/// a human claim no tool sets, so the safe state is the default state.
const TOPUP: &str = include_str!("../config/guardian-topup.json");

struct LangBank {
    tiers: HashMap<String, usize>,
    rows: Vec<BankRow>,
    sound: HashSet<String>,
    /// `None` = no ledger; else the rows whose verdict does not pass.
    failing: Option<HashSet<String>>,
}

/// The bank as the app ships it, read once per language per view.
#[derive(Default)]
pub struct LiveBank {
    cache: RefCell<HashMap<String, Rc<LangBank>>>,
}

fn parsed(src: &str) -> serde_json::Value {
    serde_json::from_str(src).unwrap_or(serde_json::Value::Null)
}

fn topup() -> &'static serde_json::Value {
    static V: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    V.get_or_init(|| parsed(TOPUP))
}

impl LiveBank {
    fn lang(&self, lang: &str) -> Rc<LangBank> {
        if let Some(b) = self.cache.borrow().get(lang) {
            return b.clone();
        }
        let mut tiers = HashMap::new();
        let mut rows = Vec::new();
        for (t, name) in crate::consts::TIER_ORDER.iter().enumerate() {
            for row in crate::words::tier_for_fixed(lang, name) {
                let word = row.split('|').next().unwrap_or(row).to_string();
                tiers.entry(word.clone()).or_insert(t);
                rows.push(BankRow { word, row: row.to_string(), tier: t });
            }
        }
        let table = crate::soundalikes::soundalikes(lang);
        let sound = if table.status == "verified" {
            crate::soundalikes::sets_of_kind(lang, &["true"])
                .iter()
                .flat_map(|s| s.members.iter().filter(|m| m.word_id.is_some()).map(|m| m.member_id.clone()))
                .collect()
        } else {
            HashSet::new()
        };
        let failing = parsed(PURITY)["langs"].get(lang).and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect());
        let b = Rc::new(LangBank { tiers, rows, sound, failing });
        self.cache.borrow_mut().insert(lang.to_string(), b.clone());
        b
    }
}

impl BankView for LiveBank {
    fn tier(&self, lang: &str, word: &str) -> Option<usize> {
        self.lang(lang).tiers.get(word).copied()
    }
    fn length(&self, word: &str) -> usize {
        crate::randospell::bank::length_of(word)
    }
    fn sound_alike(&self, lang: &str, word: &str) -> bool {
        self.lang(lang).sound.contains(word)
    }
    fn trap_rule(&self, _lang: &str, _word: &str) -> Option<String> {
        None
    }
    fn clean(&self, lang: &str, word: &str, junior: bool) -> bool {
        // German capitalises every noun, so case says nothing there.
        let proper = lang != "de" && word.chars().next().is_some_and(char::is_uppercase);
        !proper && !crate::profanity::is_blocked(word) && (!junior || crate::kid_filter::kid_allowed(lang, word))
    }
    fn has_ledger(&self, lang: &str) -> bool {
        self.lang(lang).failing.is_some()
    }
    fn purity_pass(&self, lang: &str, row: &str) -> bool {
        self.lang(lang).failing.as_ref().is_some_and(|f| !f.contains(row))
    }
    fn audited(&self, lang: &str, _word: &str) -> bool {
        topup()["langs"][lang]["auditor"].as_str().is_some_and(|a| !a.trim().is_empty())
    }
    fn rows(&self, lang: &str) -> Vec<BankRow> {
        self.lang(lang).rows.clone()
    }
    fn word_id(&self, lang: &str, word: &str) -> String {
        crate::word_id::word_id0(lang, word)
    }
}

// ----------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learner::{Channel, OutcomeClass};
    use crate::learner_query::{Fake, DEVICE};
    use crate::reports_share::tests::{at, compose_week, det, in_pool, locale, monday_of, render_in, Indianapolis, LOCALES, OCT19, OCT26};

    #[derive(Default)]
    struct FakeBank {
        tiers: BTreeMap<String, usize>,
        sound: BTreeSet<String>,
        trap: BTreeMap<String, String>,
        dirty: BTreeSet<String>,
        no_ledger: bool,
        failing: BTreeSet<String>,
        unaudited: bool,
    }

    impl FakeBank {
        fn with(rows: &[(&str, usize)]) -> FakeBank {
            FakeBank { tiers: rows.iter().map(|(w, t)| (w.to_string(), *t)).collect(), ..FakeBank::default() }
        }
    }

    impl BankView for FakeBank {
        fn tier(&self, _: &str, word: &str) -> Option<usize> {
            self.tiers.get(word).copied()
        }
        fn length(&self, word: &str) -> usize {
            word.chars().count()
        }
        fn sound_alike(&self, _: &str, word: &str) -> bool {
            self.sound.contains(word)
        }
        fn trap_rule(&self, _: &str, word: &str) -> Option<String> {
            self.trap.get(word).cloned()
        }
        fn clean(&self, _: &str, word: &str, _: bool) -> bool {
            !self.dirty.contains(word)
        }
        fn has_ledger(&self, _: &str) -> bool {
            !self.no_ledger
        }
        fn purity_pass(&self, _: &str, row: &str) -> bool {
            !self.failing.contains(row)
        }
        fn audited(&self, _: &str, _: &str) -> bool {
            !self.unaudited
        }
        fn rows(&self, _: &str) -> Vec<BankRow> {
            self.tiers.iter().map(|(w, t)| BankRow { word: w.clone(), row: w.clone(), tier: *t }).collect()
        }
        fn word_id(&self, lang: &str, word: &str) -> String {
            format!("{lang}::{word}")
        }
    }

    #[allow(non_snake_case)]
    fn NOW() -> i64 {
        at(2026, 10, 27, 8, 0, 0, -4)
    }

    fn miss(word: &str, hour: i64) -> AttemptDetail {
        det(word, Some(at(2026, 10, 20, hour, 0, 0, -4)), Some(OutcomeClass::Misspelled))
    }

    fn hit(word: &str, hour: i64) -> AttemptDetail {
        det(word, Some(at(2026, 10, 20, hour, 0, 0, -4)), Some(OutcomeClass::Correct))
    }

    fn run(fake: &Fake, bank: &FakeBank, junior: bool) -> Report {
        compose(&GuardianInput {
            base: ComposeInput { q: fake, profile: DEVICE, langs: &["en"], now_s: NOW(), zone: &Indianapolis, which: Which::LastWeek },
            bank,
            junior,
        })
    }

    /// Medium words missed, a hard one played correctly (so the cap is Hard),
    /// and a bank to top up from.
    fn fixture() -> (Fake, FakeBank) {
        let fake = Fake {
            details: vec![miss("cat", 9), miss("cat", 10), miss("dog", 9), miss("sun", 11), hit("thorough", 12)],
            ..Fake::default()
        };
        let bank = FakeBank::with(&[
            ("cat", 1),
            ("dog", 1),
            ("sun", 1),
            ("hat", 1),
            ("map", 1),
            ("pen", 1),
            ("cup", 1),
            ("box", 1),
            ("fox", 1),
            ("jam", 1),
            ("leg", 1),
            ("owl", 1),
            ("thorough", 2),
            ("zip", 3),
        ]);
        (fake, bank)
    }

    #[test]
    fn the_registry_order_is_trap_sound_long_tier() {
        let mut v = vec![WordType::Tier(0), WordType::Long, WordType::Trap("ru.e".into()), WordType::Sound, WordType::Tier(3)];
        v.sort();
        assert_eq!(v, [WordType::Trap("ru.e".into()), WordType::Sound, WordType::Long, WordType::Tier(0), WordType::Tier(3)]);
    }

    /// A10: a word that is a trap word AND long sits only in its T-TRAP group;
    /// a sound-alike that is long is SOUND.
    #[test]
    fn a_trap_word_that_is_long_is_only_a_trap_word() {
        let mut b = FakeBank::with(&[("extraordinary", 3), ("principal", 2)]);
        b.trap.insert("extraordinary".into(), "ru.e".into());
        b.sound.insert("principal".into());
        assert_eq!(primary_type(&b, "en", "extraordinary"), Some(WordType::Trap("ru.e".into())));
        assert_eq!(primary_type(&b, "en", "principal"), Some(WordType::Sound));
        b.trap.clear();
        assert_eq!(primary_type(&b, "en", "extraordinary"), Some(WordType::Long));
        assert_eq!(primary_type(&b, "en", "custom"), None, "a word outside the bank has no type");

        let words = ["extraordinary", "photosynthesis", "encyclopaedia", "possession"];
        let fake = Fake { details: words.iter().map(|w| miss(w, 9)).collect(), ..Fake::default() };
        let mut b = FakeBank::with(&words.map(|w| (w, 3)));
        for w in &words[..3] {
            b.trap.insert(w.to_string(), "ru.e".into());
        }
        let r = run(&fake, &b, false);
        let groups: Vec<(&str, usize)> = r.sections[0].types.iter().map(|g| (g.label_key.as_str(), g.count)).collect();
        assert_eq!(groups, [("greport.type.trap.ru.e", 3)], "possession is long, its group of 1 is hidden");
    }

    /// A9: a group of 2 is hidden, and its words are still in Working on and
    /// in the practice list. I12: no word is in two groups.
    #[test]
    fn a_group_of_two_is_hidden_but_its_words_stay() {
        let (mut fake, mut bank) = fixture();
        fake.details.extend([miss("necessary", 9), miss("separate", 9)]);
        bank.tiers.insert("necessary".into(), 1);
        bank.tiers.insert("separate".into(), 1);
        let r = run(&fake, &bank, false);
        let s = &r.sections[0];
        assert_eq!(s.types.len(), 1);
        assert_eq!((s.types[0].label_key.as_str(), s.types[0].count), ("greport.type.tier.medium", 3));
        assert!(!s.types.iter().any(|g| g.label_key == "greport.type.long_words"), "the long group of 2 must be hidden");
        for w in ["necessary", "separate"] {
            assert!(s.working_on.iter().any(|x| x == w), "{w} left Working on");
            assert!(s.practice.iter().any(|x| x == w), "{w} left the practice list");
        }
        let mut seen = BTreeSet::new();
        for g in &s.types {
            for w in &g.words {
                assert!(seen.insert(w), "{w} is in two groups");
            }
        }
    }

    /// F2 display: at most 4 groups, biggest first, then registry order; each
    /// shows up to 5 words.
    #[test]
    fn at_most_four_groups_biggest_first() {
        let mut details = Vec::new();
        let mut rows = Vec::new();
        let mut b = FakeBank::default();
        // sound 3, long 4, easy 6, medium 3, hard 3 -> easy, long, sound, medium.
        for (i, (kind, n)) in [("s", 3), ("l", 4), ("e", 6), ("m", 3), ("h", 3)].into_iter().enumerate() {
            for k in 0..n {
                let w = match kind {
                    "l" => format!("longword{k}{i}"),
                    _ => format!("{kind}{k}"),
                };
                let tier = match kind {
                    "e" | "s" => 0,
                    "m" | "l" => 1,
                    _ => 2,
                };
                if kind == "s" {
                    b.sound.insert(w.clone());
                }
                details.push(miss(&w, 9));
                rows.push((w, tier));
            }
        }
        b.tiers = rows.into_iter().collect();
        let r = run(&Fake { details, ..Fake::default() }, &b, false);
        let got: Vec<(&str, usize, usize)> = r.sections[0].types.iter().map(|g| (g.label_key.as_str(), g.count, g.words.len())).collect();
        assert_eq!(
            got,
            [
                ("greport.type.tier.easy", 6, 5),
                ("greport.type.long_words", 4, 4),
                ("greport.type.sound_alike", 3, 3),
                ("greport.type.tier.medium", 3, 3),
            ]
        );
    }

    /// A12: Speech, Impossible, unknown and correct-class outcomes never enter
    /// W, so they are in no group and never practice words by being missed.
    #[test]
    fn excluded_outcomes_never_enter_w() {
        let t = at(2026, 10, 20, 9, 0, 0, -4);
        let mut speech = det("spoken", Some(t), Some(OutcomeClass::Misspelled));
        speech.channel = Channel::Speech;
        let mut hard = det("impossibly", Some(t), Some(OutcomeClass::Misspelled));
        hard.impossible = true;
        let log = vec![
            speech,
            hard,
            det("pair", Some(t), Some(OutcomeClass::OtherSense)),
            det("legacy", None, Some(OutcomeClass::Misspelled)),
            det("unknown", Some(t), None),
            det("trapped", Some(t), Some(OutcomeClass::Trap)),
            det("late", Some(t), Some(OutcomeClass::TimedOut)),
        ];
        let w: Vec<String> = missed_set(&log, |_| true).into_iter().map(|m| m.word).collect();
        assert_eq!(w, ["late", "trapped"]);
    }

    /// F3.1: missed words by miss count, then most recent miss, then word id.
    #[test]
    fn missed_words_come_first_in_their_order() {
        let (fake, mut bank) = fixture();
        bank.unaudited = true;
        let r = run(&fake, &bank, false);
        assert_eq!(r.sections[0].practice, ["cat", "sun", "dog"]);
    }

    /// F3.2/F3.4: top-up fills to 10 with same-type, eligible, unseen rows,
    /// deterministically per (profile, week, language).
    #[test]
    fn top_up_fills_to_ten_deterministically() {
        let (fake, bank) = fixture();
        let a = run(&fake, &bank, false);
        let p = &a.sections[0].practice;
        assert_eq!(p.len(), PRACTICE_MAX);
        assert_eq!(&p[..3], ["cat", "sun", "dog"]);
        for w in &p[3..] {
            assert_eq!(bank.tiers[w], 1, "{w}: top-up must share the group's type (Medium)");
        }
        for _ in 0..3 {
            assert_eq!(run(&fake, &bank, false).sections[0].practice, *p, "same inputs, same list");
        }
        assert_ne!(seed(DEVICE, monday_of(OCT19), "en"), seed(DEVICE, monday_of(OCT26), "en"));
        assert_ne!(seed(DEVICE, monday_of(OCT19), "en"), seed(DEVICE, monday_of(OCT19), "es"));
    }

    /// A14: top-up never includes a word in the seen set.
    #[test]
    fn top_up_never_includes_a_seen_word() {
        let (mut fake, bank) = fixture();
        fake.seen = ["hat", "map", "pen", "cup", "box"].iter().map(|w| format!("en::{w}")).collect();
        let p = run(&fake, &bank, false).sections[0].practice.clone();
        for w in ["hat", "map", "pen", "cup", "box"] {
            assert!(!p.iter().any(|x| x == w), "{w} is seen");
        }
        assert_eq!(p.len(), 3 + 4, "fox, jam, leg, owl are what is left");
    }

    /// A13 / I5: every practice word is an audited, PURITY-passing, clean bank
    /// row within the cap, and a Spell Jr list holds no Hard or Expert row.
    #[test]
    fn every_practice_word_passes_i5() {
        let (mut fake, mut bank) = fixture();
        bank.failing.insert("hat".into());
        bank.dirty.insert("map".into());
        bank.tiers.insert("Paris".into(), 1);
        bank.dirty.insert("Paris".into());
        fake.details.push(miss("zip", 9)); // Expert, missed
        let std = run(&fake, &bank, false).sections[0].practice.clone();
        assert!(!std.iter().any(|w| w == "hat" || w == "map" || w == "Paris"), "{std:?}");
        assert!(std.iter().any(|w| w == "zip"), "the standard cap is the highest tier played (Expert)");
        let jr = run(&fake, &bank, true).sections[0].practice.clone();
        assert!(jr.iter().all(|w| bank.tiers[w] <= 1), "a Jr list held a Hard or Expert row: {jr:?}");
        assert!(!jr.is_empty());
    }

    /// Own-list words: a missed word that is no bank row joins the practice
    /// list with no cap, even in a week with no bank play at all, and `clean`
    /// still binds it. It is in no type group (I12) and never tops up.
    #[test]
    fn an_own_list_miss_joins_the_practice_list() {
        let (mut fake, mut bank) = fixture();
        fake.details.push(miss("quixotic", 13));
        fake.details.push(miss("Zanzibar", 13));
        fake.details.push(miss("blocked", 13));
        bank.dirty.insert("Zanzibar".into());
        bank.dirty.insert("blocked".into());
        bank.unaudited = true;
        let s = &run(&fake, &bank, false).sections[0];
        assert_eq!(s.practice, ["cat", "quixotic", "sun", "dog"]);
        assert!(!s.types.iter().any(|g| g.words.iter().any(|w| w == "quixotic")));
        assert_eq!(run(&fake, &bank, true).sections[0].practice, s.practice, "Jr: no tier, no cap");

        let only = Fake { details: vec![miss("quixotic", 9), miss("rhythm", 10)], ..Fake::default() };
        let p = run(&only, &FakeBank::with(&[("cat", 1)]), false).sections[0].practice.clone();
        assert_eq!(p, ["rhythm", "quixotic"], "no bank play this week, the own-list misses still count");
    }

    /// The cap is binding for the standard profile too: no tier above what was
    /// played this week, even to fill the list.
    #[test]
    fn the_standard_cap_is_the_highest_tier_played() {
        let fake = Fake { details: vec![miss("cat", 9), miss("dog", 9), miss("sun", 9)], ..Fake::default() };
        let mut bank = FakeBank::with(&[("cat", 0), ("dog", 0), ("sun", 0), ("ant", 0), ("bee", 1)]);
        bank.tiers.insert("elk".into(), 0);
        let p = run(&fake, &bank, false).sections[0].practice.clone();
        assert!(!p.iter().any(|w| w == "bee"), "Medium was never played: {p:?}");
    }

    /// A15: no PURITY ledger = no top-up; the list is the missed words.
    #[test]
    fn without_a_ledger_the_list_is_the_missed_words() {
        let (fake, mut bank) = fixture();
        bank.no_ledger = true;
        assert_eq!(run(&fake, &bank, false).sections[0].practice, ["cat", "sun", "dog"]);
    }

    /// A16: eligible rows run out, the list is shorter, and nothing relaxed:
    /// what was added still passes every filter.
    #[test]
    fn exhausted_rows_make_a_shorter_list() {
        let (fake, mut bank) = fixture();
        for w in ["hat", "map", "pen", "cup", "box", "fox"] {
            bank.failing.insert(w.into());
        }
        let p = run(&fake, &bank, false).sections[0].practice.clone();
        assert_eq!(p.len(), 3 + 3);
        for w in &p[3..] {
            assert!(["jam", "leg", "owl"].contains(&w.as_str()), "{w} came from a relaxed filter");
        }
        bank.unaudited = true;
        assert_eq!(run(&fake, &bank, false).sections[0].practice.len(), 3, "unaudited rows never top up");
    }

    /// A3: the week is Monday-based whatever the UI language: the composer
    /// takes no locale, and every locale's header starts on the same Monday.
    #[test]
    fn the_week_starts_on_monday_in_every_locale() {
        let (fake, bank) = fixture();
        let r = run(&fake, &bank, false);
        assert_eq!((r.monday + 3).rem_euclid(7), 0, "the window opens on a Monday");
        for lang in ["fr", "en", "ar", "ja"] {
            let text = render_in(&r, lang);
            assert!(text.lines().nth(1).unwrap().starts_with("10-19"), "{lang}: {text}");
        }
    }

    /// A4: the Guardian fixture renders byte-identically, run after run.
    #[test]
    fn the_guardian_report_is_byte_identical_across_runs() {
        let (fake, bank) = fixture();
        let first = render_in(&run(&fake, &bank, false), "en");
        for _ in 0..3 {
            assert_eq!(render_in(&run(&fake, &bank, false), "en"), first);
        }
        let head = "Guardian Report\n10-19 – 2026-10-25\n\nEnglish\nWord types\nMedium words (3): cat, sun, dog\nWorking on (3): cat, dog, sun\nNew this week (1): thorough\nPractice list (10): cat, sun, dog, ";
        assert!(first.starts_with(head), "{first}");
    }

    /// A11 / I13: every registry label renders from a key every locale holds,
    /// and the renderer refuses any key outside the pools.
    #[test]
    fn every_type_label_is_a_registry_key_in_every_locale() {
        let keys: Vec<String> = [WordType::Sound, WordType::Long].into_iter().chain((0..4).map(WordType::Tier)).map(|t| t.label_key()).collect();
        for lang in LOCALES {
            let loc = locale(lang);
            for k in &keys {
                assert!(in_pool(k), "{k} is outside the pools");
                assert!(loc.get(k).and_then(|v| v.as_str()).is_some_and(|v| !v.is_empty()), "{lang} lacks {k}");
            }
            for k in loc.keys().filter(|k| k.starts_with("greport.type.")) {
                assert!(keys.contains(k), "{lang}/{k} is a type label outside the registry");
            }
        }
        let bad = Report {
            monday: monday_of(OCT19),
            partial: false,
            sections: vec![rs::LangSection {
                lang: "en".into(),
                mastered: vec![],
                working_on: vec!["x".into()],
                new_words: vec![],
                types: vec![TypeGroup { label_key: "type.made_up".into(), count: 3, words: vec![] }],
                practice: vec![],
            }],
        };
        assert!(std::panic::catch_unwind(|| render_in(&bad, "en")).is_err(), "a key outside the pools must fail");
    }

    /// REPORTS-SHARE A4/A5, over the Guardian sections: nothing free, nothing
    /// prohibited, in every language.
    #[test]
    fn the_guardian_sections_speak_only_the_closed_lexicon() {
        let (fake, bank) = fixture();
        let r = run(&fake, &bank, false);
        let lists: serde_json::Value = serde_json::from_str(crate::reports_share::tests::PROHIBITED).unwrap();
        for lang in LOCALES {
            let text = render_in(&r, lang);
            assert_eq!(crate::reports_share::tests::free_text_residue(&text, &r, lang), "", "{lang}:\n{text}");
            let hits = crate::reports_share::tests::prohibited_hits(&lists, lang, &text).unwrap();
            assert!(hits.is_empty(), "{lang} says {hits:?}");
        }
        let _ = compose_week; // the REPORTS-SHARE fixture stays the base report's golden
    }

    // -- F7 ping ---------------------------------------------------------------

    fn t_en(k: &str) -> String {
        locale("en").get(k).and_then(|v| v.as_str()).unwrap_or_else(|| panic!("en lacks {k}")).to_string()
    }

    #[test]
    fn the_ping_is_next_monday_at_eight() {
        let z = Indianapolis;
        assert_eq!(next_ping_monday(at(2026, 10, 21, 15, 0, 0, -4), &z), monday_of(OCT26));
        assert_eq!(next_ping_monday(at(2026, 10, 26, 7, 59, 0, -4), &z), monday_of(OCT26), "Monday before 08:00 is today");
        assert_eq!(next_ping_monday(at(2026, 10, 26, 8, 0, 0, -4), &z), monday_of(OCT26) + 7);
    }

    /// A22 + G-D12: a week with zero sessions schedules nothing.
    #[test]
    fn a_week_without_sessions_pings_nothing() {
        assert_eq!(ping(PingCopy::Guardian, monday_of(OCT26), &[ProfileWeek::default()], &t_en), None);
        assert_eq!(ping(PingCopy::Digest, monday_of(OCT26), &[], &t_en), None);
        let (fake, bank) = fixture();
        let input = GuardianInput {
            base: ComposeInput { q: &fake, profile: DEVICE, langs: &["en"], now_s: NOW(), zone: &Indianapolis, which: Which::LastWeek },
            bank: &bank,
            junior: false,
        };
        assert!(profile_week(&input, monday_of(OCT19)).sessions);
        assert_eq!(profile_week(&input, monday_of(OCT26)), ProfileWeek::default());
    }

    /// A23 + G-D13: two profiles, one notification, the multi-profile copy.
    #[test]
    fn two_profiles_share_one_ping() {
        let w = ProfileWeek { sessions: true, practice: 8, mastered: 2 };
        let one = ping(PingCopy::Guardian, monday_of(OCT26), &[w], &t_en).unwrap();
        assert_eq!(one.body, "Guardian Report ready · 8 practice words");
        assert_eq!(one.route.as_deref(), Some("#guardian-report?week=2026-10-19"));
        assert_eq!(one.date, (2026, 10, 26));
        let single = ping(PingCopy::Guardian, monday_of(OCT26), &[ProfileWeek { practice: 1, ..w }], &t_en).unwrap();
        assert_eq!(single.body, "Guardian Report ready · 1 practice word");
        let two = ping(PingCopy::Guardian, monday_of(OCT26), &[w, w, ProfileWeek::default()], &t_en).unwrap();
        assert_eq!(two.body, "Guardian Reports ready · 2 profiles");
        let digest = ping(PingCopy::Digest, monday_of(OCT26), &[w], &t_en).unwrap();
        assert_eq!((digest.body.as_str(), digest.route), ("2 new words mastered this week", None));
    }

    /// A20 / I7: no payload, in any language or copy, carries a bank word or a
    /// profile name.
    #[test]
    fn no_payload_carries_a_word_or_a_name() {
        let (fake, bank) = fixture();
        let names = ["Maya", "Sam", "Léa"];
        let words: Vec<String> = bank.tiers.keys().cloned().chain(fake.details.iter().map(|d| d.word.clone())).collect();
        for lang in LOCALES {
            let loc = locale(lang);
            let t = |k: &str| loc.get(k).and_then(|v| v.as_str()).unwrap_or_else(|| panic!("{lang} lacks {k}")).to_string();
            for copy in [PingCopy::Guardian, PingCopy::Digest] {
                for weeks in [vec![ProfileWeek { sessions: true, practice: 8, mastered: 3 }], vec![ProfileWeek { sessions: true, ..Default::default() }; 2]] {
                    let p = ping(copy, monday_of(OCT26), &weeks, &t).unwrap();
                    let payload = format!("{}\n{}\n{}", p.title, p.body, p.route.unwrap_or_default()).to_lowercase();
                    for w in words.iter().map(String::as_str).chain(names) {
                        let hit = payload.split(|c: char| !c.is_alphanumeric()).any(|tok| tok == w.to_lowercase());
                        assert!(!hit, "{lang} {copy:?} payload carries {w}: {payload}");
                    }
                }
            }
        }
    }

    #[test]
    fn identifiers_round_trip() {
        let m = monday_of(OCT19);
        assert_eq!(iso(m), "2026-10-19");
        assert_eq!(parse_monday("2026-10-19"), Some(m));
        assert_eq!(parse_monday("2026-10-20"), None, "not a Monday");
        assert_eq!(parse_monday("2026-02-30"), None);
        assert_eq!(parse_monday("x"), None);
        assert_eq!(week_of_route(&route_for(m)), Some(m));
        assert_eq!(week_of_route("#guardian-report"), None);
        assert_eq!(list_key(DEVICE, m, "en"), "guardian:0:2026-10-19:en");
    }

    /// I1 / I2: the module names no write door and no network API.
    #[test]
    fn the_module_is_read_only_and_offline() {
        let code = crate::reports_share::tests::code_of(include_str!("guardian_report.rs"));
        for bad in [
            "fetch(", "XmlHttpRequest", "crate::api", "online_net", "web_sys::Request", "WebSocket", "spawn_local", "sendBeacon", "note_attempt",
            "note_built", "note_placement", "storage::set", "learner::reset", "add_miss", "set_json", "set_raw", "word_lists::store",
        ] {
            let hit = code.lines().filter(|l| !l.trim_start().starts_with("//")).any(|l| l.contains(bad));
            assert!(!hit, "guardian_report.rs names {bad}");
        }
    }
}

/// The live view against the shipped bank and ledgers.
#[cfg(test)]
mod live_bank_tests {
    use super::*;

    /// The failing-row file is exactly what the ledgers say about the rows the
    /// banks serve. A row with no verdict has not passed, so it fails too.
    #[test]
    fn the_purity_file_matches_the_ledgers() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/words/purity");
        let mut want = serde_json::Map::new();
        for (lang, _, _, _) in crate::consts::BUILTIN_LANGS {
            let Ok(raw) = std::fs::read_to_string(format!("{dir}/{lang}.tsv")) else { continue };
            let mut verdict: HashMap<&str, &str> = HashMap::new();
            for line in raw.lines().filter(|l| !l.starts_with('#')) {
                let mut f = line.split('\t');
                if let (Some(row), Some(v)) = (f.next(), f.next()) {
                    verdict.insert(row, v);
                }
            }
            let bank = LiveBank::default();
            let rows = bank.rows(lang);
            let failing: BTreeSet<&str> =
                rows.iter().map(|r| r.row.as_str()).filter(|r| !matches!(verdict.get(r), Some(&"dictionary") | Some(&"exception"))).collect();
            want.insert(lang.to_string(), serde_json::Value::from(failing.into_iter().collect::<Vec<_>>()));
        }
        let doc = "CC-GUARDIAN-REPORT F3: per language with a PURITY ledger, the bank rows that have not passed: verdict pending, or no verdict at all. Generated from assets/words/purity/*.tsv against the banks the app serves: GUARDIAN_PURITY_WRITE=1 cargo test --lib the_purity_file_matches_the_ledgers. A language absent here has no ledger, so no top-up.";
        let want = serde_json::json!({ "$doc": doc, "langs": want });
        if std::env::var("GUARDIAN_PURITY_WRITE").is_ok() {
            let path = concat!(env!("CARGO_MANIFEST_DIR"), "/config/guardian-purity.json");
            std::fs::write(path, serde_json::to_string_pretty(&want).unwrap() + "\n").unwrap();
            return;
        }
        assert_eq!(parsed(PURITY), want, "regenerate: GUARDIAN_PURITY_WRITE=1 cargo test --lib the_purity_file_matches_the_ledgers");
        assert!(!LiveBank::default().has_ledger("vi"), "vi has no ledger, so no top-up");
    }

    /// Top-up's audited claim is unset in every language until Eric names an
    /// auditor, so nothing tops up today; and an auditor must be a name.
    #[test]
    fn no_language_is_audited_until_someone_is_named() {
        let b = LiveBank::default();
        for (lang, _, _, _) in crate::consts::BUILTIN_LANGS {
            assert!(!b.audited(lang, "any"), "{lang} claims an auditor; record who in the commit");
        }
    }

    #[test]
    fn the_live_columns_read_the_bank() {
        let b = LiveBank::default();
        let row = &b.rows("en")[0];
        assert_eq!(b.tier("en", &row.word), Some(0));
        assert_eq!(b.length("photosynthesis"), 14);
        assert!(!b.clean("en", "Paris", false), "a capitalised row is a proper noun");
        assert!(b.clean("de", "Haus", false), "German capitalises every noun");
        assert!(b.trap_rule("ru", "молоко").is_none(), "no TRAP_MISS(rule_id) exists yet");
        let set = crate::soundalikes::sets_of_kind("en", &["true"]).into_iter().find(|s| s.members.iter().filter(|m| m.word_id.is_some()).count() > 0).unwrap();
        let member = set.members.iter().find(|m| m.word_id.is_some()).unwrap();
        assert!(b.sound_alike("en", &member.member_id), "{} is in a verified set", member.member_id);
        assert!(!b.sound_alike("vi", &member.member_id), "vi's table is not verified");
    }
}
