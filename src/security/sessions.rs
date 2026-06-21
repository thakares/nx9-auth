use rand::RngCore;
use sqlx::SqlitePool;

use crate::{
    config::SecurityConfig,
    db::{models::Session, repository::sessions as repo},
    error::AppError,
};

pub const SESSION_COOKIE: &str = "nx9_session";

/// Generate a cryptographically random session token (32 bytes → 64 hex chars).
pub fn generate_session_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Hash a raw session token using BLAKE3 (constant-time, fast).
pub fn hash_session_token(raw: &str) -> String {
    hex::encode(blake3::hash(raw.as_bytes()).as_bytes())
}

/// Create a new session in the database.
///
/// Returns `(Session row, raw_token)` — the raw token is placed in the cookie
/// and never stored. Only the BLAKE3 hash is persisted.
pub async fn create_session(
    pool: &SqlitePool,
    user_id: &str,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
    cfg: &SecurityConfig,
) -> Result<(Session, String), AppError> {
    let raw_token = generate_session_token();
    let token_hash = hash_session_token(&raw_token);

    // Absolute expiry = now + session_absolute_ttl_days
    let expires_at =
        chrono::Utc::now() + chrono::Duration::days(cfg.session_absolute_ttl_days as i64);
    let expires_at_str = expires_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let id = uuid::Uuid::new_v4().to_string();

    let session = repo::create(
        pool,
        &id,
        user_id,
        &token_hash,
        ip_address,
        user_agent,
        &expires_at_str,
    )
    .await
    .map_err(AppError::Database)?;

    Ok((session, raw_token))
}

/// Validate a raw session token from a cookie.
///
/// Enforces both absolute TTL and idle timeout. Touches `last_seen_at` on
/// every successful validation.
pub async fn validate_session(
    pool: &SqlitePool,
    raw_token: &str,
    cfg: &SecurityConfig,
) -> Result<Option<Session>, AppError> {
    let token_hash = hash_session_token(raw_token);

    let session = repo::find_by_token_hash(pool, &token_hash)
        .await
        .map_err(AppError::Database)?;

    let Some(session) = session else {
        return Ok(None);
    };

    let now = chrono::Utc::now();

    // Check absolute expiry
    if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(&session.expires_at) {
        if now > expires {
            repo::revoke(pool, &session.id)
                .await
                .map_err(AppError::Database)?;
            return Ok(None);
        }
    }

    // Check idle timeout
    if let Ok(last_seen) = chrono::DateTime::parse_from_rfc3339(&session.last_seen_at) {
        let idle_deadline = last_seen + chrono::Duration::hours(cfg.session_ttl_hours as i64);
        if now > idle_deadline {
            repo::revoke(pool, &session.id)
                .await
                .map_err(AppError::Database)?;
            return Ok(None);
        }
    }

    // Touch last_seen (fire-and-forget — don't fail the request if this errors)
    let _ = repo::update_last_seen(pool, &session.id).await;

    Ok(Some(session))
}

/// Revoke a session by its ID.
pub async fn revoke_session(pool: &SqlitePool, session_id: &str) -> Result<(), AppError> {
    repo::revoke(pool, session_id)
        .await
        .map_err(AppError::Database)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generation_and_hashing() {
        let t1 = generate_session_token();
        let t2 = generate_session_token();
        assert_ne!(t1, t2);
        assert_eq!(t1.len(), 64);

        let h1 = hash_session_token(&t1);
        let h2 = hash_session_token(&t1);
        assert_eq!(h1, h2);
        assert_ne!(h1, t1);
    }
}
