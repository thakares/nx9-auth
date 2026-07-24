use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    db::models::Tenant,
    db::models::{User, UserStatus},
    error::{AppError, Result},
    identity::{application_members as members, users as identity},
    middleware::{audit::AuditContext, auth::AuthUser, permissions::require},
    state::AppState,
};

// ── Response type ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub tenant_id: String,
    pub status: String,
    pub last_login_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id.clone(),
            username: u.username,
            tenant_id: u.tenant_id,
            status: UserStatus::from_i32(u.status).to_string(),
            last_login_at: u.last_login_at,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

// ── GET /api/v1/users ─────────────────────────────────────────────────────────

pub async fn list_users(State(state): State<AppState>, auth: AuthUser) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "users:create").await?;

    let users = identity::list_users(&state.provider, Tenant::DEFAULT_ID).await?;
    let views: Vec<UserResponse> = users.into_iter().map(UserResponse::from).collect();
    Ok(Json(json!({ "users": views })))
}

// ── POST /api/v1/users ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
}

pub async fn create_user(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Json(body): Json<CreateUserRequest>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "users:create").await?;

    let user = identity::create_user(
        &state.provider,
        &state.config.security,
        Tenant::DEFAULT_ID,
        &body.username,
        &body.password,
        Some(&auth.user.id),
        ctx.ip_address.as_deref(),
        ctx.user_agent.as_deref(),
    )
    .await?;

    Ok(Json(json!({ "user": UserResponse::from(user) })))
}

// ── GET /api/v1/users/:id ─────────────────────────────────────────────────────

pub async fn get_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    // Users may view themselves; admins may view anyone
    if id != auth.user.id {
        require(&state.provider, &auth.user.id, "users:create").await?;
    }

    let user = identity::get_user(&state.provider, &id).await?;
    Ok(Json(json!({ "user": UserResponse::from(user) })))
}

// ── PATCH /api/v1/users/:id ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub status: Option<String>,
}

pub async fn update_user(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Path(id): Path<String>,
    Json(body): Json<UpdateUserRequest>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "users:update").await?;

    if let Some(status_str) = &body.status {
        let status = match status_str.as_str() {
            "active" => UserStatus::Active as i32,
            "disabled" => UserStatus::Disabled as i32,
            "locked" => UserStatus::Locked as i32,
            other => return Err(AppError::InvalidInput(format!("unknown status: {other}"))),
        };
        identity::update_status(
            &state.provider,
            &id,
            status,
            Some(&auth.user.id),
            ctx.ip_address.as_deref(),
            ctx.user_agent.as_deref(),
        )
        .await?;
    }

    let user = identity::get_user(&state.provider, &id).await?;
    Ok(Json(json!({ "user": UserResponse::from(user) })))
}

// ── DELETE /api/v1/users/:id ──────────────────────────────────────────────────

/// Soft-deletes a user by setting status = Disabled. Never hard-deletes.
pub async fn delete_user(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "users:delete").await?;

    // Prevent self-deletion
    if id == auth.user.id {
        return Err(AppError::InvalidInput(
            "cannot disable your own account".into(),
        ));
    }

    identity::update_status(
        &state.provider,
        &id,
        UserStatus::Disabled as i32,
        Some(&auth.user.id),
        ctx.ip_address.as_deref(),
        ctx.user_agent.as_deref(),
    )
    .await?;

    Ok(Json(json!({ "success": true })))
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub password: String,
}

/// POST /api/v1/users/:id/reset-password
pub async fn reset_password(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Path(id): Path<String>,
    Json(body): Json<ResetPasswordRequest>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "users:update").await?;

    identity::reset_password(
        &state.provider,
        &state.config.security,
        &id,
        &body.password,
        Some(&auth.user.id),
        ctx.ip_address.as_deref(),
        ctx.user_agent.as_deref(),
    )
    .await?;

    Ok(Json(json!({ "success": true })))
}

/// GET /api/v1/users/:id/roles
pub async fn list_user_roles(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    if id != auth.user.id {
        require(&state.provider, &auth.user.id, "users:create").await?;
    }

    let roles = state.provider.roles().list_for_user(&id).await?;
    Ok(Json(json!({
        "roles": roles.into_iter().map(|r| {
            json!({
                "id": r.id,
                "name": r.name,
                "description": r.description,
            })
        }).collect::<Vec<_>>(),
    })))
}

/// GET /api/v1/users/:id/applications
///
/// Reverse lookup: list applications assigned to a user via membership.
/// Requires `applications:manage` (membership administration).
pub async fn list_user_applications(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    require(
        &state.provider,
        &auth.user.id,
        crate::api::applications::MANAGE_PERM,
    )
    .await?;

    let memberships = members::list_by_user(&state.provider, &id).await?;

    #[derive(Serialize)]
    struct UserApplicationView {
        id: String,
        application_id: String,
        user_id: String,
        role: String,
        enabled: bool,
        created_at: String,
        updated_at: String,
        application_name: String,
        application_slug: String,
        application_enabled: bool,
        client_id: String,
        credentials_configured: bool,
    }

    let mut views = Vec::with_capacity(memberships.len());
    for m in memberships {
        let app = state
            .provider
            .applications()
            .find_by_id(&m.application_id)
            .await
            .map_err(AppError::Database)?;

        let (name, slug, app_enabled, client_id, credentials_configured) = match app {
            Some(a) => {
                let credentials_configured = a.has_credentials();
                (
                    a.name,
                    a.slug.unwrap_or_default(),
                    a.enabled,
                    a.client_id,
                    credentials_configured,
                )
            }
            None => continue,
        };

        views.push(UserApplicationView {
            id: m.id,
            application_id: m.application_id,
            user_id: m.user_id,
            role: m.role,
            enabled: m.enabled,
            created_at: m.created_at,
            updated_at: m.updated_at,
            application_name: name,
            application_slug: slug,
            application_enabled: app_enabled,
            client_id,
            credentials_configured,
        });
    }

    Ok(Json(json!({ "applications": views })))
}
