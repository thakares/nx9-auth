use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::request::Parts,
};
use std::net::SocketAddr;

/// Request context for audit logging — captures IP and User-Agent.
///
/// Handlers include this extractor to forward client metadata to the audit log
/// without threading raw request headers through the call stack.
#[derive(Debug, Clone, Default)]
pub struct AuditContext {
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

impl<S> FromRequestParts<S> for AuditContext
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Prefer X-Forwarded-For (set by reverse proxies like Nginx)
        let ip_address = parts
            .headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .map(|s| s.trim().to_string())
            .or_else(|| {
                // Fall back to direct peer address (requires ConnectInfo extension)
                parts
                    .extensions
                    .get::<ConnectInfo<SocketAddr>>()
                    .map(|ci| ci.0.ip().to_string())
            });

        let user_agent = parts
            .headers
            .get(axum::http::header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        Ok(AuditContext {
            ip_address,
            user_agent,
        })
    }
}
