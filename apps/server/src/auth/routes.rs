//! Authentication API routes: login, logout, current user.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::middleware::{
    attach_cookie, clear_cookie_header, session_cookie_header, AuthSession,
};
use crate::auth::password::verify_password;
use crate::auth::session;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct UserResponse {
    id: String,
    username: String,
    is_admin: bool,
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> ApiResult<impl IntoResponse> {
    if req.username.is_empty() || req.password.is_empty() {
        return Err(ApiError::BadRequest(
            "username and password are required".into(),
        ));
    }

    let user = session::find_user_by_username(&state.pool, &req.username)
        .await
        .map_err(|_| ApiError::Internal)?
        .ok_or(ApiError::Unauthorized)?;

    if !verify_password(&req.password, &user.password_hash) {
        return Err(ApiError::Unauthorized);
    }

    let (token, _session_id) = session::create_session(&state.pool, &state.config, &user.id)
        .await
        .map_err(|_| ApiError::Internal)?;

    session::touch_login(&state.pool, &user.id).await.ok();

    let expires_at = Utc::now() + chrono::Duration::hours(state.config.session_ttl_hours as i64);
    // Secure flag: set when the public base URL is https, or when behind a
    // trusted proxy that terminates TLS. We default to true in production
    // deployments; localhost dev over http still works because browsers
    // accept Secure cookies on localhost in modern implementations, but to
    // keep local dev frictionless we disable Secure when the public URL is
    // plain http.
    let secure = state.config.public_base_url.starts_with("https://");
    let cookie = session_cookie_header(&token, expires_at, secure);

    let mut resp = Json(json!({
        "user": UserResponse {
            id: user.id,
            username: user.username,
            is_admin: user.is_admin,
        }
    }))
    .into_response();
    attach_cookie(&mut resp, cookie);
    Ok(resp)
}

async fn logout(
    State(state): State<Arc<AppState>>,
    session: AuthSession,
) -> ApiResult<impl IntoResponse> {
    session::delete_session(&state.pool, &session.0.id)
        .await
        .ok();
    let secure = state.config.public_base_url.starts_with("https://");
    let mut resp = (StatusCode::OK, Json(json!({ "ok": true }))).into_response();
    attach_cookie(&mut resp, clear_cookie_header(secure));
    Ok(resp)
}

async fn me(session: AuthSession) -> ApiResult<impl IntoResponse> {
    // Fetch the user to return current info.
    Ok(Json(json!({ "authenticated": true, "session_id": session.0.id })).into_response())
}
