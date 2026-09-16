//! In-memory registry of connected agents and their subscriptions.
//!
//! Used by the WebSocket gateway to register agents and by the capture
//! pipeline and replay API to dispatch delivery instructions to connected
//! agents. The registry holds an mpsc sender per connected agent; the
//! gateway owns the receiver.

use std::collections::HashMap;
use std::sync::Arc;

use rehook_protocol::DeliveryInstruction;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

/// A connected agent's outbound channel and subscription state.
struct ConnectedAgent {
    /// Unique id for this connection instance. Reconnects replace the
    /// entry; the old task must not unregister the new connection.
    conn_id: u64,
    tx: mpsc::Sender<GatewayOutbound>,
    /// Map of endpoint ID -> target ID the agent wants deliveries for
    /// that endpoint routed to.
    subscriptions: HashMap<Uuid, String>,
}

/// Outbound messages the gateway sends to the agent over its channel.
pub enum GatewayOutbound {
    Instruction(DeliveryInstruction),
    /// A control message (e.g. SubscriptionConfirmed).
    Message(rehook_protocol::ServerMessage),
    /// Sent when the agent should be disconnected (e.g. revoked mid-session).
    Disconnect,
}

/// Registry of connected agents.
#[derive(Default)]
pub struct AgentRegistry {
    agents: Mutex<HashMap<Uuid, ConnectedAgent>>,
    next_conn_id: std::sync::atomic::AtomicU64,
}

impl AgentRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Register a connected agent. Returns the connection id and the
    /// receiver for outbound messages.
    pub async fn register(&self, agent_id: Uuid) -> (u64, mpsc::Receiver<GatewayOutbound>) {
        let conn_id = self
            .next_conn_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        let (tx, rx) = mpsc::channel(32);
        let mut agents = self.agents.lock().await;
        // If an existing connection is present, signal it to disconnect.
        // Its subscriptions are replaced; the agent resubscribes on
        // every connect.
        if let Some(prev) = agents.insert(
            agent_id,
            ConnectedAgent {
                conn_id,
                tx,
                subscriptions: Default::default(),
            },
        ) {
            let _ = prev.tx.try_send(GatewayOutbound::Disconnect);
        }
        (conn_id, rx)
    }

    /// Unregister an agent's connection. Only removes the entry if it is
    /// still this connection; a reconnect may have already replaced it.
    pub async fn unregister(&self, agent_id: Uuid, conn_id: u64) {
        let mut agents = self.agents.lock().await;
        if let Some(agent) = agents.get(&agent_id) {
            if agent.conn_id == conn_id {
                agents.remove(&agent_id);
            }
        }
    }

    /// Subscribe an agent to an endpoint with a target.
    pub async fn subscribe(
        &self,
        agent_id: Uuid,
        endpoint_id: Uuid,
        target_id: String,
    ) -> Result<(), String> {
        let mut agents = self.agents.lock().await;
        let Some(agent) = agents.get_mut(&agent_id) else {
            return Err("agent not connected".into());
        };
        agent.subscriptions.insert(endpoint_id, target_id);
        Ok(())
    }

    /// Unsubscribe an agent from an endpoint.
    pub async fn unsubscribe(&self, agent_id: Uuid, endpoint_id: Uuid) -> Result<(), String> {
        let mut agents = self.agents.lock().await;
        let Some(agent) = agents.get_mut(&agent_id) else {
            return Err("agent not connected".into());
        };
        agent.subscriptions.remove(&endpoint_id);
        Ok(())
    }

    /// Send a delivery instruction to a connected agent. Returns Ok(()) if
    /// dispatched, Err with a reason otherwise. Does not check subscription;
    /// that is the caller's responsibility.
    pub async fn dispatch(
        &self,
        agent_id: Uuid,
        instruction: DeliveryInstruction,
    ) -> Result<(), String> {
        let mut agents = self.agents.lock().await;
        let Some(agent) = agents.get_mut(&agent_id) else {
            return Err("agent not connected".into());
        };
        agent
            .tx
            .try_send(GatewayOutbound::Instruction(instruction))
            .map_err(|_| "agent channel full or closed".into())
    }

    /// Whether an agent is currently connected.
    pub async fn is_connected(&self, agent_id: Uuid) -> bool {
        self.agents.lock().await.contains_key(&agent_id)
    }

    /// Signal a connected agent to disconnect (e.g. agent deleted or
    /// disabled). The connection task unregisters itself on close.
    pub async fn disconnect(&self, agent_id: Uuid) {
        let agents = self.agents.lock().await;
        if let Some(agent) = agents.get(&agent_id) {
            let _ = agent.tx.try_send(GatewayOutbound::Disconnect);
        }
    }

    /// Send a control message to a connected agent.
    pub async fn send_message(
        &self,
        agent_id: Uuid,
        msg: rehook_protocol::ServerMessage,
    ) -> Result<(), String> {
        let mut agents = self.agents.lock().await;
        let Some(agent) = agents.get_mut(&agent_id) else {
            return Err("agent not connected".into());
        };
        agent
            .tx
            .try_send(GatewayOutbound::Message(msg))
            .map_err(|_| "agent channel full or closed".into())
    }
}
