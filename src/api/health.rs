use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};

use crate::state::AppState;

/// GET /health
pub async fn health(State(state): State<AppState>) -> Json<Value> {
    let backend = state
        .config
        .database
        .resolved_url()
        .map(|(_, b)| b.to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let db_status = match state.provider.tenants().list().await {
        Ok(_) => "connected",
        Err(_) => "error",
    };

    Json(json!({
        "status": if db_status == "connected" { "ok" } else { "degraded" },
        "db_backend": backend,
        "database_status": db_status
    }))
}
