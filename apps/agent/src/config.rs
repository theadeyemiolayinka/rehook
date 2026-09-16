//! Agent configuration stored in a local TOML-like file.
//!
//! Kept deliberately simple. Credentials are NOT stored here; they live in the
//! OS keychain (see `credentials`). This file holds the server URL, agent id,
//! and the local target allowlist.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentConfig {
    pub server_url: Option<String>,
    pub agent_id: Option<Uuid>,
    pub agent_name: Option<String>,
    /// Map of target_id -> local URL. The agent only ever fetches these URLs.
    #[serde(default)]
    pub targets: HashMap<String, String>,
    /// Map of endpoint_id -> target_id for delivery routing. Routing is at
    /// the endpoint level so different endpoints in the same project can
    /// route to different local targets.
    #[serde(default)]
    pub endpoint_targets: HashMap<String, String>,
}

impl AgentConfig {
    pub fn path() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .ok_or_else(|| anyhow!("could not determine config directory"))?
            .join("rehook");
        fs::create_dir_all(&dir).context("creating config directory")?;
        Ok(dir.join("agent.json"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(&path).context("reading config")?;
        let config: Self = serde_json::from_str(&text).context("parsing config")?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        let text = serde_json::to_string_pretty(self).context("serializing config")?;
        fs::write(&path, text).context("writing config")?;
        Ok(())
    }
}
