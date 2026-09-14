//! Projects API: list, create, get, update, delete.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;

use crate::auth::middleware::AuthSession;
use crate::database::models::{slugify, CreateProjectRequest, Project, UpdateProjectRequest};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/:id", get(get_one).patch(update).delete(delete_one))
}

async fn list(
    State(state): State<Arc<AppState>>,
    _session: AuthSession,
) -> ApiResult<impl IntoResponse> {
    let projects: Vec<Project> = sqlx::query_as(
        "SELECT id, name, slug, description, enabled, created_at, updated_at
         FROM projects ORDER BY created_at DESC",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({ "projects": projects })))
}

async fn create(
    State(state): State<Arc<AppState>>,
    _session: AuthSession,
    Json(req): Json<CreateProjectRequest>,
) -> ApiResult<impl IntoResponse> {
    if req.name.trim().is_empty() {
        return Err(ApiError::BadRequest("name is required".into()));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let slug = slugify(&req.name);
    // Ensure slug uniqueness with a suffix if needed.
    let slug = ensure_unique_slug(&state.pool, &slug).await?;

    let project: Project = sqlx::query_as(
        "INSERT INTO projects (id, name, slug, description) VALUES (?, ?, ?, ?)
         RETURNING id, name, slug, description, enabled, created_at, updated_at",
    )
    .bind(&id)
    .bind(req.name.trim())
    .bind(&slug)
    .bind(req.description.trim())
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(json!(project))))
}

async fn get_one(
    State(state): State<Arc<AppState>>,
    _session: AuthSession,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let project: Project = sqlx::query_as(
        "SELECT id, name, slug, description, enabled, created_at, updated_at
         FROM projects WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;
    Ok(Json(json!(project)))
}

async fn update(
    State(state): State<Arc<AppState>>,
    _session: AuthSession,
    Path(id): Path<String>,
    Json(req): Json<UpdateProjectRequest>,
) -> ApiResult<impl IntoResponse> {
    let project: Project = sqlx::query_as(
        "UPDATE projects SET
            name = COALESCE(?, name),
            description = COALESCE(?, description),
            enabled = COALESCE(?, enabled),
            updated_at = datetime('now')
         WHERE id = ?
         RETURNING id, name, slug, description, enabled, created_at, updated_at",
    )
    .bind(req.name.as_deref().map(str::trim))
    .bind(req.description.as_deref().map(str::trim))
    .bind(req.enabled)
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;
    Ok(Json(json!(project)))
}

async fn delete_one(
    State(state): State<Arc<AppState>>,
    _session: AuthSession,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let res = sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn ensure_unique_slug(pool: &sqlx::SqlitePool, base: &str) -> ApiResult<String> {
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE slug = ?")
        .bind(base)
        .fetch_one(pool)
        .await?;
    if exists == 0 {
        return Ok(base.to_string());
    }
    for i in 2..1000 {
        let candidate = format!("{base}-{i}");
        let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE slug = ?")
            .bind(&candidate)
            .fetch_one(pool)
            .await?;
        if exists == 0 {
            return Ok(candidate);
        }
    }
    Err(ApiError::Conflict("could not generate unique slug".into()))
}
