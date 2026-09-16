//! Events API: list, get detail (with headers and body), delete.
//!
//! Sensitive headers are masked in the response by default. The raw
//! (unmasked) view is available via a query parameter to authenticated admins
//! for debugging signature verification.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::middleware::AuthSession;
use crate::database::models::EventRow;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use crate::webhook::capture::mask_sensitive_headers;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list))
        .route("/:id", get(get_one).delete(delete_one))
        .route("/:id/body", get(get_body))
        .route("/:id/deliveries", get(list_deliveries))
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    #[serde(default)]
    project_id: Option<String>,
    #[serde(default)]
    endpoint_id: Option<String>,
    #[serde(default)]
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct DetailQuery {
    #[serde(default)]
    unmasked: bool,
}

/// A list row: lightweight, no body.
#[derive(Debug, Serialize, sqlx::FromRow)]
struct EventListRow {
    id: String,
    project_id: String,
    endpoint_id: String,
    request_method: String,
    content_type: Option<String>,
    remote_address: Option<String>,
    received_at: String,
    payload_size: i64,
    delivery_state: String,
}

async fn list(
    State(state): State<Arc<AppState>>,
    _session: AuthSession,
    Query(q): Query<ListQuery>,
) -> ApiResult<impl IntoResponse> {
    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let rows: Vec<EventListRow> = if let Some(pid) = q.project_id.as_deref() {
        sqlx::query_as(
            "SELECT id, project_id, endpoint_id, request_method, content_type,
                    remote_address, received_at, payload_size, delivery_state
             FROM events WHERE project_id = ?
             ORDER BY received_at DESC LIMIT ?",
        )
        .bind(pid)
        .bind(limit)
        .fetch_all(&state.pool)
        .await?
    } else if let Some(eid) = q.endpoint_id.as_deref() {
        sqlx::query_as(
            "SELECT id, project_id, endpoint_id, request_method, content_type,
                    remote_address, received_at, payload_size, delivery_state
             FROM events WHERE endpoint_id = ?
             ORDER BY received_at DESC LIMIT ?",
        )
        .bind(eid)
        .bind(limit)
        .fetch_all(&state.pool)
        .await?
    } else {
        sqlx::query_as(
            "SELECT id, project_id, endpoint_id, request_method, content_type,
                    remote_address, received_at, payload_size, delivery_state
             FROM events
             ORDER BY received_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&state.pool)
        .await?
    };
    Ok(Json(json!({ "events": rows })))
}

async fn get_one(
    State(state): State<Arc<AppState>>,
    _session: AuthSession,
    Path(id): Path<String>,
    Query(q): Query<DetailQuery>,
) -> ApiResult<impl IntoResponse> {
    let row: EventRow = sqlx::query_as(
        "SELECT id, project_id, endpoint_id, request_method, content_type,
                remote_address, received_at, payload_size, headers_json, body, delivery_state
         FROM events WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;

    let headers: serde_json::Value =
        serde_json::from_str(&row.headers_json).unwrap_or(serde_json::Value::Null);
    let headers = if q.unmasked {
        headers
    } else {
        mask_sensitive_headers(headers)
    };

    let body_b64 = row.body.as_deref().map(|b| {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(b)
    });

    Ok(Json(json!({
        "id": row.id,
        "project_id": row.project_id,
        "endpoint_id": row.endpoint_id,
        "request_method": row.request_method,
        "content_type": row.content_type,
        "remote_address": row.remote_address,
        "received_at": row.received_at,
        "payload_size": row.payload_size,
        "delivery_state": row.delivery_state,
        "headers": headers,
        "body": body_b64,
    })))
}

async fn get_body(
    State(state): State<Arc<AppState>>,
    _session: AuthSession,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let row: EventRow = sqlx::query_as(
        "SELECT id, project_id, endpoint_id, request_method, content_type,
                remote_address, received_at, payload_size, headers_json, body, delivery_state
         FROM events WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;

    let body = row.body.unwrap_or_default();
    let content_type = row
        .content_type
        .unwrap_or_else(|| "application/octet-stream".to_string());
    Ok((
        StatusCode::OK,
        [("content-type", content_type)],
        Bytes::from(body),
    ))
}

async fn delete_one(
    State(state): State<Arc<AppState>>,
    _session: AuthSession,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let res = sqlx::query("DELETE FROM events WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// A delivery attempt row for the dashboard.
#[derive(Debug, Serialize, sqlx::FromRow)]
struct DeliveryRow {
    id: String,
    agent_id: String,
    agent_name: Option<String>,
    target_id: String,
    attempt_number: i64,
    status: String,
    http_status: Option<i64>,
    duration_ms: Option<i64>,
    error_category: Option<String>,
    error_message: Option<String>,
    started_at: String,
    completed_at: Option<String>,
}

/// List delivery attempts for an event. Used by the event detail view to show
/// replay history. The original event is never mutated by replay.
async fn list_deliveries(
    State(state): State<Arc<AppState>>,
    _session: AuthSession,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let rows: Vec<DeliveryRow> = sqlx::query_as(
        "SELECT d.id, d.agent_id, a.name AS agent_name, d.target_id, d.attempt_number,
                d.status, d.http_status, d.duration_ms, d.error_category,
                d.error_message, d.started_at, d.completed_at
         FROM deliveries d
         LEFT JOIN agents a ON a.id = d.agent_id
         WHERE d.event_id = ?
         ORDER BY d.attempt_number ASC",
    )
    .bind(&id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({ "deliveries": rows })))
}
