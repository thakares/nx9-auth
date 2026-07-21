use crate::db::repository::traits::AuditRepositoryExt;

use crate::{db::models::Role, error::AppError};

/// Assign a named role to a user. No-ops if already assigned.
pub async fn assign_role(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    user_id: &str,
    role_name: &str,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<(), AppError> {
    let role = provider
        .roles()
        .find_by_name(role_name)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    provider
        .roles()
        .assign_to_user(user_id, &role.id)
        .await
        .map_err(AppError::Database)?;

    let metadata = serde_json::json!({ "role": role_name }).to_string();
    provider
        .audit()
        .log(crate::audit::AuditEvent {
            actor_id: audit_actor_id,
            target_id: Some(user_id),
            action: "role_assigned",
            resource_type: "role",
            resource_id: Some(&role.id),
            severity: crate::db::models::AuditSeverity::Info,
            ip: audit_ip,
            ua: audit_ua,
            metadata: Some(&metadata),
        })
        .await?;

    tracing::info!(user_id = %user_id, role = %role_name, "role assigned");
    Ok(())
}

/// Remove a named role from a user. No-ops if not assigned.
pub async fn remove_role(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    user_id: &str,
    role_name: &str,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<(), AppError> {
    let role = provider
        .roles()
        .find_by_name(role_name)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    provider
        .roles()
        .remove_from_user(user_id, &role.id)
        .await
        .map_err(AppError::Database)?;

    let metadata = serde_json::json!({ "role": role_name }).to_string();
    provider
        .audit()
        .log(crate::audit::AuditEvent {
            actor_id: audit_actor_id,
            target_id: Some(user_id),
            action: "role_removed",
            resource_type: "role",
            resource_id: Some(&role.id),
            severity: crate::db::models::AuditSeverity::Info,
            ip: audit_ip,
            ua: audit_ua,
            metadata: Some(&metadata),
        })
        .await?;

    tracing::info!(user_id = %user_id, role = %role_name, "role removed");
    Ok(())
}

/// List all roles defined in the system.
pub async fn list_roles(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
) -> Result<Vec<Role>, AppError> {
    provider
        .roles()
        .list_all()
        .await
        .map_err(AppError::Database)
}

/// List roles held by a specific user.
pub async fn list_user_roles(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    user_id: &str,
) -> Result<Vec<Role>, AppError> {
    provider
        .roles()
        .list_for_user(user_id)
        .await
        .map_err(AppError::Database)
}

/// Create a new role.
pub async fn create_role(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    name: &str,
    description: Option<&str>,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<Role, AppError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::InvalidInput("role name cannot be empty".into()));
    }
    if provider
        .roles()
        .find_by_name(name)
        .await
        .map_err(AppError::Database)?
        .is_some()
    {
        return Err(AppError::Conflict(format!("role '{name}' already exists")));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let role = provider
        .roles()
        .create(&id, name, description)
        .await
        .map_err(AppError::Database)?;

    provider
        .audit()
        .log(crate::audit::AuditEvent {
            actor_id: audit_actor_id,
            target_id: None,
            action: "role_created",
            resource_type: "role",
            resource_id: Some(&role.id),
            severity: crate::db::models::AuditSeverity::Info,
            ip: audit_ip,
            ua: audit_ua,
            metadata: Some(&format!(r#"{{"name":"{name}"}}"#)),
        })
        .await?;

    Ok(role)
}

/// Update an existing role.
pub async fn update_role(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    id: &str,
    name: &str,
    description: Option<&str>,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<Role, AppError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::InvalidInput("role name cannot be empty".into()));
    }

    let existing = provider
        .roles()
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // Protect built-in admin role rename
    if existing.name == "admin" && name != "admin" {
        return Err(AppError::InvalidInput(
            "cannot rename the built-in admin role".into(),
        ));
    }

    if let Some(other) = provider
        .roles()
        .find_by_name(name)
        .await
        .map_err(AppError::Database)?
    {
        if other.id != id {
            return Err(AppError::Conflict(format!("role '{name}' already exists")));
        }
    }

    provider
        .roles()
        .update(id, name, description)
        .await
        .map_err(AppError::Database)?;

    provider
        .audit()
        .log(crate::audit::AuditEvent {
            actor_id: audit_actor_id,
            target_id: None,
            action: "role_updated",
            resource_type: "role",
            resource_id: Some(id),
            severity: crate::db::models::AuditSeverity::Info,
            ip: audit_ip,
            ua: audit_ua,
            metadata: None,
        })
        .await?;

    provider
        .roles()
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)
}

/// Delete a role (cannot delete admin).
pub async fn delete_role(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    id: &str,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<(), AppError> {
    let role = provider
        .roles()
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    if role.name == "admin" {
        return Err(AppError::InvalidInput(
            "cannot delete the built-in admin role".into(),
        ));
    }

    provider
        .roles()
        .delete(id)
        .await
        .map_err(AppError::Database)?;

    provider
        .audit()
        .log(crate::audit::AuditEvent {
            actor_id: audit_actor_id,
            target_id: None,
            action: "role_deleted",
            resource_type: "role",
            resource_id: Some(id),
            severity: crate::db::models::AuditSeverity::Warning,
            ip: audit_ip,
            ua: audit_ua,
            metadata: Some(&format!(r#"{{"name":"{}"}}"#, role.name)),
        })
        .await?;

    Ok(())
}

/// Get a role by id.
pub async fn get_role(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    id: &str,
) -> Result<Role, AppError> {
    provider
        .roles()
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)
}
