use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};

use crate::state::AppState;

/// GET /version
///
/// Returns build metadata baked in at compile time via `build.rs` and active db_backend.
pub async fn version(State(state): State<AppState>) -> Json<Value> {
    let backend = state
        .config
        .database
        .resolved_url()
        .map(|(_, b)| b.to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    Json(json!({
        "name":         env!("CARGO_PKG_NAME"),
        "version":      env!("CARGO_PKG_VERSION"),
        "git_commit":   env!("GIT_COMMIT"),
        "build_date":   env!("BUILD_DATE"),
        "rust_version": env!("RUST_VERSION"),
        "db_backend":   backend,
    }))
}
