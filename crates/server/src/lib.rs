//! The online host: accounts, friends, lobbies and matches.
//!
//! - `crypto`: password hashing, username encryption, session tokens.
//! - `store`: accounts, sessions and friends in SQLite.
//! - `limits`: how much one client may ask of the server.
//! - `http`: the JSON endpoints for accounts and friends.
//! - `page`: the download page a browser gets at `/`.
//! - `hub`: who is online, lobbies, and one task per running match.
//! - `ws`: the WebSocket a logged-in player plays over.
//!
//! The server listens on plain HTTP and is meant to sit behind something
//! that provides HTTPS (see `docs/server.md`): passwords and session tokens
//! must never cross a network unencrypted.

pub mod crypto;
pub mod http;
pub mod hub;
pub mod limits;
pub mod page;
pub mod store;
pub mod ws;

use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use owl_core::Content;
use serde_json::Value;
use tokio::sync::Mutex;

use hub::Hub;
use limits::{Clock, Limits};
use owl_lobby::bank::QuestionBank;
use store::Store;

/// How the server is run, beside its content and its store.
#[derive(Clone, Default)]
pub struct Options {
    /// The server sits behind a Cloudflare tunnel: the client's address is
    /// taken from `CF-Connecting-IP` (`OWL_TRUST_CF_IP=1`).
    pub trust_cf_ip: bool,
    /// The time the limits are measured in.
    pub clock: Clock,
}

pub struct AppState {
    pub store: Arc<Store>,
    pub content: Arc<Content>,
    /// The question bank, read from `data/questions`.
    pub questions: Arc<QuestionBank>,
    /// Cards, heroes and tunables as the client displays them.
    pub content_json: Value,
    pub hub: Arc<Mutex<Hub>>,
    /// The privacy notice shown before registering, and its version
    /// (`notice_version`): a player confirms each new version once.
    pub privacy_notice: String,
    /// Where the published build of the game is: `version.json` and the
    /// files in `DOWNLOADS`.
    pub releases: PathBuf,
    /// How much one client may ask of the server.
    pub limits: Limits,
}

impl AppState {
    pub fn new(
        store: Store,
        content: Content,
        questions: QuestionBank,
        privacy_notice: String,
        releases: PathBuf,
    ) -> Arc<Self> {
        Self::with_options(store, content, questions, privacy_notice, releases, Options::default())
    }

    pub fn with_options(
        store: Store,
        content: Content,
        questions: QuestionBank,
        privacy_notice: String,
        releases: PathBuf,
        options: Options,
    ) -> Arc<Self> {
        let content_json = serde_json::to_value(&content).unwrap_or(Value::Null);
        let store = store.with_notice_version(notice_version(&privacy_notice)).with_clock(options.clock.clone());
        Arc::new(Self {
            store: Arc::new(store),
            content: Arc::new(content),
            questions: Arc::new(questions),
            content_json,
            hub: Arc::new(Mutex::new(Hub::default())),
            privacy_notice,
            releases,
            limits: Limits::new(options.clock, options.trust_cf_ip),
        })
    }

    /// The version of the privacy notice shown now.
    pub fn notice_version(&self) -> u32 {
        self.store.notice_version()
    }
}

/// The version a privacy notice states on a line of its own, `Version 2.`
/// A notice that states none is version 1.
pub fn notice_version(notice: &str) -> u32 {
    notice
        .lines()
        .filter_map(|line| line.trim().strip_prefix("Version "))
        .find_map(|rest| rest.trim_end_matches('.').trim().parse().ok())
        .unwrap_or(1)
}

/// How often run-out sessions, guests and accounts unused for a year are erased.
pub const SWEEP_EVERY: std::time::Duration = std::time::Duration::from_secs(6 * 60 * 60);

/// Lets go of lobby members who stayed away too long, every
/// `hub::AWAY_SWEEP_EVERY`, for as long as the server runs.
pub async fn sweep_away_forever(hub: Arc<Mutex<Hub>>) {
    let mut every = tokio::time::interval(hub::AWAY_SWEEP_EVERY);
    loop {
        every.tick().await;
        hub.lock().await.sweep_away(std::time::Instant::now());
    }
}

/// The files of a published build, served from the releases folder under
/// `/download/`.
pub const DOWNLOADS: [&str; 3] = ["owl.apk", "owl.pck", "libowl_local.so"];

/// Every route of the server.
pub fn app(state: Arc<AppState>) -> Router {
    // The game itself, for a phone to download when a new build is out: the
    // whole APK, and the pack and the native library a game fetches by
    // itself. Never kept by Cloudflare or the phone: every build goes out
    // under the same names, and a kept copy would hand out the previous one.
    let download = |file: &str| {
        tower_http::set_header::SetResponseHeader::overriding(
            tower_http::services::ServeFile::new(state.releases.join(file)),
            axum::http::header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static("no-store"),
        )
    };
    let mut routes = http::routes().merge(ws::routes()).merge(page::routes());
    for file in DOWNLOADS {
        routes = routes.route_service(&format!("/download/{file}"), download(file));
    }
    routes.with_state(state)
}
