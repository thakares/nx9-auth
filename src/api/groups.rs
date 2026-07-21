use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    audit::AuditEvent,
    db::models::{AuditSeverity, Tenant},
    db::repository::traits::AuditRepositoryExt,
    error::{AppError, Result},
    middleware::{auth::AuthUser, permissions::require},
    state::AppState,
};

#[derive(Serialize)]
pub struct GroupView {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub member_count: i64,
}

#[derive(Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateGroupRequest {
    pub name: String,
    pub description: Option<String>,
}

pub async fn list_groups(State(state): State<AppState>, _auth: AuthUser) -> Result<Json<Value>> {
    let groups = state
        .provider
        .groups()
        .list(Tenant::DEFAULT_ID)
        .await
        .map_err(AppError::Database)?;

    let mut views = Vec::new();
    for group in groups {
        let member_count = state
            .provider
            .groups()
            .count_members(&group.id)
            .await
            .unwrap_or(0);

        views.push(GroupView {
            id: group.id,
            name: group.name,
            description: group.description,
            created_at: group.created_at,
            member_count,
        });
    }

    Ok(Json(json!({ "groups": views })))
}

pub async fn get_group(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    let group = state
        .provider
        .groups()
        .find_by_id(&id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    let members = state
        .provider
        .groups()
        .list_members(&id)
        .await
        .map_err(AppError::Database)?;

    #[derive(Serialize)]
    struct MemberView {
        id: String,
        username: String,
        status: String,
    }

    let member_views: Vec<MemberView> = members
        .into_iter()
        .map(|u| MemberView {
            id: u.id,
            username: u.username,
            status: if u.status == 1 {
                "active".to_string()
            } else {
                "disabled".to_string()
            },
        })
        .collect();

    Ok(Json(json!({
        "group": group,
        "members": member_views
    })))
}

pub async fn create_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateGroupRequest>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    let id = Uuid::new_v4().to_string();
    let group = state
        .provider
        .groups()
        .create(
            &id,
            Tenant::DEFAULT_ID,
            &req.name,
            req.description.as_deref(),
        )
        .await
        .map_err(AppError::Database)?;

    state
        .provider
        .audit()
        .log(AuditEvent {
            actor_id: Some(&auth.user.id),
            target_id: None,
            action: "group.create",
            resource_type: "group",
            resource_id: Some(&id),
            severity: AuditSeverity::Info,
            ip: None,
            ua: None,
            metadata: None,
        })
        .await
        .map_err(|e| {
            tracing::warn!("Failed to write audit log: {}", e);
            AppError::Database(e)
        })?;

    Ok(Json(json!({ "group": group })))
}

pub async fn update_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateGroupRequest>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    let _ = state
        .provider
        .groups()
        .find_by_id(&id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    state
        .provider
        .groups()
        .update(&id, &req.name, req.description.as_deref())
        .await
        .map_err(AppError::Database)?;

    state
        .provider
        .audit()
        .log(AuditEvent {
            actor_id: Some(&auth.user.id),
            target_id: None,
            action: "group.update",
            resource_type: "group",
            resource_id: Some(&id),
            severity: AuditSeverity::Info,
            ip: None,
            ua: None,
            metadata: None,
        })
        .await
        .map_err(|e| {
            tracing::warn!("Failed to write audit log: {}", e);
            AppError::Database(e)
        })?;

    let updated = state
        .provider
        .groups()
        .find_by_id(&id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    Ok(Json(json!({ "group": updated })))
}

pub async fn delete_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    let _ = state
        .provider
        .groups()
        .find_by_id(&id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    state
        .provider
        .groups()
        .delete(&id)
        .await
        .map_err(AppError::Database)?;

    state
        .provider
        .audit()
        .log(AuditEvent {
            actor_id: Some(&auth.user.id),
            target_id: None,
            action: "group.delete",
            resource_type: "group",
            resource_id: Some(&id),
            severity: AuditSeverity::Info,
            ip: None,
            ua: None,
            metadata: None,
        })
        .await
        .map_err(|e| {
            tracing::warn!("Failed to write audit log: {}", e);
            AppError::Database(e)
        })?;

    Ok(Json(json!({ "success": true })))
}

pub async fn add_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    let _ = state
        .provider
        .groups()
        .find_by_id(&id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    let user_id = req
        .get("user_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("user_id is required".into()))?;

    let _ = state
        .provider
        .users()
        .find_by_id(user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    state
        .provider
        .groups()
        .add_member(&id, user_id)
        .await
        .map_err(AppError::Database)?;

    state
        .provider
        .audit()
        .log(AuditEvent {
            actor_id: Some(&auth.user.id),
            target_id: Some(user_id),
            action: "group.member.add",
            resource_type: "group",
            resource_id: Some(&id),
            severity: AuditSeverity::Info,
            ip: None,
            ua: None,
            metadata: None,
        })
        .await
        .map_err(|e| {
            tracing::warn!("Failed to write audit log: {}", e);
            AppError::Database(e)
        })?;

    Ok(Json(json!({ "success": true })))
}

pub async fn remove_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, uid)): Path<(String, String)>,
) -> Result<Json<Value>> {
    require(&state.provider, &auth.user.id, "roles:manage").await?;

    state
        .provider
        .groups()
        .remove_member(&id, &uid)
        .await
        .map_err(AppError::Database)?;

    state
        .provider
        .audit()
        .log(AuditEvent {
            actor_id: Some(&auth.user.id),
            target_id: Some(&uid),
            action: "group.member.remove",
            resource_type: "group",
            resource_id: Some(&id),
            severity: AuditSeverity::Info,
            ip: None,
            ua: None,
            metadata: None,
        })
        .await
        .map_err(|e| {
            tracing::warn!("Failed to write audit log: {}", e);
            AppError::Database(e)
        })?;

    Ok(Json(json!({ "success": true })))
}
