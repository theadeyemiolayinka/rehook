//! Shared, strongly typed protocol used over the agent WebSocket connection.
//!
//! The protocol only supports explicitly defined message types. It is not a
//! generic command execution protocol. The server can never instruct an agent
//! to execute shell commands or arbitrary code, and can never instruct an
//! agent to fetch an arbitrary URL. Delivery instructions always reference a
//! configured target identifier that the agent resolves to a locally
//! configured, allowlisted destination.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Protocol version. Bumped on incompatible changes. The server rejects
/// agents advertising an unsupported major version.
pub const PROTOCOL_VERSION: u32 = 3;

/// Error category reported by the agent when a delivery fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryErrorCategory {
    ConnectionError,
    Timeout,
    InvalidTarget,
    InvalidRequest,
    ResponseError,
    AgentError,
}

/// Minimal delivery outcome the agent reports back to the server.
///
/// By design this carries only operational metadata. Response bodies and
/// arbitrary response headers are never included. See SECURITY.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryOutcome {
    pub delivery_id: Uuid,
    pub success: bool,
    pub status_code: Option<u16>,
    pub duration_ms: u64,
    pub error_category: Option<DeliveryErrorCategory>,
    /// Free-form short error string. Kept short and non-sensitive. Never
    /// includes response bodies or stack traces.
    pub error_message: Option<String>,
}

/// A single header to replay. The agent applies a hop-by-hop and
/// transport-header filter before sending these.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayHeader {
    pub name: String,
    pub value: String,
}

/// A delivery instruction. The server sends this to an agent. The agent
/// resolves `target_id` to a locally configured destination and validates
/// the instruction before performing any request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryInstruction {
    /// Unique id for this delivery attempt. Used for idempotency and ack.
    pub delivery_id: Uuid,
    /// The captured event being replayed.
    pub event_id: Uuid,
    /// The project the event belongs to.
    pub project_id: Uuid,
    /// The endpoint the event was received on. Used by the agent to
    /// resolve the correct local target when endpoint-level routing is
    /// configured.
    pub endpoint_id: Uuid,
    /// Identifier of the locally configured target. The agent resolves this
    /// to a URL from its own allowlist. The server never sends a URL.
    pub target_id: String,
    /// HTTP method to replay.
    pub method: String,
    /// Content-Type of the original request body, if any.
    pub content_type: Option<String>,
    /// Headers to replay (already filtered by the server). The agent filters
    /// again before sending.
    pub headers: Vec<ReplayHeader>,
    /// Raw request body bytes (base64-encoded in JSON).
    pub body: Option<String>,
}

/// Messages sent from the agent to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// First message after connecting. Declares protocol version and agent
    /// identity.
    Hello {
        protocol_version: u32,
        agent_id: Uuid,
        agent_token: String,
        agent_name: String,
        platform: String,
        architecture: String,
        version: String,
    },
    /// Acknowledges a successful authentication.
    Authenticated,
    /// Heartbeat to keep the connection alive.
    Heartbeat { ts: chrono::DateTime<chrono::Utc> },
    /// Subscribe to an endpoint. The agent will only receive delivery
    /// instructions for subscribed endpoints. `target_id` declares which
    /// locally configured target the agent wants deliveries for this
    /// endpoint routed to. The server stores it so new captured events
    /// can be dispatched automatically.
    Subscribe {
        endpoint_id: Uuid,
        target_id: String,
    },
    /// Unsubscribe from an endpoint.
    Unsubscribe { endpoint_id: Uuid },
    /// Acknowledges a delivery instruction was received.
    DeliveryAccepted { delivery_id: Uuid },
    /// Reports the outcome of a delivery attempt.
    DeliveryOutcome(DeliveryOutcome),
    /// Rejects a delivery instruction (e.g. unknown target, unsupported
    /// scheme, not subscribed to endpoint).
    DeliveryRejected { delivery_id: Uuid, reason: String },
}

/// Messages sent from the server to the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Server accepts the agent after a valid Hello.
    Welcome {
        protocol_version: u32,
        server_version: String,
    },
    /// Server rejects the agent (bad token, revoked, protocol mismatch).
    Rejected {
        reason: String,
    },
    HeartbeatAck {
        ts: chrono::DateTime<chrono::Utc>,
    },
    SubscriptionConfirmed {
        endpoint_id: Uuid,
    },
    SubscriptionRemoved {
        endpoint_id: Uuid,
    },
    /// Instructs the agent to deliver an event to a configured local target.
    Deliver(DeliveryInstruction),
}

/// (De)serialization helpers.
pub fn encode<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string(value)
}

pub fn decode<T: for<'de> Deserialize<'de>>(text: &str) -> Result<T, serde_json::Error> {
    serde_json::from_str(text)
}

/// Headers that are hop-by-hop or transport-negotiation headers and must
/// never be replayed blindly. They are still captured for inspection, but
/// filtered out at replay time by both the server and the agent (defense
/// in depth). `accept-encoding` is included because the original client's
/// encoding preferences are meaningless to a replay; the agent negotiates
/// its own and decodes the response before storing it.
pub const HOP_BY_HOP_HEADERS: &[&str] = &[
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
    "accept-encoding",
];

/// Returns true if a header name should be filtered out before replaying.
pub fn is_hop_by_hop(name: &str) -> bool {
    HOP_BY_HOP_HEADERS.contains(&name.to_lowercase().as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_client_hello() {
        let msg = ClientMessage::Hello {
            protocol_version: PROTOCOL_VERSION,
            agent_id: Uuid::new_v4(),
            agent_token: "secret".into(),
            agent_name: "laptop".into(),
            platform: "macos".into(),
            architecture: "aarch64".into(),
            version: "0.1.0".into(),
        };
        let text = encode(&msg).unwrap();
        let back: ClientMessage = decode(&text).unwrap();
        match back {
            ClientMessage::Hello {
                protocol_version, ..
            } => {
                assert_eq!(protocol_version, PROTOCOL_VERSION);
            }
            _ => panic!("expected hello"),
        }
    }

    #[test]
    fn roundtrip_deliver() {
        let instr = DeliveryInstruction {
            delivery_id: Uuid::new_v4(),
            event_id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            endpoint_id: Uuid::new_v4(),
            target_id: "paystack".into(),
            method: "POST".into(),
            content_type: Some("application/json".into()),
            headers: vec![ReplayHeader {
                name: "X-Signature".into(),
                value: "abc".into(),
            }],
            body: Some("aGk=".into()),
        };
        let msg = ServerMessage::Deliver(instr);
        let text = encode(&msg).unwrap();
        let back: ServerMessage = decode(&text).unwrap();
        assert!(matches!(back, ServerMessage::Deliver(_)));
    }

    #[test]
    fn roundtrip_subscribe_endpoint() {
        let msg = ClientMessage::Subscribe {
            endpoint_id: Uuid::new_v4(),
            target_id: "myapp".into(),
        };
        let text = encode(&msg).unwrap();
        let back: ClientMessage = decode(&text).unwrap();
        match back {
            ClientMessage::Subscribe { target_id, .. } => {
                assert_eq!(target_id, "myapp");
            }
            _ => panic!("expected subscribe"),
        }
    }
}
