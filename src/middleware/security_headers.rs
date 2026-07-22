//! OWASP-oriented security headers for all HTTP responses.

use axum::{
    body::Body,
    extract::State,
    http::{HeaderValue, Request, header},
    middleware::Next,
    response::Response,
};

use crate::state::AppState;

/// Inject security headers on every response.
pub async fn security_headers(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );

    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));

    // Limits credential leakage via the Referer header.
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );

    // Prevent sensitive state caching across browsers and intermediaries
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));

    // SPA + same-origin API CSP.
    // 'wasm-unsafe-eval' is required for WebAssembly instantiation in Chromium.
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; \
             script-src 'self' 'wasm-unsafe-eval'; \
             style-src 'self' 'unsafe-inline'; \
             img-src 'self' data:; \
             font-src 'self' data:; \
             connect-src 'self'; \
             worker-src 'self' blob:; \
             frame-ancestors 'none'; \
             base-uri 'self'; \
             form-action 'self'; \
             object-src 'none'",
        ),
    );

    headers.insert(
        header::HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static(
            "accelerometer=(), camera=(), geolocation=(), gyroscope=(), \
             magnetometer=(), microphone=(), payment=(), usb=()",
        ),
    );

    // HSTS only when production / secure cookies (HTTPS-facing deployment).
    if state.config.server.production || state.config.server.cookie_secure {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=63072000; includeSubDomains"),
        );
    }

    response
}
