use sqlx::PgPool;

use crate::db::models::Tenant;
use crate::db::repository::traits::TenantsRepository;

pub struct PostgresTenantsRepository {
    pub pool: PgPool,
}

#[async_trait::async_trait]
impl TenantsRepository for PostgresTenantsRepository {
    async fn find_by_id(&self, id: &str) -> Result<Option<Tenant>, sqlx::Error> {
        let row = sqlx::query_as::<_, Tenant>(
            "SELECT id, name, slug, enabled, created_at::text, updated_at::text FROM tenants WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Tenant>, sqlx::Error> {
        let row = sqlx::query_as::<_, Tenant>(
            "SELECT id, name, slug, enabled, created_at::text, updated_at::text FROM tenants WHERE slug = $1",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    async fn list(&self) -> Result<Vec<Tenant>, sqlx::Error> {
        let rows = sqlx::query_as::<_, Tenant>(
            "SELECT id, name, slug, enabled, created_at::text, updated_at::text FROM tenants ORDER BY name ASC",
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
            VALUES ($1, $2, $3, true)
            RETURNING id, name, slug, enabled, created_at::text, updated_at::text
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
            SET name = $1, slug = $2, updated_at = CURRENT_TIMESTAMP
            WHERE id = $3
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
            SET enabled = $1, updated_at = CURRENT_TIMESTAMP
            WHERE id = $2
            "#,
        )
        .bind(enabled)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM tenants WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
