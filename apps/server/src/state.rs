//! Shared application state passed to handlers.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use tokio::sync::Mutex;

use crate::config::Config;
use crate::websocket::registry::AgentRegistry;

/// In-memory login rate limiter. Tracks failed attempts per IP address.
/// Allows 5 attempts per 60-second window. Resets on successful login.
/// Entries expire after 5 minutes of inactivity.
pub struct LoginRateLimiter {
    attempts: Mutex<HashMap<IpAddr, Vec<Instant>>>,
}

impl LoginRateLimiter {
    const MAX_ATTEMPTS: usize = 5;
    const WINDOW: Duration = Duration::from_secs(60);

    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            attempts: Mutex::new(HashMap::new()),
        })
    }

    /// Check if an IP is currently rate limited. Returns true if blocked.
    pub async fn is_blocked(&self, ip: IpAddr) -> bool {
        let attempts = self.attempts.lock().await;
        if let Some(times) = attempts.get(&ip) {
            let now = Instant::now();
            let recent: Vec<_> = times
                .iter()
                .filter(|t| now.duration_since(**t) < Self::WINDOW)
                .collect();
            return recent.len() >= Self::MAX_ATTEMPTS;
        }
        false
    }

    /// Record a failed attempt for an IP.
    pub async fn record_failure(&self, ip: IpAddr) {
        let mut attempts = self.attempts.lock().await;
        let now = Instant::now();
        let times = attempts.entry(ip).or_default();
        times.retain(|t| now.duration_since(*t) < Self::WINDOW);
        times.push(now);
    }

    /// Clear attempts for an IP (on successful login).
    pub async fn clear(&self, ip: IpAddr) {
        let mut attempts = self.attempts.lock().await;
        attempts.remove(&ip);
    }
}

impl Default for LoginRateLimiter {
    fn default() -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
        }
    }
}

/// Shared application state passed to handlers.
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<Config>,
    pub agents: Arc<AgentRegistry>,
    pub login_limiter: Arc<LoginRateLimiter>,
}

impl AppState {
    pub async fn new(config: &Config) -> Result<Self> {
        // Ensure the data directory exists so SQLite can create the file.
        std::fs::create_dir_all(&config.data_dir)
            .with_context(|| format!("creating data dir {}", config.data_dir.display()))?;

        let opts: SqliteConnectOptions = config
            .database_url
            .parse::<SqliteConnectOptions>()
            .context("parsing database url")?
            .create_if_missing(true)
            // Enable WAL for better concurrent reads and durability.
            .pragma("journal_mode", "WAL")
            .pragma("synchronous", "NORMAL")
            .pragma("busy_timeout", "5000")
            .pragma("foreign_keys", "ON");

        let pool = SqlitePoolOptions::new()
            // SQLite writes are serialized; a small pool is appropriate.
            .max_connections(8)
            .connect_with(opts)
            .await
            .context("connecting to database")?;

        Ok(Self {
            pool,
            config: Arc::new(config.clone()),
            agents: AgentRegistry::new(),
            login_limiter: LoginRateLimiter::new(),
        })
    }
}
