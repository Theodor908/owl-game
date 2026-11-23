//! The JSON endpoints: accounts, friends and the user's own data.
//!
//! Everything but registering, logging in and reading the privacy notice
//! needs `Authorization: Bearer <session token>`. Errors come back as
//! `{"error": "..."}` with a fitting status. A refusal to wait (429, or 503
//! when the server is busy hashing) also says how many seconds to wait in a
//! `Retry-After` header.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::extract::{ConnectInfo, FromRequestParts, Path, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::AppState;
use crate::limits::{BUSY_RETRY, Busy, Refused, retry_seconds, wait_text};
use crate::store::{StoreError, User};

type App = State<Arc<AppState>>;

pub struct ApiError {
    status: StatusCode,
    message: String,
    /// Seconds to wait before trying again, for `Retry-After`.
    retry_after: Option<u64>,
}

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self { status, message: message.into(), retry_after: None }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut response = (self.status, Json(json!({ "error": self.message }))).into_response();
        if let Some(seconds) = self.retry_after {
            response.headers_mut().insert(header::RETRY_AFTER, HeaderValue::from(seconds));
        }
        response
    }
}

impl From<StoreError> for ApiError {
    fn from(error: StoreError) -> Self {
        let status = match error {
            StoreError::BadUsername | StoreError::BadPassword | StoreError::NoticeNotAcknowledged | StoreError::Yourself => {
                StatusCode::BAD_REQUEST
            }
            StoreError::UsernameTaken => StatusCode::CONFLICT,
            StoreError::GuestsCannot => StatusCode::FORBIDDEN,
            StoreError::WrongCredentials | StoreError::NotLoggedIn => StatusCode::UNAUTHORIZED,
            StoreError::TooManyAttempts(_) => StatusCode::TOO_MANY_REQUESTS,
            StoreError::NoSuchUser | StoreError::NoRequest => StatusCode::NOT_FOUND,
            StoreError::Database => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let retry_after = match error {
            StoreError::TooManyAttempts(seconds) => Some(seconds),
            _ => None,
        };
        ApiError { status, message: error.to_string(), retry_after }
    }
}

impl From<Refused> for ApiError {
    fn from(refused: Refused) -> Self {
        let (wait, what) = match refused {
            Refused::PasswordTries(wait) => (wait, "too many password tries from your network"),
            Refused::NewPlayersHere(wait) => (wait, "too many new players from your network"),
            Refused::NewPlayersEverywhere(wait) => (wait, "too many new players just now"),
        };
        let seconds = retry_seconds(wait);
        ApiError {
            status: StatusCode::TOO_MANY_REQUESTS,
            message: format!("{what}; try again in {}", wait_text(seconds)),
            retry_after: Some(seconds),
        }
    }
}

impl From<Busy> for ApiError {
    fn from(_: Busy) -> Self {
        ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            message: "the server is busy; try again in a moment".into(),
            retry_after: Some(retry_seconds(BUSY_RETRY)),
        }
    }
}

type Api<T> = Result<T, ApiError>;

/// The address a request came from, as the limits count it (see
/// `limits::client_address`).
pub struct ClientAddress(pub IpAddr);

impl FromRequestParts<Arc<AppState>> for ClientAddress {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &Arc<AppState>) -> Result<Self, Self::Rejection> {
        let peer = parts.extensions.get::<ConnectInfo<SocketAddr>>().map(|info| info.0);
        Ok(Self(state.limits.client_address(&parts.headers, peer)))
    }
}

fn unfinished() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "the server could not finish that")
}

/// Runs a store call off the async threads.
async fn blocking<T: Send + 'static>(
    state: &Arc<AppState>,
    work: impl FnOnce(&crate::store::Store) -> Result<T, StoreError> + Send + 'static,
) -> Api<T> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || work(&store)).await.map_err(|_| unfinished())?.map_err(ApiError::from)
}

/// Runs a store call that hashes a password, when the hash gate gives it a
/// turn. The turn is held until the work is done, even if the client has
/// gone, so no more hashes run at once than the gate allows.
async fn hashing<T: Send + 'static>(
    state: &Arc<AppState>,
    work: impl FnOnce(&crate::store::Store) -> Result<T, StoreError> + Send + 'static,
) -> Api<T> {
    let turn = state.limits.hashes.enter().await?;
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || {
        let done = work(&store);
        drop(turn);
        done
    })
    .await
    .map_err(|_| unfinished())?
    .map_err(ApiError::from)
}

fn bearer(headers: &HeaderMap) -> Api<String> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::to_string)
        .ok_or_else(|| ApiError::from(StoreError::NotLoggedIn))
}

async fn user(state: &Arc<AppState>, headers: &HeaderMap) -> Api<(User, String)> {
    let token = bearer(headers)?;
    let check = token.clone();
    let user = blocking(state, move |store| store.user_of(&check)).await?;
    Ok((user, token))
}

#[derive(Deserialize)]
struct Register {
    username: String,
    password: String,
    /// The player confirmed they read the privacy notice.
    #[serde(default)]
    notice_acknowledged: bool,
    /// Which version they read. An older one than the server's is refused.
    notice_version: Option<u32>,
}

#[derive(Deserialize)]
struct Guest {
    #[serde(default)]
    notice_acknowledged: bool,
    notice_version: Option<u32>,
}

#[derive(Deserialize)]
struct NoticeVersion {
    version: u32,
}

/// The player read a notice other than the one shown now.
fn check_notice(state: &AppState, read: Option<u32>) -> Api<()> {
    match read {
        Some(version) if version != state.notice_version() => {
            Err(ApiError::new(StatusCode::CONFLICT, "the privacy notice has changed; read the new one first"))
        }
        _ => Ok(()),
    }
}

/// What a client needs to know about the notice for a logged-in user.
async fn notice_fields(state: &Arc<AppState>, user: User) -> Api<(u32, bool)> {
    let current = state.notice_version();
    let accepted = blocking(state, move |store| store.accepted_notice(&user)).await?;
    Ok((current, accepted == current))
}

#[derive(Deserialize)]
struct Credentials {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct Password {
    password: String,
}

#[derive(Deserialize)]
struct NewPassword {
    old: String,
    new: String,
}

#[derive(Deserialize)]
struct NewUsername {
    password: String,
    username: String,
}

#[derive(Deserialize)]
struct Name {
    username: String,
}

async fn privacy(State(state): App) -> String {
    state.privacy_notice.clone()
}

/// The privacy notice with its version, for the game to show and to confirm.
async fn notice(State(state): App) -> Json<Value> {
    Json(json!({ "version": state.notice_version(), "text": state.privacy_notice }))
}

/// The user confirms the notice shown now. `version` is the one they read.
async fn accept_notice(State(state): App, headers: HeaderMap, Json(body): Json<NoticeVersion>) -> Api<StatusCode> {
    check_notice(&state, Some(body.version))?;
    let (user, _) = user(&state, &headers).await?;
    blocking(&state, move |store| store.accept_notice(&user)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn register(
    State(state): App,
    ClientAddress(address): ClientAddress,
    Json(body): Json<Register>,
) -> Api<(StatusCode, Json<Value>)> {
    check_notice(&state, body.notice_version)?;
    state.limits.new_player(address)?;
    let name = body.username.clone();
    let token = hashing(&state, move |store| store.register(&body.username, &body.password, body.notice_acknowledged)).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "token": token, "username": name, "guest": false,
            "notice_version": state.notice_version(), "notice_accepted": true,
        })),
    ))
}

/// Play without an account: a made-up name that lasts as long as the session.
async fn guest(
    State(state): App,
    ClientAddress(address): ClientAddress,
    Json(body): Json<Guest>,
) -> Api<(StatusCode, Json<Value>)> {
    check_notice(&state, body.notice_version)?;
    state.limits.new_player(address)?;
    let (token, name) = blocking(&state, move |store| store.register_guest(body.notice_acknowledged)).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "token": token, "username": name, "guest": true,
            "notice_version": state.notice_version(), "notice_accepted": true,
        })),
    ))
}

/// The newest build of the game there is, for a client to compare with its
/// own: `{"build": 7, "notes": "...", ...}`, or build 0 when nothing was
/// published. The file is passed on as `release.bat` wrote it, with the
/// signed descriptions of the pack and the library (`docs/protocol.md`).
async fn version(State(state): App) -> Json<Value> {
    Json(published_version(&state).await.unwrap_or_else(|| json!({ "build": 0, "notes": "" })))
}

/// `version.json` from the releases folder, if a build was published.
pub(crate) async fn published_version(state: &AppState) -> Option<Value> {
    let published = tokio::fs::read_to_string(state.releases.join("version.json")).await.ok()?;
    serde_json::from_str(&published).ok()
}

async fn login(State(state): App, ClientAddress(address): ClientAddress, Json(body): Json<Credentials>) -> Api<Json<Value>> {
    state.limits.password_try(address)?;
    let (token, username) = hashing(&state, move |store| store.login(&body.username, &body.password)).await?;
    let check = token.clone();
    let user = blocking(&state, move |store| store.user_of(&check)).await?;
    let (version, accepted) = notice_fields(&state, user).await?;
    Ok(Json(json!({
        "token": token, "username": username, "guest": false,
        "notice_version": version, "notice_accepted": accepted,
    })))
}

async fn logout(State(state): App, headers: HeaderMap) -> Api<StatusCode> {
    let (user, token) = user(&state, &headers).await?;
    blocking(&state, move |store| store.logout(&token)).await?;
    state.hub.lock().await.remove(&user.username);
    Ok(StatusCode::NO_CONTENT)
}

async fn me(State(state): App, headers: HeaderMap) -> Api<Json<Value>> {
    let (user, _) = user(&state, &headers).await?;
    let (name, guest) = (user.username.clone(), user.guest);
    let (version, accepted) = notice_fields(&state, user).await?;
    Ok(Json(json!({ "username": name, "guest": guest, "notice_version": version, "notice_accepted": accepted })))
}

async fn export(State(state): App, headers: HeaderMap) -> Api<Json<Value>> {
    let (user, _) = user(&state, &headers).await?;
    let data = blocking(&state, move |store| store.export(&user)).await?;
    Ok(Json(serde_json::to_value(data).unwrap_or(Value::Null)))
}

async fn delete_me(
    State(state): App,
    ClientAddress(address): ClientAddress,
    headers: HeaderMap,
    body: Option<Json<Password>>,
) -> Api<StatusCode> {
    let body = body.map_or(Password { password: String::new() }, |Json(body)| body);
    let (user, _) = user(&state, &headers).await?;
    let name = user.username.clone();
    if user.guest {
        // A guest has no password to check.
        blocking(&state, move |store| store.delete_account(&user, &body.password)).await?;
    } else {
        state.limits.password_try(address)?;
        hashing(&state, move |store| store.delete_account(&user, &body.password)).await?;
    }
    state.hub.lock().await.remove(&name);
    Ok(StatusCode::NO_CONTENT)
}

async fn change_password(
    State(state): App,
    ClientAddress(address): ClientAddress,
    headers: HeaderMap,
    Json(body): Json<NewPassword>,
) -> Api<StatusCode> {
    let (user, _) = user(&state, &headers).await?;
    let name = user.username.clone();
    state.limits.password_try(address)?;
    hashing(&state, move |store| store.change_password(&user, &body.old, &body.new)).await?;
    // Every session ended, so every connection goes too.
    state.hub.lock().await.remove(&name);
    Ok(StatusCode::NO_CONTENT)
}

async fn change_username(
    State(state): App,
    ClientAddress(address): ClientAddress,
    headers: HeaderMap,
    Json(body): Json<NewUsername>,
) -> Api<Json<Value>> {
    let (user, _) = user(&state, &headers).await?;
    let old = user.username.clone();
    let new = body.username.clone();
    state.limits.password_try(address)?;
    hashing(&state, move |store| store.change_username(&user, &body.password, &body.username)).await?;
    // The connection knows the old name; the client reconnects under the new one.
    state.hub.lock().await.remove(&old);
    Ok(Json(json!({ "username": new })))
}

/// Looks up one exact username. There is no listing and no partial search.
async fn find(State(state): App, headers: HeaderMap, Path(name): Path<String>) -> Api<Json<Value>> {
    user(&state, &headers).await?;
    let username = blocking(&state, move |store| store.find(&name)).await?;
    Ok(Json(json!({ "username": username })))
}

async fn friends(State(state): App, headers: HeaderMap) -> Api<Json<Value>> {
    let (user, _) = user(&state, &headers).await?;
    let list = blocking(&state, move |store| store.friends(&user)).await?;
    let hub = state.hub.lock().await;
    let friends: Vec<Value> =
        list.friends.iter().map(|name| json!({ "username": name, "online": hub.is_online(name) })).collect();
    Ok(Json(json!({ "friends": friends, "incoming": list.incoming, "outgoing": list.outgoing })))
}

async fn request_friend(State(state): App, headers: HeaderMap, Json(body): Json<Name>) -> Api<StatusCode> {
    let (user, _) = user(&state, &headers).await?;
    blocking(&state, move |store| store.request_friend(&user, &body.username)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn accept_friend(State(state): App, headers: HeaderMap, Json(body): Json<Name>) -> Api<StatusCode> {
    let (user, _) = user(&state, &headers).await?;
    blocking(&state, move |store| store.accept_friend(&user, &body.username)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove_friend(State(state): App, headers: HeaderMap, Json(body): Json<Name>) -> Api<StatusCode> {
    let (user, _) = user(&state, &headers).await?;
    blocking(&state, move |store| store.remove_friend(&user, &body.username)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/privacy", get(privacy))
        .route("/api/notice", get(notice))
        .route("/api/register", post(register))
        .route("/api/guest", post(guest))
        .route("/api/version", get(version))
        .route("/api/login", post(login))
        .route("/api/logout", post(logout))
        .route("/api/me", get(me).delete(delete_me))
        .route("/api/me/export", get(export))
        .route("/api/me/notice", post(accept_notice))
        .route("/api/me/password", post(change_password))
        .route("/api/me/username", post(change_username))
        .route("/api/users/{username}", get(find))
        .route("/api/friends", get(friends))
        .route("/api/friends/request", post(request_friend))
        .route("/api/friends/accept", post(accept_friend))
        .route("/api/friends/remove", post(remove_friend))
}
