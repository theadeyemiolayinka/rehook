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

/// Run the agent connection loop. Reconnects with exponential backoff. Does
/// not return unless a fatal error occurs.
pub async fn run(config: AgentConfig, db: Arc<LocalDb>) -> Result<()> {
    let server_url = config
        .server_url
        .clone()
        .ok_or_else(|| anyhow::anyhow!("no server_url configured"))?;
    let agent_id = config
        .agent_id
        .ok_or_else(|| anyhow::anyhow!("no agent_id configured"))?;
    let agent_name = config
        .agent_name
        .clone()
        .unwrap_or_else(|| "agent".to_string());

    let token = credentials::load_token()?;

    let allowlist = Arc::new(config.targets.clone());
    let project_targets = Arc::new(config.project_targets.clone());

    let mut backoff = Duration::from_secs(1);
    let max_backoff = Duration::from_secs(60);

    loop {
        tracing::info!(server = %server_url, "connecting to server");
        match connect_once(
            &server_url,
            agent_id,
            &agent_name,
            &token,
            &allowlist,
            &project_targets,
            &db,
        )
        .await
        {
            Ok(()) => {
                tracing::info!("connection closed cleanly");
                backoff = Duration::from_secs(1);
            }
            Err(e) => {
                tracing::warn!(error = %e, "connection ended");
            }
        }

        tracing::info!(?backoff, "reconnecting after backoff");
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(max_backoff);
    }
}

#[allow(clippy::too_many_arguments)]
async fn connect_once(
    server_url: &str,
    agent_id: Uuid,
    agent_name: &str,
    token: &str,
    allowlist: &Arc<std::collections::HashMap<String, String>>,
    project_targets: &Arc<std::collections::HashMap<String, String>>,
    db: &Arc<LocalDb>,
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
        }
        ServerMessage::Rejected { reason } => {
            return Err(anyhow::anyhow!("server rejected agent: {reason}"));
        }
        _ => {
            return Err(anyhow::anyhow!("unexpected first message"));
        }
    }

    // Subscribe to configured projects.
    for project_id in project_targets.keys() {
        let pid = Uuid::parse_str(project_id).unwrap_or_default();
        let sub = ClientMessage::Subscribe { project_id: pid };
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
                endpoint_id: String::new(),
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
        ServerMessage::SubscriptionConfirmed { project_id } => {
            tracing::info!(%project_id, "subscribed to project");
        }
        ServerMessage::SubscriptionRemoved { project_id } => {
            tracing::info!(%project_id, "unsubscribed from project");
        }
        ServerMessage::Welcome { .. } | ServerMessage::Rejected { .. } => {}
    }
    Ok(())
}
