//! Local HTTP delivery. Reconstructs a safe request from a delivery
//! instruction and sends it to the configured local target.
//!
//! The agent applies its own hop-by-hop header filter again before sending
//! (defense in depth; the server already filters). Response bodies are never
//! sent to the server. Only minimal metadata (status, duration, success) is
//! reported. Response headers and a bounded response body are kept only in
//! the local database for inspection in the agent UI.

use std::time::{Duration, Instant};

use base64::Engine;
use rehook_protocol::{
    is_hop_by_hop, DeliveryErrorCategory, DeliveryInstruction, DeliveryOutcome, ReplayHeader,
};
use reqwest::Method;
use uuid::Uuid;

use crate::targets::resolve_target;

const DELIVERY_TIMEOUT: Duration = Duration::from_secs(30);
/// Cap on the response body stored locally for inspection. The body is
/// never sent to the server.
const RESPONSE_BODY_STORE_LIMIT: usize = 64 * 1024;
/// Cap on stored response headers.
const RESPONSE_HEADERS_LIMIT: usize = 64;

/// Result of a local delivery. `outcome` is the minimal metadata reported
/// to the server; the response detail is stored only in the local database
/// for inspection in the agent UI.
pub struct LocalDelivery {
    pub outcome: DeliveryOutcome,
    /// The target that was actually used (resolved from routes when the
    /// instruction had an empty target_id).
    pub resolved_target_id: String,
    /// Detailed error text for local inspection. May contain the resolved
    /// local URL, which must never be sent to the server.
    pub error_detail: Option<String>,
    /// Response headers as a JSON object, for local inspection only.
    pub response_headers_json: Option<String>,
    /// Response body truncated to RESPONSE_BODY_STORE_LIMIT. Local only.
    pub response_body: Option<Vec<u8>>,
}

/// Perform a local delivery.
pub async fn deliver(
    instruction: &DeliveryInstruction,
    allowlist: &std::collections::HashMap<String, String>,
    routes: &std::collections::HashMap<String, String>,
) -> LocalDelivery {
    let started = Instant::now();

    // Resolve which target to use. The instruction may carry an explicit
    // target id (auto-dispatch, where the server stored the subscribed
    // target) or an empty one (manual replay), in which case we look up our
    // own route for the endpoint.
    let target_id = if instruction.target_id.is_empty() {
        match routes.get(&instruction.endpoint_id.to_string()) {
            Some(t) => t.clone(),
            None => {
                return failure(
                    instruction.delivery_id,
                    started,
                    DeliveryErrorCategory::InvalidTarget,
                    Some(format!(
                        "no route configured for endpoint {}",
                        instruction.endpoint_id
                    )),
                    String::new(),
                );
            }
        }
    } else {
        instruction.target_id.clone()
    };

    // Resolve and validate the target against the local allowlist.
    let url = match resolve_target(allowlist, &target_id) {
        Ok(u) => u,
        Err(e) => {
            return failure(
                instruction.delivery_id,
                started,
                DeliveryErrorCategory::InvalidTarget,
                Some(e.to_string()),
                target_id,
            );
        }
    };

    let method = match Method::from_bytes(instruction.method.as_bytes()) {
        Ok(m) => m,
        Err(e) => {
            return failure(
                instruction.delivery_id,
                started,
                DeliveryErrorCategory::InvalidRequest,
                Some(format!("invalid method: {e}")),
                target_id,
            );
        }
    };

    let client = reqwest::Client::builder()
        .timeout(DELIVERY_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build();

    let client = match client {
        Ok(c) => c,
        Err(e) => {
            return failure(
                instruction.delivery_id,
                started,
                DeliveryErrorCategory::AgentError,
                Some(format!("client build: {e}")),
                target_id,
            );
        }
    };

    let mut req = client.request(method, url.as_str());

    // Apply filtered headers.
    for header in filter_headers(&instruction.headers) {
        req = req.header(header.name.clone(), header.value.clone());
    }

    // Body.
    if let Some(body_b64) = &instruction.body {
        match base64::engine::general_purpose::STANDARD.decode(body_b64) {
            Ok(bytes) => {
                req = req.body(bytes);
            }
            Err(e) => {
                return failure(
                    instruction.delivery_id,
                    started,
                    DeliveryErrorCategory::InvalidRequest,
                    Some(format!("invalid body encoding: {e}")),
                    target_id,
                );
            }
        }
    }

    match req.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let duration_ms = started.elapsed().as_millis() as u64;
            let success = resp.status().is_success();

            // Capture response headers and a bounded body for local
            // inspection. These are never sent to the server.
            let mut headers = serde_json::Map::new();
            for (i, (name, value)) in resp.headers().iter().enumerate() {
                if i >= RESPONSE_HEADERS_LIMIT {
                    break;
                }
                headers.insert(
                    name.to_string(),
                    serde_json::Value::String(value.to_str().unwrap_or("<binary>").to_string()),
                );
            }
            let response_headers_json = serde_json::to_string(&headers).ok();

            let response_body = match resp.bytes().await {
                Ok(b) => {
                    let mut v = b.to_vec();
                    v.truncate(RESPONSE_BODY_STORE_LIMIT);
                    Some(v)
                }
                Err(_) => None,
            };

            LocalDelivery {
                outcome: DeliveryOutcome {
                    delivery_id: instruction.delivery_id,
                    success,
                    status_code: Some(status),
                    duration_ms,
                    error_category: if success {
                        None
                    } else {
                        Some(DeliveryErrorCategory::ResponseError)
                    },
                    error_message: if success {
                        None
                    } else {
                        Some(format!("HTTP {status}"))
                    },
                },
                resolved_target_id: target_id,
                error_detail: if success {
                    None
                } else {
                    Some(format!("HTTP {status}"))
                },
                response_headers_json,
                response_body,
            }
        }
        Err(e) => {
            let category = if e.is_timeout() {
                DeliveryErrorCategory::Timeout
            } else if e.is_connect() {
                DeliveryErrorCategory::ConnectionError
            } else {
                DeliveryErrorCategory::AgentError
            };
            // reqwest errors embed the resolved local URL. Send only a
            // generic message to the server; the detail stays local.
            failure(
                instruction.delivery_id,
                started,
                category,
                Some(e.to_string()),
                target_id,
            )
        }
    }
}

fn filter_headers(headers: &[ReplayHeader]) -> Vec<&ReplayHeader> {
    headers.iter().filter(|h| !is_hop_by_hop(&h.name)).collect()
}

fn failure(
    delivery_id: Uuid,
    started: Instant,
    category: DeliveryErrorCategory,
    detail: Option<String>,
    target_id: String,
) -> LocalDelivery {
    // The server only ever sees a generic label for the category, never a
    // message that could contain the resolved local URL or response data.
    let server_message = match category {
        DeliveryErrorCategory::ConnectionError => "connection to target failed",
        DeliveryErrorCategory::Timeout => "request to target timed out",
        DeliveryErrorCategory::InvalidTarget => "invalid or unknown target",
        DeliveryErrorCategory::InvalidRequest => "invalid delivery instruction",
        DeliveryErrorCategory::ResponseError => "target returned an error",
        DeliveryErrorCategory::AgentError => "agent error",
    };
    LocalDelivery {
        outcome: DeliveryOutcome {
            delivery_id,
            success: false,
            status_code: None,
            duration_ms: started.elapsed().as_millis() as u64,
            error_category: Some(category),
            error_message: Some(server_message.into()),
        },
        resolved_target_id: target_id,
        error_detail: detail.map(|m| m.chars().take(500).collect()),
        response_headers_json: None,
        response_body: None,
    }
}
