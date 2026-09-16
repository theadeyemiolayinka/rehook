//! Local API for the agent web UI.
//!
//! All endpoints serve data from the local agent state. No data is fetched
//! from the public server. Target validation reuses the same security policy
//! as CLI delivery.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use base64::Engine;
use serde::Serialize;
use serde_json::json;

use crate::config::AgentConfig;
use crate::credentials;
use crate::targets;
use crate::web::WebState;

pub fn router() -> Router<WebState> {
    Router::new()
        .route("/status", get(status))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/targets", get(list_targets).post(add_target))
        .route("/targets/:id", patch(update_target).delete(remove_target))
        .route("/routes", get(list_routes).post(add_route))
        .route(
            "/routes/:endpoint_id",
            patch(update_route).delete(remove_route),
        )
        .route("/events", get(list_events))
        .route("/events/:id", get(get_event).delete(delete_event))
        .route("/events/:id/deliveries", get(list_event_deliveries))
        .route("/events/:id/replay", post(replay_event))
        .route("/deliveries", get(list_deliveries))
        .route("/deliveries/:id", get(get_delivery))
        .route("/db/stats", get(db_stats))
        .route("/db/clear", delete(clear_db))
        .route("/projects", get(list_projects))
}

/// Reload config from disk into state so changes from CLI or web UI are
/// reflected.
fn reload_config(_state: &WebState) -> Result<AgentConfig, String> {
    AgentConfig::load().map_err(|e| format!("failed to reload config: {e}"))
}

// --- Auth ---

#[derive(serde::Deserialize)]
struct LoginRequest {
    server: String,
    agent_id: String,
    token: String,
    #[serde(default)]
    name: Option<String>,
}

async fn login(
    State(state): State<WebState>,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if req.server.trim().is_empty() {
        return Err(ApiError::BadRequest("server is required".into()));
    }
    if req.agent_id.trim().is_empty() {
        return Err(ApiError::BadRequest("agent_id is required".into()));
    }
    if req.token.trim().is_empty() {
        return Err(ApiError::BadRequest("token is required".into()));
    }

    // Validate the server is reachable (same check as CLI login).
    let health = format!("{}/healthz", req.server.trim_end_matches('/'));
    let resp = reqwest::Client::new()
        .get(&health)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| ApiError::BadRequest(format!("could not reach server: {e}")))?;

    if !resp.status().is_success() {
        return Err(ApiError::BadRequest(format!(
            "server health check failed: {}",
            resp.status()
        )));
    }

    let agent_uuid = uuid::Uuid::parse_str(req.agent_id.trim())
        .map_err(|e| ApiError::BadRequest(format!("invalid agent_id: {e}")))?;

    let name = req.name.unwrap_or_else(|| "default".to_string());

    // Store the token securely (same path as CLI).
    credentials::store_token(agent_uuid, req.token.trim())
        .map_err(|e| ApiError::Internal(format!("failed to store token: {e}")))?;

    // Cache the token in memory so the connection loop does not need
    // to hit the OS keychain (which prompts on macOS).
    state
        .conn_state
        .set_token(req.token.trim().to_string())
        .await;

    // Update config.
    let mut config = reload_config(&state)?;
    config.server_url = Some(req.server.trim().to_string());
    config.agent_id = Some(agent_uuid);
    config.agent_name = Some(name);
    config
        .save()
        .map_err(|e| ApiError::Internal(format!("failed to save config: {e}")))?;

    tracing::info!(server = %req.server, agent_id = %agent_uuid, "agent logged in via web UI");

    Ok((
        StatusCode::OK,
        Json(json!({
            "ok": true,
            "agent_id": agent_uuid.to_string(),
            "server": req.server.trim(),
        })),
    ))
}

async fn logout(State(state): State<WebState>) -> Result<impl IntoResponse, ApiError> {
    credentials::delete_token().map_err(|e| ApiError::Internal(e.to_string()))?;

    // Clear the cached token so the connection loop stops trying.
    state.conn_state.clear_token().await;

    let mut config = reload_config(&state)?;
    config.agent_id = None;
    config.server_url = None;
    config.agent_name = None;
    config
        .save()
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    tracing::info!("agent logged out via web UI");

    Ok((StatusCode::OK, Json(json!({ "ok": true }))))
}

// --- Status ---

#[derive(Serialize)]
struct ConnectionStatusResponse {
    server_url: Option<String>,
    agent_id: Option<String>,
    agent_name: Option<String>,
    authenticated: bool,
    connected: bool,
    last_connected_at: Option<String>,
    subscribed_endpoints: Vec<String>,
}

async fn status(State(state): State<WebState>) -> impl IntoResponse {
    // Reload config to get the latest state (may have changed via CLI).
    let config = reload_config(&state).unwrap_or_else(|_| state.config.clone());

    let authenticated = config.agent_id.is_some();
    let connected = state.conn_state.is_connected();
    let last_connected_at = state.conn_state.last_connected().await;
    let status = ConnectionStatusResponse {
        server_url: config.server_url.clone(),
        agent_id: config.agent_id.map(|u| u.to_string()),
        agent_name: config.agent_name.clone(),
        authenticated,
        connected,
        last_connected_at,
        subscribed_endpoints: config.endpoint_targets.keys().cloned().collect(),
    };
    Json(json!(status))
}

// --- Targets ---

#[derive(Serialize)]
struct TargetResponse {
    id: String,
    url: String,
}

async fn list_targets(State(state): State<WebState>) -> impl IntoResponse {
    let config = reload_config(&state).unwrap_or_else(|_| state.config.clone());
    let targets: Vec<TargetResponse> = config
        .targets
        .iter()
        .map(|(id, url)| TargetResponse {
            id: id.clone(),
            url: url.clone(),
        })
        .collect();
    Json(json!({ "targets": targets }))
}

#[derive(serde::Deserialize)]
struct AddTargetRequest {
    id: String,
    url: String,
}

async fn add_target(
    State(state): State<WebState>,
    Json(req): Json<AddTargetRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if req.id.trim().is_empty() {
        return Err(ApiError::BadRequest("id is required".into()));
    }
    targets::validate_url(&req.url).map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let mut config = reload_config(&state)?;
    config
        .targets
        .insert(req.id.trim().to_string(), req.url.trim().to_string());
    config
        .save()
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(json!({ "ok": true }))))
}

#[derive(serde::Deserialize)]
struct UpdateTargetRequest {
    url: String,
}

async fn update_target(
    State(state): State<WebState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateTargetRequest>,
) -> Result<impl IntoResponse, ApiError> {
    targets::validate_url(&req.url).map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let mut config = reload_config(&state)?;
    if !config.targets.contains_key(&id) {
        return Err(ApiError::NotFound);
    }
    config
        .targets
        .insert(id.clone(), req.url.trim().to_string());
    config
        .save()
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    tracing::info!(target = %id, url = %req.url, "target updated via web UI");

    Ok(Json(json!({ "ok": true, "id": id, "url": req.url.trim() })))
}

async fn remove_target(
    State(state): State<WebState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let mut config = reload_config(&state)?;
    if config.targets.remove(&id).is_none() {
        return Err(ApiError::NotFound);
    }
    // Also remove any routes pointing to this target.
    config.endpoint_targets.retain(|_, tid| tid != &id);
    config
        .save()
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

// --- Routes ---

#[derive(Serialize)]
struct RouteResponse {
    endpoint_id: String,
    target_id: String,
}

async fn list_routes(State(state): State<WebState>) -> impl IntoResponse {
    let config = reload_config(&state).unwrap_or_else(|_| state.config.clone());
    let routes: Vec<RouteResponse> = config
        .endpoint_targets
        .iter()
        .map(|(endpoint_id, target_id)| RouteResponse {
            endpoint_id: endpoint_id.clone(),
            target_id: target_id.clone(),
        })
        .collect();
    Json(json!({ "routes": routes }))
}

#[derive(serde::Deserialize)]
struct AddRouteRequest {
    endpoint_id: String,
    target_id: String,
}

async fn add_route(
    State(state): State<WebState>,
    Json(req): Json<AddRouteRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if req.endpoint_id.trim().is_empty() {
        return Err(ApiError::BadRequest("endpoint_id is required".into()));
    }
    if req.target_id.trim().is_empty() {
        return Err(ApiError::BadRequest("target_id is required".into()));
    }

    let mut config = reload_config(&state)?;

    // Validate the target exists.
    if !config.targets.contains_key(req.target_id.trim()) {
        return Err(ApiError::BadRequest(format!(
            "unknown target '{}'; add it first",
            req.target_id.trim()
        )));
    }

    // Validate the endpoint_id is a valid UUID.
    uuid::Uuid::parse_str(req.endpoint_id.trim())
        .map_err(|e| ApiError::BadRequest(format!("invalid endpoint_id: {e}")))?;

    config.endpoint_targets.insert(
        req.endpoint_id.trim().to_string(),
        req.target_id.trim().to_string(),
    );
    config
        .save()
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // If connected, send a live Subscribe so the server updates its
    // subscription without waiting for a reconnect.
    if let Ok(eid) = uuid::Uuid::parse_str(req.endpoint_id.trim()) {
        state
            .conn_state
            .send_outbound(hookrelay_protocol::ClientMessage::Subscribe {
                endpoint_id: eid,
                target_id: req.target_id.trim().to_string(),
            });
    }

    tracing::info!(
        endpoint = %req.endpoint_id,
        target = %req.target_id,
        "route added via web UI"
    );

    Ok((StatusCode::CREATED, Json(json!({ "ok": true }))))
}

#[derive(serde::Deserialize)]
struct UpdateRouteRequest {
    target_id: String,
}

async fn update_route(
    State(state): State<WebState>,
    Path(endpoint_id): Path<String>,
    Json(req): Json<UpdateRouteRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if req.target_id.trim().is_empty() {
        return Err(ApiError::BadRequest("target_id is required".into()));
    }

    let mut config = reload_config(&state)?;

    if !config.endpoint_targets.contains_key(&endpoint_id) {
        return Err(ApiError::NotFound);
    }
    if !config.targets.contains_key(req.target_id.trim()) {
        return Err(ApiError::BadRequest(format!(
            "unknown target '{}'; add it first",
            req.target_id.trim()
        )));
    }

    config
        .endpoint_targets
        .insert(endpoint_id.clone(), req.target_id.trim().to_string());
    config
        .save()
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Re-subscribe live so the server learns the new target.
    if let Ok(eid) = uuid::Uuid::parse_str(&endpoint_id) {
        state
            .conn_state
            .send_outbound(hookrelay_protocol::ClientMessage::Subscribe {
                endpoint_id: eid,
                target_id: req.target_id.trim().to_string(),
            });
    }

    tracing::info!(
        endpoint = %endpoint_id,
        target = %req.target_id,
        "route updated via web UI"
    );

    Ok(Json(json!({ "ok": true })))
}

async fn remove_route(
    State(state): State<WebState>,
    Path(endpoint_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let mut config = reload_config(&state)?;
    if config.endpoint_targets.remove(&endpoint_id).is_none() {
        return Err(ApiError::NotFound);
    }
    config
        .save()
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    if let Ok(eid) = uuid::Uuid::parse_str(&endpoint_id) {
        state
            .conn_state
            .send_outbound(hookrelay_protocol::ClientMessage::Unsubscribe { endpoint_id: eid });
    }

    Ok(StatusCode::NO_CONTENT)
}

// --- Events ---

async fn list_events(State(state): State<WebState>) -> impl IntoResponse {
    let events = state.db.recent_events(200).await.unwrap_or_default();
    let rows: Vec<serde_json::Value> = events
        .iter()
        .map(|e| {
            json!({
                "id": e.id,
                "project_id": e.project_id,
                "endpoint_id": e.endpoint_id,
                "request_method": e.request_method,
                "content_type": e.content_type,
                "received_at": e.received_at,
                "payload_size": e.payload_size,
                "headers": serde_json::from_str::<serde_json::Value>(&e.headers_json).unwrap_or(serde_json::Value::Null),
                "body": e.body.as_deref().map(|b| base64::engine::general_purpose::STANDARD.encode(b)),
            })
        })
        .collect();
    Json(json!({ "events": rows }))
}

async fn get_event(
    State(state): State<WebState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let event = state
        .db
        .get_event(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let event = event.ok_or(ApiError::NotFound)?;
    Ok(Json(json!({
        "id": event.id,
        "project_id": event.project_id,
        "endpoint_id": event.endpoint_id,
        "request_method": event.request_method,
        "content_type": event.content_type,
        "received_at": event.received_at,
        "payload_size": event.payload_size,
        "headers": serde_json::from_str::<serde_json::Value>(&event.headers_json).unwrap_or(serde_json::Value::Null),
        "body": event.body.as_deref().map(|b| base64::engine::general_purpose::STANDARD.encode(b)),
    })))
}

async fn delete_event(
    State(state): State<WebState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let deleted = state
        .db
        .delete_event(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    if !deleted {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
struct ReplayRequest {
    target_id: String,
}

async fn replay_event(
    State(state): State<WebState>,
    Path(event_id): Path<String>,
    Json(req): Json<ReplayRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let event = state
        .db
        .get_event(&event_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let event = event.ok_or(ApiError::NotFound)?;

    // Validate the target using the same security policy as CLI delivery.
    let config = reload_config(&state)?;
    let url = targets::resolve_target(&config.targets, &req.target_id)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    // Build the delivery instruction from the stored event.
    let headers: Vec<hookrelay_protocol::ReplayHeader> =
        serde_json::from_str::<serde_json::Value>(&event.headers_json)
            .ok()
            .and_then(|v| match v {
                serde_json::Value::Object(map) => {
                    let mut out = Vec::new();
                    for (name, value) in map {
                        if hookrelay_protocol::is_hop_by_hop(&name) {
                            continue;
                        }
                        match value {
                            serde_json::Value::String(s) => {
                                out.push(hookrelay_protocol::ReplayHeader { name, value: s })
                            }
                            serde_json::Value::Array(arr) => {
                                for v in arr {
                                    if let serde_json::Value::String(s) = v {
                                        out.push(hookrelay_protocol::ReplayHeader {
                                            name: name.clone(),
                                            value: s,
                                        });
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    Some(out)
                }
                _ => None,
            })
            .unwrap_or_default();

    let instruction = hookrelay_protocol::DeliveryInstruction {
        delivery_id: uuid::Uuid::new_v4(),
        event_id: uuid::Uuid::parse_str(&event.id).unwrap_or_default(),
        project_id: uuid::Uuid::parse_str(&event.project_id).unwrap_or_default(),
        endpoint_id: uuid::Uuid::parse_str(&event.endpoint_id).unwrap_or_default(),
        target_id: req.target_id.clone(),
        method: event.request_method.clone(),
        content_type: event.content_type.clone(),
        headers,
        body: event
            .body
            .as_deref()
            .map(|b| base64::engine::general_purpose::STANDARD.encode(b)),
    };

    // Perform the local delivery using the same code path as live delivery.
    let result =
        crate::delivery::deliver(&instruction, &config.targets, &config.endpoint_targets).await;

    // Record locally, including bounded response detail for the web UI.
    let rec = crate::db::DeliveryRecord {
        id: instruction.delivery_id.to_string(),
        event_id: event.id.clone(),
        target_id: result.resolved_target_id.clone(),
        attempt_number: 0,
        status: if result.outcome.success {
            "delivered".into()
        } else {
            "failed".into()
        },
        http_status: result.outcome.status_code.map(|s| s as i64),
        duration_ms: Some(result.outcome.duration_ms as i64),
        error_category: result
            .outcome
            .error_category
            .map(|c| format!("{c:?}").to_lowercase()),
        error_message: result
            .error_detail
            .clone()
            .or_else(|| result.outcome.error_message.clone()),
        response_headers_json: result.response_headers_json.clone(),
        response_body: result.response_body.clone(),
        started_at: chrono::Utc::now().to_rfc3339(),
        completed_at: Some(chrono::Utc::now().to_rfc3339()),
    };
    let _ = state.db.record(&rec).await;

    tracing::info!(
        event_id = %event_id,
        target = %url,
        success = result.outcome.success,
        status = ?result.outcome.status_code,
        "local replay from web UI"
    );

    Ok((
        StatusCode::OK,
        Json(json!({
            "success": result.outcome.success,
            "status_code": result.outcome.status_code,
            "duration_ms": result.outcome.duration_ms,
            "error": result.outcome.error_message,
        })),
    ))
}

// --- Deliveries ---

fn delivery_json(r: &crate::db::DeliveryRecord) -> serde_json::Value {
    json!({
        "id": r.id,
        "event_id": r.event_id,
        "target_id": r.target_id,
        "attempt_number": r.attempt_number,
        "status": r.status,
        "http_status": r.http_status,
        "duration_ms": r.duration_ms,
        "error_category": r.error_category,
        "error_message": r.error_message,
        "response_headers": r.response_headers_json
            .as_deref()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()),
        "response_body": r.response_body
            .as_deref()
            .map(|b| base64::engine::general_purpose::STANDARD.encode(b)),
        "started_at": r.started_at,
        "completed_at": r.completed_at,
    })
}

async fn list_deliveries(State(state): State<WebState>) -> impl IntoResponse {
    let records = state.db.recent_deliveries(200).await.unwrap_or_default();
    let rows: Vec<serde_json::Value> = records.iter().map(delivery_json).collect();
    Json(json!({ "deliveries": rows }))
}

async fn list_event_deliveries(
    State(state): State<WebState>,
    Path(event_id): Path<String>,
) -> impl IntoResponse {
    let records = state
        .db
        .deliveries_for_event(&event_id)
        .await
        .unwrap_or_default();
    let rows: Vec<serde_json::Value> = records.iter().map(delivery_json).collect();
    Json(json!({ "deliveries": rows }))
}

async fn get_delivery(
    State(state): State<WebState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let r = state
        .db
        .get_delivery(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let r = r.ok_or(ApiError::NotFound)?;
    Ok(Json(delivery_json(&r)))
}

// --- DB management ---

#[derive(Serialize)]
struct DbStatsResponse {
    deliveries_count: i64,
    events_count: i64,
    db_size_bytes: i64,
}

async fn db_stats(State(state): State<WebState>) -> impl IntoResponse {
    let deliveries_count = state.db.delivery_count().await.unwrap_or(0);
    let events_count = state.db.event_count().await.unwrap_or(0);
    let db_size_bytes = state.db.db_size().await.unwrap_or(0);
    Json(json!(DbStatsResponse {
        deliveries_count,
        events_count,
        db_size_bytes,
    }))
}

async fn clear_db(State(state): State<WebState>) -> impl IntoResponse {
    match state.db.clear_all().await {
        Ok(()) => (StatusCode::OK, Json(json!({ "ok": true }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

// --- Project lookup ---

/// Fetch project names from the server so the agent UI can display
/// human-readable names instead of raw UUIDs. Uses the stored agent
/// token for authentication.
async fn list_projects(State(state): State<WebState>) -> axum::response::Response {
    let config = reload_config(&state).unwrap_or_else(|_| state.config.clone());

    let server_url = match config.server_url.as_deref() {
        Some(s) => s.trim_end_matches('/').to_string(),
        None => {
            return Json(json!({ "projects": Vec::<serde_json::Value>::new() })).into_response();
        }
    };

    let token = match state.conn_state.token().await {
        Some(t) => t,
        None => match credentials::load_token() {
            Ok(t) => {
                state.conn_state.set_token(t.clone()).await;
                t
            }
            Err(_) => {
                return Json(json!({ "projects": Vec::<serde_json::Value>::new() }))
                    .into_response();
            }
        },
    };

    let url = format!("{server_url}/agent/projects");
    let resp = reqwest::Client::new()
        .get(&url)
        .header("authorization", format!("Bearer {token}"))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            let body = r.text().await.unwrap_or_default();
            let parsed: serde_json::Value =
                serde_json::from_str(&body).unwrap_or(json!({ "projects": [] }));
            Json(parsed).into_response()
        }
        _ => Json(json!({ "projects": Vec::<serde_json::Value>::new() })).into_response(),
    }
}

// --- Error type ---

enum ApiError {
    BadRequest(String),
    NotFound,
    Internal(String),
}

impl From<String> for ApiError {
    fn from(s: String) -> Self {
        ApiError::Internal(s)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, msg) = match self {
            ApiError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "not found".into()),
            ApiError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m),
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}
