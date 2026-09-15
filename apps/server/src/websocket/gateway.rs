//! WebSocket gateway for local agents at `/agent/ws`.
//!
//! The agent initiates an outbound connection here. The first message must be
//! a `Hello` carrying the agent id and token. The server authenticates the
//! token against the stored hash. The server never sends arbitrary commands;
//! only explicitly defined protocol messages. See SECURITY.md and
//! ARCHITECTURE.md.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, State};
use axum::response::IntoResponse;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use hookrelay_protocol::{
    encode, ClientMessage, DeliveryErrorCategory, DeliveryOutcome, ServerMessage, PROTOCOL_VERSION,
};
use uuid::Uuid;

use crate::api::agents::verify_token;
use crate::state::AppState;
use crate::websocket::registry::GatewayOutbound;

pub async fn gateway(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_connection(state, socket, addr))
}

async fn handle_connection(state: Arc<AppState>, socket: WebSocket, addr: SocketAddr) {
    tracing::info!(%addr, "agent websocket connected");

    let (mut ws_sink, mut ws_stream) = socket.split();

    // Expect the first message to be Hello within a timeout.
    let hello = match tokio::time::timeout(Duration::from_secs(10), ws_stream.next()).await {
        Ok(Some(Ok(msg))) => msg,
        _ => {
            tracing::warn!(%addr, "agent did not send hello in time");
            return;
        }
    };

    let hello_text = match hello {
        Message::Text(t) => t,
        Message::Binary(b) => match String::from_utf8(b.to_vec()) {
            Ok(s) => s,
            Err(_) => return,
        },
        _ => return,
    };

    let ClientMessage::Hello {
        protocol_version,
        agent_id,
        agent_token,
        agent_name,
        platform: _,
        architecture: _,
        version: _,
    } = (match hookrelay_protocol::decode::<ClientMessage>(&hello_text) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(%addr, error = %e, "invalid hello message");
            return;
        }
    })
    else {
        tracing::warn!(%addr, "first message was not hello");
        return;
    };

    if protocol_version != PROTOCOL_VERSION {
        let _ = ws_sink
            .send(Message::Text(
                encode(&ServerMessage::Rejected {
                    reason: format!("unsupported protocol version {protocol_version}"),
                })
                .unwrap(),
            ))
            .await;
        return;
    }

    // agent_id is already a Uuid per the protocol definition.
    let agent_uuid = agent_id;

    let row: Option<(String, bool)> =
        sqlx::query_as("SELECT token_hash, enabled FROM agents WHERE id = ?")
            .bind(agent_id.to_string())
            .fetch_optional(&state.pool)
            .await
            .ok()
            .flatten();

    let Some((token_hash, enabled)) = row else {
        let _ = ws_sink
            .send(Message::Text(
                encode(&ServerMessage::Rejected {
                    reason: "unknown agent".into(),
                })
                .unwrap(),
            ))
            .await;
        return;
    };

    if !enabled {
        let _ = ws_sink
            .send(Message::Text(
                encode(&ServerMessage::Rejected {
                    reason: "agent disabled".into(),
                })
                .unwrap(),
            ))
            .await;
        return;
    }

    let key = match state.config.session_key() {
        Ok(k) => k,
        Err(_) => return,
    };
    if !verify_token(&agent_token, &token_hash, &key) {
        let _ = ws_sink
            .send(Message::Text(
                encode(&ServerMessage::Rejected {
                    reason: "invalid token".into(),
                })
                .unwrap(),
            ))
            .await;
        return;
    }

    // Authenticated. Register in the registry.
    let mut outbound_rx = state.agents.register(agent_uuid).await;

    // Update last seen.
    sqlx::query("UPDATE agents SET last_seen_at = ? WHERE id = ?")
        .bind(Utc::now().to_rfc3339())
        .bind(agent_id.to_string())
        .execute(&state.pool)
        .await
        .ok();

    tracing::info!(agent_id = %agent_id, name = %agent_name, "agent authenticated");

    let _ = ws_sink
        .send(Message::Text(
            encode(&ServerMessage::Welcome {
                protocol_version: PROTOCOL_VERSION,
                server_version: env!("CARGO_PKG_VERSION").to_string(),
            })
            .unwrap(),
        ))
        .await;

    // Main loop: read from client, write outbound instructions.
    let state_clone = state.clone();
    let agent_id_for_task = agent_uuid;
    let mut heartbeat = tokio::time::interval(Duration::from_secs(30));
    heartbeat.tick().await; // first tick is immediate

    loop {
        tokio::select! {
            // Outbound instruction from registry.
            Some(outbound) = outbound_rx.recv() => {
                match outbound {
                    GatewayOutbound::Instruction(instr) => {
                        let text = match encode(&ServerMessage::Deliver(instr)) {
                            Ok(t) => t,
                            Err(_) => continue,
                        };
                        if ws_sink.send(Message::Text(text)).await.is_err() {
                            break;
                        }
                    }
                    GatewayOutbound::Disconnect => {
                        tracing::info!(agent_id = %agent_id, "agent disconnected by registry");
                        break;
                    }
                }
            }
            // Heartbeat ping.
            _ = heartbeat.tick() => {
                if ws_sink.send(Message::Ping(vec![1])).await.is_err() {
                    break;
                }
            }
            // Inbound from agent.
            msg = ws_stream.next() => {
                match msg {
                    Some(Ok(Message::Text(t))) => {
                        if let Err(e) = handle_client_message(&state_clone, agent_id_for_task, &t).await {
                            tracing::warn!(agent_id = %agent_id, error = %e, "client message error");
                        }
                    }
                    Some(Ok(Message::Binary(b))) => {
                        if let Ok(t) = String::from_utf8(b.to_vec()) {
                            if let Err(e) = handle_client_message(&state_clone, agent_id_for_task, &t).await {
                                tracing::warn!(agent_id = %agent_id, error = %e, "client message error");
                            }
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        // update last seen
                        sqlx::query("UPDATE agents SET last_seen_at = ? WHERE id = ?")
                            .bind(Utc::now().to_rfc3339())
                            .bind(agent_id.to_string())
                            .execute(&state_clone.pool)
                            .await
                            .ok();
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(p))) => {
                        let _ = ws_sink.send(Message::Pong(p)).await;
                    }
                    Some(Err(e)) => {
                        tracing::warn!(agent_id = %agent_id, error = %e, "ws read error");
                        break;
                    }
                }
            }
        }
    }

    state.agents.unregister(agent_uuid).await;
    tracing::info!(agent_id = %agent_id, "agent disconnected");
}

async fn handle_client_message(
    state: &Arc<AppState>,
    agent_id: Uuid,
    text: &str,
) -> Result<(), String> {
    let msg: ClientMessage = hookrelay_protocol::decode(text).map_err(|e| e.to_string())?;
    match msg {
        ClientMessage::Hello { .. } => {
            // Hello after authentication is ignored.
        }
        ClientMessage::Authenticated => {
            // informational
        }
        ClientMessage::Heartbeat { ts: _ } => {
            sqlx::query("UPDATE agents SET last_seen_at = ? WHERE id = ?")
                .bind(Utc::now().to_rfc3339())
                .bind(agent_id.to_string())
                .execute(&state.pool)
                .await
                .ok();
        }
        ClientMessage::Subscribe { endpoint_id } => {
            // Persist subscription and register in-memory.
            sqlx::query(
                "INSERT OR IGNORE INTO agent_subscriptions (agent_id, endpoint_id) VALUES (?, ?)",
            )
            .bind(agent_id.to_string())
            .bind(endpoint_id.to_string())
            .execute(&state.pool)
            .await
            .ok();
            state.agents.subscribe(agent_id, endpoint_id).await?;
        }
        ClientMessage::Unsubscribe { endpoint_id } => {
            sqlx::query("DELETE FROM agent_subscriptions WHERE agent_id = ? AND endpoint_id = ?")
                .bind(agent_id.to_string())
                .bind(endpoint_id.to_string())
                .execute(&state.pool)
                .await
                .ok();
            state.agents.unsubscribe(agent_id, endpoint_id).await?;
        }
        ClientMessage::DeliveryAccepted { delivery_id } => {
            sqlx::query("UPDATE deliveries SET status = 'dispatched' WHERE id = ?")
                .bind(delivery_id.to_string())
                .execute(&state.pool)
                .await
                .ok();
        }
        ClientMessage::DeliveryOutcome(outcome) => {
            record_delivery_outcome(state, &outcome).await?;
        }
        ClientMessage::DeliveryRejected {
            delivery_id,
            reason,
        } => {
            sqlx::query(
                "UPDATE deliveries SET status = 'failed', error_category = 'invalid_target', error_message = ? WHERE id = ?",
            )
            .bind(reason)
            .bind(delivery_id.to_string())
            .execute(&state.pool)
            .await
            .ok();
        }
    }
    Ok(())
}

async fn record_delivery_outcome(
    state: &Arc<AppState>,
    outcome: &DeliveryOutcome,
) -> Result<(), String> {
    let status = if outcome.success {
        "delivered"
    } else {
        "failed"
    };
    let error_category = outcome.error_category.map(|c| match c {
        DeliveryErrorCategory::ConnectionError => "connection_error",
        DeliveryErrorCategory::Timeout => "timeout",
        DeliveryErrorCategory::InvalidTarget => "invalid_target",
        DeliveryErrorCategory::InvalidRequest => "invalid_request",
        DeliveryErrorCategory::ResponseError => "response_error",
        DeliveryErrorCategory::AgentError => "agent_error",
    });
    sqlx::query(
        "UPDATE deliveries SET
            status = ?, http_status = ?, duration_ms = ?,
            error_category = ?, error_message = ?, completed_at = ?
         WHERE id = ?",
    )
    .bind(status)
    .bind(outcome.status_code.map(|s| s as i64))
    .bind(outcome.duration_ms as i64)
    .bind(error_category)
    .bind(&outcome.error_message)
    .bind(Utc::now().to_rfc3339())
    .bind(outcome.delivery_id.to_string())
    .execute(&state.pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}
