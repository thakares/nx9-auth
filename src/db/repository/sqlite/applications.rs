use crate::db::repository::traits::ApplicationsRepository;
use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::db::models::Application;

pub struct SqliteApplicationsRepository {
    pub pool: SqlitePool,
}

#[async_trait]
impl ApplicationsRepository for SqliteApplicationsRepository {
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
        VALUES (?, ?, ?, ?)
        RETURNING id, tenant_id, name, slug, enabled, created_at, updated_at, NULL as description, NULL as client_secret_hash, NULL as redirect_uris
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
        sqlx::query_as::<_, Application>("SELECT id, tenant_id, name, slug, enabled, created_at, updated_at, NULL as description, NULL as client_secret_hash, NULL as redirect_uris FROM applications WHERE slug = ?")
            .bind(slug)
            .fetch_optional(&self.pool)
            .await
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Application>, sqlx::Error> {
        sqlx::query_as::<_, Application>("SELECT id, tenant_id, name, slug, enabled, created_at, updated_at, NULL as description, NULL as client_secret_hash, NULL as redirect_uris FROM applications WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    async fn list(&self, tenant_id: &str) -> Result<Vec<Application>, sqlx::Error> {
        sqlx::query_as::<_, Application>(
            "SELECT id, tenant_id, name, slug, enabled, created_at, updated_at, NULL as description, NULL as client_secret_hash, NULL as redirect_uris FROM applications WHERE tenant_id = ? ORDER BY name",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn set_enabled(&self, id: &str, enabled: bool) -> Result<(), sqlx::Error> {
        sqlx::query(
        "UPDATE applications SET enabled = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
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
        SET name = ?, slug = ?, enabled = ?,
            updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
        WHERE id = ?
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
        sqlx::query("DELETE FROM applications WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn count(&self, tenant_id: &str) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM applications WHERE tenant_id = ?")
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }
}
