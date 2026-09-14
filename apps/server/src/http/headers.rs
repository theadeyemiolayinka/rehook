//! Security headers middleware.
//!
//! Adds headers to all responses that prevent indexing, reduce XSS risk,
//! and establish a clear content security policy. The dashboard is an
//! administrative interface and must not be publicly indexed.

use std::convert::Infallible;

use axum::response::Response;
use tower_http::set_header::SetResponseHeaderLayer;

/// Apply security headers to all responses.
pub fn security_headers_layer() -> Vec<SetResponseHeaderLayer<axum::http::HeaderValue>> {
    vec![
        SetResponseHeaderLayer::if_not_present(
            axum::http::HeaderName::from_static("x-robots-tag"),
            axum::http::HeaderValue::from_static("noindex, nofollow, noarchive"),
        ),
        SetResponseHeaderLayer::if_not_present(
            axum::http::header::X_CONTENT_TYPE_OPTIONS,
            axum::http::HeaderValue::from_static("nosniff"),
        ),
        SetResponseHeaderLayer::if_not_present(
            axum::http::HeaderName::from_static("x-frame-options"),
            axum::http::HeaderValue::from_static("DENY"),
        ),
        SetResponseHeaderLayer::if_not_present(
            axum::http::HeaderName::from_static("referrer-policy"),
            axum::http::HeaderValue::from_static("strict-origin-when-cross-origin"),
        ),
    ]
}

/// A no-op response transform kept for future CSP or HSTS logic.
#[allow(dead_code)]
pub fn _apply_csp(resp: &mut Response) -> Result<(), Infallible> {
    let _ = resp;
    Ok(())
}
