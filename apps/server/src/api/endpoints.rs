//! Endpoints API.
//!
//! - `GET /api/endpoints` list all endpoints.
//! - `GET/POST /api/projects/:project_id/endpoints` list/create for a project.
//! - `GET/PATCH/DELETE /api/endpoints/:id` operate on a single endpoint.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;

use crate::auth::middleware::AuthSession;
use crate::database::models::{generate_public_identifier, Endpoint};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_all))
        .route("/:id", get(get_one).patch(update).delete(delete_one))
}

pub fn project_nested_router() -> Router<Arc<AppState>> {
    Router::new().route("/", get(list_for_project).post(create_for_project))
}

#[derive(Debug, Deserialize)]
pub struct CreateEndpointRequest {
    pub name: String,
    #[serde(default)]
    pub provider: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateEndpointRequest {
    name: Option<String>,
    enabled: Option<bool>,
    provider: Option<String>,
}

async fn list_all(
    State(state): State<Arc<AppState>>,
    _session: AuthSession,
) -> ApiResult<impl IntoResponse> {
    let endpoints: Vec<Endpoint> = sqlx::query_as(
        "SELECT id, project_id, name, public_identifier, enabled, provider, created_at, updated_at
         FROM endpoints ORDER BY created_at DESC",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({ "endpoints": endpoints })))
}

async fn list_for_project(
    State(state): State<Arc<AppState>>,
    _session: AuthSession,
    Path(project_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let endpoints: Vec<Endpoint> = sqlx::query_as(
        "SELECT id, project_id, name, public_identifier, enabled, provider, created_at, updated_at
         FROM endpoints WHERE project_id = ? ORDER BY created_at DESC",
    )
    .bind(&project_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({ "endpoints": endpoints })))
}

async fn create_for_project(
    State(state): State<Arc<AppState>>,
    _session: AuthSession,
    Path(project_id): Path<String>,
    Json(req): Json<CreateEndpointRequest>,
) -> ApiResult<impl IntoResponse> {
    if req.name.trim().is_empty() {
        return Err(ApiError::BadRequest("name is required".into()));
    }
    // Verify the project exists.
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_one(&state.pool)
        .await?;
    if exists == 0 {
        return Err(ApiError::NotFound);
    }

    let id = uuid::Uuid::new_v4().to_string();
    let public_identifier = generate_public_identifier();
    let endpoint: Endpoint = sqlx::query_as(
        "INSERT INTO endpoints (id, project_id, name, public_identifier, provider)
         VALUES (?, ?, ?, ?, ?)
         RETURNING id, project_id, name, public_identifier, enabled, provider, created_at, updated_at",
    )
    .bind(&id)
    .bind(&project_id)
    .bind(req.name.trim())
    .bind(&public_identifier)
    .bind(req.provider.as_deref().map(str::trim))
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(json!(endpoint))))
}

async fn get_one(
    State(state): State<Arc<AppState>>,
    _session: AuthSession,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let endpoint: Endpoint = sqlx::query_as(
        "SELECT id, project_id, name, public_identifier, enabled, provider, created_at, updated_at
         FROM endpoints WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;
    Ok(Json(json!(endpoint)))
}

async fn update(
    State(state): State<Arc<AppState>>,
    _session: AuthSession,
    Path(id): Path<String>,
    Json(req): Json<UpdateEndpointRequest>,
) -> ApiResult<impl IntoResponse> {
    let endpoint: Endpoint = sqlx::query_as(
        "UPDATE endpoints SET
            name = COALESCE(?, name),
            enabled = COALESCE(?, enabled),
            provider = COALESCE(?, provider),
            updated_at = datetime('now')
         WHERE id = ?
         RETURNING id, project_id, name, public_identifier, enabled, provider, created_at, updated_at",
    )
    .bind(req.name.as_deref().map(str::trim))
    .bind(req.enabled)
    .bind(req.provider.as_deref().map(str::trim))
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;
    Ok(Json(json!(endpoint)))
}

async fn delete_one(
    State(state): State<Arc<AppState>>,
    _session: AuthSession,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let res = sqlx::query("DELETE FROM endpoints WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
