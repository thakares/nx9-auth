use crate::db::repository::traits::AuditRepositoryExt;

use crate::{db::models::Permission, error::AppError};

/// Return all permission names held by a user.
pub async fn list_user_permissions(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    user_id: &str,
) -> Result<Vec<String>, AppError> {
    provider
        .permissions()
        .list_for_user(user_id)
        .await
        .map_err(AppError::Database)
}

/// List all system permissions.
pub async fn list_permissions(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
) -> Result<Vec<Permission>, AppError> {
    provider
        .permissions()
        .list_all()
        .await
        .map_err(AppError::Database)
}

/// List permissions for a role.
pub async fn list_role_permissions(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    role_id: &str,
) -> Result<Vec<Permission>, AppError> {
    provider
        .permissions()
        .list_for_role(role_id)
        .await
        .map_err(AppError::Database)
}

/// Set the full permission set for a role (replace semantics).
pub async fn set_role_permissions(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    role_id: &str,
    permission_names: &[String],
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<Vec<Permission>, AppError> {
    // Resolve names → ids
    let mut permission_ids = Vec::with_capacity(permission_names.len());
    for name in permission_names {
        let perm = provider
            .permissions()
            .find_by_name(name)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::InvalidInput(format!("unknown permission: {name}")))?;
        permission_ids.push(perm.id);
    }

    // Clear existing assignments
    provider
        .permissions()
        .clear_for_role(role_id)
        .await
        .map_err(AppError::Database)?;

    for pid in &permission_ids {
        provider
            .permissions()
            .assign_to_role(role_id, pid)
            .await
            .map_err(AppError::Database)?;
    }

    let metadata = serde_json::json!({ "permissions": permission_names }).to_string();
    provider
        .audit()
        .log(crate::audit::AuditEvent {
            actor_id: audit_actor_id,
            target_id: None,
            action: "role_permissions_updated",
            resource_type: "role",
            resource_id: Some(role_id),
            severity: crate::db::models::AuditSeverity::Info,
            ip: audit_ip,
            ua: audit_ua,
            metadata: Some(&metadata),
        })
        .await?;

    list_role_permissions(provider, role_id).await
}

/// Returns true if the user holds the given named permission.
pub async fn has_permission(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    user_id: &str,
    permission: &str,
) -> Result<bool, AppError> {
    provider
        .permissions()
        .user_has_permission(user_id, permission)
        .await
        .map_err(AppError::Database)
}

/// Enforce that a user holds a permission, returning `Forbidden` otherwise.
pub async fn require_permission(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    user_id: &str,
    permission: &str,
) -> Result<(), AppError> {
    if has_permission(provider, user_id, permission).await? {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}
