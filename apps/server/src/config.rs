//! Runtime configuration loaded from environment variables with safe defaults.

use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub listen_addr: SocketAddr,
    pub database_url: String,
    pub data_dir: PathBuf,
    pub public_base_url: String,
    pub trusted_proxy_hops: usize,
    pub max_webhook_body_size: usize,
    pub max_stored_events: u64,
    pub event_retention_days: u32,
    pub session_key_hex: String,
    pub session_ttl_hours: u64,
    pub rust_log: String,
    pub admin_username: Option<String>,
    pub admin_password: Option<String>,
    /// Directory containing the built dashboard assets. In dev this is
    /// typically empty and Vite serves the dashboard on :5173.
    pub dashboard_dir: PathBuf,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let listen_addr: SocketAddr = env_or("HOOKRELAY_LISTEN_ADDR", "0.0.0.0:8080")?
            .parse()
            .context("HOOKRELAY_LISTEN_ADDR is not a valid socket address")?;

        let data_dir = PathBuf::from(env_or("HOOKRELAY_DATA_DIR", "./data")?);
        let database_url = env_or(
            "HOOKRELAY_DATABASE_URL",
            &format!("sqlite:{}", data_dir.join("hookrelay.db").display()),
        )?;

        let public_base_url = env_or("HOOKRELAY_PUBLIC_BASE_URL", "http://localhost:8080")?
            .trim_end_matches('/')
            .to_string();

        let trusted_proxy_hops: usize = env_or("HOOKRELAY_TRUSTED_PROXY_HOPS", "0")?
            .parse()
            .context("HOOKRELAY_TRUSTED_PROXY_HOPS must be a non-negative integer")?;

        let max_webhook_body_size: usize = env_or("HOOKRELAY_MAX_WEBHOOK_BODY_SIZE", "1048576")?
            .parse()
            .context("HOOKRELAY_MAX_WEBHOOK_BODY_SIZE must be an integer")?;

        let max_stored_events: u64 = env_or("HOOKRELAY_MAX_STORED_EVENTS", "10000")?
            .parse()
            .context("HOOKRELAY_MAX_STORED_EVENTS must be an integer")?;

        let event_retention_days: u32 = env_or("HOOKRELAY_EVENT_RETENTION_DAYS", "14")?
            .parse()
            .context("HOOKRELAY_EVENT_RETENTION_DAYS must be an integer")?;

        let session_key_hex = env_or("HOOKRELAY_SESSION_KEY", "")?;
        let session_key_hex = if session_key_hex.is_empty() {
            tracing::warn!("HOOKRELAY_SESSION_KEY unset; generating an ephemeral key. Sessions will not survive restarts.");
            hex::encode(&rand::random::<[u8; 32]>())
        } else {
            session_key_hex
        };

        let session_ttl_hours: u64 = env_or("HOOKRELAY_SESSION_TTL_HOURS", "720")?
            .parse()
            .context("HOOKRELAY_SESSION_TTL_HOURS must be an integer")?;

        let rust_log = env_or("RUST_LOG", "hookrelay=info,tower_http=info")?;

        let admin_username = env::var("ADMIN_USERNAME").ok().filter(|s| !s.is_empty());
        let admin_password = env::var("ADMIN_PASSWORD").ok().filter(|s| !s.is_empty());
        if admin_username.is_some() != admin_password.is_some() {
            return Err(anyhow!(
                "ADMIN_USERNAME and ADMIN_PASSWORD must both be set or both unset"
            ));
        }

        let dashboard_dir = PathBuf::from(env_or(
            "HOOKRELAY_DASHBOARD_DIR",
            "./dashboards/admin/dist",
        )?);

        Ok(Self {
            listen_addr,
            database_url,
            data_dir,
            public_base_url,
            trusted_proxy_hops,
            max_webhook_body_size,
            max_stored_events,
            event_retention_days,
            session_key_hex,
            session_ttl_hours,
            rust_log,
            admin_username,
            admin_password,
            dashboard_dir,
        })
    }

    /// Bytes of the session key, derived from the hex config.
    pub fn session_key(&self) -> Result<Vec<u8>> {
        hex::decode(&self.session_key_hex)
            .map_err(|e| anyhow!("HOOKRELAY_SESSION_KEY is not valid hex: {e}"))
    }
}

fn env_or(key: &str, default: &str) -> Result<String> {
    Ok(env::var(key).unwrap_or_else(|_| default.to_string()))
}
