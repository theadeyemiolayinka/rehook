//! Webhook signature and token validation.
//!
//! Validates inbound webhooks against the endpoint's configured validation
//! method before storing the event. Supported methods:
//!
//! - `none`: no validation (default)
//! - `hmac_sha256`: HMAC-SHA256 of body with secret, compared to a header
//! - `hmac_sha512`: HMAC-SHA512 of body with secret, compared to a header
//! - `header_token`: check a specific header equals a value
//! - `query_token`: check a query parameter equals a value

use axum::http::{HeaderMap, Uri};
use hmac::{Hmac, Mac};
use sha2::{Sha256, Sha512};

/// Result of webhook validation.
pub enum ValidationResult {
    /// Validation passed or not configured.
    Ok,
    /// Validation failed; the request should be rejected.
    Failed(String),
}

/// Validate a webhook request against the endpoint's validation config.
pub fn validate(
    validation_type: &str,
    validation_secret: Option<&str>,
    validation_header: Option<&str>,
    validation_query: Option<&str>,
    headers: &HeaderMap,
    uri: &Uri,
    body: &[u8],
) -> ValidationResult {
    match validation_type {
        "none" => ValidationResult::Ok,
        "hmac_sha256" => {
            let secret = match validation_secret {
                Some(s) if !s.is_empty() => s,
                _ => return ValidationResult::Failed("validation secret not configured".into()),
            };
            let header_name = match validation_header {
                Some(h) if !h.is_empty() => h,
                _ => return ValidationResult::Failed("validation header not configured".into()),
            };
            let expected = match headers.get(header_name).and_then(|v| v.to_str().ok()) {
                Some(v) => v,
                None => return ValidationResult::Failed(format!("missing header {header_name}")),
            };
            verify_hmac_sha256(body, secret, expected)
        }
        "hmac_sha512" => {
            let secret = match validation_secret {
                Some(s) if !s.is_empty() => s,
                _ => return ValidationResult::Failed("validation secret not configured".into()),
            };
            let header_name = match validation_header {
                Some(h) if !h.is_empty() => h,
                _ => return ValidationResult::Failed("validation header not configured".into()),
            };
            let expected = match headers.get(header_name).and_then(|v| v.to_str().ok()) {
                Some(v) => v,
                None => return ValidationResult::Failed(format!("missing header {header_name}")),
            };
            verify_hmac_sha512(body, secret, expected)
        }
        "header_token" => {
            let expected = match validation_secret {
                Some(s) if !s.is_empty() => s,
                _ => return ValidationResult::Failed("validation token not configured".into()),
            };
            let header_name = match validation_header {
                Some(h) if !h.is_empty() => h,
                _ => return ValidationResult::Failed("validation header not configured".into()),
            };
            match headers.get(header_name).and_then(|v| v.to_str().ok()) {
                Some(v) if v == expected => ValidationResult::Ok,
                Some(_) => ValidationResult::Failed("header token mismatch".into()),
                None => ValidationResult::Failed(format!("missing header {header_name}")),
            }
        }
        "query_token" => {
            let expected = match validation_secret {
                Some(s) if !s.is_empty() => s,
                _ => return ValidationResult::Failed("validation token not configured".into()),
            };
            let param_name = match validation_query {
                Some(q) if !q.is_empty() => q,
                _ => {
                    return ValidationResult::Failed("validation query param not configured".into())
                }
            };
            let query = uri.query().unwrap_or("");
            match extract_query_param(query, param_name) {
                Some(v) if v == expected => ValidationResult::Ok,
                Some(_) => ValidationResult::Failed("query token mismatch".into()),
                None => ValidationResult::Failed(format!("missing query param {param_name}")),
            }
        }
        _ => ValidationResult::Ok,
    }
}

/// Verify an HMAC-SHA256 signature. The expected header value may be a raw hex
/// digest or prefixed with an algorithm name like "sha256=...".
fn verify_hmac_sha256(body: &[u8], secret: &str, expected: &str) -> ValidationResult {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = match <HmacSha256 as Mac>::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return ValidationResult::Failed("invalid secret".into()),
    };
    mac.update(body);
    let result = mac.finalize().into_bytes();
    let hex = hex_encode(&result);

    // Try both raw hex and "sha256=..." prefixed formats.
    let clean = expected.trim_start_matches("sha256=");
    if clean.eq_ignore_ascii_case(&hex) {
        ValidationResult::Ok
    } else {
        ValidationResult::Failed("signature mismatch".into())
    }
}

/// Verify an HMAC-SHA512 signature.
fn verify_hmac_sha512(body: &[u8], secret: &str, expected: &str) -> ValidationResult {
    type HmacSha512 = Hmac<Sha512>;
    let mut mac = match <HmacSha512 as Mac>::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return ValidationResult::Failed("invalid secret".into()),
    };
    mac.update(body);
    let result = mac.finalize().into_bytes();
    let hex = hex_encode(&result);

    let clean = expected.trim_start_matches("sha512=");
    if clean.eq_ignore_ascii_case(&hex) {
        ValidationResult::Ok
    } else {
        ValidationResult::Failed("signature mismatch".into())
    }
}

/// Extract a query parameter value from a query string.
fn extract_query_param<'a>(query: &'a str, name: &str) -> Option<&'a str> {
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
            if k == name {
                return Some(v);
            }
        }
    }
    None
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_sha256_roundtrip() {
        let body = b"hello world";
        let secret = "mysecret";
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = <HmacSha256 as Mac>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let expected = hex_encode(&mac.finalize().into_bytes());

        let mut headers = HeaderMap::new();
        headers.insert("x-sig", expected.parse().unwrap());
        let uri: Uri = "/i/test".parse().unwrap();
        let result = validate(
            "hmac_sha256",
            Some(secret),
            Some("x-sig"),
            None,
            &headers,
            &uri,
            body,
        );
        assert!(matches!(result, ValidationResult::Ok));
    }

    #[test]
    fn hmac_sha256_prefixed() {
        let body = b"hello world";
        let secret = "mysecret";
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = <HmacSha256 as Mac>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let expected = format!("sha256={}", hex_encode(&mac.finalize().into_bytes()));

        let mut headers = HeaderMap::new();
        headers.insert("x-sig", expected.parse().unwrap());
        let uri: Uri = "/i/test".parse().unwrap();
        let result = validate(
            "hmac_sha256",
            Some(secret),
            Some("x-sig"),
            None,
            &headers,
            &uri,
            body,
        );
        assert!(matches!(result, ValidationResult::Ok));
    }

    #[test]
    fn header_token_match() {
        let mut headers = HeaderMap::new();
        headers.insert("x-token", "abc123".parse().unwrap());
        let uri: Uri = "/i/test".parse().unwrap();
        let result = validate(
            "header_token",
            Some("abc123"),
            Some("x-token"),
            None,
            &headers,
            &uri,
            b"",
        );
        assert!(matches!(result, ValidationResult::Ok));
    }

    #[test]
    fn query_token_match() {
        let headers = HeaderMap::new();
        let uri: Uri = "/i/test?token=abc123".parse().unwrap();
        let result = validate(
            "query_token",
            Some("abc123"),
            None,
            Some("token"),
            &headers,
            &uri,
            b"",
        );
        assert!(matches!(result, ValidationResult::Ok));
    }
}
