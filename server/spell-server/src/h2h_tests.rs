//! Online Head-to-Head: one test per rules-table row (A7.1), replay (A7.2),
//! deadline fairness (A7.3), parity with a locally driven `Versus` (A7.4),
//! word selection (A8.1, A8.2) and the audio pause rule (A9.3).

use super::*;
use crate::mode::Audience;
use std::collections::HashSet;

fn req(lang: &str, tier_mode: TierMode, tier: Option<&str>, timed: bool) -> CreateRoom {
    CreateRoom {
        mode: "h2h".into(),
        lang: lang.into(),
        tier_mode,
        tier: tier.map(str::to_string),
        timed,
        approve_joins: false,
    }
}

/// A `Ctx` from a Sim's fields, borrowing them disjointly from `m`.
macro_rules! ctx {
    ($s:expr) => {
        Ctx { now: $s.now, seats: &$s.seats, kid_safe: $s.kid, grace: $s.grace, connected: $s.connected }
    };
}

/// Drives one H2h directly, with a hand-held clock and audio that is
/// "fetched" the moment it is asked for (unless `audio_ok` is false).
struct Sim {
    m: H2h,
    now: Instant,
    seats: Vec<u8>,
    kid: bool,
    grace: [Duration; 4],
    connected: [bool; 4],
    audio_ok: bool,
    log: Vec<Event>,
}

impl Sim {
    fn new(r: CreateRoom, seed: u64, kid: bool) -> Sim {
        Sim {
            m: H2h::with_seed(&r, seed),
            now: Instant::now(),
            seats: vec![0, 1],
            kid,
            grace: [Duration::ZERO; 4],
            connected: [true; 4],
            audio_ok: true,
            log: Vec::new(),
        }
    }


    fn run(&mut self, ev: Vec<Event>) -> Vec<Event> {
        let mut out = Vec::new();
        let mut queue = ev;
        while !queue.is_empty() {
            let mut next = Vec::new();
            for e in queue {
                if let Event::FetchAudio { ticket, .. } = &e {
                    let ok = self.audio_ok;
                    let t = ticket.clone();
                    let follow = self.m.on_audio(&t, ok, &ctx!(self));
                    next.extend(follow);
                }
                out.push(e);
            }
            queue = next;
        }
        self.log.extend(out.clone());
        out
    }

    fn start(&mut self) -> Vec<Event> {
        let ev = self.m.on_start(&ctx!(self));
        self.run(ev)
    }
    fn next(&mut self, seat: u8) -> Vec<Event> {
        let ev = self.m.on_next(seat, &ctx!(self));
        self.run(ev)
    }
    fn submit(&mut self, seat: u8, typed: &str) -> Vec<Event> {
        let ev = self.m.on_submit(seat, typed, &ctx!(self));
        self.run(ev)
    }
    /// Moves the clock, firing every deadline that passes, in order.
    fn advance(&mut self, d: Duration) -> Vec<Event> {
        let until = self.now + d;
        let mut out = Vec::new();
        loop {
            match self.m.next_deadline() {
                Some(t) if t <= until => {
                    self.now = self.now.max(t);
                    let ev = self.m.on_timer(&ctx!(self));
                    let fired = !ev.is_empty();
                    out.extend(self.run(ev));
                    if !fired {
                        // Waiting out a seat's grace: step a millisecond.
                        self.now += Duration::from_millis(1);
                        if self.now > until {
                            break;
                        }
                    }
                }
                _ => break,
            }
        }
        self.now = until;
        out
    }

    fn live_entry(&self) -> Option<String> {
        match &self.m.phase {
            Phase::Live { word, .. } => Some(word.entry.clone()),
            _ => None,
        }
    }
    fn active(&self) -> u8 {
        self.m.active_seat()
    }
    /// The live word's correct answer (its citation).
    fn right(&self) -> String {
        citation(&self.live_entry().expect("a live word")).to_string()
    }
}

fn msgs(ev: &[Event]) -> Vec<&ServerMsg> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Send { msg, .. } => Some(msg),
            _ => None,
        })
        .collect()
}

fn has(ev: &[Event], f: impl Fn(&ServerMsg) -> bool) -> bool {
    msgs(ev).into_iter().any(f)
}

fn climb(lang: &str) -> CreateRoom {
    req(lang, TierMode::Climb, None, false)
}

// ---------------------------------------------------------------- A7.1, one per row

#[test]
fn row_1_exactly_two_players() {
    let m = H2h::new(&climb("en"));
    assert_eq!((m.min_seats(), m.max_seats()), (2, 2));
}

#[test]
fn row_4_three_turns_each_two_for_a_kid_safe_room() {
    let mut s = Sim::new(climb("en"), 1, false);
    let ev = s.start();
    assert!(has(&ev, |m| matches!(m, ServerMsg::TurnStart { seat: 0, turn_no: 1, turns_left: 3 })));
    let mut k = Sim::new(climb("en"), 1, true);
    let ev = k.start();
    assert!(has(&ev, |m| matches!(m, ServerMsg::TurnStart { turns_left: 2, .. })));
}

#[test]
fn rows_5_to_7_one_attempt_correct_extends_a_miss_ends_the_turn() {
    let mut s = Sim::new(climb("en"), 2, false);
    s.start();
    s.next(0);
    let right = s.right();
    let ev = s.submit(0, &right);
    assert!(has(&ev, |m| matches!(m, ServerMsg::WordResult { seat: 0, correct: true, chain: 1, best: 1, .. })));
    s.advance(correct_delay());
    assert!(s.live_entry().is_some(), "row 15: the next word comes by itself");
    let ev = s.submit(0, "zzzzqqq");
    assert!(has(&ev, |m| matches!(m, ServerMsg::WordResult { seat: 0, correct: false, chain: 0, best: 1, .. })));
    assert!(has(&ev, |m| matches!(m, ServerMsg::TurnEnd { seat: 0 })));
    // Row 5: the miss resolved the word; a second submit cannot retry it.
    let again = s.submit(0, &right);
    assert!(!has(&again, |m| matches!(m, ServerMsg::WordResult { .. })));
    assert_eq!(s.m.versus.p1.turns_left, 2);
}

#[test]
fn row_8_strict_alternation_then_the_side_with_turns_plays_out() {
    let mut s = Sim::new(climb("en"), 3, false);
    s.start();
    let mut order = Vec::new();
    for _ in 0..6 {
        let a = s.active();
        order.push(a);
        s.next(a);
        s.submit(a, "zzzzqqq");
        s.advance(MISS_DELAY);
    }
    assert_eq!(order, vec![0, 1, 0, 1, 0, 1]);
    assert!(!s.m.running(), "row 9: neither side has turns left");
}

#[test]
fn rows_9_10_the_longest_chain_wins_and_equal_is_a_tie() {
    // Seat 0: chain of 2 then misses; seat 1 misses every time.
    let mut s = Sim::new(climb("en"), 4, false);
    s.start();
    let mut last = Vec::new();
    for turn in 0..6 {
        let a = s.active();
        s.next(a);
        if turn == 0 {
            for _ in 0..2 {
                let r = s.right();
                s.submit(a, &r);
                s.advance(correct_delay());
            }
        }
        s.submit(a, "zzzzqqq");
        last = s.advance(MISS_DELAY);
    }
    assert!(has(&last, |m| matches!(m, ServerMsg::MatchResult { winner_seat: Some(0), .. })));
    assert!(last.contains(&Event::Over));

    let mut t = Sim::new(climb("en"), 5, false);
    t.start();
    for _ in 0..6 {
        let a = t.active();
        t.next(a);
        t.submit(a, "zzzzqqq");
        last = t.advance(MISS_DELAY);
    }
    assert!(has(&last, |m| matches!(m, ServerMsg::MatchResult { winner_seat: None, .. })), "0 vs 0 ties");
}

fn tier_of_live(s: &Sim) -> String {
    match &s.m.phase {
        Phase::Live { word, .. } => word.tier.clone(),
        _ => panic!("no live word"),
    }
}

#[test]
fn row_11_climb_tier_follows_the_chain_fixed_holds_and_kid_rooms_clamp() {
    let mut s = Sim::new(climb("en"), 6, false);
    s.start();
    s.next(0);
    let mut tiers = vec![tier_of_live(&s)];
    for _ in 0..12 {
        let r = s.right();
        s.submit(0, &r);
        s.advance(correct_delay());
        tiers.push(tier_of_live(&s));
    }
    // Chain before each word: 0..=12 → local tier_for_streak.
    let want: Vec<String> = (0..=12).map(|c| core::tier_for_streak(c).to_string()).collect();
    assert_eq!(tiers, want);
    assert_eq!(tiers[0], "easy");
    assert_eq!(tiers[3], "medium");
    assert_eq!(tiers[7], "hard");
    assert_eq!(tiers[12], "expert");

    let mut f = Sim::new(req("en", TierMode::Fixed, Some("hard"), false), 7, false);
    f.start();
    f.next(0);
    assert_eq!(tier_of_live(&f), "hard");

    let mut k = Sim::new(req("en", TierMode::Fixed, Some("expert"), false), 8, true);
    k.start();
    k.next(0);
    assert_eq!(tier_of_live(&k), core::clamp_base_tier(true, "expert".into()), "the Spell Jr ceiling");
    assert_eq!(tier_of_live(&k), "medium");
}

#[test]
fn row_12_timed_rooms_use_tier_time_and_untimed_rooms_send_no_deadline() {
    let mut s = Sim::new(req("en", TierMode::Fixed, Some("hard"), true), 9, false);
    s.start();
    let ev = s.next(0);
    assert!(has(&ev, |m| matches!(m, ServerMsg::WordStart { deadline_ms: Some(22_000), .. })));
    let ev = s.advance(Duration::from_millis(22_001));
    assert!(has(&ev, |m| matches!(m, ServerMsg::WordResult { correct: false, .. })), "time is up");

    let mut u = Sim::new(climb("en"), 9, false);
    u.start();
    let ev = u.next(0);
    assert!(has(&ev, |m| matches!(m, ServerMsg::WordStart { deadline_ms: None, .. })));
}

#[test]
fn row_13_no_speed_component() {
    let mut fast = Sim::new(climb("en"), 10, false);
    let mut slow = Sim::new(climb("en"), 10, false);
    for s in [&mut fast, &mut slow] {
        s.start();
        s.next(0);
    }
    fast.advance(Duration::from_millis(100));
    slow.advance(Duration::from_secs(100));
    let (a, b) = (fast.right(), slow.right());
    let ea = fast.submit(0, &a);
    let eb = slow.submit(0, &b);
    assert_eq!(msgs(&ea), msgs(&eb), "same seed, same answers, any speed: same result");
}

#[test]
fn row_14_word_start_carries_no_word_and_no_length() {
    let mut s = Sim::new(climb("de"), 11, false);
    s.start();
    let ev = s.next(0);
    let entry = s.live_entry().unwrap();
    let ws = msgs(&ev).into_iter().find(|m| matches!(m, ServerMsg::WordStart { .. })).unwrap();
    let json = serde_json::to_string(ws).unwrap();
    assert!(!json.to_lowercase().contains(&citation(&entry).to_lowercase()), "{json}");
    assert!(!json.contains("len"), "{json}");
}

#[test]
fn row_15_a_correct_word_is_followed_after_2200_ms_with_no_definition() {
    let mut s = Sim::new(climb("en"), 12, false);
    s.start();
    s.next(0);
    let r = s.right();
    let ev = s.submit(0, &r);
    let types: Vec<String> = msgs(&ev).iter().map(|m| m.type_name()).collect();
    assert_eq!(types, vec!["word_result"], "nothing else: no meaning, no definition");
    assert!(s.advance(Duration::from_millis(2199)).is_empty());
    let ev = s.advance(Duration::from_millis(1));
    assert!(has(&ev, |m| matches!(m, ServerMsg::WordStart { seat: 0, word_no: 2, .. })));
}

#[test]
fn rows_16_17_a_miss_hands_over_after_1300_ms_and_the_turn_waits_for_the_tap() {
    let mut s = Sim::new(climb("en"), 13, false);
    s.start();
    s.next(0);
    s.submit(0, "zzzzqqq");
    assert!(s.advance(Duration::from_millis(1299)).is_empty());
    let ev = s.advance(Duration::from_millis(1));
    assert!(has(&ev, |m| matches!(m, ServerMsg::TurnStart { seat: 1, .. })));
    // Row 17: no word until seat 1 taps.
    assert!(s.advance(Duration::from_secs(60)).is_empty());
    assert!(s.live_entry().is_none());
    let ev = s.next(1);
    assert!(has(&ev, |m| matches!(m, ServerMsg::WordStart { seat: 1, .. })));
}

#[test]
fn out_of_turn_inputs_are_refused() {
    let mut s = Sim::new(climb("en"), 14, false);
    s.start();
    let ev = s.next(1);
    assert!(has(&ev, |m| *m == ServerMsg::error("not_your_turn")));
    let ev = s.submit(1, "x");
    assert!(has(&ev, |m| *m == ServerMsg::error("not_your_turn")));
    assert!(s.live_entry().is_none());
    // Errors go only to the seat that erred.
    assert!(ev.iter().all(|e| matches!(e, Event::Send { to: Audience::Seat(1), .. })));
}

// ---------------------------------------------------------------- A7.2 replay

#[derive(Clone, Copy)]
enum Op {
    Next(u8),
    Right(u8),
    Wrong(u8),
    Wait(u64),
}

/// Plays a scripted match; the script is regenerated from `seed` so the
/// replay sees the same inputs at the same server timestamps.
fn scripted(seed: u64) -> (Vec<String>, Vec<(u8, u32)>) {
    let mut s = Sim::new(climb("en"), seed, seed.is_multiple_of(3));
    let mut rng = Rng(seed ^ 0xA5A5);
    s.start();
    let mut guard = 0;
    while s.m.running() && guard < 400 {
        guard += 1;
        let a = s.active();
        let op = match &s.m.phase {
            Phase::AwaitNext { .. } => Op::Next(a),
            Phase::Live { .. } => {
                if rng.next() % 10 < 7 {
                    Op::Right(a)
                } else {
                    Op::Wrong(a)
                }
            }
            _ => Op::Wait(rng.next() % 3000),
        };
        match op {
            Op::Next(x) => {
                s.next(x);
            }
            Op::Right(x) => {
                let r = s.right();
                s.advance(Duration::from_millis(rng.next() % 5000));
                if s.live_entry().is_some() {
                    s.submit(x, &r);
                }
            }
            Op::Wrong(x) => {
                s.submit(x, "zzzzqqq");
            }
            Op::Wait(ms) => {
                s.advance(Duration::from_millis(ms));
            }
        }
    }
    assert!(!s.m.running(), "seed {seed}: the match ends");
    // Tickets are random by design (never derived from the seed); mask them.
    let log = s
        .log
        .iter()
        .map(|e| match e {
            Event::Send { to, msg } => {
                let mut v = serde_json::to_value(msg).unwrap();
                if let Some(t) = v.pointer_mut("/body/audio_ticket") {
                    *t = "T".into();
                }
                format!("{to:?} {v}")
            }
            Event::FetchAudio { key, .. } => format!("fetch {key:?}"),
            Event::ExpireAudio { tickets } => format!("expire {}", tickets.len()),
            Event::Over => "over".into(),
        })
        .collect();
    let board = vec![(0, s.m.versus.p1.best), (1, s.m.versus.p2.best)];
    (log, board)
}

#[test]
fn a7_2_a_thousand_recorded_matches_replay_identically() {
    for seed in 0..1000 {
        let a = scripted(seed);
        let b = scripted(seed);
        assert_eq!(a, b, "seed {seed}");
    }
}

// ---------------------------------------------------------------- A7.3 fairness

#[test]
fn a7_3_deadlines_are_judged_on_the_server_clock_with_capped_rtt_grace() {
    let limit = Duration::from_secs(core::tier_time("easy") as u64);
    let one_way = |rtt_ms: u64| Duration::from_millis(rtt_ms / 2);
    for (rtt, early_ms, on_time) in [(40, 50, true), (250, 50, true), (40, -400i64, false), (900, 50, true)] {
        let mut s = Sim::new(req("en", TierMode::Fixed, Some("easy"), true), 20, false);
        // Half the median RTT, capped at 300 ms (here the last 5 are all equal).
        s.grace[0] = Duration::from_millis((rtt / 2).min(300));
        s.start();
        s.next(0);
        let deadline = s.now + limit;
        // The client sends `early_ms` before ITS deadline; the frame lands one-way later.
        let sent = if early_ms >= 0 {
            deadline - Duration::from_millis(early_ms as u64)
        } else {
            deadline + Duration::from_millis((-early_ms) as u64)
        };
        let recv = sent + one_way(rtt).min(Duration::from_millis(300));
        s.now = recv;
        let r = s.right();
        let ev = s.submit(0, &r);
        let correct = has(&ev, |m| matches!(m, ServerMsg::WordResult { correct: true, .. }));
        assert_eq!(correct, on_time, "rtt {rtt} ms, sent {early_ms} ms early");
    }
}

#[test]
fn the_timer_waits_out_the_grace_before_calling_time() {
    let mut s = Sim::new(req("en", TierMode::Fixed, Some("easy"), true), 21, false);
    s.grace[0] = Duration::from_millis(125);
    s.start();
    s.next(0);
    let limit = Duration::from_secs(12);
    assert!(s.advance(limit + Duration::from_millis(120)).is_empty(), "inside the grace");
    let ev = s.advance(Duration::from_millis(10));
    assert!(has(&ev, |m| matches!(m, ServerMsg::WordResult { correct: false, .. })));
}

// ---------------------------------------------------------------- A7.4 parity with local Versus

#[test]
fn a7_4_the_server_scoreboard_is_the_local_one() {
    for seed in 0..300u64 {
        let mut s = Sim::new(climb("es"), seed, false);
        let mut local = Versus::start("P1".into(), "P2".into(), 3);
        let mut rng = Rng(seed);
        s.start();
        while s.m.running() {
            let a = s.active();
            if s.live_entry().is_none() {
                s.next(a);
            }
            let correct = !rng.next().is_multiple_of(3);
            if correct {
                let r = s.right();
                s.submit(a, &r);
                local.record_correct();
                s.advance(correct_delay());
            } else {
                s.submit(a, "zzzzqqq");
                local.end_turn();
                s.advance(MISS_DELAY);
            }
            let v = &s.m.versus;
            assert_eq!((v.p1.best, v.p2.best, v.p1.turns_left, v.p2.turns_left), (local.p1.best, local.p2.best, local.p1.turns_left, local.p2.turns_left));
        }
        assert!(local.over);
        assert_eq!(s.m.versus.winner().map(|w| w as u8), local.winner().map(|w| w as u8));
    }
}

// ---------------------------------------------------------------- F8

fn match_words(lang: &str, seed: u64, kid: bool, tier_mode: TierMode, tier: Option<&str>) -> Vec<(String, String)> {
    let mut s = Sim::new(req(lang, tier_mode, tier, false), seed, kid);
    let mut rng = Rng(seed);
    s.start();
    let mut words = Vec::new();
    while s.m.running() {
        let a = s.active();
        if s.live_entry().is_none() {
            s.next(a);
            if s.live_entry().is_none() {
                break; // bank exhausted ends the match
            }
        }
        words.push((s.live_entry().unwrap(), tier_of_live(&s)));
        // Long chains: mostly right, so climb rooms reach every tier.
        if !rng.next().is_multiple_of(8) {
            let r = s.right();
            s.submit(a, &r);
            s.advance(correct_delay());
        } else {
            s.submit(a, "zzzzqqq");
            s.advance(MISS_DELAY);
        }
    }
    words
}

fn per_lang_matches() -> u64 {
    // A8.1/A8.2 are specified at 10,000 matches per language. Set
    // SPELL_A8_MATCHES to run fewer while iterating; the phase gate runs 10,000.
    std::env::var("SPELL_A8_MATCHES").ok().and_then(|v| v.parse().ok()).unwrap_or(10_000)
}

#[test]
fn a8_1_no_word_repeats_within_a_match_in_any_language() {
    let n = per_lang_matches();
    for lang in grader_parity_languages() {
        for seed in 0..n {
            let words = match_words(lang, seed, false, TierMode::Climb, None);
            let mut seen = HashSet::new();
            for (w, tier) in &words {
                assert!(seen.insert(w.clone()), "{lang} seed {seed}: {w} repeated");
                assert!(core::tier_for(lang, tier).contains(&w.as_str()), "{lang}: {w} is not in its {tier} bank");
            }
        }
    }
}

#[test]
fn a8_2_kid_safe_rooms_serve_nothing_above_the_ceiling_or_on_a_kid_list() {
    let n = per_lang_matches();
    let allowed: HashSet<String> =
        ["easy", "medium", "hard", "expert"].iter().map(|t| core::clamp_base_tier(true, t.to_string())).collect();
    for lang in grader_parity_languages() {
        for seed in 0..n {
            let mode = if seed % 2 == 0 { TierMode::Climb } else { TierMode::Fixed };
            let tier = (seed % 2 == 1).then_some("expert");
            for (w, t) in match_words(lang, seed, true, mode, tier) {
                assert!(allowed.contains(&t), "{lang}: tier {t} above the Spell Jr ceiling");
                assert!(core::kid_allowed(lang, &w), "{lang}: {w} is on the kid list");
            }
        }
    }
    assert!(!allowed.contains("hard") && !allowed.contains("expert"));
}

fn grader_parity_languages() -> Vec<&'static str> {
    core::BUILTIN_LANGS.iter().map(|r| r.0).filter(|l| core::is_active_lang(l)).collect()
}

// ---------------------------------------------------------------- A9.3

#[test]
fn a9_3_tts_outage_pauses_the_word_and_three_in_a_row_end_the_match() {
    let mut s = Sim::new(climb("en"), 30, false);
    s.start();
    s.audio_ok = false;
    for i in 1..=2 {
        let ev = s.next(0);
        assert!(has(&ev, |m| matches!(m, ServerMsg::WordPaused { reason } if reason == "audio_unavailable")), "pause {i}");
        assert!(s.m.running());
        assert_eq!(s.m.versus.p1.turns_left, 3, "a paused word scores nothing");
    }
    // A success resets the run of pauses.
    s.audio_ok = true;
    s.next(0);
    let r = s.right();
    s.submit(0, &r);
    s.advance(correct_delay());
    assert!(s.live_entry().is_some());
    assert_eq!(s.m.pauses, 0);
    s.submit(0, "zzzzqqq");
    s.advance(MISS_DELAY);
    s.audio_ok = false;
    for _ in 0..2 {
        s.next(1);
    }
    let ev = s.next(1);
    assert!(has(&ev, |m| matches!(m, ServerMsg::MatchEnded { reason } if reason == "audio_unavailable")));
    assert!(ev.contains(&Event::Over));
    assert!(!s.m.running());
}

#[test]
fn a_fetch_that_never_answers_counts_as_failed() {
    let mut s = Sim::new(climb("en"), 31, false);
    s.start();
    let ev = s.m.on_next(0, &ctx!(s));
    assert!(matches!(ev[0], Event::FetchAudio { .. })); // not run: nobody answers
    let ev = s.advance(FETCH_TIMEOUT);
    assert!(has(&ev, |m| matches!(m, ServerMsg::WordPaused { .. })));
}

#[test]
fn zh_audio_is_requested_as_the_app_requests_it() {
    let entry = core::tier_for("zh", "easy")[0];
    let k = audio_key("zh", entry, Variant::Normal);
    let (want_text, want_py) = {
        let mut p = entry.splitn(2, '|');
        let w = p.next().unwrap();
        let sp = p.next().unwrap_or("");
        core::voice_target("zh", w, sp)
    };
    assert_eq!((k.text, k.py), (want_text, want_py));
    assert!(audio_key("en", "cat", Variant::Slow).py.is_none());
}

#[test]
fn the_snapshot_never_holds_the_word() {
    let mut s = Sim::new(climb("en"), 32, false);
    s.start();
    s.next(0);
    let entry = s.live_entry().unwrap();
    let snap = s.m.snapshot().to_string();
    assert!(!snap.contains(&format!("\"{entry}\"")), "{snap}");
    assert!(snap.contains("\"live\""));
}

#[test]
fn a_turn_nobody_starts_is_a_miss_after_120_s_in_timed_and_untimed_rooms() {
    for timed in [false, true] {
        let r = if timed { req("en", TierMode::Fixed, Some("easy"), true) } else { climb("en") };
        let mut s = Sim::new(r, 40, false);
        s.start();
        assert!(s.advance(TURN_START_IDLE - Duration::from_millis(1)).is_empty(), "timed={timed}");
        let ev = s.advance(Duration::from_millis(1));
        assert!(has(&ev, |m| matches!(m, ServerMsg::TurnEnd { seat: 0 })), "timed={timed}");
        assert!(!has(&ev, |m| matches!(m, ServerMsg::WordResult { .. })), "no word was served");
        assert_eq!(s.m.versus.p1.turns_left, 2, "the turn is spent");
        let ev = s.advance(MISS_DELAY);
        assert!(has(&ev, |m| matches!(m, ServerMsg::TurnStart { seat: 1, .. })));
    }
}

#[test]
fn a_dropped_player_at_turn_start_waits_on_the_seat_hold_and_returns_to_a_fresh_limit() {
    let mut s = Sim::new(climb("en"), 41, false);
    s.start();
    s.connected[0] = false;
    let ev = s.m.on_disconnect(0, false, &ctx!(s));
    assert!(ev.is_empty());
    assert!(s.advance(Duration::from_secs(300)).is_empty(), "no idle miss while away");
    s.connected[0] = true;
    s.m.on_reconnect(0, &ctx!(s));
    assert!(s.advance(TURN_START_IDLE - Duration::from_millis(1)).is_empty());
    assert!(has(&s.advance(Duration::from_millis(1)), |m| matches!(m, ServerMsg::TurnEnd { seat: 0 })));
}
