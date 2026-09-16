//! Local web UI server for the agent.
//!
//! Binds to localhost only. Serves the agent dashboard and a local API.
//! The browser talks to this local server; it never talks to the public
//! Rehook server directly. See SECURITY.md.
//!
//! Security: the web UI binds to 127.0.0.1 by default. It does not expose
//! agent tokens in responses. Target URLs are validated using the same
//! security policy as CLI delivery.

mod api;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::Router;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use crate::config::AgentConfig;
use crate::connection;
use crate::db::LocalDb;
use crate::state::ConnectionState;

/// The compiled agent dashboard. The folder is relative to the crate
/// manifest directory (`apps/agent`). In debug builds the files are read
/// from disk so `cargo run` picks up rebuilds without recompiling.
#[derive(rust_embed::RustEmbed)]
#[folder = "../../dashboards/agent/dist"]
struct AgentDashboard;

/// Shared state for the web server.
#[derive(Clone)]
pub struct WebState {
    pub config: AgentConfig,
    pub db: Arc<LocalDb>,
    pub dashboard_dir: Option<PathBuf>,
    pub conn_state: ConnectionState,
}

/// Start the local web UI server. Binds to 127.0.0.1 by default.
/// Also starts the connection loop in the background so the agent
/// connects to the server automatically after login.
/// If the requested port is taken, tries the next available port up to
/// 100 attempts, then prints the actual URL.
pub async fn run(port: u16, dashboard_dir: Option<PathBuf>) -> Result<()> {
    let config = AgentConfig::load()?;
    let db = Arc::new(LocalDb::open().await?);
    let conn_state = ConnectionState::new();

    // Start the connection loop in the background.
    let db_clone = Arc::clone(&db);
    let conn_state_clone = conn_state.clone();
    tokio::spawn(async move {
        if let Err(e) = connection::run(db_clone, conn_state_clone).await {
            tracing::error!(error = %e, "connection loop exited");
        }
    });

    let state = WebState {
        config,
        db,
        dashboard_dir,
        conn_state,
    };

    let app = build_router(state);

    // Try the requested port. If it is taken, increment until we find one.
    let mut actual_port = port;
    let listener = loop {
        let addr: SocketAddr = ([127, 0, 0, 1], actual_port).into();
        match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => break l,
            Err(_) if actual_port < port + 100 => {
                actual_port += 1;
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "could not bind any port from {port} to {actual_port}: {e}"
                ));
            }
        }
    };

    if actual_port != port {
        println!("port {port} was taken, using {actual_port} instead");
    }
    tracing::info!(port = actual_port, "agent web UI listening");
    println!("agent web UI: http://localhost:{actual_port}");
    axum::serve(listener, app)
        .await
        .context("web server exited")?;
    Ok(())
}

fn build_router(state: WebState) -> Router {
    let api_router = api::router();

    // Dashboard assets are embedded in the binary by default. The
    // --dashboard-dir flag overrides with a directory on disk. The
    // index.html fallback enables client-side routing.
    let router = Router::new()
        .nest("/api", api_router)
        .with_state(state.clone())
        .layer(TraceLayer::new_for_http());

    match &state.dashboard_dir {
        Some(dir) => router
            .fallback_service(ServeDir::new(dir).fallback(ServeFile::new(dir.join("index.html")))),
        None => router.fallback(serve_dashboard),
    }
}

async fn serve_dashboard(uri: axum::http::Uri) -> axum::response::Response {
    use axum::http::{header, HeaderValue, StatusCode};
    use axum::response::IntoResponse;

    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    let (served_path, file) = match AgentDashboard::get(path) {
        Some(f) => (path, Some(f)),
        None => ("index.html", AgentDashboard::get("index.html")),
    };
    match file {
        Some(file) => {
            let mut resp = axum::response::Response::new(file.data.to_vec().into());
            let mime = mime_guess::from_path(served_path).first_or_octet_stream();
            if let Ok(v) = HeaderValue::from_str(mime.as_ref()) {
                resp.headers_mut().insert(header::CONTENT_TYPE, v);
            }
            if served_path.starts_with("assets/") {
                resp.headers_mut().insert(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("public, max-age=31536000, immutable"),
                );
            }
            resp
        }
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            "agent dashboard assets are not built; run `npm ci && npm run build` in dashboards/agent",
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::serve_dashboard;
    use axum::http::{header, StatusCode, Uri};

    #[tokio::test]
    async fn root_serves_index_or_hint() {
        let uri: Uri = "/".parse().unwrap();
        let resp = serve_dashboard(uri).await;
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
        let uri: Uri = "/targets".parse().unwrap();
        let resp = serve_dashboard(uri).await;
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
