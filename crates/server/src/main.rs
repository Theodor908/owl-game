//! Starts the Owl server.
//!
//! Settings come from the environment, each with a default that suits a
//! Raspberry Pi behind an HTTPS front (see `docs/server.md`):
//!
//! - `OWL_BIND`: address to listen on. Default `127.0.0.1:8080`, local only.
//! - `OWL_DATA`: the game's content directory. Default `data`.
//! - `OWL_DB`: the account database. Default `owl.db`.
//! - `OWL_KEY`: the key file, created on first start. Default `owl.key`.
//!   Keep it out of the database's directory and out of its backups.
//! - `OWL_PRIVACY`: the privacy notice shown before registering.
//!   Default `docs/privacy.md`.
//! - `OWL_RELEASES`: the folder with the newest build of the game
//!   (`version.json`, `owl.apk`, `owl.pck`, `libowl_local.so`). Default `releases`. It may be missing.
//! - `OWL_TRUST_CF_IP`: `1` when the server sits behind a Cloudflare tunnel,
//!   so the client's address is taken from `CF-Connecting-IP` for the
//!   limits. Default off: the address of the connection is used.

use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use owl_core::Content;
use owl_lobby::bank::QuestionBank;
use owl_server::crypto::{self, Keys};
use owl_server::store::Store;
use owl_server::limits::TRUST_CF_IP_SETTING;
use owl_server::{AppState, Options, SWEEP_EVERY, app};

fn setting(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_string())
}

async fn run() -> Result<(), String> {
    let data = setting("OWL_DATA", "data");
    let content = Content::load(&data).map_err(|error| format!("cannot load the content in {data}: {error}"))?;
    let questions = QuestionBank::load(&data, &content.tunables.categories)
        .map_err(|error| format!("cannot load the questions in {data}: {error}"))?;
    let key_path = PathBuf::from(setting("OWL_KEY", "owl.key"));
    let keys = Keys::load_or_create(&key_path, crypto::password_cost())
        .map_err(|error| format!("cannot use the key file {}: {error}", key_path.display()))?;
    let db_path = PathBuf::from(setting("OWL_DB", "owl.db"));
    let store = Store::open(&db_path, keys).map_err(|error| format!("cannot open {}: {error}", db_path.display()))?;
    let notice_path = setting("OWL_PRIVACY", "docs/privacy.md");
    let notice = std::fs::read_to_string(&notice_path)
        .map_err(|error| format!("cannot read the privacy notice {notice_path}: {error}"))?;

    let bind = setting("OWL_BIND", "127.0.0.1:8080");
    let listener = tokio::net::TcpListener::bind(&bind).await.map_err(|error| format!("cannot listen on {bind}: {error}"))?;
    println!("Owl server listening on {bind}");
    let releases = PathBuf::from(setting("OWL_RELEASES", "releases"));
    let trust_cf_ip = setting(TRUST_CF_IP_SETTING, "0") == "1";
    let options = Options { trust_cf_ip, ..Options::default() };
    let state = AppState::with_options(store, content, questions, notice, releases, options);
    // Erases what has run out, now and every few hours: an account unused
    // for a year goes even if nobody logs in to set off the sweep.
    let store = state.store.clone();
    tokio::spawn(async move {
        let mut every = tokio::time::interval(SWEEP_EVERY);
        loop {
            every.tick().await;
            let store = store.clone();
            if tokio::task::spawn_blocking(move || store.sweep()).await.map_or(true, |done| done.is_err()) {
                eprintln!("the sweep of old accounts failed");
            }
        }
    });
    // A lobby place kept for a dropped player runs out.
    tokio::spawn(owl_server::sweep_away_forever(state.hub.clone()));
    // The address of each connection, for the limits (`ConnectInfo`).
    axum::serve(listener, app(state).into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .map_err(|error| format!("the server stopped: {error}"))
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
