//! Outbound WebSocket connection to the server with reconnection.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use hookrelay_protocol::{encode, ClientMessage, ServerMessage, PROTOCOL_VERSION};
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

use crate::config::AgentConfig;
use crate::credentials;
use crate::db::{DeliveryRecord, LocalDb};
use crate::delivery;
use crate::state::ConnectionState;

/// Run the agent connection loop. Reconnects with exponential backoff.
/// Reloads config on each attempt so changes from the web UI or CLI
/// are picked up without restarting. Does not return unless a fatal
/// error occurs.
///
/// The token is loaded from the keychain once and cached in memory.
/// Subsequent reconnects use the cached token so the OS keychain is
/// not prompted repeatedly (which on macOS would ask for a password
/// each time).
pub async fn run(db: Arc<LocalDb>, conn_state: ConnectionState) -> Result<()> {
    let mut backoff = Duration::from_secs(1);
    let max_backoff = Duration::from_secs(60);
    // Longer wait when not logged in yet. Avoids hammering the server
    // and the keychain before the user has configured credentials.
    let not_configured_wait = Duration::from_secs(5);
    // When the server explicitly rejects the agent (bad token, unknown
    // agent), this is a configuration problem, not a transient failure.
    // Back off much longer to avoid keychain prompts and log spam.
    let rejected_wait = Duration::from_secs(30);

    loop {
        let config = match AgentConfig::load() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "could not load config");
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(max_backoff);
                continue;
            }
        };

        let server_url = match config.server_url.as_deref() {
            Some(s) => s.to_string(),
            None => {
                tracing::info!("no server_url configured; waiting for login");
                conn_state.set_connected(false);
                // Wait for a credential change notification or timeout.
                tokio::select! {
                    _ = conn_state.wait_for_change() => {}
                    _ = tokio::time::sleep(not_configured_wait) => {}
                }
                backoff = Duration::from_secs(1);
                continue;
            }
        };

        let agent_id = match config.agent_id {
            Some(id) => id,
            None => {
                tracing::info!("no agent_id configured; waiting for login");
                conn_state.set_connected(false);
                tokio::select! {
                    _ = conn_state.wait_for_change() => {}
                    _ = tokio::time::sleep(not_configured_wait) => {}
                }
                backoff = Duration::from_secs(1);
                continue;
            }
        };

        // Use cached token if available. Only hit the keychain if the
        // cache is empty (first run or after logout).
        let token = match conn_state.token().await {
            Some(t) => t,
            None => match credentials::load_token() {
                Ok(t) => {
                    conn_state.set_token(t.clone()).await;
                    t
                }
                Err(_) => {
                    tracing::info!("no token stored; waiting for login");
                    conn_state.set_connected(false);
                    tokio::select! {
                        _ = conn_state.wait_for_change() => {}
                        _ = tokio::time::sleep(not_configured_wait) => {}
                    }
                    backoff = Duration::from_secs(1);
                    continue;
                }
            },
        };

        let agent_name = config
            .agent_name
            .clone()
            .unwrap_or_else(|| "agent".to_string());

        let allowlist = Arc::new(config.targets.clone());
        let endpoint_targets = Arc::new(config.endpoint_targets.clone());

        tracing::info!(server = %server_url, "connecting to server");
        match connect_once(
            &server_url,
            agent_id,
            &agent_name,
            &token,
            &allowlist,
            &endpoint_targets,
            &db,
            &conn_state,
        )
        .await
        {
            Ok(()) => {
                tracing::info!("connection closed cleanly");
                backoff = Duration::from_secs(1);
            }
            Err(e) => {
                let err_str = e.to_string();
                tracing::warn!(error = %e, "connection ended");
                // If the server explicitly rejected the agent, the token
                // or agent_id is wrong. Back off much longer to avoid
                // keychain prompts and log spam. The user needs to re-login.
                if err_str.contains("server rejected agent") {
                    tracing::warn!(
                        "server rejected agent; backing off for 30s. Re-login if needed."
                    );
                    conn_state.set_connected(false);
                    tokio::select! {
                        _ = conn_state.wait_for_change() => {
                            backoff = Duration::from_secs(1);
                        }
                        _ = tokio::time::sleep(rejected_wait) => {
                            backoff = Duration::from_secs(1);
                        }
                    }
                    continue;
                }
            }
        }

        conn_state.set_connected(false);
        tracing::info!(?backoff, "reconnecting after backoff");
        tokio::select! {
            _ = conn_state.wait_for_change() => {
                backoff = Duration::from_secs(1);
            }
            _ = tokio::time::sleep(backoff) => {
                backoff = (backoff * 2).min(max_backoff);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn connect_once(
    server_url: &str,
    agent_id: Uuid,
    agent_name: &str,
    token: &str,
    allowlist: &Arc<std::collections::HashMap<String, String>>,
    endpoint_targets: &Arc<std::collections::HashMap<String, String>>,
    db: &Arc<LocalDb>,
    conn_state: &ConnectionState,
) -> Result<()> {
    // Build the WebSocket URL from the server URL.
    let ws_url = server_url
        .trim_end_matches('/')
        .replace("http://", "ws://")
        .replace("https://", "wss://")
        + "/agent/ws";

    let (mut ws_sink, mut ws_stream) = tokio_tungstenite::connect_async(&ws_url).await?.0.split();

    // Send Hello.
    let hello = ClientMessage::Hello {
        protocol_version: PROTOCOL_VERSION,
        agent_id,
        agent_token: token.to_string(),
        agent_name: agent_name.to_string(),
        platform: std::env::consts::OS.to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    ws_sink.send(Message::Text(encode(&hello)?)).await?;

    // Wait for Welcome or Rejected.
    let first = match ws_stream.next().await {
        Some(Ok(Message::Text(t))) => t,
        Some(Ok(Message::Binary(b))) => String::from_utf8(b.to_vec()).unwrap_or_default(),
        _ => return Err(anyhow::anyhow!("no welcome message")),
    };

    match hookrelay_protocol::decode::<ServerMessage>(&first)? {
        ServerMessage::Welcome { .. } => {
            tracing::info!("authenticated with server");
            conn_state.set_connected(true);
            conn_state
                .set_last_connected(chrono::Utc::now().to_rfc3339())
                .await;
        }
        ServerMessage::Rejected { reason } => {
            return Err(anyhow::anyhow!("server rejected agent: {reason}"));
        }
        _ => {
            return Err(anyhow::anyhow!("unexpected first message"));
        }
    }

    // Subscribe to configured endpoints.
    for endpoint_id in endpoint_targets.keys() {
        let eid = Uuid::parse_str(endpoint_id).unwrap_or_default();
        let sub = ClientMessage::Subscribe { endpoint_id: eid };
        let _ = ws_sink.send(Message::Text(encode(&sub)?)).await;
    }

    let mut heartbeat = tokio::time::interval(Duration::from_secs(20));
    heartbeat.tick().await;

    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                let hb = ClientMessage::Heartbeat { ts: chrono::Utc::now() };
                if ws_sink.send(Message::Text(encode(&hb)?)).await.is_err() {
                    break;
                }
            }
            msg = ws_stream.next() => {
                match msg {
                    Some(Ok(Message::Text(t))) => {
                        if let Err(e) = handle_server_message(
                            &t, allowlist, db, &mut ws_sink,
                        ).await {
                            tracing::warn!(error = %e, "server message error");
                        }
                    }
                    Some(Ok(Message::Binary(b))) => {
                        if let Ok(t) = String::from_utf8(b.to_vec()) {
                            if let Err(e) = handle_server_message(
                                &t, allowlist, db, &mut ws_sink,
                            ).await {
                                tracing::warn!(error = %e, "server message error");
                            }
                        }
                    }
                    Some(Ok(Message::Ping(p))) => {
                        let _ = ws_sink.send(Message::Pong(p)).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        tracing::warn!(error = %e, "ws read error");
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

async fn handle_server_message(
    text: &str,
    allowlist: &Arc<std::collections::HashMap<String, String>>,
    db: &Arc<LocalDb>,
    ws_sink: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
) -> Result<()> {
    let msg: ServerMessage = hookrelay_protocol::decode(text)?;
    match msg {
        ServerMessage::Deliver(instruction) => {
            tracing::info!(
                delivery_id = %instruction.delivery_id,
                event_id = %instruction.event_id,
                target_id = %instruction.target_id,
                "received delivery instruction"
            );

            // Acknowledge receipt.
            let ack = ClientMessage::DeliveryAccepted {
                delivery_id: instruction.delivery_id,
            };
            let _ = ws_sink.send(Message::Text(encode(&ack)?)).await;

            // Store the event locally for offline inspection and replay.
            let headers_json = serde_json::to_string(&instruction.headers.iter().fold(
                serde_json::Map::new(),
                |mut map, h| {
                    map.entry(h.name.clone())
                        .or_insert_with(|| serde_json::Value::Array(vec![]))
                        .as_array_mut()
                        .unwrap()
                        .push(serde_json::Value::String(h.value.clone()));
                    map
                },
            ))
            .unwrap_or_else(|_| "{}".into());

            let body_bytes = instruction.body.as_deref().and_then(|b| {
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b).ok()
            });

            let event = crate::db::StoredEvent {
                id: instruction.event_id.to_string(),
                project_id: instruction.project_id.to_string(),
                endpoint_id: instruction.endpoint_id.to_string(),
                request_method: instruction.method.clone(),
                content_type: instruction.content_type.clone(),
                received_at: chrono::Utc::now().to_rfc3339(),
                payload_size: body_bytes.as_ref().map(|b| b.len() as i64).unwrap_or(0),
                headers_json,
                body: body_bytes,
            };
            if let Err(e) = db.store_event(&event).await {
                tracing::warn!(error = %e, "failed to store event locally");
            }

            // Perform the local delivery.
            let outcome = delivery::deliver(&instruction, allowlist).await;

            // Record locally.
            let rec = DeliveryRecord {
                id: instruction.delivery_id.to_string(),
                event_id: instruction.event_id.to_string(),
                target_id: instruction.target_id.clone(),
                attempt_number: 0,
                status: if outcome.success {
                    "delivered".into()
                } else {
                    "failed".into()
                },
                http_status: outcome.status_code.map(|s| s as i64),
                duration_ms: Some(outcome.duration_ms as i64),
                error_category: outcome
                    .error_category
                    .map(|c| format!("{c:?}").to_lowercase()),
                error_message: outcome.error_message.clone(),
                started_at: chrono::Utc::now().to_rfc3339(),
                completed_at: Some(chrono::Utc::now().to_rfc3339()),
            };
            if let Err(e) = db.record(&rec).await {
                tracing::warn!(error = %e, "failed to record delivery locally");
            }

            // Report outcome to the server (minimal metadata only).
            let report = ClientMessage::DeliveryOutcome(outcome);
            let _ = ws_sink.send(Message::Text(encode(&report)?)).await;
        }
        ServerMessage::HeartbeatAck { .. } => {}
        ServerMessage::SubscriptionConfirmed { endpoint_id } => {
            tracing::info!(%endpoint_id, "subscribed to endpoint");
        }
        ServerMessage::SubscriptionRemoved { endpoint_id } => {
            tracing::info!(%endpoint_id, "unsubscribed from endpoint");
        }
        ServerMessage::Welcome { .. } | ServerMessage::Rejected { .. } => {}
    }
    Ok(())
}
