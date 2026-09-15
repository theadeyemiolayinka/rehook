//! In-memory registry of connected agents and their subscriptions.
//!
//! Used by the WebSocket gateway to register agents and by the replay API to
//! dispatch delivery instructions to a connected agent. The registry holds an
//! mpsc sender per connected agent; the gateway owns the receiver.

use std::collections::HashMap;
use std::sync::Arc;

use hookrelay_protocol::DeliveryInstruction;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

/// A connected agent's outbound channel and subscription state.
struct ConnectedAgent {
    tx: mpsc::Sender<GatewayOutbound>,
    /// Set of endpoint IDs the agent is subscribed to.
    subscriptions: std::collections::HashSet<Uuid>,
}

/// Outbound messages the gateway sends to the agent over its channel.
pub enum GatewayOutbound {
    Instruction(DeliveryInstruction),
    /// Sent when the agent should be disconnected (e.g. revoked mid-session).
    Disconnect,
}

/// Registry of connected agents.
#[derive(Default)]
pub struct AgentRegistry {
    agents: Mutex<HashMap<Uuid, ConnectedAgent>>,
}

impl AgentRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Register a connected agent. Returns the receiver for outbound messages.
    pub async fn register(&self, agent_id: Uuid) -> mpsc::Receiver<GatewayOutbound> {
        let (tx, rx) = mpsc::channel(32);
        let mut agents = self.agents.lock().await;
        // If an existing connection is present, signal it to disconnect.
        if let Some(prev) = agents.insert(
            agent_id,
            ConnectedAgent {
                tx,
                subscriptions: Default::default(),
            },
        ) {
            let _ = prev.tx.try_send(GatewayOutbound::Disconnect);
        }
        rx
    }

    /// Unregister an agent.
    pub async fn unregister(&self, agent_id: Uuid) {
        let mut agents = self.agents.lock().await;
        agents.remove(&agent_id);
    }

    /// Subscribe an agent to an endpoint.
    pub async fn subscribe(&self, agent_id: Uuid, endpoint_id: Uuid) -> Result<(), String> {
        let mut agents = self.agents.lock().await;
        let Some(agent) = agents.get_mut(&agent_id) else {
            return Err("agent not connected".into());
        };
        agent.subscriptions.insert(endpoint_id);
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

    /// Send a delivery instruction to a connected agent, if it is subscribed to
    /// the event's endpoint. Returns Ok(()) if dispatched, Err with a reason
    /// otherwise.
    pub async fn dispatch(
        &self,
        agent_id: Uuid,
        endpoint_id: Uuid,
        instruction: DeliveryInstruction,
    ) -> Result<(), String> {
        let mut agents = self.agents.lock().await;
        let Some(agent) = agents.get_mut(&agent_id) else {
            return Err("agent not connected".into());
        };
        if !agent.subscriptions.contains(&endpoint_id) {
            return Err("agent not subscribed to endpoint".into());
        }
        agent
            .tx
            .try_send(GatewayOutbound::Instruction(instruction))
            .map_err(|_| "agent channel full or closed".into())
    }

    /// Whether an agent is currently connected.
    pub async fn is_connected(&self, agent_id: Uuid) -> bool {
        self.agents.lock().await.contains_key(&agent_id)
    }
}
