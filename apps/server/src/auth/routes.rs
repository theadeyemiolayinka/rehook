//! Authentication API routes: login, logout, current user.

use std::net::IpAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, State};
use axum::http::HeaderValue;
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

/// Extract the client IP from the request, accounting for trusted proxies.
fn client_ip(
    x_forwarded_for: Option<&HeaderValue>,
    connect_info: Option<IpAddr>,
    trusted_proxy_hops: usize,
) -> Option<IpAddr> {
    if let Some(xff) = x_forwarded_for {
        if let Ok(s) = xff.to_str() {
            let parts: Vec<&str> = s.split(',').map(|p| p.trim()).collect();
            if !parts.is_empty() {
                // The client IP is the leftmost entry, minus trusted proxy hops.
                let idx = parts
                    .len()
                    .saturating_sub(1)
                    .saturating_sub(trusted_proxy_hops);
                if let Some(ip_str) = parts.get(idx) {
                    if let Ok(ip) = ip_str.parse::<IpAddr>() {
                        return Some(ip);
                    }
                }
            }
        }
    }
    connect_info
}

async fn login(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    headers: axum::http::HeaderMap,
    Json(req): Json<LoginRequest>,
) -> ApiResult<impl IntoResponse> {
    let ip = client_ip(
        headers.get("x-forwarded-for"),
        Some(addr.ip()),
        state.config.trusted_proxy_hops,
    )
    .unwrap_or_else(|| addr.ip());

    // Rate limit check.
    if state.login_limiter.is_blocked(ip).await {
        return Err(ApiError::RateLimited);
    }

    if req.username.is_empty() || req.password.is_empty() {
        state.login_limiter.record_failure(ip).await;
        return Err(ApiError::BadRequest(
            "username and password are required".into(),
        ));
    }

    let user = match session::find_user_by_username(&state.pool, &req.username)
        .await
        .map_err(|_| ApiError::Internal)?
    {
        Some(u) => u,
        None => {
            state.login_limiter.record_failure(ip).await;
            return Err(ApiError::Unauthorized);
        }
    };

    if !verify_password(&req.password, &user.password_hash) {
        state.login_limiter.record_failure(ip).await;
        return Err(ApiError::Unauthorized);
    }

    let (token, _session_id) = session::create_session(&state.pool, &state.config, &user.id)
        .await
        .map_err(|_| ApiError::Internal)?;

    session::touch_login(&state.pool, &user.id).await.ok();

    // Clear rate limit on successful login.
    state.login_limiter.clear(ip).await;

    let expires_at = Utc::now() + chrono::Duration::hours(state.config.session_ttl_hours as i64);
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
    Ok(Json(json!({ "authenticated": true, "session_id": session.0.id })).into_response())
}
