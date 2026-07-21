pub use crate::db::repository::sqlite::tokens::*;

use crate::db::models::ApiToken;
use crate::db::provider::DatabaseProvider;
use std::sync::Arc;

/// List tokens for a user using the provided DatabaseProvider.
pub async fn list_for_user(
    provider: &Arc<dyn DatabaseProvider>,
    user_id: &str,
) -> Result<Vec<ApiToken>, sqlx::Error> {
    provider.tokens().list_for_user(user_id).await
}

/// Find a token by its ID using the provided DatabaseProvider.
pub async fn find_by_id(
    provider: &Arc<dyn DatabaseProvider>,
    id: &str,
) -> Result<Option<ApiToken>, sqlx::Error> {
    provider.tokens().find_by_id(id).await
}
