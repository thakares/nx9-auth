use axum::{Json, extract::State};
use serde_json::{Value, json};

use crate::{
    db::models::{Tenant, UserStatus},
    error::{AppError, Result},
    identity::permissions as identity_perms,
    middleware::auth::AuthUser,
    state::AppState,
};

/// GET /api/v1/dashboard
///
/// Returns a role-aware dashboard payload. Admins get system summary cards;
/// all users get personal overview data.
pub async fn dashboard(State(state): State<AppState>, auth: AuthUser) -> Result<Json<Value>> {
    let roles = state.provider.roles().list_for_user(&auth.user.id).await?;
    let permissions = identity_perms::list_user_permissions(&state.provider, &auth.user.id).await?;
    let is_admin = roles.iter().any(|r| r.name == "admin")
        || permissions
            .iter()
            .any(|p| p == "roles:manage" || p == "audit:view");

    // Personal data
    let sessions = state
        .provider
        .sessions()
        .list_active_for_user(&auth.user.id)
        .await
        .map_err(AppError::Database)?;
    let session_views: Vec<Value> = sessions
        .into_iter()
        .map(|s| {
            json!({
                "id": s.id,
                "ip_address": s.ip_address,
                "user_agent": s.user_agent,
                "created_at": s.created_at,
                "last_seen_at": s.last_seen_at,
                "expires_at": s.expires_at,
            })
        })
        .collect();

    let tokens = state
        .provider
        .tokens()
        .list_for_user(&auth.user.id)
        .await
        .map_err(AppError::Database)?;
    let token_views: Vec<Value> = tokens
        .into_iter()
        .filter(|t| !t.revoked)
        .take(10)
        .map(|t| {
            json!({
                "id": t.id,
                "name": t.name,
                "expires_at": t.expires_at,
                "created_at": t.created_at,
                "last_used_at": t.last_used_at,
            })
        })
        .collect();

    let apps = state
        .provider
        .applications()
        .list(Tenant::DEFAULT_ID)
        .await
        .map_err(AppError::Database)?;
    let app_views: Vec<Value> = apps
        .into_iter()
        .filter(|a| a.enabled)
        .map(|a| {
            json!({
                "id": a.id,
                "name": a.name,
                "slug": a.slug,
            })
        })
        .collect();

    let recent_personal = state
        .provider
        .audit()
        .list_filtered(&crate::db::repository::audit::AuditFilter {
            actor_user_id: Some(auth.user.id.clone()),
            limit: 10,
            ..Default::default()
        })
        .await
        .map_err(AppError::Database)?;

    let personal = json!({
        "user": {
            "id": auth.user.id,
            "username": auth.user.username,
            "status": auth.user.status().to_string(),
            "last_login_at": auth.user.last_login_at,
            "created_at": auth.user.created_at,
        },
        "roles": roles.iter().map(|r| &r.name).collect::<Vec<_>>(),
        "permissions": permissions,
        "sessions": session_views,
        "tokens": token_views,
        "applications": app_views,
        "recent_audit": recent_personal,
    });

    let mut payload = json!({
        "personal": personal,
        "is_admin": is_admin,
    });

    if is_admin {
        let total_users = state
            .provider
            .users()
            .count(Tenant::DEFAULT_ID)
            .await
            .map_err(AppError::Database)?;
        let active_users = state
            .provider
            .users()
            .count_by_status(Tenant::DEFAULT_ID, UserStatus::Active as i32)
            .await
            .map_err(AppError::Database)?;
        let active_sessions = state
            .provider
            .sessions()
            .count_active()
            .await
            .map_err(AppError::Database)?;
        let roles_count = state
            .provider
            .roles()
            .list_all()
            .await
            .map_err(AppError::Database)?
            .len();
        let perms_count = state
            .provider
            .permissions()
            .list_all()
            .await
            .map_err(AppError::Database)?
            .len();
        let apps_count = state
            .provider
            .applications()
            .count(Tenant::DEFAULT_ID)
            .await
            .map_err(AppError::Database)?;
        let sa_count = state
            .provider
            .service_accounts()
            .count(Tenant::DEFAULT_ID)
            .await
            .map_err(AppError::Database)?;
        let audit_count = state
            .provider
            .audit()
            .count()
            .await
            .map_err(AppError::Database)?;

        let recent_audit = state
            .provider
            .audit()
            .list_recent(15)
            .await
            .map_err(AppError::Database)?;

        let recent_logins = state
            .provider
            .audit()
            .list_filtered(&crate::db::repository::audit::AuditFilter {
                action: Some("login_success".into()),
                limit: 10,
                ..Default::default()
            })
            .await
            .map_err(AppError::Database)?;

        let recent_users = state
            .provider
            .users()
            .list(Tenant::DEFAULT_ID)
            .await
            .map_err(AppError::Database)?;
        let recent_users: Vec<Value> = recent_users
            .into_iter()
            .take(10)
            .map(|u| {
                json!({
                    "id": u.id,
                    "username": u.username,
                    "status": u.status().to_string(),
                    "created_at": u.created_at,
                })
            })
            .collect();

        payload["admin"] = json!({
            "summary": {
                "total_users": total_users,
                "active_users": active_users,
                "active_sessions": active_sessions,
                "roles": roles_count,
                "permissions": perms_count,
                "applications": apps_count,
                "service_accounts": sa_count,
                "audit_events": audit_count,
            },
            "recent_logins": recent_logins,
            "recent_audit": recent_audit,
            "recent_users": recent_users,
            "system_health": {
                "status": "ok",
                "database": "connected",
                "note": "Placeholder — full health probes in a future release",
            },
        });
    }

    Ok(Json(payload))
}
