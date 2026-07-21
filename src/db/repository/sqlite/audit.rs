use crate::db::repository::traits::AuditRepository;
use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::db::models::AuditLog;

pub struct SqliteAuditRepository {
    pub pool: SqlitePool,
}

/// Filtered audit log query. All filters are optional.
#[derive(Debug, Default)]
pub struct AuditFilter {
    pub actor_user_id: Option<String>,
    pub action: Option<String>,
    pub resource_type: Option<String>,
    pub severity: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub search: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

#[async_trait]
impl AuditRepository for SqliteAuditRepository {
    /// Count all audit log entries.
    async fn count(&self) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_logs")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }
    #[allow(clippy::too_many_arguments)]
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
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
        sqlx::query_as::<_, AuditLog>("SELECT * FROM audit_logs ORDER BY created_at DESC LIMIT ?")
            .bind(limit)
            .fetch_all(&self.pool)
            .await
    }

    async fn list_filtered(&self, filter: &AuditFilter) -> Result<Vec<AuditLog>, sqlx::Error> {
        // Build a dynamic but simple filter using COALESCE-style optional matches.
        // Empty optionals are treated as wildcards via OR IS NULL pattern with bind of None.
        let search_like = filter
            .search
            .as_ref()
            .map(|s| format!("%{}%", s.replace('%', "\\%")));

        sqlx::query_as::<_, AuditLog>(
            r#"
        SELECT * FROM audit_logs
        WHERE (?1 IS NULL OR actor_user_id = ?1)
          AND (?2 IS NULL OR action = ?2)
          AND (?3 IS NULL OR resource_type = ?3)
          AND (?4 IS NULL OR severity = ?4)
          AND (?5 IS NULL OR created_at >= ?5)
          AND (?6 IS NULL OR created_at <= ?6)
          AND (
                ?7 IS NULL
             OR action LIKE ?7 ESCAPE '\'
             OR resource_type LIKE ?7 ESCAPE '\'
             OR resource_id LIKE ?7 ESCAPE '\'
             OR ip_address LIKE ?7 ESCAPE '\'
             OR metadata_json LIKE ?7 ESCAPE '\'
          )
        ORDER BY created_at DESC
        LIMIT ?8 OFFSET ?9
        "#,
        )
        .bind(filter.actor_user_id.as_deref())
        .bind(filter.action.as_deref())
        .bind(filter.resource_type.as_deref())
        .bind(filter.severity.as_deref())
        .bind(filter.since.as_deref())
        .bind(filter.until.as_deref())
        .bind(search_like.as_deref())
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
        WHERE (?1 IS NULL OR actor_user_id = ?1)
          AND (?2 IS NULL OR action = ?2)
          AND (?3 IS NULL OR resource_type = ?3)
          AND (?4 IS NULL OR severity = ?4)
          AND (?5 IS NULL OR created_at >= ?5)
          AND (?6 IS NULL OR created_at <= ?6)
          AND (
                ?7 IS NULL
             OR action LIKE ?7 ESCAPE '\'
             OR resource_type LIKE ?7 ESCAPE '\'
             OR resource_id LIKE ?7 ESCAPE '\'
             OR ip_address LIKE ?7 ESCAPE '\'
             OR metadata_json LIKE ?7 ESCAPE '\'
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
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }
}
