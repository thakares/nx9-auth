use crate::db::repository::traits::AuditRepositoryExt;
use rand::RngCore;

use crate::{config::SecurityConfig, db::models::ApiToken, error::AppError};

/// Prefix for all personal access tokens.
pub const PAT_PREFIX: &str = "nx9_pat_";

/// Generate a new personal access token string.
///
/// Format: `nx9_pat_<64 hex chars>` (32 random bytes)
pub fn generate_pat() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("{}{}", PAT_PREFIX, hex::encode(bytes))
}

/// Hash a raw token string using BLAKE3.
pub fn hash_token(raw: &str) -> String {
    hex::encode(blake3::hash(raw.as_bytes()).as_bytes())
}

/// Create a new personal access token for a user.
///
/// Returns `(ApiToken row, raw_token)` — the raw token is shown once and
/// never stored. Only the BLAKE3 hash is persisted.
pub async fn create_token(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    user_id: &str,
    name: &str,
    cfg: &SecurityConfig,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<(ApiToken, String), AppError> {
    let raw = generate_pat();
    let hash = hash_token(&raw);
    let id = uuid::Uuid::new_v4().to_string();

    let expires_at = chrono::Utc::now() + chrono::Duration::days(cfg.token_ttl_days as i64);
    let expires_at_str = expires_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let token = provider
        .tokens()
        .create(&id, user_id, name, &hash, Some(&expires_at_str))
        .await
        .map_err(AppError::Database)?;

    let metadata = serde_json::json!({ "token_id": token.id, "name": name }).to_string();
    provider
        .audit()
        .log(crate::audit::AuditEvent {
            actor_id: audit_actor_id,
            target_id: Some(user_id),
            action: "token_created",
            resource_type: "token",
            resource_id: Some(&token.id),
            severity: crate::db::models::AuditSeverity::Info,
            ip: audit_ip,
            ua: audit_ua,
            metadata: Some(&metadata),
        })
        .await?;

    Ok((token, raw))
}

/// Revoke a personal access token.
pub async fn revoke_token(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    id: &str,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<(), AppError> {
    let token = provider
        .tokens()
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    provider
        .tokens()
        .revoke(id)
        .await
        .map_err(AppError::Database)?;

    let metadata = serde_json::json!({ "token_id": id, "name": token.name }).to_string();
    provider
        .audit()
        .log(crate::audit::AuditEvent {
            actor_id: audit_actor_id,
            target_id: Some(&token.user_id),
            action: "token_revoked",
            resource_type: "token",
            resource_id: Some(id),
            severity: crate::db::models::AuditSeverity::Warning,
            ip: audit_ip,
            ua: audit_ua,
            metadata: Some(&metadata),
        })
        .await?;

    Ok(())
}

/// Validate a raw PAT from an Authorization header.
///
/// Strips the `nx9_pat_` prefix, hashes it, and looks it up. Returns `None`
/// if the token is unknown, revoked, or expired.
pub async fn validate_token(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    raw: &str,
) -> Result<Option<ApiToken>, AppError> {
    // Must have the expected prefix
    if !raw.starts_with(PAT_PREFIX) {
        return Ok(None);
    }

    let hash = hash_token(raw);
    let token = provider
        .tokens()
        .find_by_hash(&hash)
        .await
        .map_err(AppError::Database)?;

    let Some(token) = token else {
        return Ok(None);
    };

    // Check expiry if set
    if let Some(ref exp) = token.expires_at {
        if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(exp) {
            if chrono::Utc::now() > expires {
                return Ok(None);
            }
        }
    }

    // Touch last_used_at (fire-and-forget)
    let _ = provider.tokens().update_last_used(&token.id).await;

    Ok(Some(token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generation_and_prefix() {
        let t1 = generate_pat();
        let t2 = generate_pat();
        assert_ne!(t1, t2);
        assert!(t1.starts_with(PAT_PREFIX));

        let h1 = hash_token(&t1);
        let h2 = hash_token(&t1);
        assert_eq!(h1, h2);
        assert_ne!(h1, t1);
    }
}
