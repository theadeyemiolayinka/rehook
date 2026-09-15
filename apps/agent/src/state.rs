//! Shared connection state between the connection loop and the web server.
//!
//! The web server needs to report whether the agent is connected to the
//! server. The connection loop runs in a separate task and updates this
//! state. The web API reads it.
//!
//! The token is cached in memory after the first load so the connection
//! loop does not hit the OS keychain on every reconnect attempt (which
//! would prompt for a password on macOS each time).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::{Mutex, Notify};

/// Shared state for the agent connection.
#[derive(Clone)]
pub struct ConnectionState {
    /// Whether the WebSocket to the server is currently open.
    connected: Arc<AtomicBool>,
    /// Last time the connection was established (RFC3339).
    last_connected_at: Arc<Mutex<Option<String>>>,
    /// Cached agent token. Loaded once from the keychain and kept in
    /// memory so reconnect attempts do not trigger keychain prompts.
    token: Arc<Mutex<Option<String>>>,
    /// Notified when login or logout changes credentials, so the
    /// connection loop can retry immediately instead of waiting for
    /// its backoff timer.
    notify: Arc<Notify>,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionState {
    pub fn new() -> Self {
        Self {
            connected: Arc::new(AtomicBool::new(false)),
            last_connected_at: Arc::new(Mutex::new(None)),
            token: Arc::new(Mutex::new(None)),
            notify: Arc::new(Notify::new()),
        }
    }

    pub fn set_connected(&self, value: bool) {
        self.connected.store(value, Ordering::Relaxed);
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    pub async fn set_last_connected(&self, ts: String) {
        *self.last_connected_at.lock().await = Some(ts);
    }

    pub async fn last_connected(&self) -> Option<String> {
        self.last_connected_at.lock().await.clone()
    }

    /// Cache a token in memory and wake the connection loop. Called
    /// after login or after the first keychain load.
    pub async fn set_token(&self, token: String) {
        *self.token.lock().await = Some(token);
        self.notify.notify_one();
    }

    /// Clear the cached token and wake the connection loop. Called on
    /// logout.
    pub async fn clear_token(&self) {
        *self.token.lock().await = None;
        self.notify.notify_one();
    }

    /// Get the cached token, if any.
    pub async fn token(&self) -> Option<String> {
        self.token.lock().await.clone()
    }

    /// Wait for a credential change notification. Returns immediately
    /// if notified before this call.
    pub async fn wait_for_change(&self) {
        self.notify.notified().await;
    }
}
