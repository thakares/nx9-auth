use axum::Json;
use serde_json::{Value, json};

/// GET /health
pub async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
