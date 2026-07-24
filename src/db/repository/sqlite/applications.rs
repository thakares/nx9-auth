use crate::db::models::Application;
use crate::db::repository::sqlite::global_slugs::{
    release_slug_by_name_sqlite, release_slug_sqlite, reserve_slug_sqlite,
};
use crate::db::repository::traits::ApplicationsRepository;
use async_trait::async_trait;
use sqlx::SqlitePool;

pub struct SqliteApplicationsRepository {
    pub pool: SqlitePool,
}

#[async_trait]
impl ApplicationsRepository for SqliteApplicationsRepository {
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

        reserve_slug_sqlite(&mut tx, slug, "application", id, tenant_id).await?;

        let app = sqlx::query_as::<_, Application>(
            r#"
        INSERT INTO applications (id, tenant_id, name, slug, client_id, client_secret_hash, description, redirect_uris, scopes)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING id, tenant_id, name, slug, client_id, description, enabled, client_secret_hash, redirect_uris, scopes, created_at, updated_at
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
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
        sqlx::query_as::<_, Application>("SELECT id, tenant_id, name, slug, client_id, description, enabled, client_secret_hash, redirect_uris, scopes, created_at, updated_at FROM applications WHERE slug = ?")
            .bind(slug)
            .fetch_optional(&self.pool)
            .await
    }

    async fn find_by_client_id(&self, client_id: &str) -> Result<Option<Application>, sqlx::Error> {
        sqlx::query_as::<_, Application>("SELECT id, tenant_id, name, slug, client_id, description, enabled, client_secret_hash, redirect_uris, scopes, created_at, updated_at FROM applications WHERE client_id = ?")
            .bind(client_id)
            .fetch_optional(&self.pool)
            .await
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Application>, sqlx::Error> {
        sqlx::query_as::<_, Application>("SELECT id, tenant_id, name, slug, client_id, description, enabled, client_secret_hash, redirect_uris, scopes, created_at, updated_at FROM applications WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    async fn list(&self, tenant_id: &str) -> Result<Vec<Application>, sqlx::Error> {
        sqlx::query_as::<_, Application>(
            "SELECT id, tenant_id, name, slug, client_id, description, enabled, client_secret_hash, redirect_uris, scopes, created_at, updated_at FROM applications WHERE tenant_id = ? ORDER BY name",
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

    async fn update_secret_hash(&self, id: &str, secret_hash: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE applications SET client_secret_hash = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
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
            "UPDATE applications SET client_secret_hash = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
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
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
            SET name = ?, description = ?, redirect_uris = ?, scopes = ?, enabled = ?,
                updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
            WHERE id = ?
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

            reserve_slug_sqlite(&mut tx, slug, "application", id, &existing.tenant_id).await?;

            sqlx::query(
                r#"
            UPDATE applications
            SET name = ?, slug = ?, description = ?, redirect_uris = ?, scopes = ?, enabled = ?,
                updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
            WHERE id = ?
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
                release_slug_by_name_sqlite(&mut tx, existing_slug_str, "application", id).await?;
            }

            tx.commit().await?;
        }

        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        release_slug_sqlite(&mut tx, "application", id).await?;

        sqlx::query("DELETE FROM applications WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
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
