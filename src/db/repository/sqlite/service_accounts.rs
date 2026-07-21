use crate::db::repository::traits::ServiceAccountsRepository;
use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::db::models::ServiceAccount;

pub struct SqliteServiceAccountsRepository {
    pub pool: SqlitePool,
}

#[async_trait]
impl ServiceAccountsRepository for SqliteServiceAccountsRepository {
    async fn create(
        &self,
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
        .fetch_one(&self.pool)
        .await
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<ServiceAccount>, sqlx::Error> {
        sqlx::query_as::<_, ServiceAccount>("SELECT * FROM service_accounts WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    async fn list(&self, tenant_id: &str) -> Result<Vec<ServiceAccount>, sqlx::Error> {
        sqlx::query_as::<_, ServiceAccount>(
            "SELECT * FROM service_accounts WHERE tenant_id = ? ORDER BY name",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn set_enabled(&self, id: &str, enabled: bool) -> Result<(), sqlx::Error> {
        sqlx::query(
        "UPDATE service_accounts SET enabled = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
    )
    .bind(enabled)
    .bind(id)
    .execute(&self.pool)
    .await?;
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM service_accounts WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn count(&self, tenant_id: &str) -> Result<i64, sqlx::Error> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM service_accounts WHERE tenant_id = ?")
                .bind(tenant_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(row.0)
    }
}
