//! HTTP route definitions.

use std::sync::Arc;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::{ServeDir, ServeFile};

use crate::api::agents;
use crate::api::endpoints;
use crate::api::events;
use crate::api::projects;
use crate::api::replay;
use crate::api::settings;
use crate::auth::routes;
use crate::database::models::Project;
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
        .route("/agent/projects", get(agent_projects))
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
        .nest("/api/settings", settings::router())
        .with_state(state.clone())
        // Serve the built dashboard. In dev, Vite handles this on :5173.
        // In production, the dashboard dist is served here. The fallback to
        // index.html enables client-side routing (React Router): requests to
        // paths like /settings that do not match a static file fall back to
        // index.html so the SPA can render the correct route.
        .fallback_service(
            ServeDir::new(&state.config.dashboard_dir).fallback(ServeFile::new(
                state.config.dashboard_dir.join("index.html"),
            )),
        )
}

async fn healthz(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}

/// Endpoint for agents to look up project names by ID. Authenticates
/// the agent using its Bearer token. Returns all projects so the agent
/// can map IDs to names in its UI.
async fn agent_projects(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> axum::response::Response {
    // Extract the Bearer token.
    let token = match headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
    {
        Some(t) => t,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "missing bearer token" })),
            )
                .into_response();
        }
    };

    // Verify the token against any agent in the database.
    let session_key = match state.config.session_key() {
        Ok(k) => k,
        Err(_) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "server error" })),
            )
                .into_response();
        }
    };

    let agents: Vec<crate::database::models::AgentRow> =
        match sqlx::query_as("SELECT id, name, enabled, created_at, updated_at, last_seen_at FROM agents WHERE enabled = 1")
            .fetch_all(&state.pool)
            .await
        {
            Ok(a) => a,
            Err(_) => {
                return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "server error" }))).into_response();
            }
        };

    let mut authenticated = false;
    for agent in &agents {
        // Load the token hash for this agent.
        let hash_hex: Option<String> =
            sqlx::query_scalar("SELECT token_hash FROM agents WHERE id = ?")
                .bind(&agent.id)
                .fetch_optional(&state.pool)
                .await
                .ok()
                .flatten();

        if let Some(hash) = hash_hex {
            if crate::api::agents::verify_token(&token, &hash, &session_key) {
                authenticated = true;
                break;
            }
        }
    }

    if !authenticated {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "invalid token" })),
        )
            .into_response();
    }

    // Return all projects with their endpoints.
    let projects: Vec<Project> = match sqlx::query_as(
        "SELECT id, name, slug, description, enabled, created_at, updated_at FROM projects ORDER BY name",
    )
    .fetch_all(&state.pool)
    .await
    {
        Ok(p) => p,
        Err(_) => {
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "server error" }))).into_response();
        }
    };

    // Fetch all endpoints grouped by project.
    let endpoints: Vec<(String, String, String)> =
        match sqlx::query_as("SELECT project_id, id, name FROM endpoints ORDER BY name")
            .fetch_all(&state.pool)
            .await
        {
            Ok(e) => e,
            Err(_) => {
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "server error" })),
                )
                    .into_response();
            }
        };

    let mut endpoints_by_project: std::collections::HashMap<String, Vec<serde_json::Value>> =
        std::collections::HashMap::new();
    for (project_id, id, name) in &endpoints {
        endpoints_by_project
            .entry(project_id.clone())
            .or_default()
            .push(json!({ "id": id, "name": name }));
    }

    let mapped: Vec<serde_json::Value> = projects
        .iter()
        .map(|p| {
            json!({
                "id": p.id,
                "name": p.name,
                "slug": p.slug,
                "endpoints": endpoints_by_project.get(&p.id).cloned().unwrap_or_default(),
            })
        })
        .collect();

    Json(json!({ "projects": mapped })).into_response()
}
