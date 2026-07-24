use sqlx::PgPool;

use crate::db::models::Tenant;
use crate::db::repository::postgres::global_slugs::{
    release_slug_by_name_postgres, release_slug_postgres, reserve_slug_postgres,
};
use crate::db::repository::traits::TenantsRepository;
use crate::identity::slug::{slugify, validate_slug};

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
        let final_slug = match slug {
            Some(s) if !s.trim().is_empty() => {
                let trimmed = s.trim();
                validate_slug(trimmed).map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
                trimmed.to_string()
            }
            _ => slugify(name).map_err(|e| sqlx::Error::Protocol(e.to_string()))?,
        };

        let mut tx = self.pool.begin().await?;

        let row = sqlx::query_as::<_, Tenant>(
            r#"
            INSERT INTO tenants (id, name, slug, enabled)
            VALUES ($1, $2, $3, true)
            RETURNING id, name, slug, enabled, created_at::text, updated_at::text
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(&final_slug)
        .fetch_one(&mut *tx)
        .await?;

        reserve_slug_postgres(&mut tx, &final_slug, "tenant", id, id).await?;

        tx.commit().await?;
        Ok(row)
    }

    async fn update(&self, id: &str, name: &str, slug: Option<&str>) -> Result<(), sqlx::Error> {
        let existing = self.find_by_id(id).await?.ok_or(sqlx::Error::RowNotFound)?;

        let target_slug = match slug {
            Some(s) if !s.trim().is_empty() => {
                let trimmed = s.trim();
                validate_slug(trimmed).map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
                trimmed.to_string()
            }
            _ => existing.slug.clone().unwrap_or_else(|| id.to_string()),
        };

        let existing_slug_str = existing.slug.as_deref().unwrap_or("");

        if target_slug == existing_slug_str {
            // Unchanged slug: registry no-op
            sqlx::query(
                r#"
                UPDATE tenants
                SET name = $1, updated_at = CURRENT_TIMESTAMP
                WHERE id = $2
                "#,
            )
            .bind(name)
            .bind(id)
            .execute(&self.pool)
            .await?;
        } else {
            // Changed slug: single transaction reserve -> update -> release
            let mut tx = self.pool.begin().await?;

            reserve_slug_postgres(&mut tx, &target_slug, "tenant", id, id).await?;

            sqlx::query(
                r#"
                UPDATE tenants
                SET name = $1, slug = $2, updated_at = CURRENT_TIMESTAMP
                WHERE id = $3
                "#,
            )
            .bind(name)
            .bind(&target_slug)
            .bind(id)
            .execute(&mut *tx)
            .await?;

            if !existing_slug_str.is_empty() {
                release_slug_by_name_postgres(&mut tx, existing_slug_str, "tenant", id).await?;
            }

            tx.commit().await?;
        }

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
        let mut tx = self.pool.begin().await?;

        release_slug_postgres(&mut tx, "tenant", id).await?;

        sqlx::query("DELETE FROM tenants WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }
}
