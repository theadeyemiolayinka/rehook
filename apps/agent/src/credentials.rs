//! Agent credential storage.
//!
//! Prefers the OS keychain (macOS Keychain, Windows Credential Manager, Linux
//! Secret Service) via the `keyring` crate. Falls back to a plaintext file with
//! restrictive permissions when keychain storage is unavailable. The fallback
//! is documented and clearly inferior; the keychain is the recommended path.
//!
//! SECURITY: the agent token grants delivery access. Treat it like a
//! credential. See SECURITY.md.

use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use anyhow::{anyhow, Context, Result};
use uuid::Uuid;

const SERVICE_NAME: &str = "rehook-agent";
const KEYRING_USERNAME: &str = "agent-token";

/// Store the agent token. Tries the keychain first, falls back to a file.
/// On re-login, deletes the old keychain entry first to avoid "item
/// already exists" errors on macOS.
pub fn store_token(agent_id: Uuid, token: &str) -> Result<()> {
    // Try keychain. Fall back to file on any keychain error.
    let stored = match keyring::Entry::new(SERVICE_NAME, KEYRING_USERNAME) {
        Ok(entry) => {
            // Delete any existing entry first (macOS Keychain refuses
            // to overwrite an existing item via set_password).
            let _ = entry.delete_credential();
            match entry.set_password(token) {
                Ok(()) => true,
                Err(e) => {
                    tracing::warn!("keychain store failed ({e}); falling back to plaintext file");
                    false
                }
            }
        }
        Err(_) => {
            tracing::warn!("OS keychain unavailable; falling back to plaintext file storage");
            false
        }
    };
    if !stored {
        fallback_store(token)?;
    }
    // Persist the agent id alongside (not secret).
    let _ = agent_id;
    Ok(())
}

/// Load the agent token.
pub fn load_token() -> Result<String> {
    match keyring::Entry::new(SERVICE_NAME, KEYRING_USERNAME) {
        Ok(entry) => match entry.get_password() {
            Ok(t) => Ok(t),
            Err(keyring::Error::NoEntry) => {
                Err(anyhow!("no agent token stored; run `rehook login`"))
            }
            Err(e) => {
                tracing::warn!("keychain load failed ({e}); trying plaintext file");
                fallback_load()
            }
        },
        Err(_) => fallback_load(),
    }
}

/// Delete the stored agent token.
pub fn delete_token() -> Result<()> {
    if let Ok(entry) = keyring::Entry::new(SERVICE_NAME, KEYRING_USERNAME) {
        let _ = entry.delete_credential();
    }
    let path = fallback_path()?;
    if path.exists() {
        fs::remove_file(&path).ok();
    }
    Ok(())
}

fn fallback_path() -> Result<std::path::PathBuf> {
    let dir = dirs::config_dir()
        .ok_or_else(|| anyhow!("could not determine config directory"))?
        .join("rehook");
    fs::create_dir_all(&dir).ok();
    // The token file lives here; keep the directory owner-only on Unix.
    #[cfg(unix)]
    {
        if let Ok(meta) = fs::metadata(&dir) {
            let mut perms = meta.permissions();
            perms.set_mode(0o700);
            fs::set_permissions(&dir, perms).ok();
        }
    }
    Ok(dir.join("token"))
}

fn fallback_store(token: &str) -> Result<()> {
    let path = fallback_path()?;
    fs::write(&path, token).context("writing fallback token file")?;
    // Restrict to owner only.
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&path, perms).ok();
    }
    Ok(())
}

fn fallback_load() -> Result<String> {
    let path = fallback_path()?;
    if !path.exists() {
        return Err(anyhow!("no agent token stored; run `rehook login`"));
    }
    let token = fs::read_to_string(&path)
        .context("reading fallback token file")?
        .trim()
        .to_string();
    Ok(token)
}
