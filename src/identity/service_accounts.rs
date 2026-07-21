use crate::db::repository::traits::AuditRepositoryExt;

use crate::{db::models::ServiceAccount, error::AppError};

pub async fn create(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    tenant_id: &str,
    name: &str,
    description: Option<&str>,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<ServiceAccount, AppError> {
    let id = uuid::Uuid::new_v4().to_string();

    let sa = provider
        .service_accounts()
        .create(&id, tenant_id, name, description)
        .await
        .map_err(AppError::Database)?;

    provider
        .audit()
        .log(crate::audit::AuditEvent {
            actor_id: audit_actor_id,
            target_id: None,
            action: "service_account_created",
            resource_type: "service_account",
            resource_id: Some(&sa.id),
            severity: crate::db::models::AuditSeverity::Info,
            ip: audit_ip,
            ua: audit_ua,
            metadata: None,
        })
        .await?;

    Ok(sa)
}

pub async fn list(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    tenant_id: &str,
) -> Result<Vec<ServiceAccount>, AppError> {
    provider
        .service_accounts()
        .list(tenant_id)
        .await
        .map_err(AppError::Database)
}

pub async fn set_enabled(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    id: &str,
    enabled: bool,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<(), AppError> {
    let _ = provider.service_accounts().find_by_id(id).await?;

    provider
        .service_accounts()
        .set_enabled(id, enabled)
        .await
        .map_err(AppError::Database)?;

    let action = if enabled {
        "service_account_enabled"
    } else {
        "service_account_disabled"
    };

    provider
        .audit()
        .log(crate::audit::AuditEvent {
            actor_id: audit_actor_id,
            target_id: None,
            action,
            resource_type: "service_account",
            resource_id: Some(id),
            severity: crate::db::models::AuditSeverity::Warning,
            ip: audit_ip,
            ua: audit_ua,
            metadata: None,
        })
        .await?;

    Ok(())
}

pub async fn get(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    id: &str,
) -> Result<ServiceAccount, AppError> {
    provider
        .service_accounts()
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)
}

pub async fn delete(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    id: &str,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<(), AppError> {
    let _ = provider.service_accounts().find_by_id(id).await?;

    provider
        .service_accounts()
        .delete(id)
        .await
        .map_err(AppError::Database)?;

    provider
        .audit()
        .log(crate::audit::AuditEvent {
            actor_id: audit_actor_id,
            target_id: None,
            action: "service_account_deleted",
            resource_type: "service_account",
            resource_id: Some(id),
            severity: crate::db::models::AuditSeverity::Warning,
            ip: audit_ip,
            ua: audit_ua,
            metadata: None,
        })
        .await?;

    Ok(())
}

/// Generate a one-time display secret for a service account.
///
/// The raw secret is returned once; only a BLAKE3 hash is stored in audit metadata
/// until a dedicated secrets table is introduced (OAuth2 milestone).
pub async fn generate_secret(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    id: &str,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<String, AppError> {
    let _ = provider.service_accounts().find_by_id(id).await?;

    let mut bytes = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
    let raw = format!("nx9sa_{}", hex::encode(bytes));
    let hash = hex::encode(blake3::hash(raw.as_bytes()).as_bytes());

    let metadata = serde_json::json!({ "secret_hash": hash }).to_string();
    provider
        .audit()
        .log(crate::audit::AuditEvent {
            actor_id: audit_actor_id,
            target_id: None,
            action: "service_account_secret_rotated",
            resource_type: "service_account",
            resource_id: Some(id),
            severity: crate::db::models::AuditSeverity::Warning,
            ip: audit_ip,
            ua: audit_ua,
            metadata: Some(&metadata),
        })
        .await?;

    Ok(raw)
}
