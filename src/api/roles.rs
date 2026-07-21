use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    db::models::Role,
    error::{AppError, Result},
    identity::{permissions as identity_perms, roles as identity_roles},
    middleware::{audit::AuditContext, auth::AuthUser, permissions::require},
    state::AppState,
};

#[derive(Serialize)]
pub struct RoleResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub user_count: usize,
}

impl RoleResponse {
    async fn from_role(
        provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
        role: Role,
    ) -> Result<Self> {
        let perms = provider
            .permissions()
            .list_for_role(&role.id)
            .await
            .map_err(AppError::Database)?;
        let user_ids = provider
            .roles()
            .list_user_ids_for_role(&role.id)
            .await
            .map_err(AppError::Database)?;
        Ok(Self {
            id: role.id,
            name: role.name,
            description: role.description,
            permissions: perms.into_iter().map(|p| p.name).collect(),
            user_count: user_ids.len(),
        })
    }
}

/// GET /api/v1/roles
pub async fn list_roles(State(state): State<AppState>, auth: AuthUser) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    let roles = state.provider.roles().list_all().await?;
    let mut views = Vec::with_capacity(roles.len());
    for role in roles {
        views.push(RoleResponse::from_role(&state.provider, role).await?);
    }
    Ok(Json(json!({ "roles": views })))
}

#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: Option<String>,
}

/// POST /api/v1/roles
pub async fn create_role(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Json(body): Json<CreateRoleRequest>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    let role = identity_roles::create_role(
        &state.provider,
        &body.name,
        body.description.as_deref(),
        Some(&auth.user.id),
        ctx.ip_address.as_deref(),
        ctx.user_agent.as_deref(),
    )
    .await?;

    Ok(Json(json!({
        "role": RoleResponse::from_role(&state.provider, role).await?
    })))
}

/// GET /api/v1/roles/:id
pub async fn get_role(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    let role = identity_roles::get_role(&state.provider, &id).await?;
    let user_ids = state
        .provider
        .roles()
        .list_user_ids_for_role(&id)
        .await
        .map_err(AppError::Database)?;

    let mut users = Vec::new();
    for uid in user_ids {
        if let Ok(Some(u)) = state.provider.users().find_by_id(&uid).await {
            users.push(json!({
                "id": u.id,
                "username": u.username,
                "status": u.status().to_string(),
            }));
        }
    }

    let view = RoleResponse::from_role(&state.provider, role).await?;
    Ok(Json(json!({
        "role": view,
        "users": users,
    })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub name: String,
    pub description: Option<String>,
}

/// PATCH /api/v1/roles/:id
pub async fn update_role(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Path(id): Path<String>,
    Json(body): Json<UpdateRoleRequest>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    let role = identity_roles::update_role(
        &state.provider,
        &id,
        &body.name,
        body.description.as_deref(),
        Some(&auth.user.id),
        ctx.ip_address.as_deref(),
        ctx.user_agent.as_deref(),
    )
    .await?;

    Ok(Json(json!({
        "role": RoleResponse::from_role(&state.provider, role).await?
    })))
}

/// DELETE /api/v1/roles/:id
pub async fn delete_role(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    identity_roles::delete_role(
        &state.provider,
        &id,
        Some(&auth.user.id),
        ctx.ip_address.as_deref(),
        ctx.user_agent.as_deref(),
    )
    .await?;

    Ok(Json(json!({ "success": true })))
}

#[derive(Debug, Deserialize)]
pub struct SetPermissionsRequest {
    pub permissions: Vec<String>,
}

/// PUT /api/v1/roles/:id/permissions
pub async fn set_role_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Path(id): Path<String>,
    Json(body): Json<SetPermissionsRequest>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;
    let _ = identity_roles::get_role(&state.provider, &id).await?;

    let perms = identity_perms::set_role_permissions(
        &state.provider,
        &id,
        &body.permissions,
        Some(&auth.user.id),
        ctx.ip_address.as_deref(),
        ctx.user_agent.as_deref(),
    )
    .await?;

    Ok(Json(json!({
        "permissions": perms.into_iter().map(|p| p.name).collect::<Vec<_>>(),
    })))
}

#[derive(Debug, Deserialize)]
pub struct AssignRoleRequest {
    pub role: String,
}

/// POST /api/v1/users/:id/roles
pub async fn assign_user_role(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Path(user_id): Path<String>,
    Json(body): Json<AssignRoleRequest>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    // Assign the role to the user (user_id, role_name)
    identity_roles::assign_role(
        &state.provider,
        &user_id,
        &body.role,
        Some(&auth.user.id),
        ctx.ip_address.as_deref(),
        ctx.user_agent.as_deref(),
    )
    .await?;

    let roles = state.provider.roles().list_for_user(&user_id).await?;
    Ok(Json(json!({
        "roles": roles.into_iter().map(|r| r.name).collect::<Vec<_>>(),
    })))
}

/// DELETE /api/v1/users/:id/roles/:role
pub async fn remove_user_role(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Path((user_id, role)): Path<(String, String)>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    identity_roles::remove_role(
        &state.provider,
        &user_id,
        &role,
        Some(&auth.user.id),
        ctx.ip_address.as_deref(),
        ctx.user_agent.as_deref(),
    )
    .await?;

    let roles = state.provider.roles().list_for_user(&user_id).await?;
    Ok(Json(json!({
        "roles": roles.into_iter().map(|r| r.name).collect::<Vec<_>>(),
    })))
}
