use axum::{Json, extract::State};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

use crate::{
    error::Result,
    identity::permissions as identity_perms,
    middleware::{auth::AuthUser, permissions::require},
    state::AppState,
};

#[derive(Serialize)]
pub struct PermissionResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub group: String,
}

/// GET /api/v1/permissions
pub async fn list_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>> {
    // Readable by anyone who can manage roles or audit
    if require(&state.provider, &auth.user.id, "roles:manage")
        .await
        .is_err()
    {
        require(&state.provider, &auth.user.id, "audit:view").await?;
    }

    let perms = identity_perms::list_permissions(&state.provider).await?;
    let views: Vec<PermissionResponse> = perms
        .into_iter()
        .map(|p| {
            let group = p
                .name
                .split_once(':')
                .map(|(g, _)| g.to_string())
                .unwrap_or_else(|| "general".into());
            PermissionResponse {
                id: p.id,
                name: p.name,
                description: p.description,
                group,
            }
        })
        .collect();

    // Also group for matrix view
    let mut grouped: BTreeMap<String, Vec<&PermissionResponse>> = BTreeMap::new();
    for p in &views {
        grouped.entry(p.group.clone()).or_default().push(p);
    }

    let groups: Vec<Value> = grouped
        .into_iter()
        .map(|(group, items)| {
            json!({
                "group": group,
                "permissions": items,
            })
        })
        .collect();

    Ok(Json(json!({
        "permissions": views,
        "groups": groups,
    })))
}
