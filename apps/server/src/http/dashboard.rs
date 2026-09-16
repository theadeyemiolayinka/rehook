//! Embedded admin dashboard.
//!
//! The built dashboard assets are compiled into the binary with rust-embed
//! so `rehook-server` is a single self-contained binary. In debug builds
//! (`debug-embed` feature) assets are read from the filesystem at
//! `dashboards/admin/dist`, so `cargo run` picks up dashboard rebuilds
//! without recompiling the server.
//!
//! If the assets were not built, a plain-text hint is served instead of a
//! blank page.

use axum::body::Body;
use axum::http::{header, HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

/// The compiled admin dashboard. The folder is relative to the crate
/// manifest directory (`apps/server`).
#[derive(RustEmbed)]
#[folder = "../../dashboards/admin/dist"]
struct AdminDashboard;

/// Serve the admin dashboard. Unmatched paths fall back to index.html so
/// client-side routing (React Router) works.
pub async fn serve(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    if let Some(file) = AdminDashboard::get(path) {
        return asset_response(path, &file);
    }

    // SPA fallback: serve index.html for unknown non-asset paths.
    if let Some(file) = AdminDashboard::get("index.html") {
        return asset_response("index.html", &file);
    }

    (
        StatusCode::SERVICE_UNAVAILABLE,
        "admin dashboard assets are not built; run `npm ci && npm run build` in dashboards/admin",
    )
        .into_response()
}

fn asset_response(path: &str, file: &rust_embed::EmbeddedFile) -> Response {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let mut resp = Response::new(Body::from(file.data.to_vec()));
    if let Ok(v) = HeaderValue::from_str(mime.as_ref()) {
        resp.headers_mut().insert(header::CONTENT_TYPE, v);
    }
    // Vite emits content-hashed filenames under assets/, safe to cache
    // long. index.html is always revalidated.
    if path.starts_with("assets/") {
        resp.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[tokio::test]
    async fn root_serves_index_or_hint() {
        let uri: Uri = "/".parse().unwrap();
        let resp = serve(uri).await;
        // With a built dashboard: 200 + text/html. Without assets: a clear
        // 503 hint. Either is correct; a panic or 404 is not.
        match resp.status() {
            StatusCode::OK => assert_eq!(
                resp.headers().get(header::CONTENT_TYPE).unwrap(),
                "text/html"
            ),
            StatusCode::SERVICE_UNAVAILABLE => {}
            other => panic!("unexpected status {other}"),
        }
    }

    #[tokio::test]
    async fn spa_path_falls_back_to_index() {
        let uri: Uri = "/events/abc-123".parse().unwrap();
        let resp = serve(uri).await;
        match resp.status() {
            StatusCode::OK => assert_eq!(
                resp.headers().get(header::CONTENT_TYPE).unwrap(),
                "text/html"
            ),
            StatusCode::SERVICE_UNAVAILABLE => {}
            other => panic!("unexpected status {other}"),
        }
    }
}
