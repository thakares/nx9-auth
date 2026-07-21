use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    db::models::ApiToken,
    db::repository::tokens as token_repo,
    error::{AppError, Result},
    middleware::{audit::AuditContext, auth::AuthUser, permissions::require},
    security::tokens as token_security,
    state::AppState,
};

// ── Response type ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct TokenResponse {
    pub id: String,
    pub name: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
    pub revoked: bool,
}

impl From<ApiToken> for TokenResponse {
    fn from(t: ApiToken) -> Self {
        Self {
            id: t.id,
            name: t.name,
            last_used_at: t.last_used_at,
            expires_at: t.expires_at,
            created_at: t.created_at,
            revoked: t.revoked,
        }
    }
}

// ── POST /api/v1/tokens ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
}

/// Create a personal access token for the authenticated user.
///
/// The raw token is returned **once** in this response and never stored.
pub async fn create_token(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Json(body): Json<CreateTokenRequest>,
) -> Result<Json<Value>> {
    if body.name.trim().is_empty() {
        return Err(AppError::InvalidInput("token name cannot be empty".into()));
    }

    let (token, raw) = token_security::create_token(
        &state.provider,
        &auth.user.id,
        &body.name,
        &state.config.security,
        Some(&auth.user.id),
        ctx.ip_address.as_deref(),
        ctx.user_agent.as_deref(),
    )
    .await?;

    tracing::info!(
        event = "token_created",
        user_id = %auth.user.id,
        token_id = %token.id,
        name = %token.name,
    );

    Ok(Json(json!({
        "token": TokenResponse::from(token),
        "raw_token": raw,
        "warning": "Store this token securely — it will not be shown again.",
    })))
}

// ── GET /api/v1/tokens ────────────────────────────────────────────────────────

/// List the authenticated user's own tokens.
pub async fn list_tokens(State(state): State<AppState>, auth: AuthUser) -> Result<Json<Value>> {
    let tokens = token_repo::list_for_user(&state.provider, &auth.user.id)
        .await
        .map_err(AppError::Database)?;

    let views: Vec<TokenResponse> = tokens.into_iter().map(TokenResponse::from).collect();
    Ok(Json(json!({ "tokens": views })))
}

// ── DELETE /api/v1/tokens/:id ────────────────────────────────────────────────

/// Revoke a token. The caller must own the token or hold `tokens:revoke`.
pub async fn revoke_token(
    State(state): State<AppState>,
    auth: AuthUser,
    ctx: AuditContext,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    let token = token_repo::find_by_id(&state.provider, &id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // Must be owner or have tokens:revoke permission
    if token.user_id != auth.user.id {
        require(&state.provider, &auth.user.id, "tokens:revoke").await?;
    }

    token_security::revoke_token(
        &state.provider,
        &id,
        Some(&auth.user.id),
        ctx.ip_address.as_deref(),
        ctx.user_agent.as_deref(),
    )
    .await?;

    Ok(Json(json!({ "success": true })))
}
