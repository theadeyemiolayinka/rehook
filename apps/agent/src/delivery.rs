//! Local HTTP delivery. Reconstructs a safe request from a delivery
//! instruction and sends it to the configured local target.
//!
//! The agent applies its own hop-by-hop header filter again before sending
//! (defense in depth; the server already filters). Response bodies are never
//! sent to the server. Only minimal metadata (status, duration, success) is
//! reported.

use std::time::{Duration, Instant};

use base64::Engine;
use hookrelay_protocol::{
    is_hop_by_hop, DeliveryErrorCategory, DeliveryInstruction, DeliveryOutcome, ReplayHeader,
};
use reqwest::Method;
use uuid::Uuid;

use crate::targets::resolve_target;

const DELIVERY_TIMEOUT: Duration = Duration::from_secs(30);

/// Perform a local delivery. Returns the outcome to report to the server.
pub async fn deliver(
    instruction: &DeliveryInstruction,
    allowlist: &std::collections::HashMap<String, String>,
) -> DeliveryOutcome {
    let started = Instant::now();

    // Resolve and validate the target against the local allowlist.
    let url = match resolve_target(allowlist, &instruction.target_id) {
        Ok(u) => u,
        Err(e) => {
            return failure(
                instruction.delivery_id,
                started,
                DeliveryErrorCategory::InvalidTarget,
                e.to_string(),
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
                format!("invalid method: {e}"),
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
                format!("client build: {e}"),
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
                    format!("invalid body encoding: {e}"),
                );
            }
        }
    }

    match req.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let duration_ms = started.elapsed().as_millis() as u64;
            // We do NOT read or transmit the response body.
            let success = resp.status().is_success();
            DeliveryOutcome {
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
            failure(instruction.delivery_id, started, category, e.to_string())
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
    message: String,
) -> DeliveryOutcome {
    DeliveryOutcome {
        delivery_id,
        success: false,
        status_code: None,
        duration_ms: started.elapsed().as_millis() as u64,
        error_category: Some(category),
        // Keep the error message short and non-sensitive.
        error_message: Some(message.chars().take(200).collect()),
    }
}
