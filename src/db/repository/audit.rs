use sqlx::SqlitePool;

use crate::db::models::AuditLog;

#[allow(clippy::too_many_arguments)]
pub async fn insert(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: &str,
    actor_user_id: Option<&str>,
    target_user_id: Option<&str>,
    action: &str,
    resource_type: &str,
    resource_id: Option<&str>,
    severity: &str,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
    metadata_json: Option<&str>,
) -> Result<AuditLog, sqlx::Error> {
    sqlx::query_as::<_, AuditLog>(
        r#"
        INSERT INTO audit_logs (
            id, actor_user_id, target_user_id,
            action, resource_type, resource_id,
            severity, ip_address, user_agent, metadata_json
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(actor_user_id)
    .bind(target_user_id)
    .bind(action)
    .bind(resource_type)
    .bind(resource_id)
    .bind(severity)
    .bind(ip_address)
    .bind(user_agent)
    .bind(metadata_json)
    .fetch_one(&mut **tx)
    .await
}

pub async fn list_recent(pool: &SqlitePool, limit: i64) -> Result<Vec<AuditLog>, sqlx::Error> {
    sqlx::query_as::<_, AuditLog>("SELECT * FROM audit_logs ORDER BY created_at DESC LIMIT ?")
        .bind(limit)
        .fetch_all(pool)
        .await
}
