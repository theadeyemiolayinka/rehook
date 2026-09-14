//! Local web UI server for the agent.
//!
//! Binds to localhost only. Serves the agent dashboard and a local API.
//! The browser talks to this local server; it never talks to the public
//! HookRelay server directly. See SECURITY.md.
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
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::config::AgentConfig;
use crate::db::LocalDb;

/// Shared state for the web server.
#[derive(Clone)]
pub struct WebState {
    pub config: AgentConfig,
    pub db: Arc<LocalDb>,
    pub dashboard_dir: PathBuf,
}

/// Start the local web UI server. Binds to 127.0.0.1 by default.
/// If the requested port is taken, tries the next available port up to
/// 100 attempts, then prints the actual URL.
pub async fn run(port: u16, dashboard_dir: Option<PathBuf>) -> Result<()> {
    let config = AgentConfig::load()?;
    let db = Arc::new(LocalDb::open().await?);

    let dashboard_dir = dashboard_dir.unwrap_or_else(|| {
        // Default: look for the built agent dashboard relative to the
        // working directory, then fall back to a system path.
        let local = PathBuf::from("./dashboards/agent/dist");
        if local.exists() {
            local
        } else {
            PathBuf::from("/usr/local/share/hookrelay/agent-dashboard")
        }
    });

    let state = WebState {
        config,
        db,
        dashboard_dir: dashboard_dir.clone(),
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

    Router::new()
        .nest("/api", api_router)
        .with_state(state.clone())
        .fallback_service(ServeDir::new(&state.dashboard_dir))
        .layer(TraceLayer::new_for_http())
}
