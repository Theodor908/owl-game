//! The server from the outside: HTTP requests against the routes, and real
//! WebSocket connections against a listening server.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use argon2::Params;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Request, StatusCode};
use futures_util::{SinkExt, StreamExt};
use owl_core::Content;
use owl_lobby::bank::QuestionBank;
use owl_protocol::{ClientMessage, Envelope, ServerMessage, VERSION};
use owl_server::crypto::{KEY_BYTES, Keys};
use owl_server::limits::{self, Busy, Clock, HashGate};
use owl_server::store::Store;
use owl_server::{AppState, Options, app};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tower::ServiceExt;

const PASSWORD: &str = "a long password";

fn state() -> Arc<AppState> {
    state_in(std::env::temp_dir().join(format!("owl-test-releases-{}", std::process::id())))
}

/// A server whose published build is in `releases`.
fn state_in(releases: std::path::PathBuf) -> Arc<AppState> {
    // Cheap password hashing: these tests are about behaviour, not cost.
    let keys = Keys::new(&[5; KEY_BYTES], Params::new(64, 1, 1, None).unwrap());
    let content = Content::load(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data")).expect("data loads");
    AppState::new(Store::in_memory(keys).unwrap(), content, QuestionBank::default(), "The notice.".into(), releases)
}

/// A server run with `options`, such as a clock the test moves.
fn state_with(options: Options) -> Arc<AppState> {
    let keys = Keys::new(&[5; KEY_BYTES], Params::new(64, 1, 1, None).unwrap());
    let content = Content::load(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data")).expect("data loads");
    let releases = std::env::temp_dir().join(format!("owl-test-releases-{}", std::process::id()));
    AppState::with_options(Store::in_memory(keys).unwrap(), content, QuestionBank::default(), "The notice.".into(), releases, options)
}

/// One request against the routes. Returns the status and the JSON body, if any.
async fn call(state: &Arc<AppState>, method: &str, path: &str, token: Option<&str>, body: Value) -> (StatusCode, Value) {
    let (status, _, body) = call_from(state, Origin { peer: None, cloudflare: None }, method, path, token, body).await;
    (status, body)
}

/// Where a request comes from: the connection's own address, and what a
/// `CF-Connecting-IP` header says.
#[derive(Clone, Copy)]
struct Origin {
    peer: Option<&'static str>,
    cloudflare: Option<&'static str>,
}

fn peer(address: &'static str) -> Origin {
    Origin { peer: Some(address), cloudflare: None }
}

/// One request from a given address. Returns the status, the headers and the JSON body.
async fn call_from(
    state: &Arc<AppState>,
    from: Origin,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Value,
) -> (StatusCode, HeaderMap, Value) {
    let mut request = Request::builder().method(method).uri(path).header("content-type", "application/json");
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    if let Some(address) = from.cloudflare {
        request = request.header("cf-connecting-ip", address);
    }
    let mut request = request.body(Body::from(body.to_string())).unwrap();
    if let Some(address) = from.peer {
        let socket: SocketAddr = format!("{address}:40000").parse().unwrap();
        request.extensions_mut().insert(ConnectInfo(socket));
    }
    let response = app(state.clone()).oneshot(request).await.unwrap();
    let (status, headers) = (response.status(), response.headers().clone());
    let bytes = axum::body::to_bytes(response.into_body(), 1 << 20).await.unwrap();
    (status, headers, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

fn retry_after(headers: &HeaderMap) -> u64 {
    headers.get("retry-after").and_then(|value| value.to_str().ok()).and_then(|value| value.parse().ok()).expect("a Retry-After")
}


async fn register(state: &Arc<AppState>, name: &str) -> String {
    let body = json!({ "username": name, "password": PASSWORD, "notice_acknowledged": true });
    let (status, reply) = call(state, "POST", "/api/register", None, body).await;
    assert_eq!(status, StatusCode::CREATED, "{reply}");
    reply["token"].as_str().unwrap().to_string()
}

async fn befriend(state: &Arc<AppState>, a: (&str, &str), b: (&str, &str)) {
    let (status, _) = call(state, "POST", "/api/friends/request", Some(a.1), json!({ "username": b.0 })).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(state, "POST", "/api/friends/accept", Some(b.1), json!({ "username": a.0 })).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn an_account_is_made_used_exported_and_erased_over_http() {
    let state = state();
    let (status, notice) = {
        let response = app(state.clone()).oneshot(Request::get("/api/privacy").body(Body::empty()).unwrap()).await.unwrap();
        (response.status(), axum::body::to_bytes(response.into_body(), 1 << 20).await.unwrap())
    };
    assert_eq!((status, &notice[..]), (StatusCode::OK, &b"The notice."[..]));

    let unconfirmed = json!({ "username": "Minerva", "password": PASSWORD });
    assert_eq!(call(&state, "POST", "/api/register", None, unconfirmed).await.0, StatusCode::BAD_REQUEST);
    let short = json!({ "username": "Minerva", "password": "too short", "notice_acknowledged": true });
    assert_eq!(call(&state, "POST", "/api/register", None, short).await.0, StatusCode::BAD_REQUEST);
    let token = register(&state, "Minerva").await;
    let again = json!({ "username": "minerva", "password": PASSWORD, "notice_acknowledged": true });
    assert_eq!(call(&state, "POST", "/api/register", None, again).await.0, StatusCode::CONFLICT);

    assert_eq!(call(&state, "GET", "/api/me", None, Value::Null).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&state, "GET", "/api/me", Some("nonsense"), Value::Null).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&state, "GET", "/api/me", Some(&token), Value::Null).await.1["username"], "Minerva");

    let wrong = json!({ "username": "Minerva", "password": "not her password" });
    assert_eq!(call(&state, "POST", "/api/login", None, wrong).await.0, StatusCode::UNAUTHORIZED);
    let (status, login) = call(&state, "POST", "/api/login", None, json!({ "username": "MINERVA", "password": PASSWORD })).await;
    assert_eq!((status, login["username"].as_str()), (StatusCode::OK, Some("Minerva")));

    let other = register(&state, "Archie").await;
    befriend(&state, ("Minerva", &token), ("Archie", &other)).await;
    assert_eq!(call(&state, "GET", "/api/users/archie", Some(&token), Value::Null).await.1["username"], "Archie");
    assert_eq!(call(&state, "GET", "/api/users/nobody", Some(&token), Value::Null).await.0, StatusCode::NOT_FOUND);
    assert_eq!(call(&state, "GET", "/api/users/archie", None, Value::Null).await.0, StatusCode::UNAUTHORIZED, "search needs a login");
    let (_, friends) = call(&state, "GET", "/api/friends", Some(&token), Value::Null).await;
    assert_eq!(friends["friends"], json!([{ "username": "Archie", "online": false }]));

    let (status, export) = call(&state, "GET", "/api/me/export", Some(&token), Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!((export["username"].as_str(), &export["friends"]["friends"]), (Some("Minerva"), &json!(["Archie"])));
    assert!(!export.to_string().contains(PASSWORD));

    let wrong = json!({ "password": "not her password" });
    assert_eq!(call(&state, "DELETE", "/api/me", Some(&token), wrong).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&state, "DELETE", "/api/me", Some(&token), json!({ "password": PASSWORD })).await.0, StatusCode::NO_CONTENT);
    assert_eq!(call(&state, "GET", "/api/me", Some(&token), Value::Null).await.0, StatusCode::UNAUTHORIZED);
    let (_, friends) = call(&state, "GET", "/api/friends", Some(&other), Value::Null).await;
    assert_eq!(friends["friends"], json!([]), "the erased account is gone from her friend's list too");
}

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

struct Player {
    socket: Socket,
    seq: u64,
}

impl Player {
    async fn connect(address: &str, token: &str) -> Self {
        let (socket, _) = tokio_tungstenite::connect_async(format!("ws://{address}/ws")).await.expect("the server accepts");
        let mut player = Self { socket, seq: 0 };
        player.send(ClientMessage::Hello { v: VERSION, token: token.to_string() }).await;
        player
    }

    async fn send(&mut self, message: ClientMessage) {
        self.seq += 1;
        let text = serde_json::to_string(&Envelope::new(self.seq, message)).unwrap();
        self.socket.send(Message::Text(text.into())).await.expect("the message is sent");
    }

    /// The next message, or `None` if the server closed the connection.
    async fn next(&mut self) -> Option<ServerMessage> {
        loop {
            let message = tokio::time::timeout(Duration::from_secs(10), self.socket.next()).await.expect("a message in time");
            match message {
                Some(Ok(Message::Text(text))) => {
                    let envelope: Envelope<ServerMessage> = serde_json::from_str(text.as_str()).expect("a server message");
                    assert_eq!(envelope.v, VERSION);
                    return Some(envelope.message);
                }
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return None,
                Some(Ok(_)) => {}
            }
        }
    }

    /// Reads until a message matches.
    async fn until<T>(&mut self, mut pick: impl FnMut(ServerMessage) -> Option<T>) -> T {
        loop {
            let message = self.next().await.expect("the connection stays open");
            if let Some(found) = pick(message) {
                return found;
            }
        }
    }

    /// Reads until the player's view is in `phase`.
    async fn view_in(&mut self, phase: &str) -> Value {
        self.until(|message| match message {
            ServerMessage::Snapshot { view, .. } | ServerMessage::Events { view, .. } if view["phase"] == phase => Some(view),
            _ => None,
        })
        .await
    }
}

async fn serve(state: &Arc<AppState>) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let router = app(state.clone()).into_make_service_with_connect_info::<SocketAddr>();
    tokio::spawn(async move { axum::serve(listener, router).await });
    address
}

#[tokio::test]
async fn a_connection_must_say_hello_with_a_good_token_and_the_right_version() {
    let state = state();
    let address = serve(&state).await;
    let token = register(&state, "Minerva").await;

    let mut stranger = Player::connect(&address, "not a token").await;
    assert!(matches!(stranger.next().await, Some(ServerMessage::Error { .. })));
    assert_eq!(stranger.next().await, None, "and the connection is closed");

    let (socket, _) = tokio_tungstenite::connect_async(format!("ws://{address}/ws")).await.unwrap();
    let mut old = Player { socket, seq: 0 };
    old.send(ClientMessage::Hello { v: VERSION + 1, token: token.clone() }).await;
    assert!(matches!(old.next().await, Some(ServerMessage::Error { .. })));

    let (socket, _) = tokio_tungstenite::connect_async(format!("ws://{address}/ws")).await.unwrap();
    let mut rude = Player { socket, seq: 0 };
    rude.send(ClientMessage::CreateLobby).await;
    assert!(matches!(rude.next().await, Some(ServerMessage::Error { .. })), "nothing works before Hello");

    let mut minerva = Player::connect(&address, &token).await;
    let Some(ServerMessage::Welcome { username, content, .. }) = minerva.next().await else { panic!("no welcome") };
    assert_eq!(username, "Minerva");
    assert!(content["cards"].as_array().is_some_and(|cards| !cards.is_empty()), "the content comes with the welcome");
}

#[tokio::test]
async fn friends_gather_in_a_lobby_and_play_over_the_socket() {
    let state = state();
    let address = serve(&state).await;
    let (a_token, b_token, c_token) =
        (register(&state, "Minerva").await, register(&state, "Archie").await, register(&state, "Stranger").await);
    befriend(&state, ("Minerva", &a_token), ("Archie", &b_token)).await;

    let mut a = Player::connect(&address, &a_token).await;
    let mut b = Player::connect(&address, &b_token).await;
    let mut c = Player::connect(&address, &c_token).await;
    for player in [&mut a, &mut b, &mut c] {
        assert!(matches!(player.next().await, Some(ServerMessage::Welcome { .. })));
    }

    // The owner opens a lobby and may invite only friends who are online.
    a.send(ClientMessage::CreateLobby).await;
    let code = a.until(|m| if let ServerMessage::LobbyState { code, .. } = m { Some(code) } else { None }).await;
    a.send(ClientMessage::Invite { username: "Stranger".into() }).await;
    assert!(matches!(a.next().await, Some(ServerMessage::Error { .. })), "not a friend");
    c.send(ClientMessage::JoinLobby { code: "NOSUCH".into() }).await;
    assert!(matches!(c.next().await, Some(ServerMessage::Error { .. })), "a wrong code opens nothing");

    a.send(ClientMessage::Invite { username: "archie".into() }).await;
    let invited = b.until(|m| if let ServerMessage::Invited { from, code } = m { Some((from, code)) } else { None }).await;
    assert_eq!(invited, ("Minerva".to_string(), code.clone()));
    b.send(ClientMessage::JoinLobby { code: code.clone() }).await;
    let members = b.until(|m| if let ServerMessage::LobbyState { members, .. } = m { Some(members) } else { None }).await;
    assert_eq!(members, vec!["Minerva", "Archie"]);

    // Only the owner starts. Empty seats go to bots.
    b.send(ClientMessage::StartMatch { fill_with_bots: true }).await;
    assert!(matches!(b.next().await, Some(ServerMessage::Error { .. })));
    a.send(ClientMessage::StartMatch { fill_with_bots: true }).await;
    let seats = a.until(|m| if let ServerMessage::MatchStarted { seats, you } = m { Some((seats, you)) } else { None }).await;
    assert_eq!((seats.0.len(), seats.1), (6, 0));
    assert_eq!(&seats.0[..2], ["Minerva", "Archie"]);
    assert!(seats.0[2..].iter().all(|name| name.ends_with("(bot)")));
    let b_seat = b.until(|m| if let ServerMessage::MatchStarted { you, .. } = m { Some(you) } else { None }).await;
    assert_eq!(b_seat, 1);

    // Each sees their own hero offer and nobody else's.
    let a_view = a.view_in("HeroSelect").await;
    let b_view = b.view_in("HeroSelect").await;
    assert_eq!((a_view["you"].as_u64(), b_view["you"].as_u64()), (Some(0), Some(1)));
    let offer = |view: &Value| view["me"]["hero_offer"].as_array().unwrap().clone();
    assert_eq!(offer(&a_view).len(), 2);
    assert!(offer(&a_view).iter().all(|hero| !offer(&b_view).contains(hero)));

    // A command for someone else's seat is refused; one's own is accepted.
    let pick = |seat: u64, view: &Value| {
        serde_json::from_value(json!({ "PickHero": { "player": seat, "hero": offer(view)[0] } })).unwrap()
    };
    a.send(ClientMessage::Command { id: 1, command: pick(1, &b_view) }).await;
    let refused = a.until(|m| if let ServerMessage::Rejected { id, .. } = m { Some(id) } else { None }).await;
    assert_eq!(refused, 1);
    a.send(ClientMessage::Command { id: 2, command: pick(0, &a_view) }).await;
    b.send(ClientMessage::Command { id: 1, command: pick(1, &b_view) }).await;
    let accepted = a.until(|m| if let ServerMessage::Accepted { id } = m { Some(id) } else { None }).await;
    assert_eq!(accepted, 2);

    // Everyone has picked (the bots at once), so the first shop opens.
    let a_shop = a.view_in("Shop").await;
    assert!(!a_shop["me"]["shop"].as_array().unwrap().is_empty());
    assert_eq!(a_shop["players"].as_array().unwrap().len(), 6);
    a.send(ClientMessage::Command { id: 3, command: serde_json::from_value(json!({ "Buy": { "player": 0, "index": 0 } })).unwrap() }).await;
    let bought = a
        .until(|m| match m {
            ServerMessage::Events { view, .. } if view["me"]["hand"].as_array().is_some_and(|hand| hand.len() == 1) => Some(view),
            _ => None,
        })
        .await;
    assert_eq!(bought["me"]["gold"], 0);

    // A dropped connection keeps its seat: the same player comes back to the same match.
    drop(b);
    let mut b = Player::connect(&address, &b_token).await;
    let back = b.until(|m| if let ServerMessage::MatchStarted { you, .. } = m { Some(you) } else { None }).await;
    assert_eq!(back, 1);
    let b_shop = b.view_in("Shop").await;
    assert_eq!(b_shop["you"], 1);
    assert!(b_shop["me"]["hand"].as_array().unwrap().is_empty(), "her friend's purchase is not in his view");

    // When the owner leaves, the lobby closes for everyone.
    a.send(ClientMessage::LeaveLobby).await;
    b.until(|m| matches!(m, ServerMessage::LobbyClosed).then_some(())).await;
}

#[tokio::test]
async fn an_invitation_can_be_declined_or_taken_by_leaving_ones_own_lobby() {
    let state = state();
    let address = serve(&state).await;
    let (a_token, b_token) = (register(&state, "Minerva").await, register(&state, "Archie").await);
    befriend(&state, ("Minerva", &a_token), ("Archie", &b_token)).await;
    let mut a = Player::connect(&address, &a_token).await;
    let mut b = Player::connect(&address, &b_token).await;
    for player in [&mut a, &mut b] {
        assert!(matches!(player.next().await, Some(ServerMessage::Welcome { .. })));
    }
    let lobby_code = |m| if let ServerMessage::LobbyState { code, .. } = m { Some(code) } else { None };
    let invited_list = |m| if let ServerMessage::LobbyState { invited, .. } = m { Some(invited) } else { None };

    // Declining takes the player off the list the owner sees.
    a.send(ClientMessage::CreateLobby).await;
    let code = a.until(lobby_code).await;
    a.send(ClientMessage::Invite { username: "Archie".into() }).await;
    assert_eq!(a.until(invited_list).await, vec!["Archie"]);
    b.until(|m| matches!(m, ServerMessage::Invited { .. }).then_some(())).await;
    b.send(ClientMessage::DeclineInvite { code: code.clone() }).await;
    assert!(a.until(invited_list).await.is_empty());

    // A player in a lobby of their own is still invited, and joins by leaving it first.
    b.send(ClientMessage::CreateLobby).await;
    let own = b.until(lobby_code).await;
    assert_ne!(own, code);
    a.send(ClientMessage::Invite { username: "Archie".into() }).await;
    b.until(|m| matches!(m, ServerMessage::Invited { .. }).then_some(())).await;
    b.send(ClientMessage::JoinLobby { code: code.clone() }).await;
    assert!(matches!(b.until(|m| matches!(m, ServerMessage::Error { .. }).then_some(m)).await, ServerMessage::Error { .. }));
    b.send(ClientMessage::LeaveLobby).await;
    b.send(ClientMessage::JoinLobby { code: code.clone() }).await;
    let members = b.until(|m| match m {
        ServerMessage::LobbyState { code: joined, members, .. } if joined == code => Some(members),
        _ => None,
    })
    .await;
    assert_eq!(members, vec!["Minerva", "Archie"]);
}

#[tokio::test]
async fn a_guest_joins_by_code_and_the_owner_seats_bots() {
    let state = state();
    let address = serve(&state).await;
    let owner_token = register(&state, "Minerva").await;

    // No acknowledgement, no guest. With it: a made-up name and a session.
    assert_eq!(call(&state, "POST", "/api/guest", None, json!({})).await.0, StatusCode::BAD_REQUEST);
    let (status, guest) = call(&state, "POST", "/api/guest", None, json!({ "notice_acknowledged": true })).await;
    assert_eq!(status, StatusCode::CREATED);
    let (guest_token, guest_name) = (guest["token"].as_str().unwrap().to_string(), guest["username"].as_str().unwrap().to_string());
    assert!(guest_name.starts_with("Guest_"));
    assert_eq!(call(&state, "GET", "/api/me", Some(&guest_token), Value::Null).await.1["guest"], true);
    let rename = json!({ "password": "", "username": "Athena" });
    assert_eq!(call(&state, "POST", "/api/me/username", Some(&guest_token), rename).await.0, StatusCode::FORBIDDEN);

    let mut owner = Player::connect(&address, &owner_token).await;
    let mut visitor = Player::connect(&address, &guest_token).await;
    for player in [&mut owner, &mut visitor] {
        assert!(matches!(player.next().await, Some(ServerMessage::Welcome { .. })));
    }
    let state_of = |m| if let ServerMessage::LobbyState { code, members, bots, .. } = m { Some((code, members, bots)) } else { None };

    owner.send(ClientMessage::CreateLobby).await;
    let (code, _, _) = owner.until(state_of).await;
    // The guest is nobody's friend: the code, told by the owner, is the way in.
    visitor.send(ClientMessage::JoinLobby { code: code.to_lowercase() }).await;
    let (_, members, _) = visitor.until(state_of).await;
    assert_eq!(members, vec!["Minerva".to_string(), guest_name.clone()]);

    // Only the owner seats bots, never past six seats, never below none.
    visitor.send(ClientMessage::AddBot).await;
    assert!(matches!(visitor.until(|m| matches!(m, ServerMessage::Error { .. }).then_some(m)).await, ServerMessage::Error { .. }));
    for _ in 0..4 {
        owner.send(ClientMessage::AddBot).await;
    }
    let bots = owner.until(|m| state_of(m).filter(|(_, _, bots)| *bots == 4).map(|(_, _, bots)| bots)).await;
    assert_eq!(bots, 4);
    owner.send(ClientMessage::AddBot).await;
    assert!(matches!(owner.until(|m| matches!(m, ServerMessage::Error { .. }).then_some(m)).await, ServerMessage::Error { .. }), "six seats");
    owner.send(ClientMessage::RemoveBot).await;
    owner.until(|m| state_of(m).filter(|(_, _, bots)| *bots == 3).map(|_| ())).await;

    // The match has the two players and the three bots, no more.
    owner.send(ClientMessage::StartMatch { fill_with_bots: false }).await;
    let seats = owner.until(|m| if let ServerMessage::MatchStarted { seats, .. } = m { Some(seats) } else { None }).await;
    assert_eq!(seats.len(), 5);
    assert_eq!(seats.iter().filter(|name| name.ends_with("(bot)")).count(), 3);

    // Leaving the game erases the guest.
    assert_eq!(call(&state, "POST", "/api/logout", Some(&guest_token), Value::Null).await.0, StatusCode::NO_CONTENT);
    assert_eq!(call(&state, "GET", &format!("/api/users/{guest_name}"), Some(&owner_token), Value::Null).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_server_says_which_build_is_newest_and_hands_it_out() {
    let state = state();
    let _ = std::fs::remove_dir_all(&state.releases);
    assert_eq!(call(&state, "GET", "/api/version", None, Value::Null).await.1["build"], 0, "nothing published yet");

    std::fs::create_dir_all(&state.releases).unwrap();
    std::fs::write(state.releases.join("version.json"), r#"{"build": 7, "notes": "New heroes."}"#).unwrap();
    std::fs::write(state.releases.join("owl.apk"), b"not really an apk").unwrap();
    let (status, version) = call(&state, "GET", "/api/version", None, Value::Null).await;
    assert_eq!((status, version["build"].as_u64(), version["notes"].as_str()), (StatusCode::OK, Some(7), Some("New heroes.")));

    // The pack and the library a game fetches by itself, and what the
    // server says about them, exactly as it was published.
    let published = json!({
        "build": 8, "notes": "More questions.", "shell": "0123456789abcdef",
        "pack": {"kind": "pack", "build": 8, "shell": "0123456789abcdef", "native": "aa", "platform": "",
                 "sha256": "bb", "size": 4, "sig": "cc"},
        "lib": {"kind": "lib", "build": 7, "shell": "0123456789abcdef", "native": "aa", "platform": "android.arm64",
                "sha256": "aa", "size": 3, "sig": "dd"},
        "apk": {"build": 7, "shell": "0123456789abcdef", "native": "aa"},
    });
    std::fs::write(state.releases.join("version.json"), published.to_string()).unwrap();
    std::fs::write(state.releases.join("owl.pck"), b"pack").unwrap();
    std::fs::write(state.releases.join("libowl_local.so"), b"lib").unwrap();
    assert_eq!(call(&state, "GET", "/api/version", None, Value::Null).await.1, published);

    for (address, content) in [
        ("/download/owl.apk?build=7", &b"not really an apk"[..]),
        ("/download/owl.pck?build=8", b"pack"),
        ("/download/libowl_local.so?build=7", b"lib"),
    ] {
        let response = app(state.clone()).oneshot(Request::get(address).body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{address}");
        assert_eq!(response.headers()["cache-control"], "no-store", "a kept copy would hand out an older build");
        assert_eq!(&axum::body::to_bytes(response.into_body(), 1 << 20).await.unwrap()[..], content);
    }
    std::fs::remove_file(state.releases.join("owl.pck")).unwrap();
    let missing = app(state.clone()).oneshot(Request::get("/download/owl.pck").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND, "a release with no pack");
    let _ = std::fs::remove_dir_all(&state.releases);
}

/// One GET as a browser makes it. Returns the status, a header and the body.
async fn browse(state: &Arc<AppState>, path: &str, header: &str) -> (StatusCode, String, Vec<u8>) {
    let response = app(state.clone()).oneshot(Request::get(path).body(Body::empty()).unwrap()).await.unwrap();
    let value = response.headers().get(header).and_then(|value| value.to_str().ok()).unwrap_or_default().to_string();
    let status = response.status();
    (status, value, axum::body::to_bytes(response.into_body(), 1 << 20).await.unwrap().to_vec())
}

#[tokio::test]
async fn the_download_page_shows_the_published_build() {
    let state = state_in(std::env::temp_dir().join(format!("owl-test-page-{}", std::process::id())));
    let _ = std::fs::remove_dir_all(&state.releases);

    // Nothing published: the page is there, without a button.
    let (status, kind, body) = browse(&state, "/", "content-type").await;
    let html = String::from_utf8(body).unwrap();
    assert_eq!((status, kind.as_str()), (StatusCode::OK, "text/html; charset=utf-8"));
    assert!(html.contains("Coming soon"), "{html}");
    assert!(!html.contains("/download/owl.apk"), "no button without a build");
    assert!(!html.contains("{{"), "every placeholder was filled in");

    std::fs::create_dir_all(&state.releases).unwrap();
    std::fs::write(state.releases.join("version.json"), r#"{"build": 7, "notes": "New <heroes> & more."}"#).unwrap();
    // A build with no APK beside it cannot be handed out either.
    assert!(String::from_utf8(browse(&state, "/", "content-type").await.2).unwrap().contains("Coming soon"));

    std::fs::write(state.releases.join("owl.apk"), vec![0u8; 3 * 1024 * 1024]).unwrap();
    let (status, cache, body) = browse(&state, "/", "cache-control").await;
    let html = String::from_utf8(body).unwrap();
    assert_eq!((status, cache.as_str()), (StatusCode::OK, "no-cache"), "the page states the build, so it is not kept");
    assert!(html.contains(r#"href="/download/owl.apk?build=7""#), "{html}");
    assert!(html.contains("Build 7") && html.contains("3.0 MB"), "the build number and the size of the APK");
    assert!(html.contains("New &lt;heroes&gt; &amp; more."), "the notes, as text");
    assert!(!html.contains("Coming soon") && !html.contains("{{"));

    // Every picture and font the page names is served, and may be kept.
    let mut named = 0;
    for address in html.split('"').filter(|part| part.starts_with("/site/")) {
        let (status, cache, bytes) = browse(&state, address, "cache-control").await;
        assert_eq!(status, StatusCode::OK, "{address}");
        assert!(cache.contains("max-age") && !bytes.is_empty(), "{address}");
        named += 1;
    }
    assert!(named >= 6, "the icon, the font and the screenshots");
    assert_eq!(browse(&state, "/site/nothing.png", "cache-control").await.0, StatusCode::NOT_FOUND);
    let _ = std::fs::remove_dir_all(&state.releases);
}

#[tokio::test]
async fn logging_out_closes_the_players_connection() {
    let state = state();
    let address = serve(&state).await;
    let token = register(&state, "Minerva").await;
    let mut minerva = Player::connect(&address, &token).await;
    assert!(matches!(minerva.next().await, Some(ServerMessage::Welcome { .. })));
    assert_eq!(call(&state, "POST", "/api/logout", Some(&token), Value::Null).await.0, StatusCode::NO_CONTENT);
    assert_eq!(minerva.next().await, None);
}

#[test]
fn a_notice_states_its_version_on_a_line_of_its_own() {
    assert_eq!(owl_server::notice_version("# Notice\n\nVersion 3.\n\nText."), 3);
    assert_eq!(owl_server::notice_version("# Notice\n\nNo number here."), 1);
}

#[tokio::test]
async fn a_changed_notice_is_confirmed_before_playing_online_again() {
    let keys = || Keys::new(&[5; KEY_BYTES], Params::new(64, 1, 1, None).unwrap());
    let content = || Content::load(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data")).expect("data loads");
    let folder = std::env::temp_dir().join(format!("owl-test-notice-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let db = folder.join("owl.db");
    let _ = std::fs::remove_file(&db);

    // The account is made while the server shows version 1.
    let first = AppState::new(Store::open(&db, keys()).unwrap(), content(), QuestionBank::default(), "Version 1.".into(), folder.clone());
    let token = register(&first, "Minerva").await;
    drop(first);

    // The server is started again with a changed notice.
    let state = AppState::new(Store::open(&db, keys()).unwrap(), content(), QuestionBank::default(), "Version 2.".into(), folder.clone());
    let (status, notice) = call(&state, "GET", "/api/notice", None, Value::Null).await;
    assert_eq!((status, notice["version"].as_u64()), (StatusCode::OK, Some(2)));
    let (_, me) = call(&state, "GET", "/api/me", Some(&token), Value::Null).await;
    assert_eq!(me["notice_accepted"], json!(false));

    let address = serve(&state).await;
    let mut early = Player::connect(&address, &token).await;
    assert!(matches!(early.next().await, Some(ServerMessage::Error { .. })), "no playing online before confirming");

    let body = json!({ "username": "Archie", "password": PASSWORD, "notice_acknowledged": true, "notice_version": 1 });
    let (status, _) = call(&state, "POST", "/api/register", None, body).await;
    assert_eq!(status, StatusCode::CONFLICT, "an account cannot be made on an old notice");

    let (status, _) = call(&state, "POST", "/api/me/notice", Some(&token), json!({ "version": 1 })).await;
    assert_eq!(status, StatusCode::CONFLICT, "confirming an old version does not count");
    let (status, _) = call(&state, "POST", "/api/me/notice", Some(&token), json!({ "version": 2 })).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, me) = call(&state, "GET", "/api/me", Some(&token), Value::Null).await;
    assert_eq!(me["notice_accepted"], json!(true));
    let mut minerva = Player::connect(&address, &token).await;
    assert!(matches!(minerva.next().await, Some(ServerMessage::Welcome { .. })));
    let _ = std::fs::remove_dir_all(&folder);
}

#[tokio::test]
async fn wrong_passwords_make_a_name_wait_and_the_right_one_works_once_the_wait_is_over() {
    let clock = Clock::manual();
    let state = state_with(Options { clock: clock.clone(), ..Options::default() });
    register(&state, "Minerva").await;
    let login = |name: &str, password: &str| json!({ "username": name, "password": password });
    let from = peer("192.0.2.1");
    for _ in 0..limits::FREE_FAILURES {
        let (status, _, _) = call_from(&state, from, "POST", "/api/login", None, login("Minerva", "not her password")).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    let (status, headers, reply) = call_from(&state, from, "POST", "/api/login", None, login("minerva", PASSWORD)).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "any spelling of the name waits, even with the right password");
    let wait = retry_after(&headers);
    assert_eq!(wait, limits::FIRST_WAIT.as_secs());
    assert!(reply["error"].as_str().unwrap().contains("try again in 1 second"), "{reply}");

    // A name nobody has gets the same answers, so the two cannot be told apart.
    for _ in 0..limits::FREE_FAILURES {
        let (status, _, _) = call_from(&state, from, "POST", "/api/login", None, login("Nobody", "not her password")).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    let (status, headers, _) = call_from(&state, from, "POST", "/api/login", None, login("Nobody", PASSWORD)).await;
    assert_eq!((status, retry_after(&headers)), (StatusCode::TOO_MANY_REQUESTS, wait));

    // Once the wait is over the right password works.
    clock.advance(Duration::from_secs(wait));
    let (status, _, reply) = call_from(&state, from, "POST", "/api/login", None, login("Minerva", PASSWORD)).await;
    assert_eq!((status, reply["username"].as_str()), (StatusCode::OK, Some("Minerva")));
}

#[tokio::test]
async fn one_address_has_a_limited_number_of_password_tries() {
    let clock = Clock::manual();
    let state = state_with(Options { clock: clock.clone(), ..Options::default() });
    for n in 0..limits::PASSWORD_TRIES_PER_ADDRESS {
        let body = json!({ "username": format!("Name{n}"), "password": PASSWORD });
        assert_eq!(call_from(&state, peer("192.0.2.1"), "POST", "/api/login", None, body).await.0, StatusCode::UNAUTHORIZED);
    }
    let body = json!({ "username": "Another", "password": PASSWORD });
    let (status, headers, _) = call_from(&state, peer("192.0.2.1"), "POST", "/api/login", None, body.clone()).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(retry_after(&headers), limits::PASSWORD_TRIES_WINDOW.as_secs());
    let (status, _, _) = call_from(&state, peer("192.0.2.2"), "POST", "/api/login", None, body.clone()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "another address has its own tries");
    clock.advance(limits::PASSWORD_TRIES_WINDOW);
    let (status, _, _) = call_from(&state, peer("192.0.2.1"), "POST", "/api/login", None, body).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "a new window");
}

fn guest_body() -> Value {
    json!({ "notice_acknowledged": true })
}

fn account_body(name: &str) -> Value {
    json!({ "username": name, "password": PASSWORD, "notice_acknowledged": true })
}

#[tokio::test]
async fn new_players_are_limited_per_address_so_one_address_cannot_use_up_everyones_share() {
    let state = state_with(Options::default());
    let a = peer("192.0.2.1");
    assert_eq!(call_from(&state, a, "POST", "/api/register", None, account_body("Minerva")).await.0, StatusCode::CREATED);
    for _ in 1..limits::NEW_PLAYERS_PER_ADDRESS {
        assert_eq!(call_from(&state, a, "POST", "/api/guest", None, guest_body()).await.0, StatusCode::CREATED);
    }
    let (status, headers, reply) = call_from(&state, a, "POST", "/api/guest", None, guest_body()).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "accounts and guests share the allowance");
    assert!(retry_after(&headers) > 0);
    assert!(reply["error"].as_str().unwrap().starts_with("too many new players from your network"), "{reply}");
    let (status, _, _) = call_from(&state, a, "POST", "/api/register", None, account_body("Archie")).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);

    // Another address still has its own.
    let b = peer("192.0.2.2");
    assert_eq!(call_from(&state, b, "POST", "/api/register", None, account_body("Archie")).await.0, StatusCode::CREATED);
    assert_eq!(call_from(&state, b, "POST", "/api/guest", None, guest_body()).await.0, StatusCode::CREATED);
}

#[tokio::test]
async fn an_ipv6_network_counts_as_one_address() {
    let state = state_with(Options::default());
    const SAME_NETWORK: [&str; 6] =
        ["[2001:db8:1:2::1]", "[2001:db8:1:2::2]", "[2001:db8:1:2::3]", "[2001:db8:1:2::4]", "[2001:db8:1:2::5]", "[2001:db8:1:2::6]"];
    let allowed = limits::NEW_PLAYERS_PER_ADDRESS as usize;
    for address in &SAME_NETWORK[..allowed] {
        assert_eq!(call_from(&state, peer(address), "POST", "/api/guest", None, guest_body()).await.0, StatusCode::CREATED);
    }
    let (status, _, _) = call_from(&state, peer(SAME_NETWORK[allowed]), "POST", "/api/guest", None, guest_body()).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    let (status, _, _) = call_from(&state, peer("[2001:db8:1:3::1]"), "POST", "/api/guest", None, guest_body()).await;
    assert_eq!(status, StatusCode::CREATED, "another network");
}

#[tokio::test]
async fn the_cloudflare_header_is_trusted_only_when_the_setting_is_on() {
    // Every request comes through the tunnel, from the same local address.
    let through = |client: &'static str| Origin { peer: Some("172.18.0.2"), cloudflare: Some(client) };

    let trusted = state_with(Options { trust_cf_ip: true, ..Options::default() });
    for _ in 0..limits::NEW_PLAYERS_PER_ADDRESS {
        let (status, _, _) = call_from(&trusted, through("198.51.100.1"), "POST", "/api/guest", None, guest_body()).await;
        assert_eq!(status, StatusCode::CREATED);
    }
    let (status, _, _) = call_from(&trusted, through("198.51.100.1"), "POST", "/api/guest", None, guest_body()).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    let (status, _, _) = call_from(&trusted, through("198.51.100.2"), "POST", "/api/guest", None, guest_body()).await;
    assert_eq!(status, StatusCode::CREATED, "another player behind the same tunnel");
    let (status, _, _) = call_from(&trusted, through("not an address"), "POST", "/api/guest", None, guest_body()).await;
    assert_eq!(status, StatusCode::CREATED, "a header that is not an address counts as the tunnel itself");

    let untrusted = state_with(Options::default());
    for _ in 0..limits::NEW_PLAYERS_PER_ADDRESS {
        let (status, _, _) = call_from(&untrusted, through("198.51.100.1"), "POST", "/api/guest", None, guest_body()).await;
        assert_eq!(status, StatusCode::CREATED);
    }
    let (status, _, _) = call_from(&untrusted, through("198.51.100.2"), "POST", "/api/guest", None, guest_body()).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "without the setting the header is ignored and the connection counts");
}

#[tokio::test]
async fn password_hashes_wait_their_turn_and_a_full_queue_is_refused_at_once() {
    let gate = Arc::new(HashGate::new(2, 1));
    let first = gate.enter().await.unwrap();
    let _second = gate.enter().await.unwrap();
    let waiting = {
        let gate = gate.clone();
        tokio::spawn(async move { gate.enter().await.map(drop) })
    };
    while gate.waiting() == 0 {
        tokio::task::yield_now().await;
    }
    assert!(!waiting.is_finished(), "the third waits for a turn");
    assert_eq!(gate.enter().await.err(), Some(Busy), "the queue is full, so the fourth is refused at once");
    drop(first);
    let third = tokio::time::timeout(Duration::from_secs(5), waiting).await.expect("its turn comes").unwrap();
    assert_eq!(third, Ok(()));
    assert_eq!(gate.waiting(), 0);
}

#[tokio::test]
async fn a_busy_server_answers_503_with_a_plain_message() {
    let state = state_with(Options::default());
    let mut turns = Vec::new();
    for _ in 0..limits::HASHES_AT_ONCE {
        turns.push(state.limits.hashes.enter().await.unwrap());
    }
    let mut queued = Vec::new();
    for n in 0..limits::HASH_QUEUE {
        let state = state.clone();
        let body = json!({ "username": format!("Name{n}"), "password": PASSWORD });
        queued.push(tokio::spawn(async move { call_from(&state, peer("192.0.2.1"), "POST", "/api/login", None, body).await.0 }));
    }
    while state.limits.hashes.waiting() < limits::HASH_QUEUE {
        tokio::task::yield_now().await;
    }
    let body = json!({ "username": "Minerva", "password": PASSWORD });
    let (status, headers, reply) = call_from(&state, peer("192.0.2.2"), "POST", "/api/login", None, body).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(retry_after(&headers), limits::BUSY_RETRY.as_secs());
    assert_eq!(reply["error"], "the server is busy; try again in a moment");
    drop(turns);
    for request in queued {
        assert_eq!(request.await.unwrap(), StatusCode::UNAUTHORIZED, "the queued ones are answered in turn");
    }
}

#[tokio::test]
async fn one_address_may_keep_only_a_few_game_connections_open() {
    let state = state();
    let address = serve(&state).await;
    let url = format!("ws://{address}/ws");
    let mut open = Vec::new();
    for _ in 0..limits::SOCKETS_PER_ADDRESS {
        open.push(tokio_tungstenite::connect_async(url.as_str()).await.expect("the server accepts"));
    }
    assert!(tokio_tungstenite::connect_async(url.as_str()).await.is_err(), "one too many");
    drop(open.pop());
    // The place is free once the server has seen the connection close.
    let reopened = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(socket) = tokio_tungstenite::connect_async(url.as_str()).await {
                return socket;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    assert!(reopened.is_ok(), "a closed connection frees its place");
}
