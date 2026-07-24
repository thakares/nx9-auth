use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, header},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    db::models::{Application, ApplicationMember, Tenant},
    error::{AppError, Result},
    identity::{application_members as members, applications as identity},
    middleware::{auth::AuthUser, permissions::require},
    state::AppState,
};

pub const MANAGE_PERM: &str = "applications:manage";

#[derive(Serialize)]
pub struct ApplicationResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub client_id: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub credentials_configured: bool,
    pub redirect_urls: Vec<String>,
    pub scopes: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Application> for ApplicationResponse {
    fn from(a: Application) -> Self {
        let client_id = a.get_client_id().to_string();
        let redirect_urls = a.redirect_urls();
        let scopes = a.scopes();
        let credentials_configured = a.has_credentials();
        Self {
            id: a.id,
            name: a.name,
            slug: a.slug.unwrap_or_default(),
            client_id,
            description: a.description,
            enabled: a.enabled,
            credentials_configured,
            redirect_urls,
            scopes,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}

#[derive(Serialize)]
pub struct CreateApplicationResponse {
    pub application: ApplicationResponse,
    pub client_secret: String,
}

#[derive(Serialize)]
pub struct RotateSecretResponse {
    pub client_secret: String,
}

fn no_store_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers
}

/// GET /api/v1/applications
pub async fn list_applications(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>> {
    let _ = auth;
    let apps = identity::list(&state.provider, Tenant::DEFAULT_ID).await?;
    let views: Vec<ApplicationResponse> = apps.into_iter().map(ApplicationResponse::from).collect();
    Ok(Json(json!({ "applications": views })))
}

#[derive(Debug, Deserialize)]
pub struct CreateApplicationRequest {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub redirect_urls: Option<Vec<String>>,
    pub scopes: Option<Vec<String>>,
}

/// POST /api/v1/applications
pub async fn create_application(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateApplicationRequest>,
) -> Result<(HeaderMap, Json<CreateApplicationResponse>)> {
    require(&state.provider, &auth.user.id, MANAGE_PERM).await?;

    let (app, raw_secret) = identity::create(
        &state.provider,
        Tenant::DEFAULT_ID,
        &body.name,
        &body.slug,
        body.description.as_deref(),
        body.redirect_urls,
        body.scopes,
        Some(&auth.user.id),
        None,
        None,
    )
    .await?;

    let resp = CreateApplicationResponse {
        application: ApplicationResponse::from(app),
        client_secret: raw_secret,
    };

    Ok((no_store_headers(), Json(resp)))
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
#[serde(deny_unknown_fields)]
pub struct UpdateApplicationRequest {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub redirect_urls: Option<Vec<String>>,
    pub scopes: Option<Vec<String>>,
    pub enabled: bool,
}

/// PATCH /api/v1/applications/:id
pub async fn update_application(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateApplicationRequest>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, MANAGE_PERM).await?;

    let app = identity::update(
        &state.provider,
        &id,
        &body.name,
        &body.slug,
        body.description.as_deref(),
        body.redirect_urls,
        body.scopes,
        body.enabled,
        Some(&auth.user.id),
        None,
        None,
    )
    .await?;

    Ok(Json(
        json!({ "application": ApplicationResponse::from(app) }),
    ))
}

/// POST /api/v1/applications/:id/secret
pub async fn rotate_application_secret(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<(HeaderMap, Json<RotateSecretResponse>)> {
    require(&state.provider, &auth.user.id, MANAGE_PERM).await?;

    let raw_secret =
        identity::rotate_secret(&state.provider, &id, Some(&auth.user.id), None, None).await?;

    let resp = RotateSecretResponse {
        client_secret: raw_secret,
    };

    Ok((no_store_headers(), Json(resp)))
}

/// DELETE /api/v1/applications/:id
pub async fn delete_application(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, MANAGE_PERM).await?;
    identity::delete(&state.provider, &id, Some(&auth.user.id), None, None).await?;
    Ok(Json(json!({ "success": true })))
}

// ── Application membership ────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ApplicationMemberResponse {
    pub id: String,
    pub application_id: String,
    pub user_id: String,
    pub username: String,
    pub user_status: String,
    pub role: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl ApplicationMemberResponse {
    fn from_member(member: ApplicationMember, username: String, user_status: String) -> Self {
        Self {
            id: member.id,
            application_id: member.application_id,
            user_id: member.user_id,
            username,
            user_status,
            role: member.role,
            enabled: member.enabled,
            created_at: member.created_at,
            updated_at: member.updated_at,
        }
    }
}

async fn enrich_member(
    state: &AppState,
    member: ApplicationMember,
) -> Result<ApplicationMemberResponse> {
    let user = state
        .provider
        .users()
        .find_by_id(&member.user_id)
        .await
        .map_err(AppError::Database)?;

    let (username, user_status) = match user {
        Some(u) => (
            u.username,
            if u.status == 1 {
                "active".to_string()
            } else if u.status == 3 {
                "locked".to_string()
            } else {
                "disabled".to_string()
            },
        ),
        None => ("unknown".to_string(), "unknown".to_string()),
    };

    Ok(ApplicationMemberResponse::from_member(
        member,
        username,
        user_status,
    ))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateMemberRequest {
    pub user_id: String,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateMemberRequest {
    pub role: Option<String>,
    pub enabled: Option<bool>,
}

/// GET /api/v1/applications/:id/members
pub async fn list_application_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, MANAGE_PERM).await?;

    let members_list = members::list_by_application(&state.provider, &id).await?;
    let mut views = Vec::with_capacity(members_list.len());
    for m in members_list {
        views.push(enrich_member(&state, m).await?);
    }

    Ok(Json(json!({ "members": views })))
}

/// POST /api/v1/applications/:id/members
pub async fn add_application_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<CreateMemberRequest>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, MANAGE_PERM).await?;

    if body.user_id.trim().is_empty() {
        return Err(AppError::InvalidInput("user_id is required".into()));
    }

    let member = members::add(
        &state.provider,
        &id,
        body.user_id.trim(),
        body.role.as_deref(),
        Some(&auth.user.id),
        None,
        None,
    )
    .await?;

    let view = enrich_member(&state, member).await?;
    Ok(Json(json!({ "member": view })))
}

/// PATCH /api/v1/applications/:id/members/:user_id
pub async fn update_application_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, user_id)): Path<(String, String)>,
    Json(body): Json<UpdateMemberRequest>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, MANAGE_PERM).await?;

    let member = members::update(
        &state.provider,
        &id,
        &user_id,
        body.role.as_deref(),
        body.enabled,
        Some(&auth.user.id),
        None,
        None,
    )
    .await?;

    let view = enrich_member(&state, member).await?;
    Ok(Json(json!({ "member": view })))
}

/// DELETE /api/v1/applications/:id/members/:user_id
pub async fn remove_application_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, user_id)): Path<(String, String)>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, MANAGE_PERM).await?;

    members::remove(
        &state.provider,
        &id,
        &user_id,
        Some(&auth.user.id),
        None,
        None,
    )
    .await?;

    Ok(Json(json!({ "success": true })))
}
