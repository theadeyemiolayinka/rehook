//! Agents API: create, list, get, revoke (enable/disable).
//!
//! Agent tokens are separate from dashboard sessions. The raw token is shown
//! exactly once at creation time; only a hash is stored.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;

use crate::auth::middleware::AuthSession;
use crate::database::models::AgentRow;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/:id", get(get_one).patch(update).delete(delete_one))
}

#[derive(Debug, Deserialize)]
struct CreateAgentRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct UpdateAgentRequest {
    enabled: Option<bool>,
    name: Option<String>,
}

async fn list(
    State(state): State<Arc<AppState>>,
    _session: AuthSession,
) -> ApiResult<impl IntoResponse> {
    let agents: Vec<AgentRow> = sqlx::query_as(
        "SELECT id, name, enabled, created_at, updated_at, last_seen_at
         FROM agents ORDER BY created_at DESC",
    )
    .fetch_all(&state.pool)
    .await?;

    // Report connection status for each agent.
    let mut agent_list: Vec<serde_json::Value> = Vec::new();
    for agent in &agents {
        let agent_id = uuid::Uuid::parse_str(&agent.id).unwrap_or_default();
        let connected = state.agents.is_connected(agent_id).await;
        let mut value = serde_json::to_value(agent).unwrap_or(serde_json::Value::Null);
        if let Some(obj) = value.as_object_mut() {
            obj.insert("connected".into(), serde_json::Value::Bool(connected));
        }
        agent_list.push(value);
    }

    Ok(Json(json!({ "agents": agent_list })))
}

async fn create(
    State(state): State<Arc<AppState>>,
    _session: AuthSession,
    Json(req): Json<CreateAgentRequest>,
) -> ApiResult<impl IntoResponse> {
    if req.name.trim().is_empty() {
        return Err(ApiError::BadRequest("name is required".into()));
    }
    let id = uuid::Uuid::new_v4().to_string();
    // Generate a raw token. Format: "re_" + 32 url-safe base64 chars.
    let raw_token = format!("re_{}", generate_token_secret());
    let token_hash = hex::encode(
        hash_token(
            &raw_token,
            &state.config.session_key().map_err(|_| ApiError::Internal)?,
        )
        .map_err(|_| ApiError::Internal)?,
    );

    let agent: AgentRow = sqlx::query_as(
        "INSERT INTO agents (id, name, token_hash) VALUES (?, ?, ?)
         RETURNING id, name, enabled, created_at, updated_at, last_seen_at",
    )
    .bind(&id)
    .bind(req.name.trim())
    .bind(&token_hash)
    .fetch_one(&state.pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({ "agent": agent, "token": raw_token })),
    ))
}

async fn get_one(
    State(state): State<Arc<AppState>>,
    _session: AuthSession,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let agent: AgentRow = sqlx::query_as(
        "SELECT id, name, enabled, created_at, updated_at, last_seen_at
         FROM agents WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;
    Ok(Json(json!(agent)))
}

async fn update(
    State(state): State<Arc<AppState>>,
    _session: AuthSession,
    Path(id): Path<String>,
    Json(req): Json<UpdateAgentRequest>,
) -> ApiResult<impl IntoResponse> {
    if req
        .name
        .as_deref()
        .map(|n| n.trim().is_empty())
        .unwrap_or(false)
    {
        return Err(ApiError::BadRequest("name cannot be empty".into()));
    }
    let agent: AgentRow = sqlx::query_as(
        "UPDATE agents SET
            enabled = COALESCE(?, enabled),
            name = COALESCE(?, name),
            updated_at = datetime('now')
         WHERE id = ?
         RETURNING id, name, enabled, created_at, updated_at, last_seen_at",
    )
    .bind(req.enabled)
    .bind(req.name.as_deref().map(str::trim))
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;

    // A disabled agent must not keep a live connection.
    if req.enabled == Some(false) {
        if let Ok(agent_uuid) = uuid::Uuid::parse_str(&id) {
            state.agents.disconnect(agent_uuid).await;
        }
    }
    Ok(Json(json!(agent)))
}

async fn delete_one(
    State(state): State<Arc<AppState>>,
    _session: AuthSession,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let agent_uuid =
        uuid::Uuid::parse_str(&id).map_err(|_| ApiError::BadRequest("invalid agent id".into()))?;

    // Drop the live connection first so the agent stops immediately.
    state.agents.disconnect(agent_uuid).await;

    let res = sqlx::query("DELETE FROM agents WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    tracing::info!(agent_id = %id, "agent deleted");
    Ok(StatusCode::NO_CONTENT)
}

/// Generate a 32-byte url-safe random token secret.
fn generate_token_secret() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
}

/// HMAC-SHA256 hash of the agent token, keyed by the session key.
pub fn hash_token(raw: &str, key: &[u8]) -> Result<Vec<u8>, ApiError> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| ApiError::Internal)?;
    mac.update(raw.as_bytes());
    Ok(mac.finalize().into_bytes().to_vec())
}

/// Verify a raw agent token against the stored hash. Constant-time via HMAC.
pub fn verify_token(raw: &str, hash_hex: &str, key: &[u8]) -> bool {
    let Ok(computed) = hash_token(raw, key) else {
        return false;
    };
    let computed_hex = hex::encode(computed);
    // Constant-time compare.
    use subtle::ConstantTimeEq;
    let a = computed_hex.as_bytes();
    let b = hash_hex.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).unwrap_u8() == 1
}
