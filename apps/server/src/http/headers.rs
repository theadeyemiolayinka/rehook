//! Security headers middleware.
//!
//! Adds headers to all responses that prevent indexing, reduce XSS risk,
//! and establish a clear content security policy. The dashboard is an
//! administrative interface and must not be publicly indexed.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;
use tower_http::set_header::SetResponseHeaderLayer;

use crate::state::AppState;

/// Content-Security-Policy for the dashboard and API.
///
/// The built dashboard loads scripts, styles, fonts, and images from its own
/// origin only. Inline styles are allowed because React applies inline style
/// attributes. No external origins are permitted; the dashboard talks to its
/// own origin via fetch.
const CSP: &str = "default-src 'self'; \
     script-src 'self'; \
     style-src 'self' 'unsafe-inline'; \
     img-src 'self' data:; \
     font-src 'self'; \
     connect-src 'self'; \
     object-src 'none'; \
     base-uri 'self'; \
     form-action 'self'; \
     frame-ancestors 'none'";

/// Apply security headers to all responses.
pub fn security_headers_layer() -> Vec<SetResponseHeaderLayer<HeaderValue>> {
    vec![
        SetResponseHeaderLayer::if_not_present(
            axum::http::HeaderName::from_static("x-robots-tag"),
            HeaderValue::from_static("noindex, nofollow, noarchive"),
        ),
        SetResponseHeaderLayer::if_not_present(
            axum::http::header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ),
        SetResponseHeaderLayer::if_not_present(
            axum::http::HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ),
        SetResponseHeaderLayer::if_not_present(
            axum::http::HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ),
        SetResponseHeaderLayer::if_not_present(
            axum::http::HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static(CSP),
        ),
        SetResponseHeaderLayer::if_not_present(
            axum::http::HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static(
                "camera=(), microphone=(), geolocation=(), payment=(), usb=()",
            ),
        ),
    ]
}

/// Conditional headers that depend on request path or deployment config.
///
/// - `Strict-Transport-Security` is set only when the public base URL is
///   HTTPS; sending it on a plain-HTTP deployment would break local dev.
/// - `Cache-Control: no-store` is set on API responses so session-scoped
///   data is never written to a shared cache.
pub async fn conditional_headers(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Response {
    let is_api = req.uri().path().starts_with("/api/");
    let https = state.config.public_base_url.starts_with("https://");

    let mut resp = next.run(req).await;

    if https {
        resp.headers_mut().insert(
            axum::http::HeaderName::from_static("strict-transport-security"),
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }
    if is_api {
        resp.headers_mut().insert(
            axum::http::header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        );
    }
    resp
}
