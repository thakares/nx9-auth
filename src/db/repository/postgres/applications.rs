use crate::db::repository::traits::ApplicationsRepository;
use async_trait::async_trait;
use sqlx::PgPool;

use crate::db::models::Application;

pub struct PostgresApplicationsRepository {
    pub pool: PgPool,
}

#[async_trait]
impl ApplicationsRepository for PostgresApplicationsRepository {
    async fn create(
        &self,
        id: &str,
        tenant_id: &str,
        name: &str,
        slug: &str,
    ) -> Result<Application, sqlx::Error> {
        sqlx::query_as::<_, Application>(
            r#"
        INSERT INTO applications (id, tenant_id, name, slug)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(name)
        .bind(slug)
        .fetch_one(&self.pool)
        .await
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Application>, sqlx::Error> {
        sqlx::query_as::<_, Application>("SELECT * FROM applications WHERE slug = $1")
            .bind(slug)
            .fetch_optional(&self.pool)
            .await
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Application>, sqlx::Error> {
        sqlx::query_as::<_, Application>("SELECT * FROM applications WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    async fn list(&self, tenant_id: &str) -> Result<Vec<Application>, sqlx::Error> {
        sqlx::query_as::<_, Application>(
            "SELECT * FROM applications WHERE tenant_id = $1 ORDER BY name",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn set_enabled(&self, id: &str, enabled: bool) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE applications SET enabled = $1, updated_at = to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') WHERE id = $2",
        )
    .bind(enabled)
    .bind(id)
    .execute(&self.pool)
    .await?;
        Ok(())
    }

    async fn update(
        &self,
        id: &str,
        name: &str,
        slug: &str,
        enabled: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
        UPDATE applications
        SET name = $1, slug = $2, enabled = $3,
            updated_at = to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"')
        WHERE id = $4
        "#,
        )
        .bind(name)
        .bind(slug)
        .bind(enabled)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM applications WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn count(&self, tenant_id: &str) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM applications WHERE tenant_id = $1")
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }
}
