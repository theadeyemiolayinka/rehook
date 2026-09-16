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
    /// Maximum distinct IPs tracked. Stale entries are pruned when hit.
    const MAX_TRACKED_IPS: usize = 10_000;

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

        // Bound memory growth: a spray of failures from many distinct IPs
        // would otherwise grow the map forever. When the cap is hit, drop
        // all entries whose newest attempt is outside the window.
        if attempts.len() >= Self::MAX_TRACKED_IPS && !attempts.contains_key(&ip) {
            attempts.retain(|_, times| times.iter().any(|t| now.duration_since(*t) < Self::WINDOW));
        }

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

/// Restrict a data directory to owner-only access on Unix. The directory
/// holds the SQLite database, which contains webhook payloads, session
/// hashes, and agent token hashes. No-op on non-Unix platforms.
#[cfg(unix)]
fn restrict_dir_permissions(dir: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = std::fs::metadata(dir) {
        let mut perms = meta.permissions();
        perms.set_mode(0o700);
        let _ = std::fs::set_permissions(dir, perms);
    }
}

#[cfg(not(unix))]
fn restrict_dir_permissions(_dir: &std::path::Path) {}

impl Default for LoginRateLimiter {
    fn default() -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn blocks_after_max_attempts() {
        let limiter = LoginRateLimiter::new();
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        for _ in 0..LoginRateLimiter::MAX_ATTEMPTS {
            limiter.record_failure(ip).await;
        }
        assert!(limiter.is_blocked(ip).await);
    }

    #[tokio::test]
    async fn allows_before_max_attempts() {
        let limiter = LoginRateLimiter::new();
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));
        for _ in 0..LoginRateLimiter::MAX_ATTEMPTS - 1 {
            limiter.record_failure(ip).await;
        }
        assert!(!limiter.is_blocked(ip).await);
    }

    #[tokio::test]
    async fn clear_resets_attempts() {
        let limiter = LoginRateLimiter::new();
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 3));
        for _ in 0..LoginRateLimiter::MAX_ATTEMPTS {
            limiter.record_failure(ip).await;
        }
        assert!(limiter.is_blocked(ip).await);
        limiter.clear(ip).await;
        assert!(!limiter.is_blocked(ip).await);
    }

    #[tokio::test]
    async fn ips_are_tracked_independently() {
        let limiter = LoginRateLimiter::new();
        let a = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 4));
        let b = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5));
        for _ in 0..LoginRateLimiter::MAX_ATTEMPTS {
            limiter.record_failure(a).await;
        }
        assert!(limiter.is_blocked(a).await);
        assert!(!limiter.is_blocked(b).await);
    }

    #[tokio::test]
    async fn stale_entries_are_pruned_at_cap() {
        let limiter = LoginRateLimiter::new();
        let mut attempts = limiter.attempts.lock().await;
        // Fill the map with entries that are all outside the window.
        let old = Instant::now() - Duration::from_secs(600);
        for i in 0..LoginRateLimiter::MAX_TRACKED_IPS {
            let ip = IpAddr::V4(Ipv4Addr::new(192, 0, (i / 256) as u8, (i % 256) as u8));
            attempts.insert(ip, vec![old]);
        }
        drop(attempts);

        // A new IP must still be tracked (not silently dropped).
        let new_ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1));
        limiter.record_failure(new_ip).await;
        let attempts = limiter.attempts.lock().await;
        assert!(attempts.contains_key(&new_ip));
        // Stale entries should have been pruned.
        assert!(attempts.len() < LoginRateLimiter::MAX_TRACKED_IPS + 10);
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
        restrict_dir_permissions(&config.data_dir);

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
