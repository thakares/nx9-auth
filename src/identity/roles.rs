use sqlx::SqlitePool;

use crate::{
    db::{models::Role, repository::roles as repo},
    error::AppError,
};

/// Assign a named role to a user. No-ops if already assigned.
pub async fn assign_role(
    pool: &SqlitePool,
    user_id: &str,
    role_name: &str,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<(), AppError> {
    let role = repo::find_by_name(pool, role_name)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    repo::assign_to_user(&mut tx, user_id, &role.id)
        .await
        .map_err(AppError::Database)?;

    let metadata = serde_json::json!({ "role": role_name }).to_string();
    crate::audit::log(
        &mut tx,
        crate::audit::AuditEvent {
            actor_id: audit_actor_id,
            target_id: Some(user_id),
            action: "role_assigned",
            resource_type: "role",
            resource_id: Some(&role.id),
            severity: crate::db::models::AuditSeverity::Info,
            ip: audit_ip,
            ua: audit_ua,
            metadata: Some(&metadata),
        },
    )
    .await?;

    tx.commit().await.map_err(AppError::Database)?;

    tracing::info!(user_id = %user_id, role = %role_name, "role assigned");
    Ok(())
}

/// Remove a named role from a user. No-ops if not assigned.
pub async fn remove_role(
    pool: &SqlitePool,
    user_id: &str,
    role_name: &str,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<(), AppError> {
    let role = repo::find_by_name(pool, role_name)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    repo::remove_from_user(&mut tx, user_id, &role.id)
        .await
        .map_err(AppError::Database)?;

    let metadata = serde_json::json!({ "role": role_name }).to_string();
    crate::audit::log(
        &mut tx,
        crate::audit::AuditEvent {
            actor_id: audit_actor_id,
            target_id: Some(user_id),
            action: "role_removed",
            resource_type: "role",
            resource_id: Some(&role.id),
            severity: crate::db::models::AuditSeverity::Info,
            ip: audit_ip,
            ua: audit_ua,
            metadata: Some(&metadata),
        },
    )
    .await?;

    tx.commit().await.map_err(AppError::Database)?;

    tracing::info!(user_id = %user_id, role = %role_name, "role removed");
    Ok(())
}

/// List all roles defined in the system.
pub async fn list_roles(pool: &SqlitePool) -> Result<Vec<Role>, AppError> {
    repo::list_all(pool).await.map_err(AppError::Database)
}

/// List roles held by a specific user.
pub async fn list_user_roles(pool: &SqlitePool, user_id: &str) -> Result<Vec<Role>, AppError> {
    repo::list_for_user(pool, user_id)
        .await
        .map_err(AppError::Database)
}
