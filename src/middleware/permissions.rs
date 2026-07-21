use crate::{error::AppError, identity::permissions};

/// Enforce that the calling user has the given permission.
///
/// Alias for `permissions::require_permission` — imported in handlers for
/// readability: `require(&state.provider, user_id, "users:create").await?`
#[inline]
pub async fn require(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    user_id: &str,
    permission: &str,
) -> Result<(), AppError> {
    permissions::require_permission(provider, user_id, permission).await
}
