use crate::db::models::ApplicationMember;
use crate::db::repository::traits::ApplicationMembersRepository;
use async_trait::async_trait;
use sqlx::SqlitePool;

pub struct SqliteApplicationMembersRepository {
    pub pool: SqlitePool,
}

#[async_trait]
impl ApplicationMembersRepository for SqliteApplicationMembersRepository {
    async fn list_by_application(
        &self,
        application_id: &str,
    ) -> Result<Vec<ApplicationMember>, sqlx::Error> {
        sqlx::query_as::<_, ApplicationMember>(
            r#"
            SELECT id, application_id, user_id, role, enabled, created_at, updated_at
            FROM application_members
            WHERE application_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(application_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn list_by_user(&self, user_id: &str) -> Result<Vec<ApplicationMember>, sqlx::Error> {
        sqlx::query_as::<_, ApplicationMember>(
            r#"
            SELECT id, application_id, user_id, role, enabled, created_at, updated_at
            FROM application_members
            WHERE user_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn find(
        &self,
        application_id: &str,
        user_id: &str,
    ) -> Result<Option<ApplicationMember>, sqlx::Error> {
        sqlx::query_as::<_, ApplicationMember>(
            r#"
            SELECT id, application_id, user_id, role, enabled, created_at, updated_at
            FROM application_members
            WHERE application_id = ? AND user_id = ?
            "#,
        )
        .bind(application_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn add(
        &self,
        id: &str,
        application_id: &str,
        user_id: &str,
        role: &str,
    ) -> Result<ApplicationMember, sqlx::Error> {
        sqlx::query_as::<_, ApplicationMember>(
            r#"
            INSERT INTO application_members (id, application_id, user_id, role, enabled)
            VALUES (?, ?, ?, ?, 1)
            RETURNING id, application_id, user_id, role, enabled, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(application_id)
        .bind(user_id)
        .bind(role)
        .fetch_one(&self.pool)
        .await
    }

    async fn update_role(
        &self,
        application_id: &str,
        user_id: &str,
        role: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE application_members
            SET role = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
            WHERE application_id = ? AND user_id = ?
            "#,
        )
        .bind(role)
        .bind(application_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn set_enabled(
        &self,
        application_id: &str,
        user_id: &str,
        enabled: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE application_members
            SET enabled = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
            WHERE application_id = ? AND user_id = ?
            "#,
        )
        .bind(enabled)
        .bind(application_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove(&self, application_id: &str, user_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM application_members
            WHERE application_id = ? AND user_id = ?
            "#,
        )
        .bind(application_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn add_with_audit(
        &self,
        id: &str,
        application_id: &str,
        user_id: &str,
        role: &str,
        audit_event: Option<crate::audit::AuditEvent<'_>>,
    ) -> Result<ApplicationMember, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let app_tenant: Option<(String,)> =
            sqlx::query_as("SELECT tenant_id FROM applications WHERE id = ?")
                .bind(application_id)
                .fetch_optional(&mut *tx)
                .await?;
        let app_tenant_id = match app_tenant {
            Some(t) => t.0,
            None => return Err(sqlx::Error::RowNotFound),
        };

        let user_tenant: Option<(String,)> =
            sqlx::query_as("SELECT tenant_id FROM users WHERE id = ?")
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;
        let user_tenant_id = match user_tenant {
            Some(t) => t.0,
            None => return Err(sqlx::Error::RowNotFound),
        };

        if app_tenant_id != user_tenant_id {
            return Err(sqlx::Error::Protocol(
                "user and application must belong to the same tenant".into(),
            ));
        }

        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM application_members WHERE application_id = ? AND user_id = ?",
        )
        .bind(application_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;
        if existing.is_some() {
            return Err(sqlx::Error::Protocol(
                "user is already a member of this application".into(),
            ));
        }

        let member = sqlx::query_as::<_, ApplicationMember>(
            r#"
            INSERT INTO application_members (id, application_id, user_id, role, enabled)
            VALUES (?, ?, ?, ?, 1)
            RETURNING id, application_id, user_id, role, enabled, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(application_id)
        .bind(user_id)
        .bind(role)
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
        Ok(member)
    }

    async fn update_role_with_audit(
        &self,
        application_id: &str,
        user_id: &str,
        role: &str,
        audit_event: Option<crate::audit::AuditEvent<'_>>,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let app_tenant: Option<(String,)> =
            sqlx::query_as("SELECT tenant_id FROM applications WHERE id = ?")
                .bind(application_id)
                .fetch_optional(&mut *tx)
                .await?;
        let app_tenant_id = match app_tenant {
            Some(t) => t.0,
            None => return Err(sqlx::Error::RowNotFound),
        };

        let user_tenant: Option<(String,)> =
            sqlx::query_as("SELECT tenant_id FROM users WHERE id = ?")
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;
        let user_tenant_id = match user_tenant {
            Some(t) => t.0,
            None => return Err(sqlx::Error::RowNotFound),
        };

        if app_tenant_id != user_tenant_id {
            return Err(sqlx::Error::Protocol(
                "user and application must belong to the same tenant".into(),
            ));
        }

        let result = sqlx::query(
            r#"
            UPDATE application_members
            SET role = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
            WHERE application_id = ? AND user_id = ?
            "#,
        )
        .bind(role)
        .bind(application_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() != 1 {
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

    async fn set_enabled_with_audit(
        &self,
        application_id: &str,
        user_id: &str,
        enabled: bool,
        audit_event: Option<crate::audit::AuditEvent<'_>>,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let app_tenant: Option<(String,)> =
            sqlx::query_as("SELECT tenant_id FROM applications WHERE id = ?")
                .bind(application_id)
                .fetch_optional(&mut *tx)
                .await?;
        let app_tenant_id = match app_tenant {
            Some(t) => t.0,
            None => return Err(sqlx::Error::RowNotFound),
        };

        let user_tenant: Option<(String,)> =
            sqlx::query_as("SELECT tenant_id FROM users WHERE id = ?")
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;
        let user_tenant_id = match user_tenant {
            Some(t) => t.0,
            None => return Err(sqlx::Error::RowNotFound),
        };

        if app_tenant_id != user_tenant_id {
            return Err(sqlx::Error::Protocol(
                "user and application must belong to the same tenant".into(),
            ));
        }

        let result = sqlx::query(
            r#"
            UPDATE application_members
            SET enabled = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
            WHERE application_id = ? AND user_id = ?
            "#,
        )
        .bind(enabled)
        .bind(application_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() != 1 {
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

    async fn remove_with_audit(
        &self,
        application_id: &str,
        user_id: &str,
        audit_event: Option<crate::audit::AuditEvent<'_>>,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let app_tenant: Option<(String,)> =
            sqlx::query_as("SELECT tenant_id FROM applications WHERE id = ?")
                .bind(application_id)
                .fetch_optional(&mut *tx)
                .await?;
        let app_tenant_id = match app_tenant {
            Some(t) => t.0,
            None => return Err(sqlx::Error::RowNotFound),
        };

        let user_tenant: Option<(String,)> =
            sqlx::query_as("SELECT tenant_id FROM users WHERE id = ?")
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;
        let user_tenant_id = match user_tenant {
            Some(t) => t.0,
            None => return Err(sqlx::Error::RowNotFound),
        };

        if app_tenant_id != user_tenant_id {
            return Err(sqlx::Error::Protocol(
                "user and application must belong to the same tenant".into(),
            ));
        }

        let result = sqlx::query(
            r#"
            DELETE FROM application_members
            WHERE application_id = ? AND user_id = ?
            "#,
        )
        .bind(application_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() != 1 {
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
}
