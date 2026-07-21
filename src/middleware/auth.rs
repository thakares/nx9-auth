use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_extra::extract::CookieJar;

use crate::{
    db::models::User,
    error::AppError,
    security::{sessions, tokens},
    state::AppState,
};

/// Describes how the current request was authenticated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMethod {
    Session,
    Token,
}

/// Axum extractor that resolves the authenticated user from either a session
/// cookie or a Bearer token in the Authorization header.
///
/// Handlers that need an authenticated user simply include `auth: AuthUser`
/// in their parameter list.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user: User,
    pub method: AuthMethod,
    /// Session ID — populated when `method == Session`, used for logout.
    pub session_id: Option<String>,
}

impl<S> FromRequestParts<S> for AuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, AppError> {
        let app_state = AppState::from_ref(state);

        // 1. Try session cookie first
        let jar = CookieJar::from_headers(&parts.headers);
        if let Some(cookie) = jar.get(sessions::SESSION_COOKIE) {
            let raw = cookie.value();
            if let Some(session) =
                sessions::validate_session(&app_state.provider, raw, &app_state.config.security)
                    .await?
            {
                let user = app_state
                    .provider
                    .users()
                    .find_by_id(&session.user_id)
                    .await
                    .map_err(AppError::Database)?
                    .ok_or(AppError::Unauthorized)?;

                if !user.is_active() {
                    return Err(AppError::Unauthorized);
                }

                return Ok(AuthUser {
                    user,
                    method: AuthMethod::Session,
                    session_id: Some(session.id),
                });
            }
        }

        // 2. Try Authorization: Bearer — PAT first, then session token.
        //    Session tokens are returned from /auth/login for SPA clients that
        //    cannot rely solely on the HttpOnly cookie.
        if let Some(auth_header) = parts.headers.get(axum::http::header::AUTHORIZATION) {
            if let Ok(value) = auth_header.to_str() {
                if let Some(raw) = value.strip_prefix("Bearer ") {
                    let raw = raw.trim();

                    // 2a. Personal access token
                    if let Some(token) = tokens::validate_token(&app_state.provider, raw).await? {
                        let user = app_state
                            .provider
                            .users()
                            .find_by_id(&token.user_id)
                            .await
                            .map_err(AppError::Database)?
                            .ok_or(AppError::Unauthorized)?;

                        if !user.is_active() {
                            return Err(AppError::Unauthorized);
                        }

                        return Ok(AuthUser {
                            user,
                            method: AuthMethod::Token,
                            session_id: None,
                        });
                    }

                    // 2b. Session token (same value as nx9_session cookie)
                    if let Some(session) = sessions::validate_session(
                        &app_state.provider,
                        raw,
                        &app_state.config.security,
                    )
                    .await?
                    {
                        let user = app_state
                            .provider
                            .users()
                            .find_by_id(&session.user_id)
                            .await
                            .map_err(AppError::Database)?
                            .ok_or(AppError::Unauthorized)?;

                        if !user.is_active() {
                            return Err(AppError::Unauthorized);
                        }

                        return Ok(AuthUser {
                            user,
                            method: AuthMethod::Session,
                            session_id: Some(session.id),
                        });
                    }
                }
            }
        }

        Err(AppError::Unauthorized)
    }
}
