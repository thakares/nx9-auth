use crate::db::repository::traits::AuditRepositoryExt;
use crate::{db::models::Application, error::AppError};
use subtle::ConstantTimeEq;

pub const CLIENT_ID_PREFIX: &str = "nx9_app_";
pub const CLIENT_SECRET_PREFIX: &str = "nx9_secret_";

/// Generate a new unique server-side Client ID.
pub fn generate_client_id() -> String {
    let mut bytes = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
    format!("{}{}", CLIENT_ID_PREFIX, hex::encode(bytes))
}

/// Generate a new CSPRNG Client Secret.
pub fn generate_client_secret() -> String {
    let mut bytes = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
    format!("{}{}", CLIENT_SECRET_PREFIX, hex::encode(bytes))
}

/// Hash a raw client secret string into a hex-encoded BLAKE3 digest.
pub fn hash_client_secret(raw: &str) -> String {
    hex::encode(blake3::hash(raw.as_bytes()).as_bytes())
}

/// Hash a raw client secret string into a 32-byte BLAKE3 digest.
pub fn hash_secret_bytes(raw: &str) -> [u8; 32] {
    *blake3::hash(raw.as_bytes()).as_bytes()
}

/// Constant-time byte array comparison using subtle::ConstantTimeEq.
pub fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}

pub fn validate_redirect_uris(uris: &[String]) -> Result<(), AppError> {
    if uris.len() > 10 {
        return Err(AppError::InvalidInput(
            "maximum 10 redirect URIs allowed".into(),
        ));
    }
    for uri in uris {
        let trimmed = uri.trim();
        if trimmed.is_empty() {
            return Err(AppError::InvalidInput(
                "redirect URI cannot be empty".into(),
            ));
        }
        if trimmed.len() > 512 {
            return Err(AppError::InvalidInput(
                "redirect URI exceeds maximum length of 512 characters".into(),
            ));
        }
        let parsed = url::Url::parse(trimmed).map_err(|e| {
            AppError::InvalidInput(format!("invalid redirect URI '{trimmed}': {e}"))
        })?;
        if parsed.fragment().is_some() {
            return Err(AppError::InvalidInput(format!(
                "redirect URI '{trimmed}' must not contain a fragment"
            )));
        }
        if !parsed.username().is_empty() || parsed.password().is_some() {
            return Err(AppError::InvalidInput(format!(
                "redirect URI '{trimmed}' must not contain user credentials"
            )));
        }
        match parsed.scheme() {
            "https" => {}
            "http" => {
                let host = parsed.host_str().unwrap_or("");
                if host != "localhost" && host != "127.0.0.1" && host != "[::1]" && host != "::1" {
                    return Err(AppError::InvalidInput(format!(
                        "redirect URI '{trimmed}' with http scheme is only allowed for localhost/loopback development"
                    )));
                }
            }
            other => {
                return Err(AppError::InvalidInput(format!(
                    "redirect URI '{trimmed}' has unsupported scheme '{other}'; only https (or http for localhost) is allowed"
                )));
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn create(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    tenant_id: &str,
    name: &str,
    slug: &str,
    description: Option<&str>,
    redirect_uris: Option<Vec<String>>,
    scopes: Option<Vec<String>>,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<(Application, String), AppError> {
    let name = name.trim();
    let slug = slug.trim();
    if name.is_empty() || slug.is_empty() {
        return Err(AppError::InvalidInput(
            "name and slug cannot be empty".into(),
        ));
    }
    if let Some(ref uris) = redirect_uris {
        validate_redirect_uris(uris)?;
    }
    if provider
        .applications()
        .find_by_slug(slug)
        .await
        .map_err(AppError::Database)?
        .is_some()
    {
        return Err(AppError::Conflict(format!("slug '{slug}' already exists")));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let client_id = generate_client_id();
    let raw_secret = generate_client_secret();
    let secret_hash = hash_client_secret(&raw_secret);

    let redirect_json = redirect_uris.map(|v| serde_json::to_string(&v).unwrap_or_default());
    let scopes_json = scopes.map(|v| serde_json::to_string(&v).unwrap_or_default());

    let metadata = serde_json::json!({
        "application_id": id,
        "name": name,
        "client_id": client_id,
    })
    .to_string();

    let audit_event = crate::audit::AuditEvent {
        actor_id: audit_actor_id,
        target_id: None,
        action: "application.created",
        resource_type: "application",
        resource_id: Some(&id),
        severity: crate::db::models::AuditSeverity::Info,
        ip: audit_ip,
        ua: audit_ua,
        metadata: Some(&metadata),
    };

    let app = provider
        .applications()
        .create_with_audit(
            &id,
            tenant_id,
            name,
            slug,
            &client_id,
            Some(&secret_hash),
            description,
            redirect_json.as_deref(),
            scopes_json.as_deref(),
            Some(audit_event),
        )
        .await
        .map_err(AppError::Database)?;

    Ok((app, raw_secret))
}

pub async fn list(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    tenant_id: &str,
) -> Result<Vec<Application>, AppError> {
    provider
        .applications()
        .list(tenant_id)
        .await
        .map_err(AppError::Database)
}

pub async fn get(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    id: &str,
) -> Result<Application, AppError> {
    provider
        .applications()
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)
}

pub async fn find_by_slug(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    slug: &str,
) -> Result<Application, AppError> {
    provider
        .applications()
        .find_by_slug(slug)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)
}

pub async fn rotate_secret(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    id: &str,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<String, AppError> {
    let app = provider
        .applications()
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    let raw_secret = generate_client_secret();
    let secret_hash = hash_client_secret(&raw_secret);

    let metadata = serde_json::json!({
        "application_id": id,
        "name": app.name,
        "client_id": app.get_client_id(),
    })
    .to_string();

    let audit_event = crate::audit::AuditEvent {
        actor_id: audit_actor_id,
        target_id: None,
        action: "application.secret_rotated",
        resource_type: "application",
        resource_id: Some(id),
        severity: crate::db::models::AuditSeverity::Warning,
        ip: audit_ip,
        ua: audit_ua,
        metadata: Some(&metadata),
    };

    provider
        .applications()
        .rotate_secret_with_audit(id, &secret_hash, Some(audit_event))
        .await
        .map_err(AppError::Database)?;

    Ok(raw_secret)
}

pub async fn validate_client_credentials(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    client_id: &str,
    client_secret: &str,
) -> Result<Application, AppError> {
    let supplied_digest = hash_secret_bytes(client_secret);
    let app = provider
        .applications()
        .find_by_client_id(client_id)
        .await
        .map_err(AppError::Database)?;

    let dummy_digest = [0u8; 32];

    let (valid_app, stored_digest_opt) = match app {
        Some(ref a) if a.enabled => {
            let digest_opt = a
                .client_secret_hash
                .as_ref()
                .and_then(|h| hex::decode(h).ok())
                .and_then(|vec| <[u8; 32]>::try_from(vec).ok());
            (digest_opt.is_some(), digest_opt)
        }
        _ => (false, None),
    };

    let target_digest = stored_digest_opt.as_ref().unwrap_or(&dummy_digest);
    let matches = constant_time_compare(&supplied_digest, target_digest);

    if valid_app && matches {
        Ok(app.unwrap())
    } else {
        Err(AppError::Unauthorized)
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn update(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    id: &str,
    name: &str,
    slug: &str,
    description: Option<&str>,
    redirect_uris: Option<Vec<String>>,
    scopes: Option<Vec<String>>,
    enabled: bool,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<Application, AppError> {
    let existing = provider
        .applications()
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    let name = name.trim();
    let slug = slug.trim();
    if name.is_empty() || slug.is_empty() {
        return Err(AppError::InvalidInput(
            "name and slug cannot be empty".into(),
        ));
    }
    if let Some(ref uris) = redirect_uris {
        validate_redirect_uris(uris)?;
    }
    if let Some(other) = provider
        .applications()
        .find_by_slug(slug)
        .await
        .map_err(AppError::Database)?
    {
        if other.id != id {
            return Err(AppError::Conflict(format!("slug '{slug}' already exists")));
        }
    }

    let redirect_json = redirect_uris.map(|v| serde_json::to_string(&v).unwrap_or_default());
    let scopes_json = scopes.map(|v| serde_json::to_string(&v).unwrap_or_default());

    provider
        .applications()
        .update(
            id,
            name,
            slug,
            description,
            redirect_json.as_deref(),
            scopes_json.as_deref(),
            enabled,
        )
        .await
        .map_err(AppError::Database)?;

    let updated = provider
        .applications()
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    let action = if existing.enabled != enabled {
        if enabled {
            "application.enabled"
        } else {
            "application.disabled"
        }
    } else {
        "application.updated"
    };

    let metadata = serde_json::json!({
        "application_id": id,
        "name": updated.name,
        "client_id": updated.get_client_id(),
        "enabled": enabled,
    })
    .to_string();

    provider
        .audit()
        .log(crate::audit::AuditEvent {
            actor_id: audit_actor_id,
            target_id: None,
            action,
            resource_type: "application",
            resource_id: Some(id),
            severity: crate::db::models::AuditSeverity::Info,
            ip: audit_ip,
            ua: audit_ua,
            metadata: Some(&metadata),
        })
        .await?;

    Ok(updated)
}

pub async fn set_enabled(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    id: &str,
    enabled: bool,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<(), AppError> {
    let app = provider
        .applications()
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    provider
        .applications()
        .set_enabled(id, enabled)
        .await
        .map_err(AppError::Database)?;

    let action = if enabled {
        "application.enabled"
    } else {
        "application.disabled"
    };

    let metadata = serde_json::json!({
        "application_id": id,
        "name": app.name,
        "client_id": app.get_client_id(),
        "enabled": enabled,
    })
    .to_string();

    provider
        .audit()
        .log(crate::audit::AuditEvent {
            actor_id: audit_actor_id,
            target_id: None,
            action,
            resource_type: "application",
            resource_id: Some(id),
            severity: crate::db::models::AuditSeverity::Info,
            ip: audit_ip,
            ua: audit_ua,
            metadata: Some(&metadata),
        })
        .await?;

    Ok(())
}

pub async fn delete(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    id: &str,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<(), AppError> {
    let app = provider
        .applications()
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    provider
        .applications()
        .delete(id)
        .await
        .map_err(AppError::Database)?;

    let metadata = serde_json::json!({
        "application_id": id,
        "name": app.name,
        "client_id": app.get_client_id(),
    })
    .to_string();

    provider
        .audit()
        .log(crate::audit::AuditEvent {
            actor_id: audit_actor_id,
            target_id: None,
            action: "application.deleted",
            resource_type: "application",
            resource_id: Some(id),
            severity: crate::db::models::AuditSeverity::Warning,
            ip: audit_ip,
            ua: audit_ua,
            metadata: Some(&metadata),
        })
        .await?;

    Ok(())
}
