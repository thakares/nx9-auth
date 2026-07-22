use crate::db::repository::traits::AuditRepositoryExt;

use crate::{config::SecurityConfig, db::models::User, error::AppError, security::passwords};

/// Create a new user account in the given tenant.
///
/// Fails with `Conflict` if the username is already taken.
#[allow(clippy::too_many_arguments)]
pub async fn create_user(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    cfg: &SecurityConfig,
    tenant_id: &str,
    username: &str,
    password: &str,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<User, AppError> {
    if username.trim().is_empty() {
        return Err(AppError::InvalidInput("username cannot be empty".into()));
    }
    passwords::validate_password_strength(password, false)?;

    if provider
        .users()
        .username_exists(tenant_id, username)
        .await
        .map_err(AppError::Database)?
    {
        return Err(AppError::Conflict(format!(
            "username '{username}' is already taken"
        )));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let hash = passwords::hash_password(password, cfg)?;

    let user = provider
        .users()
        .create(&id, tenant_id, username, &hash)
        .await
        .map_err(AppError::Database)?;

    provider
        .audit()
        .log(crate::audit::AuditEvent {
            actor_id: audit_actor_id,
            target_id: Some(&user.id),
            action: "user_created",
            resource_type: "user",
            resource_id: Some(&user.id),
            severity: crate::db::models::AuditSeverity::Info,
            ip: audit_ip,
            ua: audit_ua,
            metadata: None,
        })
        .await?;

    tracing::info!(user_id = %user.id, username = %username, "user created");
    Ok(user)
}

/// Retrieve a user by ID.
pub async fn get_user(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    id: &str,
) -> Result<User, AppError> {
    provider
        .users()
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)
}

/// Retrieve a user by username.
pub async fn get_user_by_username(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    username: &str,
) -> Result<User, AppError> {
    provider
        .users()
        .find_by_username(username)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)
}

/// List all users in a tenant.
pub async fn list_users(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    tenant_id: &str,
) -> Result<Vec<User>, AppError> {
    provider
        .users()
        .list(tenant_id)
        .await
        .map_err(AppError::Database)
}

/// Set a user's status (Active=1, Disabled=2, Locked=3).
pub async fn update_status(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    user_id: &str,
    status: i32,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<(), AppError> {
    // Verify user exists first
    let _user = provider.users().find_by_id(user_id).await?;

    provider
        .users()
        .update_status(user_id, status)
        .await
        .map_err(AppError::Database)?;

    let action = match status {
        1 => "user_enabled",
        2 => "user_disabled",
        3 => "user_locked",
        _ => "user_updated",
    };

    let severity = match status {
        1 => crate::db::models::AuditSeverity::Info,
        _ => crate::db::models::AuditSeverity::Warning,
    };

    provider
        .audit()
        .log(crate::audit::AuditEvent {
            actor_id: audit_actor_id,
            target_id: Some(user_id),
            action,
            resource_type: "user",
            resource_id: Some(user_id),
            severity,
            ip: audit_ip,
            ua: audit_ua,
            metadata: None,
        })
        .await?;

    tracing::info!(user_id = %user_id, status = %status, "user status updated");
    Ok(())
}

/// Reset a user's password.
pub async fn reset_password(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    cfg: &SecurityConfig,
    user_id: &str,
    new_password: &str,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<(), AppError> {
    let user = provider
        .users()
        .find_by_id(user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;
    let user_roles = provider
        .roles()
        .list_for_user(&user.id)
        .await
        .map_err(AppError::Database)?;
    let is_admin = user_roles.iter().any(|r| r.name == "admin");
    passwords::validate_password_strength(new_password, is_admin)?;
    let hash = passwords::hash_password(new_password, cfg)?;

    provider
        .users()
        .update_password_hash(user_id, &hash)
        .await
        .map_err(AppError::Database)?;

    provider
        .audit()
        .log(crate::audit::AuditEvent {
            actor_id: audit_actor_id,
            target_id: Some(user_id),
            action: "password_reset",
            resource_type: "user",
            resource_id: Some(user_id),
            severity: crate::db::models::AuditSeverity::Warning,
            ip: audit_ip,
            ua: audit_ua,
            metadata: None,
        })
        .await?;

    tracing::info!(user_id = %user_id, "password reset");
    Ok(())
}
