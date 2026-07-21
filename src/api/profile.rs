use crate::db::repository::traits::AuditRepositoryExt;
use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    error::{AppError, Result},
    identity::users as identity_users,
    middleware::{audit::AuditContext, auth::AuthUser},
    security::passwords,
    state::AppState,
};

/// GET /api/v1/profile
pub async fn get_profile(State(state): State<AppState>, auth: AuthUser) -> Result<Json<Value>> {
    let profile = state
        .provider
        .users()
        .get_profile(&auth.user.id)
        .await
        .map_err(AppError::Database)?;
    let user_roles = state.provider.roles().list_for_user(&auth.user.id).await?;
    let sessions = state
        .provider
        .sessions()
        .list_active_for_user(&auth.user.id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(json!({
        "user": {
            "id": auth.user.id,
            "username": auth.user.username,
            "status": auth.user.status().to_string(),
            "last_login_at": auth.user.last_login_at,
            "created_at": auth.user.created_at,
        },
        "profile": {
            "email": profile.as_ref().and_then(|p| p.email.clone()),
            "full_name": profile.as_ref().and_then(|p| p.full_name.clone()),
            "avatar_url": profile.as_ref().and_then(|p| p.avatar_url.clone()),
        },
        "roles": user_roles.into_iter().map(|r| r.name).collect::<Vec<_>>(),
        "sessions": sessions.into_iter().map(|s| json!({
            "id": s.id,
            "ip_address": s.ip_address,
            "user_agent": s.user_agent,
            "created_at": s.created_at,
            "last_seen_at": s.last_seen_at,
            "expires_at": s.expires_at,
        })).collect::<Vec<_>>(),
        "placeholders": {
            "avatar": "coming_soon",
            "mfa": "coming_soon",
            "recovery_codes": "coming_soon",
        },
    })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub email: Option<String>,
    pub full_name: Option<String>,
}

/// PATCH /api/v1/profile
pub async fn update_profile(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Json(body): Json<UpdateProfileRequest>,
) -> Result<Json<Value>> {
    let profile = state
        .provider
        .users()
        .upsert_profile(
            &auth.user.id,
            body.email.as_deref(),
            body.full_name.as_deref(),
        )
        .await
        .map_err(AppError::Database)?;

    state
        .provider
        .audit()
        .log(crate::audit::AuditEvent {
            actor_id: Some(&auth.user.id),
            target_id: Some(&auth.user.id),
            action: "profile_updated",
            resource_type: "user",
            resource_id: Some(&auth.user.id),
            severity: crate::db::models::AuditSeverity::Info,
            ip: ctx.ip_address.as_deref(),
            ua: ctx.user_agent.as_deref(),
            metadata: None,
        })
        .await?;

    Ok(Json(json!({
        "profile": {
            "email": profile.email,
            "full_name": profile.full_name,
            "avatar_url": profile.avatar_url,
        }
    })))
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// POST /api/v1/profile/password
pub async fn change_password(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Json(body): Json<ChangePasswordRequest>,
) -> Result<Json<Value>> {
    // Verify current password
    let ok = passwords::verify_password(&body.current_password, &auth.user.password_hash)?;
    if !ok {
        return Err(AppError::Unauthorized);
    }

    identity_users::reset_password(
        &state.provider,
        &state.config.security,
        &auth.user.id,
        &body.new_password,
        Some(&auth.user.id),
        ctx.ip_address.as_deref(),
        ctx.user_agent.as_deref(),
    )
    .await?;

    Ok(Json(json!({ "success": true })))
}
