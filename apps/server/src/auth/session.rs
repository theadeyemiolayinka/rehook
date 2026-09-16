//! Session management and authentication helpers.

use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rand::RngCore;
use sqlx::SqlitePool;

use crate::config::Config;

/// The name of the session cookie.
pub const COOKIE_NAME: &str = "rehook_session";

/// A session row.
#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub token_hash: String,
    pub created_at: String,
    pub expires_at: String,
    pub last_seen_at: String,
}

/// Hash a raw session token with HMAC-SHA256 using the session key.
/// The cookie carries the raw token; the DB stores only the hash, so a DB
/// leak does not immediately yield valid tokens.
pub fn hash_token(raw: &[u8], key: &[u8]) -> Result<Vec<u8>> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(key).map_err(|e| anyhow::anyhow!("hmac key: {e}"))?;
    mac.update(raw);
    Ok(mac.finalize().into_bytes().to_vec())
}

/// Generate a cryptographically random session token (32 bytes, url-safe base64).
pub fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
}

/// Create a new session for a user. Returns the raw token (to put in the
/// cookie) and the session row id.
pub async fn create_session(
    pool: &SqlitePool,
    config: &Config,
    user_id: &str,
) -> Result<(String, String)> {
    let raw = generate_token();
    let key = config.session_key()?;
    let hash = hex::encode(hash_token(raw.as_bytes(), &key)?);
    let id = uuid::Uuid::new_v4().to_string();
    let expires_at = Utc::now() + Duration::from_secs(config.session_ttl_hours * 3600);
    sqlx::query("INSERT INTO sessions (id, user_id, token_hash, expires_at) VALUES (?, ?, ?, ?)")
        .bind(&id)
        .bind(user_id)
        .bind(&hash)
        .bind(expires_at.to_rfc3339())
        .execute(pool)
        .await
        .context("inserting session")?;
    Ok((raw, id))
}

/// Look up a session by its raw token. Returns None if not found, expired, or
/// the hash does not match (constant-time comparison).
pub async fn validate_session(
    pool: &SqlitePool,
    config: &Config,
    raw_token: &str,
) -> Result<Option<Session>> {
    let key = config.session_key()?;
    let expected = hex::encode(hash_token(raw_token.as_bytes(), &key)?);

    let session: Option<Session> = sqlx::query_as(
        "SELECT id, user_id, token_hash, created_at, expires_at, last_seen_at
         FROM sessions WHERE token_hash = ?",
    )
    .bind(&expected)
    .fetch_optional(pool)
    .await
    .context("looking up session")?;

    let Some(session) = session else {
        return Ok(None);
    };

    // Check expiry.
    if let Ok(expires) = DateTime::parse_from_rfc3339(&session.expires_at) {
        if Utc::now() > expires.with_timezone(&Utc) {
            return Ok(None);
        }
    }

    // Bump last seen.
    sqlx::query("UPDATE sessions SET last_seen_at = ? WHERE id = ?")
        .bind(Utc::now().to_rfc3339())
        .bind(&session.id)
        .execute(pool)
        .await
        .ok();

    Ok(Some(session))
}

/// Delete a session (logout).
pub async fn delete_session(pool: &SqlitePool, session_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(session_id)
        .execute(pool)
        .await
        .ok();
    Ok(())
}

/// A minimal user record.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub is_admin: bool,
}

/// Look up a user by username.
pub async fn find_user_by_username(pool: &SqlitePool, username: &str) -> Result<Option<User>> {
    let user: Option<User> = sqlx::query_as(
        "SELECT id, username, password_hash, is_admin FROM users WHERE username = ?",
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .context("looking up user")?;
    Ok(user)
}

/// Record a login time.
pub async fn touch_login(pool: &SqlitePool, user_id: &str) -> Result<()> {
    sqlx::query("UPDATE users SET last_login_at = ? WHERE id = ?")
        .bind(Utc::now().to_rfc3339())
        .bind(user_id)
        .execute(pool)
        .await
        .ok();
    Ok(())
}
