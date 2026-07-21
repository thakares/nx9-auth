use sqlx::SqlitePool;

use crate::db::models::Tenant;
use crate::db::repository::traits::TenantsRepository;

pub struct SqliteTenantsRepository {
    pub pool: SqlitePool,
}

#[async_trait::async_trait]
impl TenantsRepository for SqliteTenantsRepository {
    async fn find_by_id(&self, id: &str) -> Result<Option<Tenant>, sqlx::Error> {
        let row = sqlx::query_as::<_, Tenant>(
            "SELECT id, name, slug, enabled, created_at, updated_at FROM tenants WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Tenant>, sqlx::Error> {
        let row = sqlx::query_as::<_, Tenant>(
            "SELECT id, name, slug, enabled, created_at, updated_at FROM tenants WHERE slug = ?",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    async fn list(&self) -> Result<Vec<Tenant>, sqlx::Error> {
        let rows = sqlx::query_as::<_, Tenant>(
            "SELECT id, name, slug, enabled, created_at, updated_at FROM tenants ORDER BY name ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn create(
        &self,
        id: &str,
        name: &str,
        slug: Option<&str>,
    ) -> Result<Tenant, sqlx::Error> {
        let slug = slug.unwrap_or(id);
        let row = sqlx::query_as::<_, Tenant>(
            r#"
            INSERT INTO tenants (id, name, slug, enabled)
            VALUES (?, ?, ?, 1)
            RETURNING id, name, slug, enabled, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(slug)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    async fn update(&self, id: &str, name: &str, slug: Option<&str>) -> Result<(), sqlx::Error> {
        let slug = slug.unwrap_or(name);
        sqlx::query(
            r#"
            UPDATE tenants
            SET name = ?, slug = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
            WHERE id = ?
            "#,
        )
        .bind(name)
        .bind(slug)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn set_enabled(&self, id: &str, enabled: bool) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE tenants
            SET enabled = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
            WHERE id = ?
            "#,
        )
        .bind(enabled as i32)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM tenants WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
