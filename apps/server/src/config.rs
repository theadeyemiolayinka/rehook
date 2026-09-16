//! Runtime configuration loaded from environment variables with safe defaults.

use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::Parser;

/// Command-line arguments for `rehook-server`. Each flag has a matching
/// environment variable; the flag takes precedence.
#[derive(Debug, Default, Parser)]
#[command(
    name = "rehook-server",
    version,
    about = "Rehook server: webhook capture, storage, and delivery dispatch"
)]
pub struct CliArgs {
    /// Address to bind the HTTP server to.
    #[arg(long, env = "REHOOK_LISTEN_ADDR")]
    pub listen_addr: Option<SocketAddr>,

    /// Public URL that webhook providers use to reach this server.
    /// Displayed in the dashboard when copying inbound webhook URLs.
    /// Set this to your public domain in production.
    #[arg(long, env = "REHOOK_PUBLIC_BASE_URL")]
    pub public_base_url: Option<String>,

    /// Directory for the SQLite database and server data.
    #[arg(long, env = "REHOOK_DATA_DIR")]
    pub data_dir: Option<PathBuf>,

    /// Serve dashboard assets from this directory instead of the
    /// assets embedded in the binary.
    #[arg(long, env = "REHOOK_DASHBOARD_DIR")]
    pub dashboard_dir: Option<PathBuf>,
}

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
    /// Directory containing built dashboard assets. When unset (the
    /// default), the dashboard embedded into the binary is served instead.
    /// Setting this overrides the embedded assets, e.g. to serve a
    /// modified or locally built dashboard.
    pub dashboard_dir: Option<PathBuf>,
}

impl Config {
    /// Load configuration from the parsed CLI args. Flags take precedence
    /// over their matching environment variables; unset flags fall back to
    /// env vars, then defaults.
    pub fn from_args(args: CliArgs) -> Result<Self> {
        let listen_addr: SocketAddr = match args.listen_addr {
            Some(a) => a,
            None => env_or("REHOOK_LISTEN_ADDR", "0.0.0.0:8080")?
                .parse()
                .context("REHOOK_LISTEN_ADDR is not a valid socket address")?,
        };

        let data_dir = args.data_dir.unwrap_or_else(|| {
            PathBuf::from(env_or("REHOOK_DATA_DIR", "./data").unwrap_or_default())
        });

        // An explicit REHOOK_DATABASE_URL always wins. Otherwise the
        // database lives at <data-dir>/rehook.db, so --data-dir alone is
        // enough to relocate storage.
        let database_url = match env::var("REHOOK_DATABASE_URL") {
            Ok(u) if !u.is_empty() => u,
            _ => format!("sqlite:{}", data_dir.join("rehook.db").display()),
        };

        let public_base_url = args
            .public_base_url
            .or_else(|| env::var("REHOOK_PUBLIC_BASE_URL").ok())
            .unwrap_or_else(|| "http://localhost:8080".into())
            .trim_end_matches('/')
            .to_string();

        let trusted_proxy_hops: usize = env_or("REHOOK_TRUSTED_PROXY_HOPS", "0")?
            .parse()
            .context("REHOOK_TRUSTED_PROXY_HOPS must be a non-negative integer")?;

        let max_webhook_body_size: usize = env_or("REHOOK_MAX_WEBHOOK_BODY_SIZE", "1048576")?
            .parse()
            .context("REHOOK_MAX_WEBHOOK_BODY_SIZE must be an integer")?;

        let max_stored_events: u64 = env_or("REHOOK_MAX_STORED_EVENTS", "10000")?
            .parse()
            .context("REHOOK_MAX_STORED_EVENTS must be an integer")?;

        let event_retention_days: u32 = env_or("REHOOK_EVENT_RETENTION_DAYS", "14")?
            .parse()
            .context("REHOOK_EVENT_RETENTION_DAYS must be an integer")?;

        let session_key_hex = env_or("REHOOK_SESSION_KEY", "")?;
        let session_key_hex = if session_key_hex.is_empty() {
            tracing::warn!("REHOOK_SESSION_KEY unset; generating an ephemeral key. Sessions will not survive restarts.");
            hex::encode(rand::random::<[u8; 32]>())
        } else {
            session_key_hex
        };

        let session_ttl_hours: u64 = env_or("REHOOK_SESSION_TTL_HOURS", "720")?
            .parse()
            .context("REHOOK_SESSION_TTL_HOURS must be an integer")?;

        let rust_log = env_or("RUST_LOG", "rehook=info,tower_http=info")?;

        let admin_username = env::var("ADMIN_USERNAME").ok().filter(|s| !s.is_empty());
        let admin_password = env::var("ADMIN_PASSWORD").ok().filter(|s| !s.is_empty());
        if admin_username.is_some() != admin_password.is_some() {
            return Err(anyhow!(
                "ADMIN_USERNAME and ADMIN_PASSWORD must both be set or both unset"
            ));
        }

        // Optional dashboard asset override. When unset, the assets embedded
        // into the binary are served.
        let dashboard_dir = args.dashboard_dir;

        // Safety: ServeDir serves files from this directory to any HTTP
        // client. A misconfiguration pointing at a sensitive directory (the
        // filesystem root, the data directory, or a path with parent
        // traversal components) would expose files that should not be public.
        if let Some(dir) = &dashboard_dir {
            validate_dashboard_dir(dir)?;
        }

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
            .map_err(|e| anyhow!("REHOOK_SESSION_KEY is not valid hex: {e}"))
    }
}

fn env_or(key: &str, default: &str) -> Result<String> {
    Ok(env::var(key).unwrap_or_else(|_| default.to_string()))
}

/// Validate the dashboard directory before it is passed to ServeDir.
///
/// ServeDir serves files from this directory to any HTTP client. A
/// misconfiguration pointing at a sensitive directory (the filesystem root,
/// the data directory, or a path with parent traversal components) would
/// expose files that should not be public. This check runs at startup so
/// the server fails fast instead of silently serving the wrong files.
fn validate_dashboard_dir(dir: &std::path::Path) -> Result<()> {
    // Reject parent traversal components. A canonical path is not used here
    // because the directory may not exist yet in some dev setups; the
    // component check is sufficient to catch the dangerous cases.
    for component in dir.components() {
        if let std::path::Component::ParentDir = component {
            return Err(anyhow!(
                "REHOOK_DASHBOARD_DIR must not contain '..' components: {} \
                 (this would risk exposing files outside the intended directory)",
                dir.display()
            ));
        }
    }

    // Reject the filesystem root. Serving from root exposes the entire
    // filesystem to any HTTP client.
    let parent = dir.parent();
    if parent.is_none() || dir.as_os_str().is_empty() {
        return Err(anyhow!(
            "REHOOK_DASHBOARD_DIR must not be the filesystem root \
             (this would expose the entire filesystem)"
        ));
    }

    // Warn (but do not fail) if index.html is missing. This is common in dev
    // when the dashboard has not been built yet, but in production it means
    // the SPA will not load.
    if !dir.join("index.html").exists() {
        tracing::warn!(
            "REHOOK_DASHBOARD_DIR ({}) does not contain index.html. \
             The dashboard will not load until the admin dashboard is built.",
            dir.display()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_dashboard_dir;
    use std::path::PathBuf;

    #[test]
    fn rejects_parent_traversal() {
        let dir = PathBuf::from("../../etc");
        assert!(validate_dashboard_dir(&dir).is_err());
    }

    #[test]
    fn rejects_root() {
        let dir = PathBuf::from("/");
        assert!(validate_dashboard_dir(&dir).is_err());
    }

    #[test]
    fn accepts_normal_path() {
        let dir = PathBuf::from("./dashboards/admin/dist");
        // May warn about missing index.html but should not error.
        assert!(validate_dashboard_dir(&dir).is_ok());
    }

    #[test]
    fn args_override_listen_addr() {
        let args = super::CliArgs {
            listen_addr: Some("127.0.0.1:9999".parse().unwrap()),
            ..Default::default()
        };
        let config = super::Config::from_args(args).unwrap();
        assert_eq!(config.listen_addr.port(), 9999);
    }

    #[test]
    fn args_data_dir_derives_database_url() {
        let args = super::CliArgs {
            data_dir: Some(PathBuf::from("/tmp/rehook-test-cfg")),
            ..Default::default()
        };
        // Ensure env does not leak an explicit database url into the test.
        // If REHOOK_DATABASE_URL is set in the environment this test would
        // be misleading; skip asserting in that case.
        if std::env::var("REHOOK_DATABASE_URL").is_err() {
            let config = super::Config::from_args(args).unwrap();
            assert_eq!(config.database_url, "sqlite:/tmp/rehook-test-cfg/rehook.db");
        }
    }

    #[test]
    fn args_public_base_url_trims_trailing_slash() {
        let args = super::CliArgs {
            public_base_url: Some("https://hooks.example.com/".into()),
            ..Default::default()
        };
        let config = super::Config::from_args(args).unwrap();
        assert_eq!(config.public_base_url, "https://hooks.example.com");
    }

    #[test]
    fn args_dashboard_dir_override() {
        let args = super::CliArgs {
            dashboard_dir: Some(PathBuf::from("./dashboards/admin/dist")),
            ..Default::default()
        };
        let config = super::Config::from_args(args).unwrap();
        assert_eq!(
            config.dashboard_dir,
            Some(PathBuf::from("./dashboards/admin/dist"))
        );
    }

    #[test]
    fn default_dashboard_dir_is_embedded() {
        let config = super::Config::from_args(super::CliArgs::default()).unwrap();
        if std::env::var("REHOOK_DASHBOARD_DIR").is_err() {
            assert!(config.dashboard_dir.is_none());
        }
    }
}
