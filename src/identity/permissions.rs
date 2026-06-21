use sqlx::SqlitePool;

use crate::{db::repository::permissions as repo, error::AppError};

/// Return all permission names held by a user.
pub async fn list_user_permissions(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<String>, AppError> {
    repo::list_for_user(pool, user_id)
        .await
        .map_err(AppError::Database)
}

/// Returns true if the user holds the given named permission.
pub async fn has_permission(
    pool: &SqlitePool,
    user_id: &str,
    permission: &str,
) -> Result<bool, AppError> {
    repo::user_has_permission(pool, user_id, permission)
        .await
        .map_err(AppError::Database)
}

/// Enforce that a user holds a permission, returning `Forbidden` otherwise.
pub async fn require_permission(
    pool: &SqlitePool,
    user_id: &str,
    permission: &str,
) -> Result<(), AppError> {
    if has_permission(pool, user_id, permission).await? {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}
