use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    db::models::{AuditFilter, AuditLog},
    db::repository::audit as audit_repo,
    error::{AppError, Result},
    middleware::{auth::AuthUser, permissions::require},
    state::AppState,
};

#[derive(Serialize)]
pub struct AuditLogResponse {
    pub id: String,
    pub actor_user_id: Option<String>,
    pub target_user_id: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub severity: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: String,
    /// Convenience flag for success/failure filters in the UI.
    pub success: bool,
}

impl From<AuditLog> for AuditLogResponse {
    fn from(a: AuditLog) -> Self {
        let success =
            !a.action.contains("fail") && !a.action.contains("denied") && a.severity != "critical";
        Self {
            id: a.id,
            actor_user_id: a.actor_user_id,
            target_user_id: a.target_user_id,
            action: a.action,
            resource_type: a.resource_type,
            resource_id: a.resource_id,
            severity: a.severity,
            ip_address: a.ip_address,
            user_agent: a.user_agent,
            metadata_json: a.metadata_json,
            created_at: a.created_at,
            success,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    pub actor: Option<String>,
    pub action: Option<String>,
    pub resource_type: Option<String>,
    pub severity: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub q: Option<String>,
    pub success: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// GET /api/v1/audit
pub async fn list_audit(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<AuditQuery>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "audit:view").await?;

    let limit = query.limit.unwrap_or(50).clamp(1, 500);
    let offset = query.offset.unwrap_or(0).max(0);

    let filter = AuditFilter {
        actor_user_id: query.actor,
        action: query.action,
        resource_type: query.resource_type,
        severity: query.severity,
        since: query.since,
        until: query.until,
        search: query.q,
        limit,
        offset,
    };

    let total = audit_repo::count_filtered(&state.provider, &filter)
        .await
        .map_err(AppError::Database)?;

    let mut entries = audit_repo::list_filtered(&state.provider, &filter)
        .await
        .map_err(AppError::Database)?;

    if let Some(success) = query.success {
        entries.retain(|e| {
            let ok = !e.action.contains("fail")
                && !e.action.contains("denied")
                && e.severity != "critical";
            ok == success
        });
    }

    let views: Vec<AuditLogResponse> = entries.into_iter().map(AuditLogResponse::from).collect();

    Ok(Json(json!({
        "entries": views,
        "total": total,
        "limit": limit,
        "offset": offset,
    })))
}
