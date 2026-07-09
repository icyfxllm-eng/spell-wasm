use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Prefs {
    pub glow: Option<String>,
    #[serde(rename = "bgColor")]
    pub bg_color: Option<String>,
    #[serde(rename = "orbColor")]
    pub orb_color: Option<String>,
    #[serde(rename = "lastLang")]
    pub last_lang: Option<String>,
    pub kid: bool,
    pub readable: bool,
    #[serde(rename = "bigText", default)]
    pub big_text: bool,
    pub slow: bool,
    pub volume: Option<f32>,
    #[serde(default)]
    pub remind: bool,
    #[serde(rename = "remindTime", default)]
    pub remind_time: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct TierStat {
    pub seen: u32,
    pub correct: u32,
}

/// lang key -> tier -> stat
pub type Stats = HashMap<String, HashMap<String, TierStat>>;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct CustomSet {
    pub words: Vec<String>,
    #[serde(rename = "speakLang")]
    pub speak_lang: String,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct AchState {
    pub unlocked: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MissEntry {
    pub word: String,
    pub lang: String,
    pub tier: String,
    pub misses: u32,
    pub box_: u32,
    pub due: f64,
    pub ts: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BoardEntry {
    pub name: String,
    pub streak: u32,
    pub lang: String,
    pub level: String,
    pub timed: bool,
    pub ts: f64,
}

pub const PREFS_KEY: &str = "byear_prefs_v1";
pub const STATS_KEY: &str = "byear_stats_v1";
pub const LB_KEY: &str = "byear_leaderboard_v1";
pub const CUSTOM_KEY: &str = "byear_custom_v1";
pub const ACH_KEY: &str = "byear_ach_v1";
pub const MISS_KEY: &str = "byear_misses_v1";
pub const MISS_CAP: usize = 300;

/// Mirrors the JS `state` object.
pub struct AppState {
    pub lang: String,
    pub level: String,
    pub timed: bool,
    pub review: bool,
    pub word: String,
    /// The player's in-progress spelling. Held here (not in a DOM `<input>`) so
    /// the iOS system keyboard — with its dictation key and autocorrect — never
    /// opens during a round. Driven by the custom on-screen keyboard, physical
    /// keydown (desktop), the mic, and handwriting OCR; rendered into #letters.
    pub answer: String,
    pub cur_lang: String,
    pub cur_tier: String,
    pub tries_left: u32,
    pub streak: u32,
    pub best: u32,
    pub answered: bool,
    pub rate: f32,
    pub glow: String,
    pub bg_color: String,
    pub orb_color: String,
    pub last_lang: Option<String>,
    pub kid: bool,
    /// True when a stored "kid" age-gate verdict locks the app in Kid Mode;
    /// leaving Kid Mode then requires the parent gate. Session-only, derived at
    /// boot from the persisted verdict (see `crate::agegate`).
    pub age_locked: bool,
    pub readable: bool,
    pub big_text: bool,
    pub slow: bool,
    pub volume: f32,
    /// Daily practice reminder (native local notification). `remind_time` is
    /// "HH:MM" 24h. Suppressed while Kid Mode is on.
    pub remind: bool,
    pub remind_time: String,

    pub custom: CustomSet,
    pub misses: Vec<MissEntry>,
    pub achievements: AchState,
    pub stats: Stats,
    pub saved_name: String,
    pub pending_score: u32,
    pub prev_letter_len: usize,
    /// Unix ms when the current solo chain started (streak 0 -> 1), used to send
    /// a plausible run duration to The Climb's anti-cheat. Session-only.
    pub run_start_ms: f64,
    /// Head-to-head match state. Session-only (never persisted); `enabled` is
    /// false during normal single-player play. See `crate::versus`.
    pub versus: crate::versus::Versus,
    /// Shuffled-deck word selection, one per lang+tier pool (keyed
    /// `"{lang}:{tier}"`) plus `"__review"` for misses practice. Session-only
    /// — not persisted to storage.
    pub decks: HashMap<String, crate::deck::Deck>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            lang: crate::consts::EN.into(),
            level: "climb".into(),
            timed: false,
            review: false,
            word: String::new(),
            answer: String::new(),
            cur_lang: crate::consts::EN.into(),
            cur_tier: "easy".into(),
            tries_left: crate::consts::MAX_TRIES,
            streak: 0,
            best: 0,
            answered: false,
            rate: 0.9,
            glow: "#ffb14d".into(),
            bg_color: "#1c1830".into(),
            orb_color: "#ffb14d".into(),
            last_lang: None,
            kid: false,
            age_locked: false,
            readable: false,
            big_text: false,
            slow: false,
            volume: 1.0,
            remind: false,
            remind_time: "17:00".into(),
            custom: CustomSet { words: Vec::new(), speak_lang: "en-US".into() },
            misses: Vec::new(),
            achievements: AchState::default(),
            stats: Stats::default(),
            saved_name: String::new(),
            pending_score: 0,
            prev_letter_len: 0,
            run_start_ms: 0.0,
            versus: crate::versus::Versus::default(),
            decks: HashMap::new(),
        }
    }
}
