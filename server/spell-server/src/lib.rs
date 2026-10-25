//! CC-GAME-SERVER: the authoritative room server for online play.
//!
//! Phase A: protocol (F1), sessions (F2), rooms and knock-to-join (F3), abuse
//! limits (F4), reconnect and resume (F5), the shared grader (F6), and the
//! health/admin/log surface (F11). Online Head-to-Head itself is Phase B.

pub mod app;
pub mod config;
pub mod conn;
pub mod grader;
pub mod hub;
pub mod limits;
pub mod mode;
pub mod names;
pub mod protocol;
pub mod store;

use std::net::SocketAddr;
use std::sync::Arc;

/// Binds both listeners and serves until a drain completes. Returns the
/// bound addresses through `ready` (tests bind port 0).
pub async fn serve(
    app: Arc<app::AppState>,
    ready: Option<tokio::sync::oneshot::Sender<(SocketAddr, SocketAddr)>>,
) -> std::io::Result<()> {
    let public = tokio::net::TcpListener::bind(app.cfg.bind_public).await?;
    let admin = tokio::net::TcpListener::bind(app.cfg.bind_admin).await?;
    let addrs = (public.local_addr()?, admin.local_addr()?);
    app::spawn_background(app.clone());
    let pub_svc = app::public_router(app.clone()).into_make_service_with_connect_info::<SocketAddr>();
    let adm_svc = app::admin_router(app.clone()).into_make_service_with_connect_info::<SocketAddr>();
    let a1 = app.clone();
    let a2 = app.clone();
    let p = tokio::spawn(async move {
        axum::serve(public, pub_svc).with_graceful_shutdown(async move { a1.drained.notified().await }).await
    });
    let d = tokio::spawn(async move {
        axum::serve(admin, adm_svc).with_graceful_shutdown(async move { a2.drained.notified().await }).await
    });
    if let Some(tx) = ready {
        let _ = tx.send(addrs);
    }
    tracing::info!(event = "listening");
    let _ = p.await;
    let _ = d.await;
    Ok(())
}
