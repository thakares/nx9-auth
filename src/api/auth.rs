use crate::db::repository::traits::AuditRepositoryExt;

// Authentication endpoints.
//
// Login is POST-only with a JSON body. Credentials must never appear in
// query strings, path segments, or server access logs of request URIs.

use axum::{Json, extract::State};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    audit::AuditEvent,
    db::models::AuditSeverity,
    error::{AppError, Result},
    middleware::{audit::AuditContext, auth::AuthUser},
    security::{passwords, sessions},
    state::AppState,
};

// ── Login ─────────────────────────────────────────────────────────────────────

/// Login request body. Deserialized from JSON only (never from query params).
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginUserView {
    pub id: String,
    pub username: String,
    pub status: String,
    pub last_login_at: Option<String>,
    pub created_at: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    /// Opaque access token (session). Send as `Authorization: Bearer …`.
    pub access_token: String,
    /// Opaque refresh token. Longer-lived; used to obtain a new access token.
    pub refresh_token: String,
    /// Access token lifetime in seconds (idle TTL).
    pub expires_in: u64,
    pub token_type: &'static str,
    pub user: LoginUserView,
}

/// POST /api/v1/auth/login
///
/// Accepts JSON `{ "username", "password" }` only. No GET handler exists.
pub async fn login(
    State(state): State<AppState>,
    ctx: AuditContext,
    jar: CookieJar,
    Json(body): Json<LoginRequest>,
) -> Result<(CookieJar, Json<LoginResponse>)> {
    let ip = ctx.ip_address.as_deref();

    // Reject empty credentials early without revealing which field failed.
    if body.username.trim().is_empty() || body.password.is_empty() {
        return Err(AppError::InvalidCredentials);
    }

    // Rate limit check (per IP)
    if let Some(ip_str) = &ctx.ip_address {
        if let Ok(ip_addr) = ip_str.parse::<std::net::IpAddr>() {
            state.rate_limiter.check(ip_addr)?;
        }
    }

    // Look up user — always run comparable work on failure paths (timing).
    let user_opt = state
        .provider
        .users()
        .find_by_username(body.username.trim())
        .await
        .map_err(AppError::Database)?;

    let mut is_authed = false;
    let mut final_user = None;

    if let Some(user) = user_opt {
        // Constant-time Argon2id verify (argon2 crate).
        let password_ok = passwords::verify_password(&body.password, &user.password_hash)?;
        if password_ok && user.is_active() {
            is_authed = true;
            final_user = Some(user);
        }
    } else {
        // Dummy verify to reduce username enumeration via timing.
        passwords::verify_dummy(&state.config.security)?;
    }

    // Zeroize is best-effort; String drop is immediate after this function.
    // Do not log body.password anywhere.
    let _ = &body.password;

    if !is_authed {
        record_login_failure(&state, body.username.trim(), ip, ctx.user_agent.as_deref()).await;
        if let Some(ip_str) = &ctx.ip_address {
            if let Ok(ip_addr) = ip_str.parse::<std::net::IpAddr>() {
                state.rate_limiter.record_failure(ip_addr);
            }
        }
        // Non-enumerating error for both unknown user and bad password.
        return Err(AppError::InvalidCredentials);
    }

    let user = final_user.expect("authenticated user");

    // Clear rate limit on success
    if let Some(ip_str) = &ctx.ip_address {
        if let Ok(ip_addr) = ip_str.parse::<std::net::IpAddr>() {
            state.rate_limiter.record_success(ip_addr);
        }
    }

    // Session fixation mitigation: revoke prior sessions + refresh tokens.
    let _ = state
        .provider
        .sessions()
        .revoke_all_for_user(&user.id)
        .await;
    let _ = state
        .provider
        .refresh_tokens()
        .revoke_all_for_user(&user.id)
        .await;

    // Create new session (new ID + new token) — rotation on every login.

    let session_id = uuid::Uuid::new_v4().to_string();
    let access_token = crate::security::sessions::generate_session_token();
    let token_hash = crate::security::sessions::hash_session_token(&access_token);
    let ttl_mins = (state.config.security.session_ttl_hours * 60) as i64;
    let expires = chrono::Utc::now() + chrono::Duration::minutes(ttl_mins);
    let expires_str = expires.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let session = state
        .provider
        .sessions()
        .create(
            &session_id,
            &user.id,
            &token_hash,
            ip,
            ctx.user_agent.as_deref(),
            &expires_str,
        )
        .await
        .map_err(AppError::Database)?;

    // Refresh token (opaque, BLAKE3-hashed at rest). Longer absolute lifetime.
    let refresh_raw = sessions::generate_session_token();
    let refresh_hash = sessions::hash_session_token(&refresh_raw);
    let refresh_id = uuid::Uuid::new_v4().to_string();
    let refresh_ttl_days = state.config.security.session_absolute_ttl_days.max(1) as i64;
    let refresh_expires = chrono::Utc::now() + chrono::Duration::days(refresh_ttl_days);
    let refresh_expires_str = refresh_expires.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    state
        .provider
        .refresh_tokens()
        .create(&refresh_id, &user.id, &refresh_hash, &refresh_expires_str)
        .await
        .map_err(AppError::Database)?;

    let user_roles = state.provider.roles().list_for_user(&user.id).await?;
    let user_perms = state.provider.permissions().list_for_user(&user.id).await?;
    let role_names: Vec<String> = user_roles.into_iter().map(|r| r.name).collect();

    // Update last_login_at and audit (never log password / tokens).
    let _ = state.provider.users().set_last_login(&user.id).await;
    let _ = state
        .provider
        .audit()
        .log(AuditEvent {
            actor_id: Some(&user.id),
            target_id: Some(&user.id),
            action: "login_success",
            resource_type: "session",
            resource_id: Some(&session.id),
            severity: AuditSeverity::Info,
            ip,
            ua: ctx.user_agent.as_deref(),
            metadata: None,
        })
        .await;

    // Structured log: identity + outcome only (no secrets).
    tracing::info!(
        event = "login_success",
        user_id = %user.id,
        username = %user.username,
        ip = ip.unwrap_or("unknown"),
    );

    let expires_in = (state.config.security.session_ttl_hours as u64).saturating_mul(3600);
    let max_age_secs = state.config.security.session_absolute_ttl_days as i64 * 86400;

    let mut cookie = Cookie::new(sessions::SESSION_COOKIE, access_token.clone());
    cookie.set_http_only(true);
    cookie.set_secure(state.config.server.cookie_secure);
    cookie.set_same_site(axum_extra::extract::cookie::SameSite::Lax);
    cookie.set_path("/");
    cookie.set_max_age(time::Duration::seconds(max_age_secs));

    let response = LoginResponse {
        access_token,
        refresh_token: refresh_raw,
        expires_in,
        token_type: "Bearer",
        user: LoginUserView {
            id: user.id.clone(),
            username: user.username.clone(),
            status: user.status().to_string(),
            last_login_at: user.last_login_at.clone(),
            created_at: user.created_at.clone(),
            roles: role_names,
            permissions: user_perms,
        },
    };

    Ok((jar.add(cookie), Json(response)))
}

async fn record_login_failure(
    state: &AppState,
    username: &str,
    ip: Option<&str>,
    ua: Option<&str>,
) {
    // Audit: username + outcome only — never password.
    let metadata = format!(
        r#"{{"username":{}}}"#,
        serde_json::to_string(username).unwrap_or_else(|_| "\"\"".into())
    );
    let _ = state
        .provider
        .audit()
        .log(AuditEvent {
            actor_id: None,
            target_id: None,
            action: "login_failed",
            resource_type: "session",
            resource_id: None,
            severity: AuditSeverity::Warning,
            ip,
            ua,
            metadata: Some(&metadata),
        })
        .await;

    tracing::warn!(
        event = "login_failed",
        username = %username,
        ip = ip.unwrap_or("unknown"),
    );
}

// ── Logout ────────────────────────────────────────────────────────────────────

/// POST /api/v1/auth/logout
pub async fn logout(
    State(state): State<AppState>,
    auth: AuthUser,
    jar: CookieJar,
) -> Result<(CookieJar, Json<Value>)> {
    if let Some(session_id) = &auth.session_id {
        state.provider.sessions().revoke(session_id).await?;

        let _ = state
            .provider
            .audit()
            .log(AuditEvent {
                actor_id: Some(&auth.user.id),
                target_id: Some(&auth.user.id),
                action: "logout",
                resource_type: "session",
                resource_id: Some(session_id),
                severity: AuditSeverity::Info,
                ip: None,
                ua: None,
                metadata: None,
            })
            .await;
    }

    // Revoke refresh tokens for this user on logout (full session end).
    let _ = state
        .provider
        .refresh_tokens()
        .revoke_all_for_user(&auth.user.id)
        .await;

    let mut removal = Cookie::from(sessions::SESSION_COOKIE);
    removal.set_path("/");
    let removed = jar.remove(removal);
    Ok((removed, Json(json!({ "success": true }))))
}

// ── Me ────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct MeResponse {
    pub user: UserView,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

#[derive(Serialize)]
pub struct UserView {
    pub id: String,
    pub username: String,
    pub status: String,
    pub last_login_at: Option<String>,
    pub created_at: String,
}

/// GET /api/v1/auth/me
pub async fn me(State(state): State<AppState>, auth: AuthUser) -> Result<Json<MeResponse>> {
    let user_roles = state.provider.roles().list_for_user(&auth.user.id).await?;
    let user_perms = state
        .provider
        .permissions()
        .list_for_user(&auth.user.id)
        .await?;

    Ok(Json(MeResponse {
        user: UserView {
            id: auth.user.id.clone(),
            username: auth.user.username.clone(),
            status: auth.user.status().to_string(),
            last_login_at: auth.user.last_login_at.clone(),
            created_at: auth.user.created_at.clone(),
        },
        roles: user_roles.into_iter().map(|r| r.name).collect(),
        permissions: user_perms,
    }))
}
