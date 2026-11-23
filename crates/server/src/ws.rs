//! The WebSocket a logged-in player plays over.
//!
//! The first message must be `Hello` with the protocol version and the
//! session token; nothing else is accepted or sent before it, and a
//! connection that does not say it in time is closed. The token travels in
//! that message and never in the address, which servers tend to log.

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use owl_protocol::{ClientMessage, Envelope, ServerMessage, VERSION};
use tokio::sync::mpsc;

use crate::AppState;
use crate::http::ClientAddress;
use crate::hub::{self, GameInput};

/// The largest message a client may send.
const MAX_MESSAGE_BYTES: usize = 16 * 1024;
/// How long a new connection has to say `Hello`.
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);
/// How often an open connection's session is checked again.
const SESSION_CHECK: Duration = Duration::from_secs(30 * 60);

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/ws", get(upgrade))
}

/// Opens a connection, if its address has a place left (`limits`). The
/// place is held for as long as the connection is open. `Hello` checks
/// only a session token, which costs no password hash.
async fn upgrade(
    State(state): State<Arc<AppState>>,
    ClientAddress(address): ClientAddress,
    socket: WebSocketUpgrade,
) -> Response {
    let Some(place) = state.limits.sockets.open(address) else {
        return (StatusCode::TOO_MANY_REQUESTS, "too many game connections from your network").into_response();
    };
    socket.max_message_size(MAX_MESSAGE_BYTES).on_upgrade(move |socket| async move {
        serve(socket, state).await;
        drop(place);
    })
}

async fn send(socket: &mut WebSocket, seq: &mut u64, message: ServerMessage) -> bool {
    *seq += 1;
    let Ok(text) = serde_json::to_string(&Envelope::new(*seq, message)) else { return false };
    socket.send(Message::Text(text.into())).await.is_ok()
}

fn parse(message: &Message) -> Option<Envelope<ClientMessage>> {
    match message {
        Message::Text(text) => serde_json::from_str(text.as_str()).ok(),
        _ => None,
    }
}

/// The player a token belongs to, and whether they confirmed the privacy
/// notice shown now.
async fn username_of(state: &Arc<AppState>, token: &str) -> Option<(String, bool)> {
    let (store, token) = (state.store.clone(), token.to_string());
    tokio::task::spawn_blocking(move || {
        let user = store.user_of(&token).ok()?;
        let current = store.accepted_notice(&user).ok()? == store.notice_version();
        Some((user.username, current))
    })
    .await
    .ok()?
}

async fn serve(mut socket: WebSocket, state: Arc<AppState>) {
    let mut seq = 0;
    // Nothing happens until the client has said who it is.
    let hello = tokio::time::timeout(HELLO_TIMEOUT, socket.recv()).await;
    let Ok(Some(Ok(first))) = hello else { return };
    let Some(Envelope { message: ClientMessage::Hello { v, token }, .. }) = parse(&first) else {
        send(&mut socket, &mut seq, ServerMessage::Error { reason: "say Hello first".into() }).await;
        return;
    };
    if v != VERSION {
        let reason = format!("this server speaks protocol version {VERSION}; update the game");
        send(&mut socket, &mut seq, ServerMessage::Error { reason }).await;
        return;
    }
    let Some((name, notice_current)) = username_of(&state, &token).await else {
        send(&mut socket, &mut seq, ServerMessage::Error { reason: "you are not logged in".into() }).await;
        return;
    };
    if !notice_current {
        let reason = "the privacy notice has changed; read and confirm it under Profile".to_string();
        send(&mut socket, &mut seq, ServerMessage::Error { reason }).await;
        return;
    }

    let (outbox, mut outgoing) = mpsc::unbounded_channel();
    let connection = state.hub.lock().await.connect(&name, outbox);
    let welcome = ServerMessage::Welcome { username: name.clone(), content: state.content_json.clone(), now: 0 };
    if send(&mut socket, &mut seq, welcome).await {
        state.hub.lock().await.resume(&name);
        let mut session_check = tokio::time::interval(SESSION_CHECK);
        session_check.tick().await;
        loop {
            tokio::select! {
                message = outgoing.recv() => {
                    // The hub dropped this connection: logged out, or replaced by a newer one.
                    let Some(message) = message else { break };
                    if !send(&mut socket, &mut seq, message).await {
                        break;
                    }
                }
                incoming = socket.recv() => {
                    let Some(Ok(incoming)) = incoming else { break };
                    if matches!(incoming, Message::Close(_)) {
                        break;
                    }
                    let Some(envelope) = parse(&incoming) else { continue };
                    if let Some(reason) = handle(&state, &name, envelope.message).await
                        && !send(&mut socket, &mut seq, ServerMessage::Error { reason }).await
                    {
                        break;
                    }
                }
                _ = session_check.tick() => {
                    if username_of(&state, &token).await.map(|(who, _)| who).as_deref() != Some(name.as_str()) {
                        break;
                    }
                }
            }
        }
    }
    state.hub.lock().await.disconnect(&name, connection);
}

/// Acts on one message. Returns the reason if it was refused.
async fn handle(state: &Arc<AppState>, name: &str, message: ClientMessage) -> Option<String> {
    let refused = |reason: &str| Some(reason.to_string());
    match message {
        ClientMessage::Hello { .. } => refused("you already said Hello"),
        ClientMessage::CreateLobby => state.hub.lock().await.create(name).err().map(str::to_string),
        ClientMessage::Invite { username } => {
            // Only friends can be invited; the store knows who they are.
            let (store, me, friend) = (state.store.clone(), name.to_string(), username.clone());
            let friends = tokio::task::spawn_blocking(move || {
                store.friends_of_name(&me).is_ok_and(|list| list.iter().any(|f| f.eq_ignore_ascii_case(&friend)))
            })
            .await
            .unwrap_or(false);
            if !friends {
                return refused("you can only invite your friends");
            }
            state.hub.lock().await.invite(name, &username).err().map(str::to_string)
        }
        ClientMessage::JoinLobby { code } => state.hub.lock().await.join(name, &code).err().map(str::to_string),
        ClientMessage::AddBot => state.hub.lock().await.change_bots(name, true).err().map(str::to_string),
        ClientMessage::RemoveBot => state.hub.lock().await.change_bots(name, false).err().map(str::to_string),
        ClientMessage::DeclineInvite { code } => {
            state.hub.lock().await.decline(name, &code);
            None
        }
        ClientMessage::LeaveLobby => {
            state.hub.lock().await.leave(name);
            None
        }
        ClientMessage::StartMatch { fill_with_bots } => {
            hub::start_match(&state.hub, &state.content, &state.questions, name, fill_with_bots).await.err().map(str::to_string)
        }
        ClientMessage::Command { id, command } => {
            let Some((game, seat)) = state.hub.lock().await.seat_of(name) else {
                return refused("you are not in a match");
            };
            let _ = game.send(GameInput::Command { seat, id, command });
            None
        }
        ClientMessage::Sync => {
            let Some((game, seat)) = state.hub.lock().await.seat_of(name) else {
                return refused("you are not in a match");
            };
            let _ = game.send(GameInput::Sync { seat });
            None
        }
    }
}
