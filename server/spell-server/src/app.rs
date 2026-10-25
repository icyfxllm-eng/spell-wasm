//! F11 + F4 — the two listeners, their route tables, CORS/Origin checks,
//! health, admin, and the handling-time stats.
//!
//! Public (127.0.0.1:8080, behind the tunnel) routes exactly four paths;
//! everything else is 404. Admin (127.0.0.1:8081, tailnet SSH only) is a
//! separate router on a separate listener, so no admin path can ever be
//! reached through the tunnel (A11.1).

use std::collections::VecDeque;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{ConnectInfo, Path, RawQuery, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use tokio::time::Instant;

use crate::config::Config;
use crate::hub::Hub;
use crate::limits::truncate_ip;
use crate::store::Store;

pub const HEALTH_CACHE: Duration = Duration::from_secs(30);

/// Handling-time samples for p50/p95 (`/admin/stats`).
#[derive(Default)]
pub struct Timing {
    samples: Mutex<VecDeque<u32>>,
    count: AtomicU64,
}

impl Timing {
    const KEEP: usize = 8192;

    pub fn record(&self, d: Duration) {
        let us = d.as_micros().min(u32::MAX as u128) as u32;
        let mut s = self.samples.lock().unwrap();
        if s.len() == Self::KEEP {
            s.pop_front();
        }
        s.push_back(us);
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn percentiles(&self) -> (u32, u32, u64) {
        let mut v: Vec<u32> = self.samples.lock().unwrap().iter().copied().collect();
        if v.is_empty() {
            return (0, 0, 0);
        }
        v.sort_unstable();
        let at = |p: f64| v[((v.len() - 1) as f64 * p).round() as usize];
        (at(0.50), at(0.95), self.count.load(Ordering::Relaxed))
    }
}

#[derive(Clone, Copy)]
struct Cached {
    ok: bool,
    at: Option<Instant>,
}

pub struct AppState {
    pub cfg: Config,
    pub store: Store,
    pub hub: Hub,
    pub timing: Timing,
    pub conns: AtomicUsize,
    pub next_conn: AtomicU64,
    health: Mutex<Cached>,
    tts: Mutex<Cached>,
    http: reqwest::Client,
    /// Fires when a drain completes (main exits 0).
    pub drained: tokio::sync::Notify,
}

impl AppState {
    pub fn new(cfg: Config, store: Store) -> Arc<AppState> {
        Arc::new(AppState {
            cfg,
            store,
            hub: Hub::new(),
            timing: Timing::default(),
            conns: AtomicUsize::new(0),
            next_conn: AtomicU64::new(1),
            health: Mutex::new(Cached { ok: false, at: None }),
            tts: Mutex::new(Cached { ok: false, at: None }),
            http: reqwest::Client::builder().timeout(Duration::from_secs(3)).build().expect("http client"),
            drained: tokio::sync::Notify::new(),
        })
    }

    fn cached(slot: &Mutex<Cached>, now: Instant) -> Option<bool> {
        let c = *slot.lock().unwrap();
        c.at.filter(|t| now.duration_since(*t) < HEALTH_CACHE).map(|_| c.ok)
    }

    /// Process up and DB writable, probed at most every 30 s.
    pub fn healthy(&self) -> bool {
        let now = Instant::now();
        if let Some(ok) = Self::cached(&self.health, now) {
            return ok;
        }
        let ok = self.store.probe();
        *self.health.lock().unwrap() = Cached { ok, at: Some(now) };
        ok
    }

    pub async fn tts_reachable(&self) -> bool {
        let now = Instant::now();
        if let Some(ok) = Self::cached(&self.tts, now) {
            return ok;
        }
        let url = format!("{}/api/health", self.cfg.tts_base);
        let ok = matches!(self.http.get(url).send().await, Ok(r) if r.status().is_success());
        *self.tts.lock().unwrap() = Cached { ok, at: Some(now) };
        ok
    }

    /// The client IP: CF-Connecting-IP when trusted (loopback bind), else the peer.
    pub fn client_ip(&self, headers: &HeaderMap, peer: SocketAddr) -> IpAddr {
        if self.cfg.trust_proxy {
            if let Some(ip) = headers
                .get("cf-connecting-ip")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.trim().parse().ok())
            {
                return ip;
            }
        }
        peer.ip()
    }

    fn origin_ok(&self, headers: &HeaderMap) -> Option<HeaderValue> {
        let o = headers.get(header::ORIGIN)?;
        let s = o.to_str().ok()?;
        self.cfg.allowed_origins.iter().any(|a| a == s).then(|| o.clone())
    }

    /// Ends the process's matches once drained (F11).
    pub async fn drain(self: &Arc<Self>) {
        let deadline = self.store.clock.unix_ms() + self.cfg.drain.as_millis() as u64;
        self.hub.start_drain(deadline);
        let until = Instant::now() + self.cfg.drain;
        loop {
            if self.hub.live_matches() == 0 || Instant::now() >= until {
                break;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        self.hub.end_all("server_update");
        tracing::info!(event = "drain_complete");
        // Give connection tasks a moment to flush their close frames.
        tokio::time::sleep(Duration::from_millis(200)).await;
        self.drained.notify_waiters();
    }
}

fn not_found() -> Response {
    StatusCode::NOT_FOUND.into_response()
}

fn with_cors(mut r: Response, origin: HeaderValue) -> Response {
    let h = r.headers_mut();
    h.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
    h.insert(header::VARY, HeaderValue::from_static("Origin"));
    r
}

fn preflight(origin: HeaderValue) -> Response {
    let mut r = StatusCode::NO_CONTENT.into_response();
    let h = r.headers_mut();
    h.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
    h.insert(header::ACCESS_CONTROL_ALLOW_METHODS, HeaderValue::from_static("GET, POST"));
    h.insert(header::ACCESS_CONTROL_ALLOW_HEADERS, HeaderValue::from_static("authorization"));
    h.insert(header::ACCESS_CONTROL_MAX_AGE, HeaderValue::from_static("600"));
    h.insert(header::VARY, HeaderValue::from_static("Origin"));
    r
}

// ---------------------------------------------------------------- public

async fn healthz(State(app): State<Arc<AppState>>) -> Response {
    if app.healthy() {
        (StatusCode::OK, "ok").into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "unhealthy").into_response()
    }
}

async fn session(
    State(app): State<Arc<AppState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    let Some(origin) = app.origin_ok(&headers) else { return StatusCode::FORBIDDEN.into_response() };
    if method == Method::OPTIONS {
        return preflight(origin);
    }
    let ip = app.client_ip(&headers, peer);
    if !app.hub.allow_mint(ip, Instant::now()) {
        return with_cors(StatusCode::TOO_MANY_REQUESTS.into_response(), origin);
    }
    match app.store.mint() {
        Ok((player_id, token)) => {
            tracing::info!(event = "session_minted", player_id = %player_id, ip = %truncate_ip(ip));
            let mut r = Json(json!({ "player_id": player_id, "token": token })).into_response();
            r.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            with_cors(r, origin)
        }
        Err(_) => with_cors(StatusCode::SERVICE_UNAVAILABLE.into_response(), origin),
    }
}

async fn ws(
    State(app): State<Arc<AppState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
    upgrade: Result<WebSocketUpgrade, axum::extract::ws::rejection::WebSocketUpgradeRejection>,
) -> Response {
    // A2.2: credentials never ride in a URL.
    if query.as_deref().is_some_and(|q| !q.is_empty()) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    if app.origin_ok(&headers).is_none() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Ok(upgrade) = upgrade else { return StatusCode::BAD_REQUEST.into_response() };
    // A4.5: the global cap, enforced before the upgrade completes.
    let prev = app.conns.fetch_add(1, Ordering::Relaxed);
    if prev >= app.cfg.max_conns {
        app.conns.fetch_sub(1, Ordering::Relaxed);
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    let ip = app.client_ip(&headers, peer);
    let app2 = app.clone();
    upgrade
        // Our own 4 KiB check closes with 1009; this is only a hard ceiling.
        .max_message_size(64 * 1024)
        .max_frame_size(64 * 1024)
        .on_failed_upgrade(move |_| {
            app2.conns.fetch_sub(1, Ordering::Relaxed);
        })
        .on_upgrade(move |socket| crate::conn::run(socket, app, ip))
}

async fn audio(
    State(app): State<Arc<AppState>>,
    Path(_ticket): Path<String>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    let Some(origin) = app.origin_ok(&headers) else { return StatusCode::FORBIDDEN.into_response() };
    if method == Method::OPTIONS {
        return preflight(origin);
    }
    // Phase B fills tickets. Until then no ticket exists, and an unknown
    // ticket is 404, never 403 (A9.2).
    with_cors(not_found(), origin)
}

pub fn public_router(app: Arc<AppState>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/session", post(session).options(session))
        .route("/v1/ws", get(ws))
        .route("/v1/audio/{ticket}", get(audio).options(audio))
        .fallback(|| async { not_found() })
        .method_not_allowed_fallback(|| async { not_found() })
        .with_state(app)
}

// ---------------------------------------------------------------- admin

async fn stats(State(app): State<Arc<AppState>>) -> Response {
    let h = app.hub.stats();
    let (p50, p95, handled) = app.timing.percentiles();
    Json(json!({
        "connections": app.conns.load(Ordering::Relaxed),
        "sessions_live": h.connections,
        "rooms": h.rooms,
        "matches": h.matches,
        "players_seated": h.players_seated,
        "players_stored": app.store.player_count(),
        "handled": handled,
        "handling_us_p50": p50,
        "handling_us_p95": p95,
        "draining": app.hub.draining(),
    }))
    .into_response()
}

async fn readyz(State(app): State<Arc<AppState>>) -> Response {
    let healthy = app.healthy();
    let draining = app.hub.draining();
    let tts = app.tts_reachable().await;
    let ready = healthy && !draining && tts;
    let body = Json(json!({ "ready": ready, "healthy": healthy, "draining": draining, "tts": tts }));
    let code = if ready { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (code, body).into_response()
}

async fn drain(State(app): State<Arc<AppState>>) -> Response {
    let a = app.clone();
    tokio::spawn(async move { a.drain().await });
    (StatusCode::ACCEPTED, "draining").into_response()
}

async fn snapshot(State(app): State<Arc<AppState>>) -> Response {
    let dir = std::path::Path::new(&app.cfg.db_path).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let dest = dir.join("backup").join("snapshot.db");
    match app.store.snapshot(&dest.to_string_lossy()) {
        Ok(()) => (StatusCode::OK, Json(json!({ "path": dest }))).into_response(),
        Err(e) => {
            tracing::warn!(event = "snapshot_failed", code = %e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn revoke(State(app): State<Arc<AppState>>, Path(player_id): Path<String>) -> Response {
    match app.store.revoke(&player_id) {
        Ok(true) => {
            app.hub.kick(&player_id, crate::protocol::close::REVOKED, "revoked");
            tracing::info!(event = "revoked", player_id = %player_id);
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => not_found(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub fn admin_router(app: Arc<AppState>) -> Router {
    Router::new()
        .route("/admin/stats", get(stats))
        .route("/admin/readyz", get(readyz))
        .route("/admin/drain", post(drain))
        .route("/admin/snapshot", post(snapshot))
        .route("/admin/revoke/{player_id}", post(revoke))
        .fallback(|| async { not_found() })
        .method_not_allowed_fallback(|| async { not_found() })
        .with_state(app)
}

/// Housekeeping: hub ticks every 250 ms, retention sweep daily.
pub fn spawn_background(app: Arc<AppState>) {
    let a = app.clone();
    tokio::spawn(async move {
        let mut t = tokio::time::interval(Duration::from_millis(250));
        loop {
            t.tick().await;
            a.hub.tick(Instant::now());
        }
    });
    tokio::spawn(async move {
        let mut t = tokio::time::interval(Duration::from_secs(86_400));
        loop {
            t.tick().await;
            if let Ok(n) = app.store.sweep() {
                tracing::info!(event = "retention_sweep", deleted = n);
            }
        }
    });
}
