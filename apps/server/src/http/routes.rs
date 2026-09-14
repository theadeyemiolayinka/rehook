//! HTTP route definitions.

use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::ServeDir;

use crate::api::agents;
use crate::api::endpoints;
use crate::api::events;
use crate::api::projects;
use crate::api::replay;
use crate::auth::routes;
use crate::state::AppState;
use crate::webhook::capture;
use crate::websocket::gateway;

pub fn root(state: Arc<AppState>) -> Router {
    let max_body = state.config.max_webhook_body_size;

    // Public inbound webhook endpoints. Body is bounded by config and scoped
    // to this router only, so dashboard APIs are not constrained by the
    // webhook body limit.
    let inbound = Router::new()
        .route(
            "/i/:public_identifier",
            post(capture::capture)
                .get(capture::capture)
                .put(capture::capture)
                .patch(capture::capture)
                .delete(capture::capture),
        )
        .layer(RequestBodyLimitLayer::new(max_body));

    Router::new()
        .route("/healthz", get(healthz))
        .merge(inbound)
        // Agent WebSocket gateway. Authenticated via the protocol Hello.
        .route("/agent/ws", get(gateway::gateway))
        .nest("/api/auth", routes::router())
        .nest("/api/projects", projects::router())
        .nest(
            "/api/projects/:id/endpoints",
            endpoints::project_nested_router(),
        )
        .nest("/api/endpoints", endpoints::router())
        .nest("/api/events", events::router())
        .nest("/api/events", replay::router())
        .nest("/api/agents", agents::router())
        .with_state(state.clone())
        // Serve the built dashboard. In dev, Vite handles this on :5173.
        // In production, the dashboard dist is embedded at build time.
        .fallback_service(ServeDir::new(&state.config.dashboard_dir))
}

async fn healthz(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}
