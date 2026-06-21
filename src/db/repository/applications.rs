use sqlx::SqlitePool;

use crate::db::models::Application;

pub async fn create(
    pool: &SqlitePool,
    id: &str,
    tenant_id: &str,
    name: &str,
    slug: &str,
) -> Result<Application, sqlx::Error> {
    sqlx::query_as::<_, Application>(
        r#"
        INSERT INTO applications (id, tenant_id, name, slug)
        VALUES (?, ?, ?, ?)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(tenant_id)
    .bind(name)
    .bind(slug)
    .fetch_one(pool)
    .await
}

pub async fn find_by_slug(
    pool: &SqlitePool,
    slug: &str,
) -> Result<Option<Application>, sqlx::Error> {
    sqlx::query_as::<_, Application>("SELECT * FROM applications WHERE slug = ?")
        .bind(slug)
        .fetch_optional(pool)
        .await
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Application>, sqlx::Error> {
    sqlx::query_as::<_, Application>("SELECT * FROM applications WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn list(pool: &SqlitePool, tenant_id: &str) -> Result<Vec<Application>, sqlx::Error> {
    sqlx::query_as::<_, Application>("SELECT * FROM applications WHERE tenant_id = ? ORDER BY name")
        .bind(tenant_id)
        .fetch_all(pool)
        .await
}

pub async fn set_enabled(pool: &SqlitePool, id: &str, enabled: bool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE applications SET enabled = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
    )
    .bind(enabled)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}
