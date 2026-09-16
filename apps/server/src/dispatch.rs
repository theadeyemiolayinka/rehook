//! Dispatch captured events to connected agents.
//!
//! When a webhook is captured, the server looks up which agents are
//! subscribed to that endpoint and dispatches a `DeliveryInstruction` to
//! each connected one. When an agent subscribes (or resubscribes after a
//! reconnect), the server catches up on events it missed while offline:
//! any event on that endpoint with no prior delivery attempt for this
//! agent is dispatched.
//!
//! The server never sends URLs. Each delivery references the target
//! identifier the agent declared at subscribe time; the agent resolves it
//! against its own local allowlist. See SECURITY.md.

use std::sync::Arc;

use base64::Engine;
use chrono::Utc;
use rehook_protocol::{is_hop_by_hop, DeliveryInstruction, ReplayHeader};
use uuid::Uuid;

use crate::database::models::EventRow;
use crate::state::AppState;

/// Dispatch a single stored event to a single agent. Creates a deliveries
/// row and sends the instruction over the agent's channel. Returns the
/// delivery id on success.
pub async fn dispatch_event_to_agent(
    state: &Arc<AppState>,
    event: &EventRow,
    agent_id: Uuid,
    target_id: &str,
) -> Result<Uuid, String> {
    let event_id = event.id.clone();
    let endpoint_uuid = Uuid::parse_str(&event.endpoint_id).unwrap_or_default();

    // Compute the next attempt number for this event+agent pair.
    let attempt: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(attempt_number), 0) + 1 FROM deliveries
         WHERE event_id = ? AND agent_id = ?",
    )
    .bind(&event_id)
    .bind(agent_id.to_string())
    .fetch_one(&state.pool)
    .await
    .map_err(|e| e.to_string())?;

    let delivery_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO deliveries (id, event_id, agent_id, target_id, attempt_number, status)
         VALUES (?, ?, ?, ?, ?, 'pending')",
    )
    .bind(delivery_id.to_string())
    .bind(&event_id)
    .bind(agent_id.to_string())
    .bind(target_id)
    .bind(attempt)
    .execute(&state.pool)
    .await
    .map_err(|e| e.to_string())?;

    let headers = build_replay_headers(&event.headers_json);
    let body_b64 = event
        .body
        .as_deref()
        .map(|b| base64::engine::general_purpose::STANDARD.encode(b));

    let instruction = DeliveryInstruction {
        delivery_id,
        event_id: Uuid::parse_str(&event.id).unwrap_or_default(),
        project_id: Uuid::parse_str(&event.project_id).unwrap_or_default(),
        endpoint_id: endpoint_uuid,
        target_id: target_id.to_string(),
        method: event.request_method.clone(),
        content_type: event.content_type.clone(),
        headers,
        body: body_b64,
    };

    match state.agents.dispatch(agent_id, instruction).await {
        Ok(()) => {
            sqlx::query("UPDATE events SET delivery_state = 'dispatched' WHERE id = ?")
                .bind(&event_id)
                .execute(&state.pool)
                .await
                .ok();
            Ok(delivery_id)
        }
        Err(reason) => {
            sqlx::query(
                "UPDATE deliveries SET status = 'failed', error_message = ?, completed_at = ? WHERE id = ?",
            )
            .bind(&reason)
            .bind(Utc::now().to_rfc3339())
            .bind(delivery_id.to_string())
            .execute(&state.pool)
            .await
            .ok();
            Err(reason)
        }
    }
}

/// After an agent subscribes to an endpoint, dispatch any events it has
/// not yet received a delivery for. This handles the "agent was offline"
/// case: events captured while disconnected are delivered on resubscribe.
/// Events that already have a delivery row for this agent (pending,
/// dispatched, delivered, or failed) are skipped to avoid duplicates.
pub async fn catch_up_subscribed_endpoint(
    state: &Arc<AppState>,
    agent_id: Uuid,
    endpoint_id: Uuid,
    target_id: &str,
) {
    let events: Vec<EventRow> = match sqlx::query_as(
        "SELECT e.id, e.project_id, e.endpoint_id, e.request_method, e.content_type,
                e.remote_address, e.received_at, e.payload_size, e.headers_json, e.body,
                e.delivery_state
         FROM events e
         WHERE e.endpoint_id = ?
           AND NOT EXISTS (
             SELECT 1 FROM deliveries d
             WHERE d.event_id = e.id AND d.agent_id = ?
           )
         ORDER BY e.received_at ASC
         LIMIT 200",
    )
    .bind(endpoint_id.to_string())
    .bind(agent_id.to_string())
    .fetch_all(&state.pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = ?e, "catch-up query failed");
            return;
        }
    };

    if events.is_empty() {
        return;
    }

    tracing::info!(
        agent = %agent_id,
        endpoint = %endpoint_id,
        count = events.len(),
        "catching up missed events for agent"
    );

    for event in events {
        if let Err(reason) = dispatch_event_to_agent(state, &event, agent_id, target_id).await {
            tracing::warn!(
                agent = %agent_id,
                event = %event.id,
                reason = %reason,
                "catch-up dispatch failed"
            );
            // If the agent channel is gone, stop trying the rest.
            if reason.contains("not connected") || reason.contains("closed") {
                break;
            }
        }
    }
}

/// Dispatch a freshly captured event to every subscribed, connected
/// agent. Called from the capture handler after the event is stored.
pub async fn dispatch_new_event(state: &Arc<AppState>, event: &EventRow) {
    let subscriptions: Vec<(String, String)> = match sqlx::query_as(
        "SELECT agent_id, target_id FROM agent_subscriptions WHERE endpoint_id = ?",
    )
    .bind(&event.endpoint_id)
    .fetch_all(&state.pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = ?e, "subscription lookup failed");
            return;
        }
    };

    for (agent_id_str, target_id) in subscriptions {
        let Ok(agent_uuid) = Uuid::parse_str(&agent_id_str) else {
            continue;
        };
        if target_id.is_empty() {
            // Legacy subscription without a target; the agent will
            // resubscribe with one.
            continue;
        }
        if !state.agents.is_connected(agent_uuid).await {
            continue;
        }
        match dispatch_event_to_agent(state, event, agent_uuid, &target_id).await {
            Ok(delivery_id) => {
                tracing::info!(
                    delivery = %delivery_id,
                    event = %event.id,
                    agent = %agent_uuid,
                    "event auto-dispatched to agent"
                );
            }
            Err(reason) => {
                tracing::warn!(
                    event = %event.id,
                    agent = %agent_uuid,
                    reason = %reason,
                    "auto-dispatch failed"
                );
            }
        }
    }
}

/// Build the replay header list, filtering hop-by-hop and transport headers.
fn build_replay_headers(headers_json: &str) -> Vec<ReplayHeader> {
    let Ok(headers) = serde_json::from_str::<serde_json::Value>(headers_json) else {
        return Vec::new();
    };
    let serde_json::Value::Object(map) = headers else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (name, value) in map {
        if is_hop_by_hop(&name) {
            continue;
        }
        match value {
            serde_json::Value::String(v) => {
                out.push(ReplayHeader { name, value: v });
            }
            serde_json::Value::Array(arr) => {
                for v in arr {
                    if let serde_json::Value::String(s) = v {
                        out.push(ReplayHeader {
                            name: name.clone(),
                            value: s,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    out
}
