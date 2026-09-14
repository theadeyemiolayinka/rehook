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
use base64::Engine;
use chrono::Utc;
use hookrelay_protocol::{DeliveryInstruction, ReplayHeader};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::auth::middleware::AuthSession;
use crate::database::models::EventRow;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use hookrelay_protocol::is_hop_by_hop;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/:id/replay", post(replay))
}

#[derive(Debug, Deserialize)]
struct ReplayRequest {
    /// The agent to deliver to.
    agent_id: String,
    /// The target identifier the agent resolves locally. The agent must have
    /// this configured in its allowlist.
    target_id: String,
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

    // Verify the agent exists and is subscribed to the project.
    let agent_enabled: Option<bool> = sqlx::query_scalar("SELECT enabled FROM agents WHERE id = ?")
        .bind(&req.agent_id)
        .fetch_optional(&state.pool)
        .await?;
    let enabled = agent_enabled.ok_or(ApiError::NotFound)?;
    if !enabled {
        return Err(ApiError::BadRequest("agent is disabled".into()));
    }

    let subscribed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_subscriptions WHERE agent_id = ? AND project_id = ?",
    )
    .bind(&req.agent_id)
    .bind(&event.project_id)
    .fetch_one(&state.pool)
    .await?;
    if subscribed == 0 {
        return Err(ApiError::BadRequest(
            "agent is not subscribed to this project".into(),
        ));
    }

    // Compute the next attempt number.
    let attempt: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(attempt_number), 0) + 1 FROM deliveries WHERE event_id = ?",
    )
    .bind(&event_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    let delivery_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO deliveries (id, event_id, agent_id, target_id, attempt_number, status)
         VALUES (?, ?, ?, ?, ?, 'pending')",
    )
    .bind(delivery_id.to_string())
    .bind(&event_id)
    .bind(&req.agent_id)
    .bind(&req.target_id)
    .bind(attempt)
    .execute(&state.pool)
    .await?;

    // Build the delivery instruction with filtered headers.
    let headers = build_replay_headers(&event.headers_json);
    let body_b64 = event
        .body
        .as_deref()
        .map(|b| base64::engine::general_purpose::STANDARD.encode(b));

    let instruction = DeliveryInstruction {
        delivery_id,
        event_id: Uuid::parse_str(&event.id).unwrap_or_default(),
        project_id: Uuid::parse_str(&event.project_id).unwrap_or_default(),
        target_id: req.target_id,
        method: event.request_method,
        content_type: event.content_type,
        headers,
        body: body_b64,
    };

    // Dispatch to the agent via the registry.
    let project_uuid = Uuid::parse_str(&event.project_id).unwrap_or_default();
    match state
        .agents
        .dispatch(agent_uuid, project_uuid, instruction)
        .await
    {
        Ok(()) => {
            sqlx::query("UPDATE events SET delivery_state = 'dispatched' WHERE id = ?")
                .bind(&event_id)
                .execute(&state.pool)
                .await
                .ok();
            Ok((
                StatusCode::ACCEPTED,
                Json(json!({ "delivery_id": delivery_id, "status": "dispatched" })),
            )
                .into_response())
        }
        Err(reason) => {
            sqlx::query(
                "UPDATE deliveries SET status = 'failed', error_message = ?, completed_at = ? WHERE id = ?",
            )
            .bind(&reason)
            .bind(Utc::now().to_rfc3339())
            .bind(delivery_id.to_string())
            .execute(&state.pool)
            .await
            .ok();
            Err(ApiError::BadRequest(reason))
        }
    }
}

/// Build the replay header list, filtering hop-by-hop and transport headers.
fn build_replay_headers(headers_json: &str) -> Vec<ReplayHeader> {
    let Ok(headers) = serde_json::from_str::<serde_json::Value>(headers_json) else {
        return Vec::new();
    };
    let serde_json::Value::Object(map) = headers else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (name, value) in map {
        if is_hop_by_hop(&name) {
            continue;
        }
        match value {
            serde_json::Value::String(v) => {
                out.push(ReplayHeader { name, value: v });
            }
            serde_json::Value::Array(arr) => {
                for v in arr {
                    if let serde_json::Value::String(s) = v {
                        out.push(ReplayHeader {
                            name: name.clone(),
                            value: s,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    out
}
