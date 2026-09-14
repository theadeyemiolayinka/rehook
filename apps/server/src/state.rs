//! Shared application state passed to handlers.

use std::sync::Arc;

use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::config::Config;
use crate::websocket::registry::AgentRegistry;

/// Shared application state passed to handlers.
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<Config>,
    pub agents: Arc<AgentRegistry>,
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
        })
    }
}
