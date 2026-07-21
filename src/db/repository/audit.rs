pub use crate::db::repository::sqlite::audit::*;

use crate::db::models::AuditLog;
use crate::db::provider::DatabaseProvider;
use std::sync::Arc;
// Removed direct import of AuditFilter to avoid conflict with traits version

/// Insert an audit log entry using the provided DatabaseProvider.
#[allow(clippy::too_many_arguments)]
pub async fn insert(
    provider: &Arc<dyn DatabaseProvider>,
    id: &str,
    actor_user_id: Option<&str>,
    target_user_id: Option<&str>,
    action: &str,
    resource_type: &str,
    resource_id: Option<&str>,
    severity: &str,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
    metadata_json: Option<&str>,
) -> Result<AuditLog, sqlx::Error> {
    provider
        .audit()
        .insert(
            id,
            actor_user_id,
            target_user_id,
            action,
            resource_type,
            resource_id,
            severity,
            ip_address,
            user_agent,
            metadata_json,
        )
        .await
}

/// Count filtered audit logs using the provided DatabaseProvider.
pub async fn count_filtered(
    provider: &Arc<dyn DatabaseProvider>,
    filter: &AuditFilter,
) -> Result<i64, sqlx::Error> {
    provider.audit().count_filtered(filter).await
}

/// List filtered audit logs using the provided DatabaseProvider.
pub async fn list_filtered(
    provider: &Arc<dyn DatabaseProvider>,
    filter: &AuditFilter,
) -> Result<Vec<AuditLog>, sqlx::Error> {
    provider.audit().list_filtered(filter).await
}
