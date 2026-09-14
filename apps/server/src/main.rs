//! HookRelay server entrypoint.

mod api;
mod auth;
mod config;
mod database;
mod error;
mod http;
mod state;
mod webhook;
mod websocket;

use std::sync::Arc;

use anyhow::{Context, Result};
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Handle --version early.
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("hookrelay-server {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let config = Config::from_env().context("loading configuration")?;
    init_tracing(&config);
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "starting hookrelay server"
    );

    let state = AppState::new(&config).await.context("initializing state")?;
    database::migrate(&state.pool)
        .await
        .context("running migrations")?;
    database::bootstrap_admin(&state.pool, &config)
        .await
        .context("bootstrapping admin")?;

    let app = build_router(Arc::new(state));
    let listener = tokio::net::TcpListener::bind(&config.listen_addr)
        .await
        .with_context(|| format!("binding {}", config.listen_addr))?;
    tracing::info!(addr = %config.listen_addr, "listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("server exited")?;
    Ok(())
}

fn build_router(state: Arc<AppState>) -> Router {
    let router = http::routes::root(state).layer(TraceLayer::new_for_http());
    let mut router = router;
    for layer in http::headers::security_headers_layer() {
        router = router.layer(layer);
    }
    router
}

fn init_tracing(config: &Config) {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.rust_log));
    fmt().with_env_filter(filter).with_target(false).init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("installing ctrl-c handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("installing terminate handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
