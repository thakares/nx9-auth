use axum::Json;
use serde_json::{Value, json};

/// GET /version
///
/// Returns build metadata baked in at compile time via `build.rs`.
pub async fn version() -> Json<Value> {
    Json(json!({
        "name":         env!("CARGO_PKG_NAME"),
        "version":      env!("CARGO_PKG_VERSION"),
        "git_commit":   env!("GIT_COMMIT"),
        "build_date":   env!("BUILD_DATE"),
        "rust_version": env!("RUST_VERSION"),
    }))
}
