//! Database row models and serialization DTOs.

use rand::RngCore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct Endpoint {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub public_identifier: String,
    pub enabled: bool,
    pub provider: Option<String>,
    pub validation_type: String,
    pub validation_secret: Option<String>,
    pub validation_header: Option<String>,
    pub validation_query: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EventRow {
    pub id: String,
    pub project_id: String,
    pub endpoint_id: String,
    pub request_method: String,
    pub content_type: Option<String>,
    pub remote_address: Option<String>,
    pub received_at: String,
    pub payload_size: i64,
    pub headers_json: String,
    pub body: Option<Vec<u8>>,
    pub delivery_state: String,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct AgentRow {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_seen_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
}

/// Generate a slug from a name: lowercase, alphanumerics and hyphens.
pub fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_dash = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            for lc in c.to_lowercase() {
                out.push(lc);
            }
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "project".to_string()
    } else {
        trimmed
    }
}

/// Generate an unguessable public identifier for an inbound endpoint.
/// 20 base64url chars = ~120 bits of entropy.
pub fn generate_public_identifier() -> String {
    let mut bytes = [0u8; 15];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
}
