//! Replay API: trigger delivery of a captured event to a connected agent.
//!
//! The server sends a delivery instruction referencing the project and a
//! target identifier. The agent resolves the target to a locally configured
//! URL and validates it. The server never sends a URL. See SECURITY.md.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::auth::middleware::AuthSession;
use crate::database::models::EventRow;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/:id/replay", post(replay))
}

#[derive(Debug, Deserialize)]
struct ReplayRequest {
    /// The agent to deliver to.
    agent_id: String,
    /// Optional target identifier override. Normally omitted: the agent
    /// resolves the destination from its own route for this endpoint. If
    /// given, the agent still validates it against its local allowlist.
    #[serde(default)]
    target_id: Option<String>,
}

async fn replay(
    State(state): State<Arc<AppState>>,
    _session: AuthSession,
    Path(event_id): Path<String>,
    Json(req): Json<ReplayRequest>,
) -> ApiResult<impl IntoResponse> {
    let agent_uuid = Uuid::parse_str(&req.agent_id)
        .map_err(|_| ApiError::BadRequest("invalid agent_id".into()))?;

    // Load the event.
    let event: EventRow = sqlx::query_as(
        "SELECT id, project_id, endpoint_id, request_method, content_type,
                remote_address, received_at, payload_size, headers_json, body, delivery_state
         FROM events WHERE id = ?",
    )
    .bind(&event_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;

    // Verify the agent exists and is enabled.
    let agent_enabled: Option<bool> = sqlx::query_scalar("SELECT enabled FROM agents WHERE id = ?")
        .bind(&req.agent_id)
        .fetch_optional(&state.pool)
        .await?;
    let enabled = agent_enabled.ok_or(ApiError::NotFound)?;
    if !enabled {
        return Err(ApiError::BadRequest("agent is disabled".into()));
    }

    // Resolve the target identifier to send. Prefer an explicit override;
    // otherwise use the target the agent declared in its subscription for
    // this endpoint; otherwise empty and the agent resolves it from its own
    // route configuration. The agent validates every case locally.
    let target_id = match req.target_id.as_deref() {
        Some(t) if !t.trim().is_empty() => t.trim().to_string(),
        _ => sqlx::query_scalar::<_, String>(
            "SELECT target_id FROM agent_subscriptions WHERE agent_id = ? AND endpoint_id = ?",
        )
        .bind(&req.agent_id)
        .bind(&event.endpoint_id)
        .fetch_optional(&state.pool)
        .await?
        .unwrap_or_default(),
    };

    // Dispatch to the agent via the registry. For manual replay we do not
    // require a prior subscription; the admin explicitly chose this agent.
    // The agent's local allowlist validates the target.
    match crate::dispatch::dispatch_event_to_agent(&state, &event, agent_uuid, &target_id).await {
        Ok(delivery_id) => Ok((
            StatusCode::ACCEPTED,
            Json(json!({ "delivery_id": delivery_id, "status": "dispatched" })),
        )
            .into_response()),
        Err(reason) => Err(ApiError::BadRequest(reason)),
    }
}
