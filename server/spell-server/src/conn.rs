//! F1/F2/F4/F5 — one WebSocket connection: the `hello` handshake, frame
//! limits, message-rate budget, keepalive, and handing frames to the hub.

use std::net::IpAddr;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket};
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio::time::{interval_at, timeout, Instant};

use crate::app::AppState;
use crate::hub::{Out, Session};
use crate::limits::{truncate_ip, Bucket, Window};
use crate::names;
use crate::protocol::{close, decode, encode_server, ClientMsg, DecodeError, ProfileKind, ServerMsg, MAX_FRAME};
use crate::store::Auth;

pub const HELLO_TIMEOUT: Duration = Duration::from_secs(10);
pub const PING_EVERY: Duration = Duration::from_secs(15);
pub const PONG_DEADLINE: Duration = Duration::from_secs(45);
pub const BAD_FRAMES_MAX: usize = 5;
pub const BAD_FRAMES_WINDOW: Duration = Duration::from_secs(60);

async fn close_with(ws: &mut WebSocket, code: u16, reason: &'static str) {
    let _ = ws.send(Message::Close(Some(CloseFrame { code, reason: Utf8Bytes::from_static(reason) }))).await;
}

async fn send(ws: &mut WebSocket, msg: &ServerMsg) -> bool {
    ws.send(Message::Text(encode_server(0, msg).into())).await.is_ok()
}

/// Runs one connection to completion.
pub async fn run(mut ws: WebSocket, app: Arc<AppState>, ip: IpAddr) {
    let _guard = ConnGuard::new(&app);
    // ---- hello (A1.4: any other first frame closes with 4002)
    let first = match timeout(HELLO_TIMEOUT, ws.next()).await {
        Ok(Some(Ok(Message::Text(t)))) => t,
        Ok(Some(Ok(Message::Binary(_)))) => return close_with(&mut ws, close::UNSUPPORTED_DATA, "text only").await,
        _ => return close_with(&mut ws, close::NO_HELLO, "hello first").await,
    };
    if first.len() > MAX_FRAME {
        return close_with(&mut ws, close::TOO_LARGE, "frame too large").await;
    }
    let hello = match decode(&first) {
        Ok(inb) => match inb.msg {
            ClientMsg::Hello(h) => h,
            _ => return close_with(&mut ws, close::NO_HELLO, "hello first").await,
        },
        Err(_) => return close_with(&mut ws, close::NO_HELLO, "hello first").await,
    };
    // ---- protocol window (G-D15, A1.2)
    if hello.protocol < app.cfg.protocol_min || hello.protocol > crate::protocol::PROTOCOL_CURRENT {
        send(
            &mut ws,
            &ServerMsg::UpdateRequired { min: app.cfg.protocol_min, current: crate::protocol::PROTOCOL_CURRENT },
        )
        .await;
        return close_with(&mut ws, close::UPDATE_REQUIRED, "update required").await;
    }
    // ---- identity (F2, A2.3)
    let player = match app.store.authenticate(&hello.token) {
        Ok(Auth::Ok { player_id }) => player_id,
        Ok(Auth::Expired) => {
            send(&mut ws, &ServerMsg::error("session_expired")).await;
            return close_with(&mut ws, close::SESSION, "session expired").await;
        }
        Ok(Auth::Unknown) => {
            send(&mut ws, &ServerMsg::error("session_invalid")).await;
            return close_with(&mut ws, close::SESSION, "session invalid").await;
        }
        Ok(Auth::Revoked) => return close_with(&mut ws, close::REVOKED, "revoked").await,
        Err(_) => {
            send(&mut ws, &ServerMsg::error("server_error")).await;
            return close_with(&mut ws, close::SHUTDOWN, "server error").await;
        }
    };
    drop(hello.token);
    // ---- UI language: one the app ships (kid names are localized by it)
    if !names::ui_supported(&hello.ui_lang) {
        send(&mut ws, &ServerMsg::error("bad_ui_lang")).await;
        return close_with(&mut ws, close::NAME, "unsupported ui_lang").await;
    }
    // ---- name (G-D4; A10.3: a kid's display_name is ignored)
    let name = match hello.profile_kind {
        ProfileKind::Kid => names::kid(&hello.ui_lang),
        ProfileKind::Adult => match names::adult(hello.display_name.as_deref().unwrap_or("")) {
            Ok(n) => n,
            Err(_) => {
                send(&mut ws, &ServerMsg::error("name_rejected")).await;
                return close_with(&mut ws, close::NAME, "name rejected").await;
            }
        },
    };
    let conn_id = app.next_conn.fetch_add(1, Ordering::Relaxed);
    let sess = Session { player: player.clone(), conn_id, kind: hello.profile_kind, name, ip };
    let (tx, mut rx) = mpsc::unbounded_channel::<Out>();
    let resume = hello.resume.map(|r| (r.room.to_ascii_uppercase(), r.last_seq));
    app.hub.attach(&sess, tx, resume, Instant::now());
    tracing::info!(event = "conn_open", player_id = %player, ip = %truncate_ip(ip));

    // ---- main loop
    let mut bucket = Bucket::new(Instant::now());
    let mut bad: Window<()> = Window::new(BAD_FRAMES_WINDOW);
    // The first ping goes out at once so a seat has an RTT before its first word.
    let born = Instant::now();
    let mut ping = interval_at(born, PING_EVERY);
    let mut last_pong = born;
    let reason: &str = loop {
        tokio::select! {
            incoming = ws.next() => {
                let now = Instant::now();
                match incoming {
                    Some(Ok(Message::Text(t))) => {
                        if t.len() > MAX_FRAME {
                            close_with(&mut ws, close::TOO_LARGE, "frame too large").await;
                            break "too_large";
                        }
                        if !bucket.take(now) {
                            close_with(&mut ws, close::FLOOD, "slow down").await;
                            break "flood";
                        }
                        match decode(&t) {
                            Ok(inb) => {
                                let started = std::time::Instant::now();
                                app.hub.handle(&sess, inb, now);
                                app.timing.record(started.elapsed());
                            }
                            Err(e) => {
                                if e == DecodeError::TooLarge {
                                    close_with(&mut ws, close::TOO_LARGE, "frame too large").await;
                                    break "too_large";
                                }
                                let _ = send(&mut ws, &ServerMsg::error(e.code())).await;
                                if bad.hit((), now) >= BAD_FRAMES_MAX {
                                    close_with(&mut ws, close::TOO_MANY_BAD, "too many bad frames").await;
                                    break "bad_frames";
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Binary(_))) => {
                        close_with(&mut ws, close::UNSUPPORTED_DATA, "text only").await;
                        break "binary";
                    }
                    Some(Ok(Message::Pong(p))) => {
                        last_pong = now;
                        // G-D17: the payload is our send time, ms since `born`.
                        if let Ok(b) = <[u8; 8]>::try_from(p.as_ref()) {
                            let sent = u64::from_be_bytes(b);
                            let elapsed = now.duration_since(born).as_millis() as u64;
                            if let Some(rtt) = elapsed.checked_sub(sent) {
                                app.hub.note_rtt(&player, rtt.min(60_000) as u32);
                            }
                        }
                    }
                    Some(Ok(Message::Ping(_))) => last_pong = now,
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break "closed",
                }
            }
            out = rx.recv() => {
                match out {
                    Some(Out::Frame(f)) => {
                        if ws.send(Message::Text(f.into())).await.is_err() {
                            break "send_failed";
                        }
                    }
                    Some(Out::Close(code, why)) => {
                        close_with(&mut ws, code, why).await;
                        break why;
                    }
                    None => break "hub_closed",
                }
            }
            _ = ping.tick() => {
                if Instant::now().duration_since(last_pong) >= PONG_DEADLINE {
                    close_with(&mut ws, close::IDLE, "no pong").await;
                    break "no_pong";
                }
                let stamp = Instant::now().duration_since(born).as_millis() as u64;
                if ws.send(Message::Ping(stamp.to_be_bytes().to_vec().into())).await.is_err() {
                    break "send_failed";
                }
            }
        }
    };
    app.hub.detach(&player, conn_id, Instant::now());
    tracing::info!(event = "conn_closed", player_id = %player, code = reason);
}

/// Counts live sockets for the global cap (A4.5).
struct ConnGuard {
    app: Arc<AppState>,
}

impl ConnGuard {
    fn new(app: &Arc<AppState>) -> ConnGuard {
        ConnGuard { app: app.clone() }
    }
}

impl Drop for ConnGuard {
    fn drop(&mut self) {
        self.app.conns.fetch_sub(1, Ordering::Relaxed);
    }
}
