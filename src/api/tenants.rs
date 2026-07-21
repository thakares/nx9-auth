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
