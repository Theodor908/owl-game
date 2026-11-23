//! The download page at `/`: what a browser shows a person who wants the game.
//!
//! The page is put together on the server from the published `version.json`
//! and the size of `owl.apk`, so it always states what a tap on Download
//! gives, and works without JavaScript. Its pictures and its font are
//! compiled into the program and served under `/site/`.

use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;

use crate::AppState;

const TEMPLATE: &str = include_str!("../assets/page.html");

/// The page loads nothing from anywhere else and runs no scripts.
const CONTENT_SECURITY_POLICY: &str = "default-src 'none'; img-src 'self'; font-src 'self'; style-src 'unsafe-inline'; \
     base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

/// The pictures and the font change only with the server, so a browser and
/// Cloudflare may keep them for a day.
const ASSET_CACHE: &str = "public, max-age=86400";

/// The files under `/site/`: name, content type, bytes.
const ASSETS: [(&str, &str, &[u8]); 9] = [
    ("icon.webp", "image/webp", include_bytes!("../assets/icon.webp")),
    ("favicon.png", "image/png", include_bytes!("../assets/favicon.png")),
    ("heading.ttf", "font/ttf", include_bytes!("../assets/heading.ttf")),
    ("shot_home.webp", "image/webp", include_bytes!("../assets/shot_home.webp")),
    ("shot_heroes.webp", "image/webp", include_bytes!("../assets/shot_heroes.webp")),
    ("shot_shop.webp", "image/webp", include_bytes!("../assets/shot_shop.webp")),
    ("shot_hand.webp", "image/webp", include_bytes!("../assets/shot_hand.webp")),
    ("shot_question.webp", "image/webp", include_bytes!("../assets/shot_question.webp")),
    ("shot_reveal.webp", "image/webp", include_bytes!("../assets/shot_reveal.webp")),
];

/// What is published, as far as the page shows it.
struct Published {
    build: u64,
    notes: String,
    apk_bytes: u64,
}

/// The published build, or nothing while there is no build or no APK to hand out.
async fn published(state: &AppState) -> Option<Published> {
    let version = crate::http::published_version(state).await?;
    let build = version["build"].as_u64().filter(|build| *build > 0)?;
    let apk = tokio::fs::metadata(state.releases.join("owl.apk")).await.ok()?;
    let notes = version["notes"].as_str().unwrap_or_default().trim().to_string();
    Some(Published { build, notes, apk_bytes: apk.len() })
}

/// Text made safe to stand inside HTML.
fn escape(text: &str) -> String {
    let mut safe = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => safe.push_str("&amp;"),
            '<' => safe.push_str("&lt;"),
            '>' => safe.push_str("&gt;"),
            '"' => safe.push_str("&quot;"),
            '\'' => safe.push_str("&#39;"),
            other => safe.push(other),
        }
    }
    safe
}

/// A file size as a person reads it: `48 MB`, `7.3 MB`, `512 KB`.
fn size(bytes: u64) -> String {
    const MB: u64 = 1024 * 1024;
    if bytes >= 10 * MB {
        format!("{} MB", (bytes + MB / 2) / MB)
    } else if bytes >= MB {
        // Tenths, rounded.
        let tenths = (bytes * 10 + MB / 2) / MB;
        format!("{}.{} MB", tenths / 10, tenths % 10)
    } else {
        format!("{} KB", bytes.div_ceil(1024))
    }
}

fn fact(value: &str, label: &str) -> String {
    format!("        <li><strong>{value}</strong><span>{label}</span></li>\n")
}

fn render(published: Option<&Published>) -> String {
    let (facts, download, notes) = match published {
        Some(Published { build, notes, apk_bytes }) => {
            let size = size(*apk_bytes);
            let facts = fact(&build.to_string(), "Build") + &fact(&size, "Download") + &fact("2 to 6", "Players");
            let download = format!(
                r#"      <a class="button" href="/download/owl.apk?build={build}" download="owl-{build}.apk">Download for Android</a>
      <p class="under">Build {build} · {size} · APK</p>
      <div class="install">
        <h2>How to install</h2>
        <ol>
          <li>Tap Download, then open the file when it is done.</li>
          <li>If Android asks, allow your browser to install unknown apps.</li>
          <li>Tap Install.</li>
        </ol>
        <p>You do this once. After that, Owl updates itself when you open it.</p>
      </div>"#
            );
            let notes = if notes.is_empty() {
                String::new()
            } else {
                format!(
                    r#"    <section class="notes">
      <h2>What is new</h2>
      <p class="build">Build {build}</p>
      <p>{}</p>
    </section>"#,
                    escape(notes)
                )
            };
            (facts, download, notes)
        }
        None => {
            let facts = fact("Soon", "Build") + &fact("Android", "Phone") + &fact("2 to 6", "Players");
            let download = r#"      <p class="soon">Coming soon</p>
      <p class="under">No build is published yet. Come back in a while.</p>"#;
            (facts, download.to_string(), String::new())
        }
    };
    TEMPLATE.replace("{{facts}}", facts.trim_end()).replace("{{download}}", &download).replace("{{notes}}", &notes)
}

/// The page. Never kept by Cloudflare or the browser: it states the build.
async fn page(State(state): State<Arc<AppState>>) -> Response {
    let html = render(published(&state).await.as_ref());
    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
            (header::CONTENT_SECURITY_POLICY, CONTENT_SECURITY_POLICY),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (header::REFERRER_POLICY, "no-referrer"),
        ],
        html,
    )
        .into_response()
}

async fn asset(Path(name): Path<String>) -> Response {
    match ASSETS.iter().find(|(file, ..)| *file == name) {
        Some((_, content_type, bytes)) => (
            [
                (header::CONTENT_TYPE, *content_type),
                (header::CACHE_CONTROL, ASSET_CACHE),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            *bytes,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/", get(page)).route("/site/{file}", get(asset))
}

#[cfg(test)]
mod tests {
    use super::{escape, size};

    #[test]
    fn a_size_reads_as_a_person_would_say_it() {
        assert_eq!(size(48 * 1024 * 1024 + 300_000), "48 MB");
        assert_eq!(size(7_282_080), "6.9 MB");
        assert_eq!(size(17), "1 KB");
    }

    #[test]
    fn release_notes_cannot_put_markup_in_the_page() {
        assert_eq!(escape(r#"<b a="1">Tom & 'Jerry'</b>"#), "&lt;b a=&quot;1&quot;&gt;Tom &amp; &#39;Jerry&#39;&lt;/b&gt;");
    }
}
