//! Inbound webhook capture at `/i/{public_identifier}`.
//!
//! The capture path is unauthenticated (it is the public webhook endpoint).
//! Endpoint identifiers are unguessable but unguessability is not
//! authentication. Disabled and unknown endpoints respond identically to
//! avoid leaking existence. See SECURITY.md.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{ConnectInfo, Path, State};
use axum::http::{HeaderMap, Method, Uri};
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

use super::validate::{validate, ValidationResult};
use crate::error::ApiError;
use crate::state::AppState;

/// Minimal endpoint info needed for capture and validation.
#[derive(Debug, sqlx::FromRow)]
struct EndpointInfo {
    id: String,
    project_id: String,
    enabled: bool,
    validation_type: String,
    validation_secret: Option<String>,
    validation_header: Option<String>,
    validation_query: Option<String>,
}

/// Capture an inbound webhook. Accepts the common webhook methods.
pub async fn capture(
    State(state): State<Arc<AppState>>,
    Path(public_identifier): Path<String>,
    method: Method,
    headers: HeaderMap,
    uri: Uri,
    addr: ConnectInfo<SocketAddr>,
    body: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    // Resolve the endpoint. Unknown and disabled endpoints respond the same.
    let endpoint: Option<EndpointInfo> = sqlx::query_as(
        "SELECT id, project_id, enabled, validation_type, validation_secret, validation_header, validation_query
         FROM endpoints WHERE public_identifier = ?",
    )
    .bind(&public_identifier)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "endpoint lookup failed");
        ApiError::Internal
    })?;

    let Some(ep) = endpoint else {
        // Unknown endpoint: respond 202 to avoid leaking existence.
        tracing::debug!(identifier = %public_identifier, "webhook to unknown endpoint");
        return Ok(Json(json!({ "accepted": true })).into_response());
    };

    if !ep.enabled {
        // Disabled endpoint: respond identically to avoid leaking state.
        tracing::debug!(identifier = %public_identifier, "webhook to disabled endpoint");
        return Ok(Json(json!({ "accepted": true })).into_response());
    }

    // Enforce body size (belt and suspenders; the layer also limits this).
    if body.len() > state.config.max_webhook_body_size {
        return Err(ApiError::PayloadTooLarge);
    }

    // Validate the webhook signature/token if configured.
    match validate(
        &ep.validation_type,
        ep.validation_secret.as_deref(),
        ep.validation_header.as_deref(),
        ep.validation_query.as_deref(),
        &headers,
        &uri,
        &body,
    ) {
        ValidationResult::Ok => {}
        ValidationResult::Failed(reason) => {
            tracing::warn!(
                identifier = %public_identifier,
                reason = %reason,
                "webhook validation failed"
            );
            return Err(ApiError::Forbidden);
        }
    }

    // Determine remote address using trusted proxy configuration.
    let remote_address = resolve_remote(&state, &headers, Some(addr.0));

    // Capture headers as JSON. Multiple values per name become arrays.
    let headers_json = serialize_headers(&headers);

    let event_id = uuid::Uuid::new_v4().to_string();
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    sqlx::query(
        "INSERT INTO events
            (id, project_id, endpoint_id, request_method, content_type,
             remote_address, payload_size, headers_json, body, delivery_state)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending')",
    )
    .bind(&event_id)
    .bind(&ep.project_id)
    .bind(&ep.id)
    .bind(method.as_str())
    .bind(&content_type)
    .bind(&remote_address)
    .bind(body.len() as i64)
    .bind(&headers_json)
    .bind(body.as_ref())
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "event insert failed");
        ApiError::Internal
    })?;

    tracing::info!(
        event_id = %event_id,
        endpoint_id = %ep.id,
        method = %method,
        size = body.len(),
        "webhook captured"
    );

    // Dispatch to connected agents subscribed to this endpoint.
    let event = crate::database::models::EventRow {
        id: event_id.clone(),
        project_id: ep.project_id.clone(),
        endpoint_id: ep.id.clone(),
        request_method: method.as_str().to_string(),
        content_type: content_type.clone(),
        remote_address: remote_address.clone(),
        received_at: chrono::Utc::now().to_rfc3339(),
        payload_size: body.len() as i64,
        headers_json: headers_json.clone(),
        body: Some(body.as_ref().to_vec()),
        delivery_state: "pending".into(),
    };
    crate::dispatch::dispatch_new_event(&state, &event).await;

    // Enforce retention policy: delete events older than the retention window
    // and trim to the maximum stored events count. Runs after each capture.
    enforce_retention(&state).await;

    // 202 Accepted: the webhook was received and stored.
    Ok(Json(json!({ "accepted": true, "event_id": event_id })).into_response())
}

/// Delete events older than the retention window and trim to max stored events.
async fn enforce_retention(state: &Arc<AppState>) {
    // Delete by age.
    if state.config.event_retention_days > 0 {
        let cutoff =
            chrono::Utc::now() - chrono::Duration::days(state.config.event_retention_days as i64);
        if let Err(e) = sqlx::query("DELETE FROM events WHERE received_at < ?")
            .bind(cutoff.to_rfc3339())
            .execute(&state.pool)
            .await
        {
            tracing::warn!(error = ?e, "retention age cleanup failed");
        }
    }

    // Trim by count. Delete the oldest events exceeding the limit.
    if state.config.max_stored_events > 0 {
        let max = state.config.max_stored_events as i64;
        if let Err(e) = sqlx::query(
            "DELETE FROM events WHERE id IN (
                SELECT id FROM events ORDER BY received_at DESC LIMIT -1 OFFSET ?
            )",
        )
        .bind(max)
        .execute(&state.pool)
        .await
        {
            tracing::warn!(error = ?e, "retention count cleanup failed");
        }
    }
}

/// Serialize headers into a JSON object. Repeated header names become arrays.
fn serialize_headers(headers: &HeaderMap) -> String {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for (name, value) in headers.iter() {
        let key = name.as_str().to_lowercase();
        let val = match value.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => {
                // Non-ASCII: base64-encode the raw bytes.
                use base64::Engine;
                base64::engine::general_purpose::STANDARD.encode(value.as_bytes())
            }
        };
        map.entry(key).or_default().push(val);
    }
    // Collapse single-value arrays to scalars for cleaner display.
    let mut out = serde_json::Map::new();
    for (k, mut v) in map {
        if v.len() == 1 {
            out.insert(k, serde_json::Value::String(v.remove(0)));
        } else {
            out.insert(
                k,
                serde_json::Value::Array(v.into_iter().map(serde_json::Value::String).collect()),
            );
        }
    }
    serde_json::Value::Object(out).to_string()
}

/// Resolve the client remote address according to trusted proxy configuration.
///
/// If `trusted_proxy_hops` is 0, the direct socket peer is used and forwarding
/// headers are ignored. If > 0, the `X-Forwarded-For` header is parsed and the
/// address `hops` from the right is taken as the client.
fn resolve_remote(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    direct: Option<SocketAddr>,
) -> Option<String> {
    let hops = state.config.trusted_proxy_hops;
    if hops == 0 {
        return direct.map(|a| a.ip().to_string());
    }
    let xff: Vec<String> = headers
        .get_all("x-forwarded-for")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|s| s.split(','))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if xff.len() >= hops {
        // The client is `hops` from the right of the XFF chain.
        return Some(xff[xff.len() - hops].clone());
    }
    // Not enough hops in XFF; fall back to the direct peer.
    direct.map(|a| a.ip().to_string())
}

/// Header names that should be masked in dashboard display by default.
pub const SENSITIVE_HEADER_NAMES: &[&str] = &[
    "authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "api-key",
    "stripe-signature",
    "x-paystack-signature",
    "x-hub-signature",
    "x-hub-signature-256",
    "webhook-signature",
];

/// Mask sensitive header values for dashboard display, keeping a short prefix
/// to aid identification without revealing the full secret.
pub fn mask_sensitive_headers(headers: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    let Value::Object(map) = headers else {
        return headers;
    };
    let mut out = serde_json::Map::new();
    for (k, v) in map {
        if SENSITIVE_HEADER_NAMES.contains(&k.as_str()) {
            out.insert(k, Value::String(mask_value(&v)));
        } else {
            out.insert(k, v);
        }
    }
    Value::Object(out)
}

fn mask_value(v: &serde_json::Value) -> String {
    use serde_json::Value;
    let s = match v {
        Value::String(s) => s.clone(),
        Value::Array(arr) => arr
            .first()
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        _ => return "<redacted>".to_string(),
    };
    if s.len() <= 8 {
        return "<redacted>".to_string();
    }
    format!("{}…<redacted>", &s[..8])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_authorization() {
        let headers = serde_json::json!({
            "authorization": "Bearer s3cr3ttok3nvalu3",
            "content-type": "application/json"
        });
        let masked = mask_sensitive_headers(headers);
        assert_eq!(masked["content-type"], "application/json");
        let auth = masked["authorization"].as_str().unwrap();
        assert!(auth.ends_with("<redacted>"));
        assert!(!auth.contains("s3cr3ttok3nvalu3"));
    }

    #[test]
    fn hop_by_hop_list_is_complete() {
        // Sanity: the spec lists these as must-not-replay.
        for h in [
            "connection",
            "keep-alive",
            "proxy-authenticate",
            "proxy-authorization",
            "te",
            "trailer",
            "transfer-encoding",
            "upgrade",
            "host",
            "content-length",
        ] {
            assert!(
                hookrelay_protocol::HOP_BY_HOP_HEADERS.contains(&h),
                "missing {h}"
            );
        }
    }
}
