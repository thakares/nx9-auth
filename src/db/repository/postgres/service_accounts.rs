use crate::db::repository::traits::ServiceAccountsRepository;
use async_trait::async_trait;
use sqlx::PgPool;

use crate::db::models::ServiceAccount;

pub struct PostgresServiceAccountsRepository {
    pub pool: PgPool,
}

#[async_trait]
impl ServiceAccountsRepository for PostgresServiceAccountsRepository {
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
        VALUES ($1, $2, $3, $4)
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
        sqlx::query_as::<_, ServiceAccount>("SELECT * FROM service_accounts WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    async fn list(&self, tenant_id: &str) -> Result<Vec<ServiceAccount>, sqlx::Error> {
        sqlx::query_as::<_, ServiceAccount>(
            "SELECT * FROM service_accounts WHERE tenant_id = $1 ORDER BY name",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn set_enabled(&self, id: &str, enabled: bool) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE service_accounts SET enabled = $1, updated_at = to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') WHERE id = $2",
        )
    .bind(enabled)
    .bind(id)
    .execute(&self.pool)
    .await?;
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM service_accounts WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn count(&self, tenant_id: &str) -> Result<i64, sqlx::Error> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM service_accounts WHERE tenant_id = $1")
                .bind(tenant_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(row.0)
    }
}
