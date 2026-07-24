//! Application membership domain logic.
//!
//! Assigns existing NX9-Auth users to registered applications.
//! Membership roles (owner/admin/member) are lightweight metadata only and
//! MUST NOT grant global RBAC permissions such as `applications:manage`.

use crate::db::models::{Application, ApplicationMember, ApplicationMembershipRole};
use crate::error::AppError;
use uuid::Uuid;

fn parse_role(role: Option<&str>) -> Result<ApplicationMembershipRole, AppError> {
    match role {
        None | Some("") => Ok(ApplicationMembershipRole::Member),
        Some(r) => ApplicationMembershipRole::parse(r).ok_or_else(|| {
            AppError::InvalidInput(format!(
                "invalid membership role '{r}'; allowed values are owner, admin, member"
            ))
        }),
    }
}

/// List members of an application.
pub async fn list_by_application(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    application_id: &str,
) -> Result<Vec<ApplicationMember>, AppError> {
    // Ensure application exists
    let _ = provider
        .applications()
        .find_by_id(application_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    provider
        .application_members()
        .list_by_application(application_id)
        .await
        .map_err(AppError::Database)
}

/// List application memberships for a user.
pub async fn list_by_user(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    user_id: &str,
) -> Result<Vec<ApplicationMember>, AppError> {
    let _ = provider
        .users()
        .find_by_id(user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    provider
        .application_members()
        .list_by_user(user_id)
        .await
        .map_err(AppError::Database)
}

/// Find a single membership.
pub async fn find(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    application_id: &str,
    user_id: &str,
) -> Result<Option<ApplicationMember>, AppError> {
    provider
        .application_members()
        .find(application_id, user_id)
        .await
        .map_err(AppError::Database)
}

/// Assign an existing same-tenant user to an application.
pub async fn add(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    application_id: &str,
    user_id: &str,
    role: Option<&str>,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<ApplicationMember, AppError> {
    let role = parse_role(role)?;

    let app = provider
        .applications()
        .find_by_id(application_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    let user = provider
        .users()
        .find_by_id(user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // Tenant isolation: never allow cross-tenant assignment.
    if user.tenant_id != app.tenant_id {
        return Err(AppError::NotFound);
    }

    if provider
        .application_members()
        .find(application_id, user_id)
        .await
        .map_err(AppError::Database)?
        .is_some()
    {
        return Err(AppError::Conflict(
            "user is already a member of this application".into(),
        ));
    }

    let id = Uuid::new_v4().to_string();

    let metadata = serde_json::json!({
        "application_id": application_id,
        "user_id": user_id,
        "role": role.as_str(),
    })
    .to_string();

    let audit_event = crate::audit::AuditEvent {
        actor_id: audit_actor_id,
        target_id: Some(user_id),
        action: "application.member_added",
        resource_type: "application",
        resource_id: Some(application_id),
        severity: crate::db::models::AuditSeverity::Info,
        ip: audit_ip,
        ua: audit_ua,
        metadata: Some(&metadata),
    };

    let member = provider
        .application_members()
        .add_with_audit(
            &id,
            application_id,
            user_id,
            role.as_str(),
            Some(audit_event),
        )
        .await
        .map_err(AppError::Database)?;

    let _ = app;
    Ok(member)
}

/// Update membership role and/or enabled state.
#[allow(clippy::too_many_arguments)]
pub async fn update(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    application_id: &str,
    user_id: &str,
    role: Option<&str>,
    enabled: Option<bool>,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<ApplicationMember, AppError> {
    if role.is_none() && enabled.is_none() {
        return Err(AppError::InvalidInput(
            "at least one of role or enabled must be provided".into(),
        ));
    }

    // Ensure application exists
    let _ = provider
        .applications()
        .find_by_id(application_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    let existing = provider
        .application_members()
        .find(application_id, user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    if let Some(role_str) = role {
        let new_role = parse_role(Some(role_str))?;
        if new_role.as_str() != existing.role {
            let previous_role = existing.role.clone();
            let metadata = serde_json::json!({
                "application_id": application_id,
                "user_id": user_id,
                "previous_role": previous_role,
                "new_role": new_role.as_str(),
            })
            .to_string();

            let audit_event = crate::audit::AuditEvent {
                actor_id: audit_actor_id,
                target_id: Some(user_id),
                action: "application.member_role_changed",
                resource_type: "application",
                resource_id: Some(application_id),
                severity: crate::db::models::AuditSeverity::Info,
                ip: audit_ip,
                ua: audit_ua,
                metadata: Some(&metadata),
            };

            provider
                .application_members()
                .update_role_with_audit(
                    application_id,
                    user_id,
                    new_role.as_str(),
                    Some(audit_event),
                )
                .await
                .map_err(AppError::Database)?;
        }
    }

    if let Some(new_enabled) = enabled {
        if new_enabled != existing.enabled {
            let action = if new_enabled {
                "application.member_enabled"
            } else {
                "application.member_disabled"
            };

            let metadata = serde_json::json!({
                "application_id": application_id,
                "user_id": user_id,
                "role": existing.role,
                "enabled": new_enabled,
            })
            .to_string();

            let audit_event = crate::audit::AuditEvent {
                actor_id: audit_actor_id,
                target_id: Some(user_id),
                action,
                resource_type: "application",
                resource_id: Some(application_id),
                severity: crate::db::models::AuditSeverity::Info,
                ip: audit_ip,
                ua: audit_ua,
                metadata: Some(&metadata),
            };

            provider
                .application_members()
                .set_enabled_with_audit(application_id, user_id, new_enabled, Some(audit_event))
                .await
                .map_err(AppError::Database)?;
        }
    }

    provider
        .application_members()
        .find(application_id, user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)
}

/// Remove a user from an application (does not delete the user account).
pub async fn remove(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    application_id: &str,
    user_id: &str,
    audit_actor_id: Option<&str>,
    audit_ip: Option<&str>,
    audit_ua: Option<&str>,
) -> Result<(), AppError> {
    // Ensure application exists
    let _ = provider
        .applications()
        .find_by_id(application_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    let existing = provider
        .application_members()
        .find(application_id, user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    let metadata = serde_json::json!({
        "application_id": application_id,
        "user_id": user_id,
        "role": existing.role,
    })
    .to_string();

    let audit_event = crate::audit::AuditEvent {
        actor_id: audit_actor_id,
        target_id: Some(user_id),
        action: "application.member_removed",
        resource_type: "application",
        resource_id: Some(application_id),
        severity: crate::db::models::AuditSeverity::Info,
        ip: audit_ip,
        ua: audit_ua,
        metadata: Some(&metadata),
    };

    provider
        .application_members()
        .remove_with_audit(application_id, user_id, Some(audit_event))
        .await
        .map_err(AppError::Database)?;

    Ok(())
}

/// Helper used by API responses that need application details for a membership.
pub async fn load_application(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    application_id: &str,
) -> Result<Application, AppError> {
    provider
        .applications()
        .find_by_id(application_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)
}
