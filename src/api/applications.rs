use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    db::models::{Application, Tenant},
    error::Result,
    identity::applications as identity,
    middleware::{auth::AuthUser, permissions::require},
    state::AppState,
};

#[derive(Serialize)]
pub struct ApplicationResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    /// Client ID — currently the application slug (OAuth2-ready).
    pub client_id: String,
    pub enabled: bool,
    pub redirect_urls: Vec<String>,
    pub scopes: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Application> for ApplicationResponse {
    fn from(a: Application) -> Self {
        Self {
            id: a.id,
            name: a.name,
            client_id: a.slug.clone().unwrap_or_default(),
            slug: a.slug.unwrap_or_default(),
            enabled: a.enabled,
            // Placeholder until OAuth2 tables land
            redirect_urls: Vec::new(),
            scopes: Vec::new(),
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}

/// GET /api/v1/applications
pub async fn list_applications(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>> {
    // Any authenticated user can see registered apps; mutations need roles:manage
    let apps = identity::list(&state.provider, Tenant::DEFAULT_ID).await?;
    let views: Vec<ApplicationResponse> = apps.into_iter().map(ApplicationResponse::from).collect();
    let _ = auth;
    Ok(Json(json!({ "applications": views })))
}

#[derive(Debug, Deserialize)]
pub struct CreateApplicationRequest {
    pub name: String,
    pub slug: String,
}

/// POST /api/v1/applications
pub async fn create_application(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateApplicationRequest>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    let app = identity::create(&state.provider, Tenant::DEFAULT_ID, &body.name, &body.slug).await?;
    Ok(Json(
        json!({ "application": ApplicationResponse::from(app) }),
    ))
}

/// GET /api/v1/applications/:id
pub async fn get_application(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    let _ = auth;
    let app = identity::get(&state.provider, &id).await?;
    Ok(Json(
        json!({ "application": ApplicationResponse::from(app) }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct UpdateApplicationRequest {
    pub name: String,
    pub slug: String,
    pub enabled: bool,
}

/// PATCH /api/v1/applications/:id
pub async fn update_application(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateApplicationRequest>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    let app = identity::update(&state.provider, &id, &body.name, &body.slug, body.enabled).await?;
    Ok(Json(
        json!({ "application": ApplicationResponse::from(app) }),
    ))
}

/// DELETE /api/v1/applications/:id
pub async fn delete_application(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;
    identity::delete(&state.provider, &id).await?;
    Ok(Json(json!({ "success": true })))
}
