use crate::db::repository::traits::AuditRepository;
use async_trait::async_trait;
use sqlx::PgPool;

use crate::db::models::{AuditFilter, AuditLog};

pub struct PostgresAuditRepository {
    pub pool: PgPool,
}

#[async_trait]
impl AuditRepository for PostgresAuditRepository {
    async fn count(&self) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_logs")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }

    #[allow(clippy::too_many_arguments)]
    async fn insert(
        &self,
        id: &str,
        actor_user_id: Option<&str>,
        target_user_id: Option<&str>,
        action: &str,
        resource_type: &str,
        resource_id: Option<&str>,
        severity: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        metadata_json: Option<&str>,
    ) -> Result<AuditLog, sqlx::Error> {
        sqlx::query_as::<_, AuditLog>(
            r#"
        INSERT INTO audit_logs (
            id, actor_user_id, target_user_id,
            action, resource_type, resource_id,
            severity, ip_address, user_agent, metadata_json
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING *
        "#,
        )
        .bind(id)
        .bind(actor_user_id)
        .bind(target_user_id)
        .bind(action)
        .bind(resource_type)
        .bind(resource_id)
        .bind(severity)
        .bind(ip_address)
        .bind(user_agent)
        .bind(metadata_json)
        .fetch_one(&self.pool)
        .await
    }

    async fn list_recent(&self, limit: i64) -> Result<Vec<AuditLog>, sqlx::Error> {
        sqlx::query_as::<_, AuditLog>("SELECT * FROM audit_logs ORDER BY created_at DESC LIMIT $1")
            .bind(limit)
            .fetch_all(&self.pool)
            .await
    }

    async fn list_filtered(&self, filter: &AuditFilter) -> Result<Vec<AuditLog>, sqlx::Error> {
        let search_like = filter
            .search
            .as_ref()
            .map(|s| format!("%{}%", s.replace('%', "\\%")));

        sqlx::query_as::<_, AuditLog>(
            r#"
        SELECT * FROM audit_logs
        WHERE ($1::text IS NULL OR actor_user_id = $1)
          AND ($2::text IS NULL OR action = $2)
          AND ($3::text IS NULL OR resource_type = $3)
          AND ($4::text IS NULL OR severity = $4)
          AND ($5::text IS NULL OR created_at >= $5)
          AND ($6::text IS NULL OR created_at <= $6)
          AND (
                $7::text IS NULL
             OR action LIKE $7 ESCAPE '\'
             OR resource_type LIKE $7 ESCAPE '\'
             OR resource_id LIKE $7 ESCAPE '\'
             OR ip_address LIKE $7 ESCAPE '\'
             OR metadata_json LIKE $7 ESCAPE '\'
          )
          AND (
                $8::boolean IS NULL
             OR ($8::boolean = TRUE AND action NOT LIKE '%fail%' AND action NOT LIKE '%denied%' AND severity != 'critical')
             OR ($8::boolean = FALSE AND (action LIKE '%fail%' OR action LIKE '%denied%' OR severity = 'critical'))
          )
        ORDER BY created_at DESC
        LIMIT $9 OFFSET $10
        "#,
        )
        .bind(filter.actor_user_id.as_deref())
        .bind(filter.action.as_deref())
        .bind(filter.resource_type.as_deref())
        .bind(filter.severity.as_deref())
        .bind(filter.since.as_deref())
        .bind(filter.until.as_deref())
        .bind(search_like.as_deref())
        .bind(filter.success)
        .bind(filter.limit)
        .bind(filter.offset)
        .fetch_all(&self.pool)
        .await
    }

    async fn count_filtered(&self, filter: &AuditFilter) -> Result<i64, sqlx::Error> {
        let search_like = filter
            .search
            .as_ref()
            .map(|s| format!("%{}%", s.replace('%', "\\%")));

        let row: (i64,) = sqlx::query_as(
            r#"
        SELECT COUNT(*) FROM audit_logs
        WHERE ($1::text IS NULL OR actor_user_id = $1)
          AND ($2::text IS NULL OR action = $2)
          AND ($3::text IS NULL OR resource_type = $3)
          AND ($4::text IS NULL OR severity = $4)
          AND ($5::text IS NULL OR created_at >= $5)
          AND ($6::text IS NULL OR created_at <= $6)
          AND (
                $7::text IS NULL
             OR action LIKE $7 ESCAPE '\'
             OR resource_type LIKE $7 ESCAPE '\'
             OR resource_id LIKE $7 ESCAPE '\'
             OR ip_address LIKE $7 ESCAPE '\'
             OR metadata_json LIKE $7 ESCAPE '\'
          )
          AND (
                $8::boolean IS NULL
             OR ($8::boolean = TRUE AND action NOT LIKE '%fail%' AND action NOT LIKE '%denied%' AND severity != 'critical')
             OR ($8::boolean = FALSE AND (action LIKE '%fail%' OR action LIKE '%denied%' OR severity = 'critical'))
          )
        "#,
        )
        .bind(filter.actor_user_id.as_deref())
        .bind(filter.action.as_deref())
        .bind(filter.resource_type.as_deref())
        .bind(filter.severity.as_deref())
        .bind(filter.since.as_deref())
        .bind(filter.until.as_deref())
        .bind(search_like.as_deref())
        .bind(filter.success)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }
}
