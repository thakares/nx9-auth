use axum::{Json, extract::State};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    audit::{self, AuditEvent},
    db::models::AuditSeverity,
    db::repository::users as user_repo,
    error::{AppError, Result},
    identity::{permissions, roles},
    middleware::{audit::AuditContext, auth::AuthUser},
    security::{passwords, sessions},
    state::AppState,
};

// ── Login ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// POST /api/v1/auth/login
pub async fn login(
    State(state): State<AppState>,
    ctx: AuditContext,
    jar: CookieJar,
    Json(body): Json<LoginRequest>,
) -> Result<(CookieJar, Json<Value>)> {
    let ip = ctx.ip_address.as_deref();

    // Rate limit check
    if let Some(ip_str) = &ctx.ip_address {
        if let Ok(ip_addr) = ip_str.parse::<std::net::IpAddr>() {
            state.rate_limiter.check(ip_addr)?;
        }
    }

    // Look up user
    let user_opt = user_repo::find_by_username(&state.pool, &body.username)
        .await
        .map_err(AppError::Database)?;

    let mut is_authed = false;
    let mut final_user = None;

    if let Some(user) = user_opt {
        let password_ok = passwords::verify_password(&body.password, &user.password_hash)?;
        if password_ok && user.is_active() {
            is_authed = true;
            final_user = Some(user);
        }
    } else {
        // Run dummy verify to take same execution time
        passwords::verify_dummy(&state.config.security)?;
    }

    if !is_authed {
        record_login_failure(&state, &body.username, ip, ctx.user_agent.as_deref()).await;
        if let Some(ip_str) = &ctx.ip_address {
            if let Ok(ip_addr) = ip_str.parse::<std::net::IpAddr>() {
                state.rate_limiter.record_failure(ip_addr);
            }
        }
        return Err(AppError::Unauthorized);
    }

    let user = final_user.unwrap();

    // Clear rate limit on success
    if let Some(ip_str) = &ctx.ip_address {
        if let Ok(ip_addr) = ip_str.parse::<std::net::IpAddr>() {
            state.rate_limiter.record_success(ip_addr);
        }
    }

    // Create session
    let (session, raw_token) = sessions::create_session(
        &state.pool,
        &user.id,
        ip,
        ctx.user_agent.as_deref(),
        &state.config.security,
    )
    .await?;

    // Update last_login_at and audit in the same transaction
    if let Ok(mut tx) = state.pool.begin().await {
        let _ = user_repo::set_last_login(&mut tx, &user.id).await;
        let _ = audit::log(
            &mut tx,
            AuditEvent {
                actor_id: Some(&user.id),
                target_id: Some(&user.id),
                action: "login_success",
                resource_type: "session",
                resource_id: Some(&session.id),
                severity: AuditSeverity::Info,
                ip,
                ua: ctx.user_agent.as_deref(),
                metadata: None,
            },
        )
        .await;
        let _ = tx.commit().await;
    }

    tracing::info!(
        event = "login_success",
        user_id = %user.id,
        username = %user.username,
        ip = ip.unwrap_or("unknown"),
    );

    // Build secure session cookie using time::Duration for max_age
    let max_age_secs = state.config.security.session_absolute_ttl_days as i64 * 86400;
    let mut cookie = Cookie::new(sessions::SESSION_COOKIE, raw_token);
    cookie.set_http_only(true);
    cookie.set_secure(true);
    cookie.set_same_site(axum_extra::extract::cookie::SameSite::Lax);
    cookie.set_path("/");
    cookie.set_max_age(time::Duration::seconds(max_age_secs));

    Ok((jar.add(cookie), Json(json!({ "success": true }))))
}

async fn record_login_failure(
    state: &AppState,
    username: &str,
    ip: Option<&str>,
    ua: Option<&str>,
) {
    if let Ok(mut tx) = state.pool.begin().await {
        let _ = audit::log(
            &mut tx,
            AuditEvent {
                actor_id: None,
                target_id: None,
                action: "login_failed",
                resource_type: "session",
                resource_id: None,
                severity: AuditSeverity::Warning,
                ip,
                ua,
                metadata: Some(&format!(r#"{{"username":"{}"}}"#, username)),
            },
        )
        .await;
        let _ = tx.commit().await;
    }

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
        sessions::revoke_session(&state.pool, session_id).await?;

        // Audit log for logout
        if let Ok(mut tx) = state.pool.begin().await {
            let _ = audit::log(
                &mut tx,
                AuditEvent {
                    actor_id: Some(&auth.user.id),
                    target_id: Some(&auth.user.id),
                    action: "logout",
                    resource_type: "session",
                    resource_id: Some(session_id),
                    severity: AuditSeverity::Info,
                    ip: None,
                    ua: None,
                    metadata: None,
                },
            )
            .await;
            let _ = tx.commit().await;
        }
    }

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
    let user_roles = roles::list_user_roles(&state.pool, &auth.user.id).await?;
    let user_perms = permissions::list_user_permissions(&state.pool, &auth.user.id).await?;

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
