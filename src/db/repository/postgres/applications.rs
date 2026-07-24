use crate::db::models::Application;
use crate::db::repository::postgres::global_slugs::{
    release_slug_by_name_postgres, release_slug_postgres, reserve_slug_postgres,
};
use crate::db::repository::traits::ApplicationsRepository;
use async_trait::async_trait;
use sqlx::PgPool;

pub struct PostgresApplicationsRepository {
    pub pool: PgPool,
}

#[async_trait]
impl ApplicationsRepository for PostgresApplicationsRepository {
    async fn create_with_audit(
        &self,
        id: &str,
        tenant_id: &str,
        name: &str,
        slug: &str,
        client_id: &str,
        client_secret_hash: Option<&str>,
        description: Option<&str>,
        redirect_uris: Option<&str>,
        scopes: Option<&str>,
        audit_event: Option<crate::audit::AuditEvent<'_>>,
    ) -> Result<Application, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        reserve_slug_postgres(&mut tx, slug, "application", id, tenant_id).await?;

        let app = sqlx::query_as::<_, Application>(
            r#"
        INSERT INTO applications (id, tenant_id, name, slug, client_id, client_secret_hash, description, redirect_uris, scopes)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(name)
        .bind(slug)
        .bind(client_id)
        .bind(client_secret_hash)
        .bind(description)
        .bind(redirect_uris)
        .bind(scopes)
        .fetch_one(&mut *tx)
        .await?;

        if let Some(event) = audit_event {
            let audit_id = uuid::Uuid::new_v4().to_string();
            let severity_str = event.severity.to_string();
            sqlx::query(
                r#"
            INSERT INTO audit_logs (
                id, actor_user_id, target_user_id,
                action, resource_type, resource_id,
                severity, ip_address, user_agent, metadata_json
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            )
            .bind(&audit_id)
            .bind(event.actor_id)
            .bind(event.target_id)
            .bind(event.action)
            .bind(event.resource_type)
            .bind(event.resource_id)
            .bind(&severity_str)
            .bind(event.ip)
            .bind(event.ua)
            .bind(event.metadata)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(app)
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Application>, sqlx::Error> {
        sqlx::query_as::<_, Application>("SELECT * FROM applications WHERE slug = $1")
            .bind(slug)
            .fetch_optional(&self.pool)
            .await
    }

    async fn find_by_client_id(&self, client_id: &str) -> Result<Option<Application>, sqlx::Error> {
        sqlx::query_as::<_, Application>("SELECT * FROM applications WHERE client_id = $1")
            .bind(client_id)
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

    async fn update_secret_hash(&self, id: &str, secret_hash: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE applications SET client_secret_hash = $1, updated_at = to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') WHERE id = $2",
        )
        .bind(secret_hash)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn rotate_secret_with_audit(
        &self,
        id: &str,
        secret_hash: &str,
        audit_event: Option<crate::audit::AuditEvent<'_>>,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let res = sqlx::query(
            "UPDATE applications SET client_secret_hash = $1, updated_at = to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') WHERE id = $2",
        )
        .bind(secret_hash)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        if res.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        if let Some(event) = audit_event {
            let audit_id = uuid::Uuid::new_v4().to_string();
            let severity_str = event.severity.to_string();
            sqlx::query(
                r#"
            INSERT INTO audit_logs (
                id, actor_user_id, target_user_id,
                action, resource_type, resource_id,
                severity, ip_address, user_agent, metadata_json
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            )
            .bind(&audit_id)
            .bind(event.actor_id)
            .bind(event.target_id)
            .bind(event.action)
            .bind(event.resource_type)
            .bind(event.resource_id)
            .bind(&severity_str)
            .bind(event.ip)
            .bind(event.ua)
            .bind(event.metadata)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn update(
        &self,
        id: &str,
        name: &str,
        slug: &str,
        description: Option<&str>,
        redirect_uris: Option<&str>,
        scopes: Option<&str>,
        enabled: bool,
    ) -> Result<(), sqlx::Error> {
        let existing = self.find_by_id(id).await?.ok_or(sqlx::Error::RowNotFound)?;

        let existing_slug_str = existing.slug.as_deref().unwrap_or("");

        if slug == existing_slug_str {
            // Unchanged slug: registry no-op
            sqlx::query(
                r#"
            UPDATE applications
            SET name = $1, description = $2, redirect_uris = $3, scopes = $4, enabled = $5,
                updated_at = to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"')
            WHERE id = $6
            "#,
            )
            .bind(name)
            .bind(description)
            .bind(redirect_uris)
            .bind(scopes)
            .bind(enabled)
            .bind(id)
            .execute(&self.pool)
            .await?;
        } else {
            // Changed slug: single transaction reserve -> update -> release
            let mut tx = self.pool.begin().await?;

            reserve_slug_postgres(&mut tx, slug, "application", id, &existing.tenant_id).await?;

            sqlx::query(
                r#"
            UPDATE applications
            SET name = $1, slug = $2, description = $3, redirect_uris = $4, scopes = $5, enabled = $6,
                updated_at = to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"')
            WHERE id = $7
            "#,
            )
            .bind(name)
            .bind(slug)
            .bind(description)
            .bind(redirect_uris)
            .bind(scopes)
            .bind(enabled)
            .bind(id)
            .execute(&mut *tx)
            .await?;

            if !existing_slug_str.is_empty() {
                release_slug_by_name_postgres(&mut tx, existing_slug_str, "application", id)
                    .await?;
            }

            tx.commit().await?;
        }

        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        release_slug_postgres(&mut tx, "application", id).await?;

        sqlx::query("DELETE FROM applications WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
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
