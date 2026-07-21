use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;
use serde_json::{Value, json};

use crate::{
    db::models::Session,
    error::{AppError, Result},
    middleware::auth::AuthUser,
    state::AppState,
};

/// Session view sent to the client (never includes token_hash)
#[derive(Serialize)]
pub struct SessionView {
    pub id: String,
    pub user_id: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: String,
    pub expires_at: String,
    pub last_seen_at: String,
    pub is_current: bool,
}

impl SessionView {
    fn from_session(s: Session, current_id: Option<&str>) -> Self {
        let is_current = current_id.map(|id| id == s.id).unwrap_or(false);
        Self {
            id: s.id,
            user_id: s.user_id,
            ip_address: s.ip_address,
            user_agent: s.user_agent,
            created_at: s.created_at,
            expires_at: s.expires_at,
            last_seen_at: s.last_seen_at,
            is_current,
        }
    }
}

/// GET /api/v1/sessions
/// Admins see all active sessions; regular users see only their own.
pub async fn list_sessions(State(state): State<AppState>, auth: AuthUser) -> Result<Json<Value>> {
    let is_admin = state
        .provider
        .permissions()
        .user_has_permission(&auth.user.id, "audit:view")
        .await
        .map_err(AppError::Database)?;

    let sessions = if is_admin {
        state
            .provider
            .sessions()
            .list_all_active()
            .await
            .map_err(AppError::Database)?
    } else {
        state
            .provider
            .sessions()
            .list_active_for_user(&auth.user.id)
            .await
            .map_err(AppError::Database)?
    };

    let current_id = auth.session_id.as_deref();
    let views: Vec<SessionView> = sessions
        .into_iter()
        .map(|s| SessionView::from_session(s, current_id))
        .collect();
    let total = views.len();

    Ok(Json(json!({ "sessions": views, "total": total })))
}

/// DELETE /api/v1/sessions/others
pub async fn terminate_others(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>> {
    let session_id = auth.session_id.as_deref().ok_or_else(|| {
        AppError::InvalidInput("Current session not found (perhaps authenticated via token)".into())
    })?;

    let count = state
        .provider
        .sessions()
        .revoke_others(&auth.user.id, session_id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(json!({ "success": true, "terminated": count })))
}

/// DELETE /api/v1/sessions/{id}
pub async fn terminate_session(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    // If the user is trying to terminate the current session, disallow it
    if let Some(current_id) = auth.session_id.as_deref() {
        if id == current_id {
            return Err(AppError::InvalidInput(
                "Cannot terminate current session".into(),
            ));
        }
    }

    // Admins can terminate any session, users can only terminate their own
    let is_admin = state
        .provider
        .permissions()
        .user_has_permission(&auth.user.id, "audit:view")
        .await
        .map_err(AppError::Database)?;

    if !is_admin {
        // Since we don't have a `find_by_id` that returns a session easily,
        // we can fetch active sessions for the user and check if the ID is in the list
        let sessions = state
            .provider
            .sessions()
            .list_active_for_user(&auth.user.id)
            .await
            .map_err(AppError::Database)?;

        let owns_session = sessions.iter().any(|s| s.id == id);
        if !owns_session {
            return Err(AppError::Forbidden);
        }
    }

    state
        .provider
        .sessions()
        .revoke(&id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(json!({ "success": true })))
}
