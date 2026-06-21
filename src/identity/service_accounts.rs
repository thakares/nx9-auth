use sqlx::SqlitePool;

use crate::{
    db::{models::ServiceAccount, repository::service_accounts as repo},
    error::AppError,
};

pub async fn create(
    pool: &SqlitePool,
    tenant_id: &str,
    name: &str,
    description: Option<&str>,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<ServiceAccount, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    let sa = repo::create(&mut tx, &id, tenant_id, name, description)
        .await
        .map_err(AppError::Database)?;

    crate::audit::log(
        &mut tx,
        crate::audit::AuditEvent {
            actor_id: audit_actor_id,
            target_id: None,
            action: "service_account_created",
            resource_type: "service_account",
            resource_id: Some(&sa.id),
            severity: crate::db::models::AuditSeverity::Info,
            ip: audit_ip,
            ua: audit_ua,
            metadata: None,
        },
    )
    .await?;

    tx.commit().await.map_err(AppError::Database)?;
    Ok(sa)
}

pub async fn list(pool: &SqlitePool, tenant_id: &str) -> Result<Vec<ServiceAccount>, AppError> {
    repo::list(pool, tenant_id)
        .await
        .map_err(AppError::Database)
}

pub async fn set_enabled(
    pool: &SqlitePool,
    id: &str,
    enabled: bool,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    repo::set_enabled(&mut tx, id, enabled)
        .await
        .map_err(AppError::Database)?;

    let action = if enabled {
        "service_account_enabled"
    } else {
        "service_account_disabled"
    };

    crate::audit::log(
        &mut tx,
        crate::audit::AuditEvent {
            actor_id: audit_actor_id,
            target_id: None,
            action,
            resource_type: "service_account",
            resource_id: Some(id),
            severity: crate::db::models::AuditSeverity::Warning,
            ip: audit_ip,
            ua: audit_ua,
            metadata: None,
        },
    )
    .await?;

    tx.commit().await.map_err(AppError::Database)?;
    Ok(())
}
