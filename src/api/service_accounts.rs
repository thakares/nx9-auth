use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    db::models::{ServiceAccount, Tenant},
    error::Result,
    identity::service_accounts as identity,
    middleware::{audit::AuditContext, auth::AuthUser, permissions::require},
    state::AppState,
};

#[derive(Serialize)]
pub struct ServiceAccountResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ServiceAccount> for ServiceAccountResponse {
    fn from(sa: ServiceAccount) -> Self {
        Self {
            id: sa.id,
            name: sa.name,
            description: sa.description,
            enabled: sa.enabled,
            created_at: sa.created_at,
            updated_at: sa.updated_at,
        }
    }
}

/// GET /api/v1/service-accounts
pub async fn list_service_accounts(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    let items = identity::list(&state.provider, Tenant::DEFAULT_ID).await?;
    let views: Vec<ServiceAccountResponse> = items
        .into_iter()
        .map(ServiceAccountResponse::from)
        .collect();
    Ok(Json(json!({ "service_accounts": views })))
}

#[derive(Debug, Deserialize)]
pub struct CreateServiceAccountRequest {
    pub name: String,
    pub description: Option<String>,
}

/// POST /api/v1/service-accounts
pub async fn create_service_account(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Json(body): Json<CreateServiceAccountRequest>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    let sa = identity::create(
        &state.provider,
        Tenant::DEFAULT_ID,
        &body.name,
        body.description.as_deref(),
        Some(&auth.user.id),
        ctx.ip_address.as_deref(),
        ctx.user_agent.as_deref(),
    )
    .await?;

    Ok(Json(json!({
        "service_account": ServiceAccountResponse::from(sa)
    })))
}

/// GET /api/v1/service-accounts/:id
pub async fn get_service_account(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;
    let sa = identity::get(&state.provider, &id).await?;
    Ok(Json(json!({
        "service_account": ServiceAccountResponse::from(sa)
    })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateServiceAccountRequest {
    pub enabled: Option<bool>,
}

/// PATCH /api/v1/service-accounts/:id
pub async fn update_service_account(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Path(id): Path<String>,
    Json(body): Json<UpdateServiceAccountRequest>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    if let Some(enabled) = body.enabled {
        identity::set_enabled(
            &state.provider,
            &id,
            enabled,
            Some(&auth.user.id),
            ctx.ip_address.as_deref(),
            ctx.user_agent.as_deref(),
        )
        .await?;
    }

    let sa = identity::get(&state.provider, &id).await?;
    Ok(Json(json!({
        "service_account": ServiceAccountResponse::from(sa)
    })))
}

/// DELETE /api/v1/service-accounts/:id
pub async fn delete_service_account(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    identity::delete(
        &state.provider,
        &id,
        Some(&auth.user.id),
        ctx.ip_address.as_deref(),
        ctx.user_agent.as_deref(),
    )
    .await?;

    Ok(Json(json!({ "success": true })))
}

/// POST /api/v1/service-accounts/:id/secret
pub async fn rotate_secret(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    let raw = identity::generate_secret(
        &state.provider,
        &id,
        Some(&auth.user.id),
        ctx.ip_address.as_deref(),
        ctx.user_agent.as_deref(),
    )
    .await?;

    Ok(Json(json!({
        "raw_secret": raw,
        "warning": "Store this secret securely — it will not be shown again.",
    })))
}
