use sqlx::SqlitePool;

use crate::db::models::ServiceAccount;

pub async fn create(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: &str,
    tenant_id: &str,
    name: &str,
    description: Option<&str>,
) -> Result<ServiceAccount, sqlx::Error> {
    sqlx::query_as::<_, ServiceAccount>(
        r#"
        INSERT INTO service_accounts (id, tenant_id, name, description)
        VALUES (?, ?, ?, ?)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(tenant_id)
    .bind(name)
    .bind(description)
    .fetch_one(&mut **tx)
    .await
}

pub async fn find_by_id(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<ServiceAccount>, sqlx::Error> {
    sqlx::query_as::<_, ServiceAccount>("SELECT * FROM service_accounts WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn list(pool: &SqlitePool, tenant_id: &str) -> Result<Vec<ServiceAccount>, sqlx::Error> {
    sqlx::query_as::<_, ServiceAccount>(
        "SELECT * FROM service_accounts WHERE tenant_id = ? ORDER BY name",
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await
}

pub async fn set_enabled(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: &str,
    enabled: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE service_accounts SET enabled = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
    )
    .bind(enabled)
    .bind(id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn delete(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM service_accounts WHERE id = ?")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn count(pool: &SqlitePool, tenant_id: &str) -> Result<i64, sqlx::Error> {
    let row: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM service_accounts WHERE tenant_id = ?")
            .bind(tenant_id)
            .fetch_one(pool)
            .await?;
    Ok(row.0)
}
