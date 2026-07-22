use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RefreshToken {
    pub id: String,
    pub user_id: String,
    #[serde(skip_serializing)]
    pub token_hash: String,
    pub expires_at: String,
    pub created_at: String,
    pub revoked: bool,
}
