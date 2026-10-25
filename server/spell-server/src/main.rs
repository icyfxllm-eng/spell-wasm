//! spell-server: configured by environment (§8), logs JSON to stdout
//! (journald), drains on SIGTERM and exits 0 (F11).

use spell_server::{app::AppState, config::Config, store::Store};

#[tokio::main]
async fn main() {
    let cfg = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("spell-server: {e}");
            std::process::exit(2);
        }
    };
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::new(&cfg.log))
        .with_current_span(false)
        .init();
    let store = match Store::open(&cfg.db_path, Default::default()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("spell-server: cannot open {}: {e}", cfg.db_path);
            std::process::exit(2);
        }
    };
    let app = AppState::new(cfg, store);
    let a = app.clone();
    tokio::spawn(async move {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("SIGTERM");
        tokio::select! {
            _ = term.recv() => {}
            _ = tokio::signal::ctrl_c() => {}
        }
        a.drain().await;
    });
    if let Err(e) = spell_server::serve(app, None).await {
        eprintln!("spell-server: {e}");
        std::process::exit(1);
    }
}
