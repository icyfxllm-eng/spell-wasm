//! CC-TELEMETRY-FOUNDATION v1.1 — crash reporting (F1) transport.
//!
//! WHAT LEAVES. Only the records in `schema.rs`: an error code, the study
//! language, the last mode entered, and a hash of where the code broke. No
//! word, answer, miss, attempt, or anything else the learning engine keeps
//! (I3, CC-LEARNING-ENGINE D5 as annotated by R9).
//!
//! WHO SENDS WHAT (F6, I6, D7). The route is decided from live inputs every
//! time, never cached across an age-gate answer:
//! - Standard player → per-error events with a per-launch random session id.
//! - Jr, age not yet answered, or the Education edition → daily aggregate
//!   counts by error code, with no identifier of any kind.
//! - Toggle off, or the remote kill switch off → nothing; the queue is cleared.
//!
//! THE KILL SWITCH (I7, R4). The Worker's `GET /v1/flags` is cached in
//! `spell_flag_telemetry_enabled`. Until the server has answered once, errors
//! are held locally and nothing is sent; "off" clears everything on the next
//! launch (and at once, when the answer arrives mid-session).
//!
//! NEVER IN THE WAY (I1). Recording is a bounded localStorage write; sending
//! is a spawned future whose failure only leaves the queue for next time.
//! Nothing here returns an error to a caller or touches the game state.

pub mod schema;

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::consts;
use crate::experience::{self, Audience};
use crate::storage;
use schema::*;

const QUEUE_KEY: &str = "spell_tel_queue_v1";
const AGG_KEY: &str = "spell_tel_agg_v1";
/// F5 histogram cells for a standard player, "metric|bucket|lang" → count.
const PERF_KEY: &str = "spell_tel_perf_v1";
/// Written by index.html's pre-WASM error hook; drained into the route here.
pub const JSBUF_KEY: &str = "spell_tel_jsbuf_v1";
pub const JSBUF_MAX: usize = 20;
/// F7's toggle ("Help improve SpellGame"). Absent = the edition default (D3, D7).
pub const OPT_KEY: &str = "spell_telemetry_opt_v1";
/// The cached remote kill switch (R4). Same key shape as every local flag.
const REMOTE_FLAG_KEY: &str = "spell_flag_telemetry_enabled";

const FLUSH_EVERY_MS: i32 = 10 * 60 * 1000;
const DAY_MS: f64 = 86_400_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Off,
    /// Hold locally, send nothing: the server has never said telemetry is on.
    Hold,
    Aggregate,
    Events,
}

/// The whole routing policy, pure. `remote` is the cached kill switch
/// (`None` = never fetched), `opted` the F7 toggle (`None` = never touched).
pub fn route_for(remote: Option<bool>, opted: Option<bool>, default_on: bool, education: bool, audience: Audience) -> Route {
    if remote == Some(false) || !opted.unwrap_or(default_on) {
        return Route::Off;
    }
    let route = if education || audience != Audience::Standard { Route::Aggregate } else { Route::Events };
    if remote.is_none() { Route::Hold } else { route }
}

// ---- Stored state ----------------------------------------------------------

/// A queued standard-player event. The session id is attached at send time,
/// so nothing that could link two launches is ever written to storage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Queued {
    c: String,
    l: String,
    m: String,
    h: String,
}

impl Queued {
    fn to_event(&self) -> Option<ErrorEvent> {
        Some(ErrorEvent {
            error_code: ErrorCode::from_wire(&self.c)?,
            lang: Lang::from_wire(&self.l)?,
            mode: Mode::from_wire(&self.m)?,
            stack_hash: Hash64::parse(&self.h)?,
        })
    }
    fn of(e: &ErrorEvent) -> Self {
        Queued { c: e.error_code.as_str().into(), l: e.lang.as_str().into(), m: e.mode.as_str().into(), h: format!("{:016x}", e.stack_hash.0) }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct Agg {
    counts: BTreeMap<String, u32>,
    /// F5 cells with no language: "metric|bucket" → count.
    #[serde(default)]
    perf: BTreeMap<String, u32>,
    /// Epoch ms before which no aggregate may be sent (F6: at most daily).
    due: f64,
}

/// One JS-side error, as index.html's hook buffers it.
#[derive(Debug, Clone, Deserialize)]
struct JsBuffered {
    c: String,
    h: String,
}

/// Append with the transport caps: oldest dropped past 200 events or 128 KB.
fn push_capped(q: &mut Vec<Queued>, item: Queued) {
    q.push(item);
    let over = q.len().saturating_sub(QUEUE_MAX_EVENTS as usize);
    q.drain(..over);
    while q.len() > 1 && serde_json::to_string(q).map_or(0, |s| s.len()) > QUEUE_MAX_BYTES {
        q.remove(0);
    }
}

fn bump(agg: &mut Agg, code: ErrorCode) {
    add(&mut agg.counts, code.as_str().to_string(), 1);
}

fn add(map: &mut BTreeMap<String, u32>, key: String, n: u32) {
    let v = map.entry(key).or_insert(0);
    *v = v.saturating_add(n).min(AGG_MAX_COUNT as u32);
}

fn perf_key(metric: PerfMetric, bucket: Bucket, lang: Option<Lang>) -> String {
    match lang {
        Some(l) => format!("{}|{}|{}", metric.as_str(), bucket.as_str(), l.as_str()),
        None => format!("{}|{}", metric.as_str(), bucket.as_str()),
    }
}

fn perf_rows(map: &BTreeMap<String, u32>) -> Vec<PerfRow> {
    map.iter()
        .filter_map(|(k, n)| {
            let mut it = k.split('|');
            let (m, b, l) = (it.next()?, it.next()?, it.next()?);
            Some(PerfRow { metric: PerfMetric::from_wire(m)?, bucket: Bucket::from_wire(b)?, lang: Lang::from_wire(l)?, count: *n })
        })
        .collect()
}

fn agg_perf_rows(map: &BTreeMap<String, u32>) -> Vec<AggPerfRow> {
    map.iter()
        .filter_map(|(k, n)| {
            let (m, b) = k.split_once('|')?;
            Some(AggPerfRow { metric: PerfMetric::from_wire(m)?, bucket: Bucket::from_wire(b)?, count: *n })
        })
        .collect()
}

/// Demote a standard player's cells to aggregate ones: the language is dropped.
fn fold_perf(into: &mut Agg, cells: &BTreeMap<String, u32>) {
    for r in perf_rows(cells) {
        add(&mut into.perf, perf_key(r.metric, r.bucket, None), r.count);
    }
}

/// Subtract what was sent from what is stored now (anything recorded while
/// the POST was in flight survives).
fn subtract(now: &mut BTreeMap<String, u32>, sent: &BTreeMap<String, u32>) {
    for (k, n) in sent {
        if let Some(v) = now.get_mut(k) {
            *v = v.saturating_sub(*n);
        }
    }
    now.retain(|_, v| *v > 0);
}

/// Next permitted aggregate send: a random moment 24–48 h after `now`, so two
/// sends are always at least a day apart and never at a fixed hour (F6).
fn next_due(now: f64, rand01: f64) -> f64 {
    now + DAY_MS + rand01.clamp(0.0, 1.0) * DAY_MS
}

/// Where a Rust panic happened, stripped of the machine it was built on:
/// `src/game.rs:1547:9`, never a message (which could hold a word).
fn panic_site(file: &str, line: u32, col: u32) -> String {
    let file = file.rsplit_once("/src/").map(|(_, rest)| format!("src/{rest}")).unwrap_or_else(|| file.to_string());
    format!("{file}:{line}:{col}")
}

// ---- Live context -----------------------------------------------------------

thread_local! {
    static MODE: Cell<Mode> = const { Cell::new(Mode::Home) };
    static LANG: Cell<Lang> = const { Cell::new(Lang::Other) };
    static SESSION: Cell<u64> = const { Cell::new(0) };
    static KILLED: Cell<bool> = const { Cell::new(false) };
    static FLUSHING: Cell<bool> = const { Cell::new(false) };
    static LISTENERS: RefCell<Option<(Closure<dyn FnMut()>, Closure<dyn FnMut()>)>> = const { RefCell::new(None) };
}

/// The last mode entered (a coarse hint for F1; never a lifecycle event).
pub fn set_mode(m: Mode) {
    MODE.with(|c| c.set(m));
}

/// The current study language code (`consts::MINE` for My Words).
pub fn set_lang(code: &str) {
    let l = if code == consts::MINE { Lang::Mine } else { Lang::from_wire(code).unwrap_or(Lang::Other) };
    LANG.with(|c| c.set(l));
}

fn remote() -> Option<bool> {
    match storage::get_raw(REMOTE_FLAG_KEY).as_deref() {
        Some("on") => Some(true),
        Some("off") => Some(false),
        _ => None,
    }
}

fn opted() -> Option<bool> {
    match storage::get_raw(OPT_KEY).as_deref() {
        Some("on") => Some(true),
        Some("off") => Some(false),
        _ => None,
    }
}

/// R3 — read the resolver, never compute age: the locked verdict or the saved
/// Jr toggle, and whether the gate has been answered at all.
fn audience_now() -> Audience {
    let kid = crate::agegate::is_kid_locked()
        || storage::get_json::<crate::model::Prefs>(crate::model::PREFS_KEY).is_some_and(|p| p.kid);
    experience::audience(kid, crate::agegate::stored().is_some())
}

fn route_now() -> Route {
    if KILLED.with(Cell::get) {
        return Route::Off;
    }
    route_for(remote(), opted(), consts::telemetry_default_on(), consts::EDITION == consts::Edition::Education, audience_now())
}

fn clear_all() {
    for k in [QUEUE_KEY, AGG_KEY, PERF_KEY, JSBUF_KEY] {
        storage::remove(k);
    }
}

// ---- Recording ---------------------------------------------------------------

/// F1 — record one error. Cheap, bounded, and silent on every failure.
pub fn record_error(code: ErrorCode, stack_hash: Hash64) {
    record(code, LANG.with(Cell::get), stack_hash);
}

/// As `record_error`, for a failure that belongs to another language than the
/// one being played (a pack download for `lang_code`).
pub fn record_error_in(code: ErrorCode, lang_code: &str, stack_hash: Hash64) {
    record(code, Lang::from_wire(lang_code).unwrap_or(Lang::Other), stack_hash);
}

fn record(code: ErrorCode, lang: Lang, stack_hash: Hash64) {
    let ev = ErrorEvent { error_code: code, lang, mode: MODE.with(Cell::get), stack_hash };
    match route_now() {
        Route::Off => {}
        Route::Events | Route::Hold => {
            let mut q: Vec<Queued> = storage::get_json(QUEUE_KEY).unwrap_or_default();
            push_capped(&mut q, Queued::of(&ev));
            storage::set_json(QUEUE_KEY, &q);
        }
        Route::Aggregate => {
            let mut a: Agg = storage::get_json(AGG_KEY).unwrap_or_default();
            bump(&mut a, code);
            storage::set_json(AGG_KEY, &a);
        }
    }
}

/// F5 — count one measurement in its histogram cell. A bucket that doesn't
/// belong to the metric is dropped rather than sent.
pub fn record_perf(metric: PerfMetric, bucket: Bucket, lang_code: &str) {
    if !bucket.fits(metric) {
        return;
    }
    let lang = if lang_code == consts::MINE { Lang::Mine } else { Lang::from_wire(lang_code).unwrap_or(Lang::Other) };
    match route_now() {
        Route::Off => {}
        Route::Events | Route::Hold => {
            let mut m: BTreeMap<String, u32> = storage::get_json(PERF_KEY).unwrap_or_default();
            add(&mut m, perf_key(metric, bucket, Some(lang)), 1);
            storage::set_json(PERF_KEY, &m);
        }
        Route::Aggregate => {
            let mut a: Agg = storage::get_json(AGG_KEY).unwrap_or_default();
            add(&mut a.perf, perf_key(metric, bucket, None), 1);
            storage::set_json(AGG_KEY, &a);
        }
    }
}

/// Milliseconds since the page started loading (`performance.now()`).
pub fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &JsValue::from_str("performance")).ok())
        .and_then(|p| {
            let f: js_sys::Function = js_sys::Reflect::get(&p, &JsValue::from_str("now")).ok()?.dyn_into().ok()?;
            f.call0(&p).ok()?.as_f64()
        })
        .unwrap_or(0.0)
}

/// The panic hook: console output as before, then one `wasm_panic` keyed by
/// source location only. Must not panic itself.
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        console_error_panic_hook::hook(info);
        let site = info.location().map(|l| panic_site(l.file(), l.line(), l.column())).unwrap_or_default();
        record_error(ErrorCode::WasmPanic, Hash64::of(site.as_bytes()));
    }));
}

/// F1 — MetricKit's crash and hang diagnostics, drained from the iOS bridge.
/// Swift reports a kind and a signature; the mapping to error codes is here,
/// so the schema stays the only place an event is defined (I9).
async fn drain_metrickit() {
    let Some(kit) = web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &JsValue::from_str("Capacitor")).ok())
        .and_then(|c| js_sys::Reflect::get(&c, &JsValue::from_str("Plugins")).ok())
        .and_then(|p| js_sys::Reflect::get(&p, &JsValue::from_str("NativeLanguageKit")).ok())
        .filter(|k| !k.is_undefined() && !k.is_null())
    else {
        return;
    };
    let Some(f) = js_sys::Reflect::get(&kit, &JsValue::from_str("metricKitDrain")).ok().and_then(|f| f.dyn_into::<js_sys::Function>().ok()) else {
        return;
    };
    let Some(p) = f.call0(&kit).ok().and_then(|p| p.dyn_into::<js_sys::Promise>().ok()) else { return };
    let Ok(res) = wasm_bindgen_futures::JsFuture::from(p).await else { return };
    let items = js_sys::Reflect::get(&res, &JsValue::from_str("items")).unwrap_or(JsValue::UNDEFINED);
    let Ok(items) = items.dyn_into::<js_sys::Array>() else { return };
    for it in items.iter().take(JSBUF_MAX) {
        let get = |k: &str| js_sys::Reflect::get(&it, &JsValue::from_str(k)).ok().and_then(|v| v.as_string());
        if let (Some(kind), Some(sig)) = (get("kind"), get("sig").and_then(|s| Hash64::parse(&s))) {
            if let Some(code) = native_code(&kind) {
                record_error(code, sig);
            }
        }
    }
}

fn native_code(kind: &str) -> Option<ErrorCode> {
    match kind {
        "crash" => Some(ErrorCode::NativeCrash),
        "hang" => Some(ErrorCode::NativeHang),
        _ => None,
    }
}

/// Move whatever index.html's pre-WASM hook buffered into the live route.
fn drain_js_buffer() {
    let buf: Vec<JsBuffered> = storage::get_json(JSBUF_KEY).unwrap_or_default();
    storage::remove(JSBUF_KEY);
    for b in buf.into_iter().take(JSBUF_MAX) {
        if let (Some(c), Some(h)) = (ErrorCode::from_wire(&b.c), Hash64::parse(&b.h)) {
            record_error(c, h);
        }
    }
}

// ---- Sending -----------------------------------------------------------------

fn window_str(name: &str) -> Option<String> {
    let w: JsValue = web_sys::window()?.into();
    js_sys::Reflect::get(&w, &JsValue::from_str(name)).ok()?.as_string()
}

fn base() -> String {
    window_str("SPELL_TELEMETRY_BASE").unwrap_or_else(|| format!("{}/telemetry", crate::api::api_base()))
}

fn platform() -> Platform {
    let Some(w) = web_sys::window() else { return Platform::Web };
    let get = |o: &JsValue, k: &str| js_sys::Reflect::get(o, &JsValue::from_str(k)).ok().filter(|v| !v.is_undefined() && !v.is_null());
    if get(&w, "__TAURI__").is_some() || get(&w, "__TAURI_INTERNALS__").is_some() {
        return Platform::Desktop;
    }
    let p = get(&w, "Capacitor")
        .and_then(|cap| {
            let f: js_sys::Function = get(&cap, "getPlatform")?.dyn_into().ok()?;
            f.call0(&cap).ok()?.as_string()
        })
        .unwrap_or_default();
    match p.as_str() {
        "ios" => Platform::Ios,
        "android" => Platform::Android,
        _ => Platform::Web,
    }
}

fn build() -> BuildId {
    BuildId::parse(&window_str("SPELL_BUILD").unwrap_or_default())
}

/// POST through index.html's gzip helper when present, plain JSON otherwise.
async fn post(url: &str, body: String) -> bool {
    let helper = web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &JsValue::from_str("SpellTelemetryPost")).ok())
        .and_then(|f| f.dyn_into::<js_sys::Function>().ok());
    match helper {
        Some(f) => match f.call2(&JsValue::NULL, &JsValue::from_str(url), &JsValue::from_str(&body)) {
            Ok(p) => match p.dyn_into::<js_sys::Promise>() {
                Ok(p) => wasm_bindgen_futures::JsFuture::from(p).await.ok().and_then(|v| v.as_bool()).unwrap_or(false),
                Err(_) => false,
            },
            Err(_) => false,
        },
        None => storage::fetch_post_json(url, &body).await.is_ok(),
    }
}

/// Send what the current route allows. At most one flush in flight.
pub fn flush() {
    // JS errors buffered since the last flush join the route first.
    drain_js_buffer();
    if FLUSHING.with(|f| f.replace(true)) {
        return;
    }
    wasm_bindgen_futures::spawn_local(async {
        flush_inner().await;
        FLUSHING.with(|f| f.set(false));
    });
}

async fn flush_inner() {
    match route_now() {
        Route::Off => clear_all(),
        Route::Hold => {}
        Route::Events => {
            let q: Vec<Queued> = storage::get_json(QUEUE_KEY).unwrap_or_default();
            let cells: BTreeMap<String, u32> = storage::get_json(PERF_KEY).unwrap_or_default();
            if q.is_empty() && cells.is_empty() {
                return;
            }
            let events: Vec<ErrorEvent> = q.iter().filter_map(Queued::to_event).collect();
            let batch = EventBatch {
                v: SCHEMA_VERSION,
                build: build(),
                platform: platform(),
                session_id: Hash64(SESSION.with(Cell::get)),
                events,
                perf: perf_rows(&cells),
            };
            let Ok(body) = serde_json::to_string(&batch) else { return };
            if post(&format!("{}/v1/events", base()), body).await {
                // Keep anything recorded while the POST was in flight.
                let mut now: Vec<Queued> = storage::get_json(QUEUE_KEY).unwrap_or_default();
                let sent = q.len().min(now.len());
                now.drain(..sent);
                storage::set_json(QUEUE_KEY, &now);
                let mut now_cells: BTreeMap<String, u32> = storage::get_json(PERF_KEY).unwrap_or_default();
                subtract(&mut now_cells, &cells);
                storage::set_json(PERF_KEY, &now_cells);
            }
        }
        Route::Aggregate => {
            // Standard-era events still queued are demoted to counts: an
            // aggregate device never sends an event, a lang, or a session id.
            let q: Vec<Queued> = storage::get_json(QUEUE_KEY).unwrap_or_default();
            let mut a: Agg = storage::get_json(AGG_KEY).unwrap_or_default();
            if !q.is_empty() {
                for e in q.iter().filter_map(Queued::to_event) {
                    bump(&mut a, e.error_code);
                }
                storage::remove(QUEUE_KEY);
            }
            let cells: BTreeMap<String, u32> = storage::get_json(PERF_KEY).unwrap_or_default();
            if !cells.is_empty() {
                fold_perf(&mut a, &cells);
                storage::remove(PERF_KEY);
            }
            let now = js_sys::Date::now();
            if a.due == 0.0 {
                a.due = now + js_sys::Math::random() * DAY_MS;
            }
            storage::set_json(AGG_KEY, &a);
            if (a.counts.is_empty() && a.perf.is_empty()) || now < a.due {
                return;
            }
            let rows: Vec<AggRow> = a
                .counts
                .iter()
                .filter_map(|(c, n)| Some(AggRow { error_code: ErrorCode::from_wire(c)?, count: *n }))
                .collect();
            let batch = AggBatch { v: SCHEMA_VERSION, build: build(), platform: platform(), rows, perf: agg_perf_rows(&a.perf) };
            let Ok(body) = serde_json::to_string(&batch) else { return };
            if post(&format!("{}/v1/aggregate", base()), body).await {
                let mut latest: Agg = storage::get_json(AGG_KEY).unwrap_or_default();
                subtract(&mut latest.counts, &a.counts);
                subtract(&mut latest.perf, &a.perf);
                latest.due = next_due(now, js_sys::Math::random());
                storage::set_json(AGG_KEY, &latest);
            }
        }
    }
}

/// R4 — ask the Worker whether telemetry is on; cache the answer for the next
/// launch, and honour an "off" immediately.
async fn refresh_flags() {
    let Ok(text) = storage::fetch_text(&format!("{}/v1/flags", base())).await else { return };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { return };
    if schema::validate("flags", &v).is_err() {
        return;
    }
    let on = v["telemetry_enabled"].as_bool() == Some(true);
    storage::set_raw(REMOTE_FLAG_KEY, if on { "on" } else { "off" });
    if !on {
        KILLED.with(|k| k.set(true));
        clear_all();
    }
}

/// Called once from `start()`, after prefs and the age gate are read and
/// before anything that could fail loudly. Everything here is best-effort.
pub fn init(lang: &str) {
    set_lang(lang);
    let route = route_now();
    if route == Route::Off {
        // I7 — a kill switch or an opt-out on the last run: nothing survives.
        clear_all();
    }
    // A random per-launch id, held only in memory (D4).
    let hi = (js_sys::Math::random() * 4_294_967_296.0) as u64;
    let lo = (js_sys::Math::random() * 4_294_967_296.0) as u64;
    SESSION.with(|s| s.set((hi << 32) | lo));
    drain_js_buffer();

    if let Some(w) = web_sys::window() {
        let on_hidden = Closure::<dyn FnMut()>::new(|| {
            let hidden = web_sys::window().and_then(|w| w.document()).is_some_and(|d| d.visibility_state() == web_sys::VisibilityState::Hidden);
            if hidden {
                flush();
            }
        });
        let tick = Closure::<dyn FnMut()>::new(flush);
        if let Some(d) = w.document() {
            let _ = d.add_event_listener_with_callback("visibilitychange", on_hidden.as_ref().unchecked_ref());
        }
        let _ = w.set_interval_with_callback_and_timeout_and_arguments_0(tick.as_ref().unchecked_ref(), FLUSH_EVERY_MS);
        LISTENERS.with(|l| *l.borrow_mut() = Some((on_hidden, tick)));
    }

    wasm_bindgen_futures::spawn_local(async {
        drain_metrickit().await;
        refresh_flags().await;
        flush();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_policy() {
        use Audience::*;
        // Standard, server said on, toggle untouched, consumer build.
        assert_eq!(route_for(Some(true), None, true, false, Standard), Route::Events);
        // I6 — Jr and unknown age never get events.
        assert_eq!(route_for(Some(true), None, true, false, Junior), Route::Aggregate);
        assert_eq!(route_for(Some(true), None, true, false, Unknown), Route::Aggregate);
        // D7 — Education: off unless the school turns it on, then aggregate only.
        assert_eq!(route_for(Some(true), None, false, true, Standard), Route::Off);
        assert_eq!(route_for(Some(true), Some(true), false, true, Standard), Route::Aggregate);
        // I7 — the kill switch beats everything, including an explicit opt-in.
        assert_eq!(route_for(Some(false), Some(true), true, false, Standard), Route::Off);
        // F7 — the player's toggle.
        assert_eq!(route_for(Some(true), Some(false), true, false, Standard), Route::Off);
        // R4 — never heard from the server: hold, don't send.
        assert_eq!(route_for(None, None, true, false, Standard), Route::Hold);
        assert_eq!(route_for(None, None, true, false, Junior), Route::Hold);
    }

    #[test]
    fn queue_caps_drop_oldest() {
        let mut q = Vec::new();
        for i in 0..(QUEUE_MAX_EVENTS + 25) {
            push_capped(&mut q, Queued { c: "wasm_panic".into(), l: "en".into(), m: "home".into(), h: format!("{i:016x}") });
        }
        assert_eq!(q.len(), QUEUE_MAX_EVENTS as usize);
        assert_eq!(q[0].h, format!("{:016x}", 25));
        assert!(serde_json::to_string(&q).unwrap().len() <= QUEUE_MAX_BYTES);
    }

    #[test]
    fn aggregate_is_counts_only_and_bounded() {
        let mut a = Agg::default();
        for _ in 0..3 {
            bump(&mut a, ErrorCode::JsUncaught);
        }
        bump(&mut a, ErrorCode::WasmPanic);
        assert_eq!(a.counts.get("js_uncaught"), Some(&3));
        assert_eq!(a.counts.len(), 2);
        // The key space is the error-code enum and nothing else.
        assert!(a.counts.keys().all(|k| ErrorCode::from_wire(k).is_some()));
    }

    #[test]
    fn aggregate_sends_are_a_day_apart_at_a_random_time() {
        let now = 1_000_000.0;
        assert_eq!(next_due(now, 0.0), now + DAY_MS);
        assert_eq!(next_due(now, 1.0), now + 2.0 * DAY_MS);
        assert!(next_due(now, 0.37) > now + DAY_MS);
    }

    #[test]
    fn perf_cells_round_trip_and_demote_without_lang() {
        let mut cells = BTreeMap::new();
        add(&mut cells, perf_key(PerfMetric::TapToAudioMs, Bucket::Lt250, Some(Lang::Ru)), 2);
        add(&mut cells, perf_key(PerfMetric::TapToAudioMs, Bucket::Lt250, Some(Lang::En)), 3);
        add(&mut cells, perf_key(PerfMetric::AudioResolution, Bucket::Unavailable, Some(Lang::Zh)), 1);
        assert_eq!(perf_rows(&cells).len(), 3);
        let mut a = Agg::default();
        fold_perf(&mut a, &cells);
        // ru + en collapse into one language-free cell.
        assert_eq!(a.perf.get("tap_to_audio_ms|lt250"), Some(&5));
        assert_eq!(a.perf.get("audio_resolution|unavailable"), Some(&1));
        assert!(a.perf.keys().all(|k| k.split('|').count() == 2));
        let rows = agg_perf_rows(&a.perf);
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn subtract_keeps_what_arrived_in_flight() {
        let mut sent = BTreeMap::new();
        add(&mut sent, "a".into(), 2);
        let mut now = BTreeMap::new();
        add(&mut now, "a".into(), 5);
        add(&mut now, "b".into(), 1);
        subtract(&mut now, &sent);
        assert_eq!(now.get("a"), Some(&3));
        assert_eq!(now.get("b"), Some(&1));
        let all = now.clone();
        subtract(&mut now, &all);
        assert!(now.is_empty());
    }

    #[test]
    fn metrickit_kinds_map_to_native_codes_only() {
        assert_eq!(native_code("crash"), Some(ErrorCode::NativeCrash));
        assert_eq!(native_code("hang"), Some(ErrorCode::NativeHang));
        assert_eq!(native_code("CRASH"), None);
        assert_eq!(native_code("launch"), None);
    }

    #[test]
    fn panic_site_is_location_only() {
        assert_eq!(panic_site("/Users/eric/repos/spell-wasm/src/game.rs", 1547, 9), "src/game.rs:1547:9");
        assert_eq!(panic_site("src/lib.rs", 1, 1), "src/lib.rs:1:1");
        // Same site on two machines hashes the same.
        assert_eq!(
            Hash64::of(panic_site("/a/src/x.rs", 3, 4).as_bytes()),
            Hash64::of(panic_site("/b/c/src/x.rs", 3, 4).as_bytes())
        );
    }

    #[test]
    fn queued_round_trips_and_rejects_junk() {
        let e = ErrorEvent { error_code: ErrorCode::PackLoadFailed, lang: Lang::Fil, mode: Mode::Practice, stack_hash: Hash64(42) };
        assert_eq!(Queued::of(&e).to_event(), Some(e));
        let junk = Queued { c: "wasm_panic".into(), l: "klingon".into(), m: "home".into(), h: "00".into() };
        assert_eq!(junk.to_event(), None);
    }
}
