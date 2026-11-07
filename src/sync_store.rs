//! CC-ACCOUNTS Phase 3 -- the sync cycle: collect, push, pull, apply, commit.
//!
//! Runs only with `accounts_enabled` AND `accounts_sync` on and a live session. It never blocks the
//! UI (everything is a spawned future), never raises an error the player has to read, and backs off
//! after a failure. Local data is changed only by applying records the server returned, one storage
//! write per kind, and the cursor moves last; a cycle that dies halfway is simply run again (I5).

use std::cell::{Cell, RefCell};

use wasm_bindgen::{closure::Closure, JsCast};
use wasm_bindgen_futures::spawn_local;

use crate::accounts_net as net;
use crate::sync_model as sm;
use crate::{dom, flags, misses, storage, themes, word_lists, App};

const META_KEY: &str = "spell_acct_sync_v1";
const MIN_GAP_MS: f64 = 2_000.0;
const EVERY_MS: i32 = 5 * 60 * 1000;
const FIRST_DELAY_MS: i32 = 4_000;
const BACKOFF_BASE_MS: f64 = 15_000.0;
const BACKOFF_MAX_MS: f64 = 15.0 * 60.0 * 1000.0;

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    static RUNNING: Cell<bool> = const { Cell::new(false) };
    static LAST_START: Cell<f64> = const { Cell::new(0.0) };
    static FAILS: Cell<u32> = const { Cell::new(0) };
    static NOT_BEFORE: Cell<f64> = const { Cell::new(0.0) };
}

fn now() -> f64 {
    js_sys::Date::now()
}

fn on() -> bool {
    flags::accounts_enabled() && flags::accounts_sync()
}

fn load_meta() -> sm::Meta {
    storage::get_json(META_KEY).unwrap_or_default()
}

fn gid() -> String {
    format!("g{:x}{:08x}{:08x}", now() as u64, (js_sys::Math::random() * 4_294_967_295.0) as u32, (js_sys::Math::random() * 4_294_967_295.0) as u32)
}

/// Everything this device would send, as records. Mints global ids for lists seen for the first time.
fn collect(app: &App, meta: &mut sm::Meta, at: f64) -> Vec<sm::SyncRecord> {
    let lists = word_lists::load();
    let mut out = sm::collect_lists(&lists, meta, at, &mut gid);
    out.extend(sm::collect_misses(&app.borrow().misses, meta, at));
    out.extend(sm::collect_themes(&themes::load().unlocked, meta, at));
    out
}

/// Start a cycle soon, unless one is running, one just ran, or we are backing off.
pub fn kick() {
    if !on() || RUNNING.with(Cell::get) {
        return;
    }
    let t = now();
    if t < NOT_BEFORE.with(Cell::get) || t - LAST_START.with(Cell::get) < MIN_GAP_MS {
        return;
    }
    let Some(app) = APP.with(|a| a.borrow().clone()) else { return };
    RUNNING.with(|r| r.set(true));
    LAST_START.with(|l| l.set(t));
    spawn_local(async move {
        let outcome = cycle(&app).await;
        RUNNING.with(|r| r.set(false));
        match outcome {
            Outcome::Done | Outcome::Skipped => FAILS.with(|f| f.set(0)),
            Outcome::Failed => {
                let n = FAILS.with(|f| {
                    f.set(f.get().saturating_add(1));
                    f.get()
                });
                let wait = (BACKOFF_BASE_MS * 2f64.powi(n as i32 - 1)).min(BACKOFF_MAX_MS);
                NOT_BEFORE.with(|b| b.set(now() + wait));
            }
        }
    });
}

enum Outcome {
    Done,
    Skipped,
    Failed,
}

async fn cycle(app: &App) -> Outcome {
    if !net::signed_in().await {
        return Outcome::Skipped;
    }
    let started = now();
    let mut meta = load_meta();
    let cur = collect(app, &mut meta, started);
    let tombs = sm::tombstones(&cur, &meta, started);
    let push = sm::to_push(&cur, tombs, &meta);

    let (mut since, mut sent, mut pulled) = (meta.cursor, false, Vec::new());
    loop {
        let batch: &[sm::SyncRecord] = if sent { &[] } else { &push };
        let resp = match net::sync_call(since, batch).await {
            Ok(r) => r,
            Err(_) => return Outcome::Failed,
        };
        sent = true;
        since = resp.cursor;
        pulled.extend(resp.records);
        if !resp.more {
            break;
        }
    }

    // Apply to what is on the device NOW, not to what it held before the network: the player may have
    // edited a list while we waited.
    let at = now();
    let mut meta_now = load_meta();
    meta_now.list_ids.extend(meta.list_ids.clone());
    let fresh = collect(app, &mut meta_now, at);
    let local = sm::ts_map(&fresh);

    let mut applied = Vec::new();
    let mut lists = word_lists::load();
    let list_changes = sm::apply_lists(&mut lists, &mut meta_now, &local, &pulled);
    // CC-ORTHO-FIX-FIL-ROMANCE 4d: a pulled record can still carry an old spelling; repair it before it is stored. The old record
    // then leaves this device and the existing tombstone step tells the server, so no server-side data is edited.
    let sweep_log = crate::saved_words::rewrite_lists(&mut lists, at);
    crate::saved_words::record(&sweep_log);
    if !list_changes.is_empty() || !sweep_log.is_empty() {
        word_lists::store(&lists);
        applied.extend(list_changes);
    }
    let miss_changes = {
        let mut st = app.borrow_mut();
        let changes = sm::apply_misses(&mut st.misses, &local, &pulled, crate::model::MISS_CAP);
        let sweep_log = crate::saved_words::rewrite_misses(&mut st.misses);
        crate::saved_words::record(&sweep_log);
        if !changes.is_empty() || !sweep_log.is_empty() {
            misses::save(&st);
        }
        changes
    };
    applied.extend(miss_changes);
    let mut store = themes::load();
    if sm::apply_themes(&mut store.unlocked, &pulled) {
        themes::save(&store);
    }

    let after = collect(app, &mut meta_now, at);
    sm::commit(&mut meta_now, &push, &pulled, &applied, &after, since);
    storage::set_json(META_KEY, &meta_now);
    Outcome::Done
}

/// Wire the triggers: shortly after launch, on coming back to the foreground, and every few minutes
/// while the app stays open. Sign-in calls `kick` itself.
pub fn setup(app: &App) {
    APP.with(|a| *a.borrow_mut() = Some(app.clone()));
    dom::after_ms(FIRST_DELAY_MS, kick);
    let vis = Closure::<dyn FnMut()>::new(|| {
        if dom::doc().visibility_state() == web_sys::VisibilityState::Visible {
            kick();
        }
    });
    let _ = dom::doc().add_event_listener_with_callback("visibilitychange", vis.as_ref().unchecked_ref());
    vis.forget();
    let tick = Closure::<dyn FnMut()>::new(kick);
    let _ = dom::window().set_interval_with_callback_and_timeout_and_arguments_0(tick.as_ref().unchecked_ref(), EVERY_MS);
    tick.forget();
}
