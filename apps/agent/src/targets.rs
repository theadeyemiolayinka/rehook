//! Local target validation.
//!
//! This is the core security boundary of the agent. The server sends a target
//! identifier; the agent resolves it to a locally configured URL from its
//! allowlist. The server can never send an arbitrary URL. The agent rejects
//! unsupported schemes and any target not in its allowlist.
//!
//! See SECURITY.md.

use anyhow::{anyhow, Result};
use url::Url;

/// Allowed URL schemes for local delivery.
const ALLOWED_SCHEMES: &[&str] = &["http", "https"];

/// Schemes that must never be permitted.
const FORBIDDEN_SCHEMES: &[&str] = &["file", "ftp", "gopher", "data", "javascript", "ws", "wss"];

/// Resolve a target identifier to a configured URL. The target_id must exist
/// in the allowlist. Returns an error if unknown or if the configured URL is
/// invalid.
pub fn resolve_target(
    allowlist: &std::collections::HashMap<String, String>,
    target_id: &str,
) -> Result<Url> {
    let raw = allowlist
        .get(target_id)
        .ok_or_else(|| anyhow!("unknown target id: {target_id}"))?;
    validate_url(raw)
}

/// Validate a URL for local delivery. Checks the scheme explicitly.
pub fn validate_url(raw: &str) -> Result<Url> {
    let url = Url::parse(raw).map_err(|e| anyhow!("invalid url: {e}"))?;
    let scheme = url.scheme();
    if FORBIDDEN_SCHEMES.contains(&scheme) {
        return Err(anyhow!("forbidden scheme: {scheme}"));
    }
    if !ALLOWED_SCHEMES.contains(&scheme) {
        return Err(anyhow!("unsupported scheme: {scheme}"));
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn resolves_known_target() {
        let mut map = HashMap::new();
        map.insert(
            "paystack".into(),
            "http://armoroflight.test/api/webhooks/paystack".into(),
        );
        let url = resolve_target(&map, "paystack").unwrap();
        assert_eq!(url.scheme(), "http");
        assert_eq!(url.host_str(), Some("armoroflight.test"));
    }

    #[test]
    fn rejects_unknown_target() {
        let map = HashMap::new();
        assert!(resolve_target(&map, "paystack").is_err());
    }

    #[test]
    fn rejects_file_scheme() {
        assert!(validate_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn rejects_ftp_scheme() {
        assert!(validate_url("ftp://example.com").is_err());
    }

    #[test]
    fn rejects_gopher_scheme() {
        assert!(validate_url("gopher://example.com").is_err());
    }

    #[test]
    fn rejects_data_scheme() {
        assert!(validate_url("data:text/plain,hi").is_err());
    }

    #[test]
    fn rejects_javascript_scheme() {
        assert!(validate_url("javascript:alert(1)").is_err());
    }

    #[test]
    fn allows_https() {
        assert!(validate_url("https://localhost:8443/hook").is_ok());
    }

    #[test]
    fn allows_localhost() {
        assert!(validate_url("http://127.0.0.1:8000/hook").is_ok());
        assert!(validate_url("http://localhost:8000/hook").is_ok());
    }

    #[test]
    fn allows_test_domain() {
        assert!(validate_url("http://armoroflight.test/api/webhooks/paystack").is_ok());
    }
}
