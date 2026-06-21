use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A session row from the `sessions` table.
///
/// `token_hash` is the BLAKE3 hex-encoded hash of the raw session token.
/// The raw token is stored in a cookie and never persisted.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    /// BLAKE3 hex hash of the raw cookie value.
    #[serde(skip_serializing)]
    pub token_hash: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: String,
    /// Absolute expiry — the session is dead after this regardless of activity.
    pub expires_at: String,
    /// Idle timeout — updated on each authenticated request.
    pub last_seen_at: String,
    pub revoked: bool,
}
