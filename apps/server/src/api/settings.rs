//! Read-only server settings. These values are configured through
//! environment variables and are not editable at runtime. The dashboard shows
//! them so operators can verify the active configuration.

use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;

use crate::auth::middleware::AuthSession;
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", get(show))
}

async fn show(State(state): State<Arc<AppState>>, _session: AuthSession) -> impl IntoResponse {
    let max_body_kb = (state.config.max_webhook_body_size as f64 / 1024.0).round() as u64;
    Json(json!({
        "public_base_url": state.config.public_base_url,
        "max_webhook_body_kb": max_body_kb,
        "max_stored_events": state.config.max_stored_events,
        "event_retention_days": state.config.event_retention_days,
        "session_ttl_hours": state.config.session_ttl_hours,
        "trusted_proxy_hops": state.config.trusted_proxy_hops,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
