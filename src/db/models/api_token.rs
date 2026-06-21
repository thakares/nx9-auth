use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A personal access token row from the `api_tokens` table.
///
/// `token_hash` is the BLAKE3 hex-encoded hash of the raw `nx9_pat_...` token.
/// The raw token is displayed exactly once at creation time and never stored.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiToken {
    pub id: String,
    pub user_id: String,
    pub name: String,
    /// BLAKE3 hex hash — never expose in API responses.
    #[serde(skip_serializing)]
    pub token_hash: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
    pub revoked: bool,
}
