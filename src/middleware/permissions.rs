use sqlx::SqlitePool;

use crate::{error::AppError, identity::permissions};

/// Enforce that the calling user has the given permission.
///
/// Alias for `permissions::require_permission` — imported in handlers for
/// readability: `require(pool, user_id, "users:create").await?`
#[inline]
pub async fn require(pool: &SqlitePool, user_id: &str, permission: &str) -> Result<(), AppError> {
    permissions::require_permission(pool, user_id, permission).await
}
