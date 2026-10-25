//! F4 — abuse limits. Every one is enforced here, server-side; Cloudflare is
//! only ever an extra outer layer.

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::net::IpAddr;
use std::time::Duration;

use tokio::time::Instant;

/// Counts events per key in a sliding window.
pub struct Window<K> {
    span: Duration,
    hits: HashMap<K, VecDeque<Instant>>,
}

impl<K: Eq + Hash + Clone> Window<K> {
    pub fn new(span: Duration) -> Window<K> {
        Window { span, hits: HashMap::new() }
    }

    fn trim(&mut self, key: &K, now: Instant) -> usize {
        let span = self.span;
        match self.hits.get_mut(key) {
            Some(q) => {
                while q.front().is_some_and(|t| now.duration_since(*t) >= span) {
                    q.pop_front();
                }
                q.len()
            }
            None => 0,
        }
    }

    pub fn count(&mut self, key: &K, now: Instant) -> usize {
        self.trim(key, now)
    }

    pub fn hit(&mut self, key: K, now: Instant) -> usize {
        self.trim(&key, now);
        let q = self.hits.entry(key).or_default();
        q.push_back(now);
        q.len()
    }

    /// Drops keys with no hits in the window, so the maps cannot grow forever.
    pub fn gc(&mut self, now: Instant) {
        let span = self.span;
        self.hits.retain(|_, q| q.back().is_some_and(|t| now.duration_since(*t) < span));
    }
}

/// Per-connection message budget: 20/s sustained, burst 40 (A4.3).
pub struct Bucket {
    tokens: f64,
    rate: f64,
    burst: f64,
    last: Instant,
}

impl Bucket {
    pub const RATE: f64 = 20.0;
    pub const BURST: f64 = 40.0;

    pub fn new(now: Instant) -> Bucket {
        Bucket { tokens: Self::BURST, rate: Self::RATE, burst: Self::BURST, last: now }
    }

    /// Takes one token; `false` means the connection is flooding.
    pub fn take(&mut self, now: Instant) -> bool {
        let dt = now.duration_since(self.last).as_secs_f64();
        self.last = now;
        self.tokens = (self.tokens + dt * self.rate).min(self.burst);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

pub const JOIN_FAILS_PER_MIN: usize = 10;
pub const CREATES_PER_MIN: usize = 5;
pub const MINTS_PER_HOUR: usize = 5;
pub const PENALTY: Duration = Duration::from_secs(60);

/// The shared limiters (join failures, room creation, session minting).
pub struct Limits {
    join_fail_ip: Window<IpAddr>,
    join_fail_player: Window<String>,
    blocked_ip: HashMap<IpAddr, Instant>,
    blocked_player: HashMap<String, Instant>,
    creates: Window<String>,
    mints: Window<IpAddr>,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            join_fail_ip: Window::new(Duration::from_secs(60)),
            join_fail_player: Window::new(Duration::from_secs(60)),
            blocked_ip: HashMap::new(),
            blocked_player: HashMap::new(),
            creates: Window::new(Duration::from_secs(60)),
            mints: Window::new(Duration::from_secs(3600)),
        }
    }
}

impl Limits {
    /// Is this joiner inside a 60 s penalty?
    pub fn join_blocked(&mut self, ip: IpAddr, player: &str, now: Instant) -> bool {
        fn live<K: Eq + Hash>(m: &mut HashMap<K, Instant>, k: &K, now: Instant) -> bool {
            match m.get(k) {
                Some(until) if *until > now => true,
                Some(_) => {
                    m.remove(k);
                    false
                }
                None => false,
            }
        }
        live(&mut self.blocked_ip, &ip, now) || live(&mut self.blocked_player, &player.to_string(), now)
    }

    /// Records a failed join. Returns true when this failure trips the limit.
    pub fn join_failed(&mut self, ip: IpAddr, player: &str, now: Instant) -> bool {
        let by_ip = self.join_fail_ip.hit(ip, now);
        let by_player = self.join_fail_player.hit(player.to_string(), now);
        let mut tripped = false;
        if by_ip > JOIN_FAILS_PER_MIN {
            self.blocked_ip.insert(ip, now + PENALTY);
            tripped = true;
        }
        if by_player > JOIN_FAILS_PER_MIN {
            self.blocked_player.insert(player.to_string(), now + PENALTY);
            tripped = true;
        }
        tripped
    }

    /// One `create_room`; false once a player passes 5 in a minute.
    pub fn allow_create(&mut self, player: &str, now: Instant) -> bool {
        if self.creates.count(&player.to_string(), now) >= CREATES_PER_MIN {
            return false;
        }
        self.creates.hit(player.to_string(), now);
        true
    }

    /// One session mint; false for the sixth from an IP within an hour (A2.4).
    pub fn allow_mint(&mut self, ip: IpAddr, now: Instant) -> bool {
        if self.mints.count(&ip, now) >= MINTS_PER_HOUR {
            return false;
        }
        self.mints.hit(ip, now);
        true
    }

    pub fn gc(&mut self, now: Instant) {
        self.join_fail_ip.gc(now);
        self.join_fail_player.gc(now);
        self.creates.gc(now);
        self.mints.gc(now);
        self.blocked_ip.retain(|_, t| *t > now);
        self.blocked_player.retain(|_, t| *t > now);
    }
}

/// F11: IPs are logged truncated to /24 (IPv4) or /48 (IPv6).
pub fn truncate_ip(ip: IpAddr) -> String {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            format!("{}.{}.{}.0/24", o[0], o[1], o[2])
        }
        IpAddr::V6(v6) => {
            let s = v6.segments();
            format!("{:x}:{:x}:{:x}::/48", s[0], s[1], s[2])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn a4_1_the_eleventh_bad_code_in_a_minute_trips() {
        let mut l = Limits::default();
        let ip: IpAddr = "203.0.113.9".parse().unwrap();
        let now = Instant::now();
        for i in 0..10 {
            assert!(!l.join_failed(ip, "p", now), "failure {} is within budget", i + 1);
            assert!(!l.join_blocked(ip, "p", now));
        }
        assert!(l.join_failed(ip, "p", now));
        assert!(l.join_blocked(ip, "p", now));
        assert!(l.join_blocked(ip, "other-player", now), "the IP is blocked for everyone on it");
        assert!(!l.join_blocked(ip, "p", now + PENALTY + Duration::from_millis(1)), "the penalty lasts 60 s");
    }

    #[tokio::test(start_paused = true)]
    async fn a2_4_sixth_mint_from_one_ip_in_an_hour_is_refused() {
        let mut l = Limits::default();
        let ip: IpAddr = "198.51.100.1".parse().unwrap();
        let now = Instant::now();
        for _ in 0..5 {
            assert!(l.allow_mint(ip, now));
        }
        assert!(!l.allow_mint(ip, now));
        assert!(l.allow_mint("198.51.100.2".parse().unwrap(), now), "other IPs are unaffected");
        assert!(l.allow_mint(ip, now + Duration::from_secs(3600)), "the window slides");
    }

    #[tokio::test(start_paused = true)]
    async fn create_room_is_five_per_minute() {
        let mut l = Limits::default();
        let now = Instant::now();
        for _ in 0..5 {
            assert!(l.allow_create("p", now));
        }
        assert!(!l.allow_create("p", now));
        assert!(l.allow_create("p", now + Duration::from_secs(60)));
    }

    #[tokio::test(start_paused = true)]
    async fn bucket_allows_burst_then_sustained_rate() {
        let now = Instant::now();
        let mut b = Bucket::new(now);
        for _ in 0..40 {
            assert!(b.take(now));
        }
        assert!(!b.take(now), "41st in the same instant floods");
        assert!(b.take(now + Duration::from_millis(60)), "a token refills every 50 ms");
    }

    #[test]
    fn truncation() {
        assert_eq!(truncate_ip("203.0.113.77".parse().unwrap()), "203.0.113.0/24");
        assert_eq!(truncate_ip("2001:db8:abcd:12::1".parse().unwrap()), "2001:db8:abcd::/48");
    }
}
