//! Authentication middleware and extractors.

use std::sync::Arc;

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::http::request::Parts;
use axum::response::Response;
use chrono::Utc;

use crate::auth::session::{self, Session, COOKIE_NAME};
use crate::error::ApiError;
use crate::state::AppState;

/// Extractor that requires a valid session. Returns the session on success,
/// or `ApiError::Unauthorized` otherwise.
pub struct AuthSession(pub Session);

#[async_trait]
impl FromRequestParts<Arc<AppState>> for AuthSession {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_cookie(parts, COOKIE_NAME).ok_or(ApiError::Unauthorized)?;
        let session = session::validate_session(&state.pool, &state.config, &token)
            .await
            .map_err(|_| ApiError::Internal)?;
        match session {
            Some(s) => Ok(AuthSession(s)),
            None => Err(ApiError::Unauthorized),
        }
    }
}

/// Parse a named cookie value from the Cookie header.
fn extract_cookie(parts: &Parts, name: &str) -> Option<String> {
    let header = parts.headers.get(COOKIE)?;
    let header = header.to_str().ok()?;
    for pair in header.split(';') {
        let pair = pair.trim();
        if let Some((k, v)) = pair.split_once('=') {
            if k.trim() == name {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

/// Build a Set-Cookie header value for a session.
pub fn session_cookie_header(
    token: &str,
    expires_at: chrono::DateTime<Utc>,
    secure: bool,
) -> String {
    let max_age = (expires_at - Utc::now()).num_seconds().max(0);
    format!(
        "{name}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure}",
        name = COOKIE_NAME,
        secure = if secure { "; Secure" } else { "" },
    )
}

/// Build a Set-Cookie header value that clears the session cookie.
pub fn clear_cookie_header(secure: bool) -> String {
    format!(
        "{name}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{secure}",
        name = COOKIE_NAME,
        secure = if secure { "; Secure" } else { "" },
    )
}

/// Attach a Set-Cookie header to a response.
pub fn attach_cookie(resp: &mut Response, value: String) {
    if let Ok(hv) = value.try_into() {
        resp.headers_mut().append(SET_COOKIE, hv);
    }
}
