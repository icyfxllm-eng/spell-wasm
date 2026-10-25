//! F9 — audio behind opaque tickets.
//!
//! The server fetches the clip from the Mac mini's `/api/speak` (over the
//! tailnet) with exactly the request the app makes, strips any tag that
//! could carry text, keeps it in a 64 MB in-memory LRU, and serves it only
//! to a member of the ticket's room. The word never travels to a client
//! before it resolves (G-D12): not in a URL, a header, or the bytes.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::Instant;

use crate::hub::AudioJob;
use crate::mode::AudioKey;
use crate::protocol::Variant;

pub const CACHE_BYTES: usize = 64 * 1024 * 1024;
/// F9: three retries within five seconds, then the word pauses.
pub const RETRIES: u32 = 3;
pub const RETRY_WINDOW: Duration = Duration::from_secs(5);

pub type Fetch<'a> = Pin<Box<dyn Future<Output = Result<Vec<u8>, String>> + Send + 'a>>;

/// Where clips come from. Production: the Mac mini. Tests: a fake.
pub trait AudioSource: Send + Sync {
    fn fetch<'a>(&'a self, key: &'a AudioKey) -> Fetch<'a>;
}

/// `GET {base}/api/speak?word=&variant=&lang=[&py=]`, the app's request (R4).
pub struct HttpSource {
    base: String,
    client: reqwest::Client,
}

impl HttpSource {
    pub fn new(base: &str) -> HttpSource {
        HttpSource {
            base: base.trim_end_matches('/').to_string(),
            client: reqwest::Client::builder().timeout(Duration::from_secs(3)).build().expect("http client"),
        }
    }

    pub fn url(base: &str, key: &AudioKey) -> String {
        let variant = match key.variant {
            Variant::Normal => "normal",
            Variant::Slow => "slow",
        };
        let mut params = vec![("word", key.text.as_str()), ("variant", variant), ("lang", key.lang.as_str())];
        if let Some(py) = &key.py {
            params.push(("py", py.as_str()));
        }
        reqwest::Url::parse_with_params(&format!("{base}/api/speak"), &params).expect("tts url").to_string()
    }
}

impl AudioSource for HttpSource {
    fn fetch<'a>(&'a self, key: &'a AudioKey) -> Fetch<'a> {
        Box::pin(async move {
            let r = self.client.get(HttpSource::url(&self.base, key)).send().await.map_err(|e| e.to_string())?;
            if !r.status().is_success() {
                return Err(format!("tts status {}", r.status()));
            }
            Ok(r.bytes().await.map_err(|e| e.to_string())?.to_vec())
        })
    }
}

/// Removes ID3v2 (head), ID3v1 (tail) and APEv2 (tail) tags: the places an
/// MP3 can carry a title, which could be the word (F9).
pub fn strip_tags(bytes: &[u8]) -> Vec<u8> {
    let mut b = bytes;
    // ID3v2, possibly more than one, each with a syncsafe size.
    while b.len() >= 10 && &b[..3] == b"ID3" {
        let size = ((b[6] as usize & 0x7f) << 21)
            | ((b[7] as usize & 0x7f) << 14)
            | ((b[8] as usize & 0x7f) << 7)
            | (b[9] as usize & 0x7f);
        let footer = if b[5] & 0x10 != 0 { 10 } else { 0 };
        let total = 10 + size + footer;
        if total > b.len() {
            return Vec::new(); // a lying header: serve nothing rather than guess
        }
        b = &b[total..];
    }
    // ID3v1: the last 128 bytes start with "TAG".
    if b.len() >= 128 && &b[b.len() - 128..b.len() - 125] == b"TAG" {
        b = &b[..b.len() - 128];
    }
    // APEv2 footer: 32 bytes, "APETAGEX", size (LE) covers items + footer.
    if b.len() >= 32 && &b[b.len() - 32..b.len() - 24] == b"APETAGEX" {
        let f = &b[b.len() - 32..];
        let size = u32::from_le_bytes([f[12], f[13], f[14], f[15]]) as usize;
        let has_header = f[23] & 0x80 != 0;
        let total = size + if has_header { 32 } else { 0 };
        if total <= b.len() {
            b = &b[..b.len() - total];
        }
    }
    b.to_vec()
}

struct Lru {
    map: HashMap<AudioKey, (Arc<Vec<u8>>, u64)>,
    bytes: usize,
    tick: u64,
    cap: usize,
}

impl Lru {
    fn get(&mut self, k: &AudioKey) -> Option<Arc<Vec<u8>>> {
        self.tick += 1;
        let t = self.tick;
        self.map.get_mut(k).map(|(v, at)| {
            *at = t;
            v.clone()
        })
    }

    fn put(&mut self, k: AudioKey, v: Arc<Vec<u8>>) {
        if v.len() > self.cap {
            return;
        }
        self.tick += 1;
        if let Some((old, _)) = self.map.insert(k, (v.clone(), self.tick)) {
            self.bytes -= old.len();
        }
        self.bytes += v.len();
        while self.bytes > self.cap {
            let Some(oldest) = self.map.iter().min_by_key(|(_, (_, at))| *at).map(|(k, _)| k.clone()) else { break };
            if let Some((gone, _)) = self.map.remove(&oldest) {
                self.bytes -= gone.len();
            }
        }
    }
}

pub struct AudioService {
    source: Arc<dyn AudioSource>,
    cache: Mutex<Lru>,
}

impl AudioService {
    pub fn new(source: Arc<dyn AudioSource>) -> AudioService {
        AudioService::with_cap(source, CACHE_BYTES)
    }

    pub fn with_cap(source: Arc<dyn AudioSource>, cap: usize) -> AudioService {
        AudioService { source, cache: Mutex::new(Lru { map: HashMap::new(), bytes: 0, tick: 0, cap }) }
    }

    pub fn cached_bytes(&self) -> usize {
        self.cache.lock().unwrap().bytes
    }

    /// The clip, from cache or the source with up to `RETRIES` retries inside
    /// `RETRY_WINDOW`. `Err` is the explicit "audio unavailable" state.
    pub async fn get(&self, key: &AudioKey) -> Result<Arc<Vec<u8>>, String> {
        if let Some(v) = self.cache.lock().unwrap().get(key) {
            return Ok(v);
        }
        let started = Instant::now();
        let mut last = String::new();
        for attempt in 0..=RETRIES {
            if attempt > 0 {
                let elapsed = Instant::now().duration_since(started);
                if elapsed >= RETRY_WINDOW {
                    break;
                }
                let pause = Duration::from_millis(400 * attempt as u64).min(RETRY_WINDOW - elapsed);
                tokio::time::sleep(pause).await;
            }
            let left = RETRY_WINDOW.saturating_sub(Instant::now().duration_since(started));
            if left.is_zero() {
                break;
            }
            match tokio::time::timeout(left, self.source.fetch(key)).await {
                Ok(Ok(bytes)) => {
                    let clean = strip_tags(&bytes);
                    if clean.is_empty() {
                        last = "empty clip".into();
                        continue;
                    }
                    let v = Arc::new(clean);
                    self.cache.lock().unwrap().put(key.clone(), v.clone());
                    return Ok(v);
                }
                Ok(Err(e)) => last = e,
                Err(_) => last = "tts timeout".into(),
            }
        }
        Err(last)
    }
}

/// Runs fetch jobs from the hub and reports each result back.
pub fn spawn_worker(app: Arc<crate::app::AppState>, mut rx: UnboundedReceiver<AudioJob>) {
    tokio::spawn(async move {
        while let Some(job) = rx.recv().await {
            let a = app.clone();
            tokio::spawn(async move {
                let ok = a.audio.get(&job.key).await.is_ok();
                if !ok {
                    tracing::warn!(event = "tts_unavailable", code = "audio_unavailable");
                }
                a.hub.audio_done(&job.code, &job.ticket, ok, Instant::now());
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn key(text: &str) -> AudioKey {
        AudioKey { lang: "en".into(), text: text.into(), py: None, variant: Variant::Normal }
    }

    #[test]
    fn the_url_is_the_apps_request() {
        let mut k = key("café");
        k.lang = "fr".into();
        assert_eq!(HttpSource::url("http://100.64.0.2:8000", &k), "http://100.64.0.2:8000/api/speak?word=caf%C3%A9&variant=normal&lang=fr");
        let z = AudioKey { lang: "zh".into(), text: "你好".into(), py: Some("ni2 hao3".into()), variant: Variant::Slow };
        assert!(HttpSource::url("http://h", &z).ends_with("variant=slow&lang=zh&py=ni2+hao3"));
    }

    #[test]
    fn tags_that_could_hold_the_word_are_stripped() {
        let audio = vec![0xFFu8, 0xFB, 0x90, 0x00, 1, 2, 3, 4];
        let mut id3v2 = b"ID3\x04\x00\x00\x00\x00\x00\x0c".to_vec();
        id3v2.extend(b"TIT2cat_____");
        let mut v1 = b"TAG".to_vec();
        v1.extend(b"cat");
        v1.resize(128, 0);
        let mut file = id3v2.clone();
        file.extend(&audio);
        file.extend(&v1);
        assert_eq!(strip_tags(&file), audio);
        assert_eq!(strip_tags(&audio), audio, "untagged audio is untouched");
        let mut liar = b"ID3\x04\x00\x00\x7f\x7f\x7f\x7f".to_vec();
        liar.extend(&audio);
        assert!(strip_tags(&liar).is_empty());
    }

    struct Flaky {
        fails: AtomicU32,
        calls: AtomicU32,
    }

    impl AudioSource for Flaky {
        fn fetch<'a>(&'a self, _key: &'a AudioKey) -> Fetch<'a> {
            Box::pin(async move {
                self.calls.fetch_add(1, Ordering::SeqCst);
                if self.fails.load(Ordering::SeqCst) > 0 {
                    self.fails.fetch_sub(1, Ordering::SeqCst);
                    return Err("down".into());
                }
                Ok(vec![1, 2, 3])
            })
        }
    }

    #[tokio::test(start_paused = true)]
    async fn three_retries_then_unavailable_and_a_hit_is_cached() {
        let f = Arc::new(Flaky { fails: AtomicU32::new(99), calls: AtomicU32::new(0) });
        let svc = AudioService::new(f.clone());
        assert!(svc.get(&key("a")).await.is_err());
        assert_eq!(f.calls.load(Ordering::SeqCst), 1 + RETRIES);

        let f = Arc::new(Flaky { fails: AtomicU32::new(2), calls: AtomicU32::new(0) });
        let svc = AudioService::new(f.clone());
        assert_eq!(*svc.get(&key("a")).await.unwrap(), vec![1, 2, 3]);
        assert_eq!(f.calls.load(Ordering::SeqCst), 3);
        svc.get(&key("a")).await.unwrap();
        assert_eq!(f.calls.load(Ordering::SeqCst), 3, "cached");
    }

    #[test]
    fn the_cache_is_byte_capped_least_recently_used() {
        let svc = AudioService::with_cap(Arc::new(Flaky { fails: AtomicU32::new(0), calls: AtomicU32::new(0) }), 10);
        let mut c = svc.cache.lock().unwrap();
        c.put(key("a"), Arc::new(vec![0; 4]));
        c.put(key("b"), Arc::new(vec![0; 4]));
        c.get(&key("a"));
        c.put(key("c"), Arc::new(vec![0; 4]));
        assert!(c.map.contains_key(&key("a")) && c.map.contains_key(&key("c")));
        assert!(!c.map.contains_key(&key("b")), "the least recently used goes first");
        assert!(c.bytes <= 10);
    }
}
