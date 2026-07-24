use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    db::models::Tenant,
    error::Result,
    middleware::{audit::AuditContext, auth::AuthUser, permissions::require},
    state::AppState,
};

#[derive(Serialize)]
pub struct TenantView {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
}

impl From<Tenant> for TenantView {
    fn from(t: Tenant) -> Self {
        Self {
            id: t.id,
            name: t.name,
            slug: t.slug.unwrap_or_else(|| "default".to_string()),
            description: None,
        }
    }
}

/// GET /api/v1/tenants
pub async fn list_tenants(State(state): State<AppState>, _auth: AuthUser) -> Result<Json<Value>> {
    let tenants = state.provider.tenants().list().await?;
    let views: Vec<TenantView> = tenants.into_iter().map(|t| t.into()).collect();
    Ok(Json(json!({ "tenants": views })))
}

#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
    pub slug: Option<String>,
}

/// POST /api/v1/tenants
pub async fn create_tenant(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Json(body): Json<CreateTenantRequest>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    if let Some(ref s) = body.slug {
        if !s.trim().is_empty() {
            crate::identity::slug::validate_slug(s)?;
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    let tenant = state
        .provider
        .tenants()
        .create(&id, &body.name, body.slug.as_deref())
        .await?;

    let _ = state
        .provider
        .audit()
        .insert(
            &uuid::Uuid::new_v4().to_string(),
            Some(&auth.user.id),
            None,
            "tenant.create",
            "tenant",
            Some(&tenant.id),
            "info",
            ctx.ip_address.as_deref(),
            ctx.user_agent.as_deref(),
            None,
        )
        .await;

    Ok(Json(json!({
        "tenant": TenantView::from(tenant)
    })))
}

/// GET /api/v1/tenants/:id
pub async fn get_tenant(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    let tenant = state
        .provider
        .tenants()
        .find_by_id(&id)
        .await?
        .ok_or(crate::error::AppError::NotFound)?;

    Ok(Json(json!({
        "tenant": TenantView::from(tenant)
    })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateTenantRequest {
    pub name: String,
    pub slug: Option<String>,
}

/// PATCH /api/v1/tenants/:id
pub async fn update_tenant(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Path(id): Path<String>,
    Json(body): Json<UpdateTenantRequest>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    if let Some(ref s) = body.slug {
        if !s.trim().is_empty() {
            crate::identity::slug::validate_slug(s)?;
        }
    }

    state
        .provider
        .tenants()
        .update(&id, &body.name, body.slug.as_deref())
        .await?;

    let tenant = state
        .provider
        .tenants()
        .find_by_id(&id)
        .await?
        .ok_or(crate::error::AppError::NotFound)?;

    let _ = state
        .provider
        .audit()
        .insert(
            &uuid::Uuid::new_v4().to_string(),
            Some(&auth.user.id),
            None,
            "tenant.update",
            "tenant",
            Some(&id),
            "info",
            ctx.ip_address.as_deref(),
            ctx.user_agent.as_deref(),
            None,
        )
        .await;

    Ok(Json(json!({
        "tenant": TenantView::from(tenant)
    })))
}

/// DELETE /api/v1/tenants/:id
pub async fn delete_tenant(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    state.provider.tenants().delete(&id).await?;

    let _ = state
        .provider
        .audit()
        .insert(
            &uuid::Uuid::new_v4().to_string(),
            Some(&auth.user.id),
            None,
            "tenant.delete",
            "tenant",
            Some(&id),
            "warn",
            ctx.ip_address.as_deref(),
            ctx.user_agent.as_deref(),
            None,
        )
        .await;

    Ok(Json(json!({ "success": true })))
}

/// GET /api/v1/tenants/:id/users
pub async fn list_tenant_users(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    let users = state.provider.users().list(&id).await?;
    let views: Vec<crate::api::users::UserResponse> = users
        .into_iter()
        .map(crate::api::users::UserResponse::from)
        .collect();
    Ok(Json(json!({ "users": views })))
}

#[derive(Debug, Deserialize)]
pub struct AssignTenantUserRequest {
    pub user_id: String,
}

/// POST /api/v1/tenants/:id/users
pub async fn assign_tenant_user(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Path(id): Path<String>,
    Json(body): Json<AssignTenantUserRequest>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    let _tenant = state
        .provider
        .tenants()
        .find_by_id(&id)
        .await?
        .ok_or(crate::error::AppError::NotFound)?;

    let user = state
        .provider
        .users()
        .find_by_id(&body.user_id)
        .await?
        .ok_or(crate::error::AppError::NotFound)?;

    let from_tenant_id = user.tenant_id.clone();
    if from_tenant_id == id {
        return Ok(Json(
            json!({ "user": crate::api::users::UserResponse::from(user) }),
        ));
    }

    if state
        .provider
        .users()
        .username_exists(&id, &user.username)
        .await?
    {
        return Err(crate::error::AppError::Conflict(format!(
            "username '{}' already exists in target tenant",
            user.username
        )));
    }

    state
        .provider
        .users()
        .reassign_user_tenant_with_audit(
            &user.id,
            &id,
            Some(&auth.user.id),
            ctx.ip_address.as_deref(),
            ctx.user_agent.as_deref(),
        )
        .await?;

    let updated_user = state
        .provider
        .users()
        .find_by_id(&user.id)
        .await?
        .ok_or(crate::error::AppError::NotFound)?;

    Ok(Json(
        json!({ "user": crate::api::users::UserResponse::from(updated_user) }),
    ))
}

/// DELETE /api/v1/tenants/:id/users/:user_id
pub async fn remove_tenant_user(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Path((id, user_id)): Path<(String, String)>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    if id == Tenant::DEFAULT_ID {
        return Err(crate::error::AppError::InvalidInput(
            "users cannot be moved out of default tenant without specifying a destination tenant"
                .into(),
        ));
    }

    let user = state
        .provider
        .users()
        .find_by_id(&user_id)
        .await?
        .ok_or(crate::error::AppError::NotFound)?;

    if user.tenant_id != id {
        return Err(crate::error::AppError::InvalidInput(
            "user does not belong to the specified tenant".into(),
        ));
    }

    let target_tenant = Tenant::DEFAULT_ID;

    state
        .provider
        .users()
        .reassign_user_tenant_with_audit(
            &user.id,
            target_tenant,
            Some(&auth.user.id),
            ctx.ip_address.as_deref(),
            ctx.user_agent.as_deref(),
        )
        .await?;

    Ok(Json(json!({ "success": true })))
}

/// GET /api/v1/tenants/:id/applications
pub async fn list_tenant_applications(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    let apps = state.provider.applications().list(&id).await?;
    let views: Vec<crate::api::applications::ApplicationResponse> = apps
        .into_iter()
        .map(crate::api::applications::ApplicationResponse::from)
        .collect();
    Ok(Json(json!({ "applications": views })))
}
