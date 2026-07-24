use crate::db::models::{AuditFilter, AuditLog};
use crate::db::repository::traits::AuditRepository;
use async_trait::async_trait;
use sqlx::SqlitePool;

pub struct SqliteAuditRepository {
    pub pool: SqlitePool,
}

#[async_trait]
impl AuditRepository for SqliteAuditRepository {
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
        sqlx::query_as::<_, AuditLog>(r#"
            INSERT INTO audit_logs (id, actor_user_id, target_user_id, action, resource_type, resource_id, severity, ip_address, user_agent, metadata_json)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING *
        "#)
        .bind(id).bind(actor_user_id).bind(target_user_id).bind(action).bind(resource_type)
        .bind(resource_id).bind(severity).bind(ip_address).bind(user_agent).bind(metadata_json)
        .fetch_one(&self.pool).await
    }

    async fn list_recent(&self, limit: i64) -> Result<Vec<AuditLog>, sqlx::Error> {
        sqlx::query_as::<_, AuditLog>("SELECT * FROM audit_logs ORDER BY created_at DESC LIMIT ?")
            .bind(limit)
            .fetch_all(&self.pool)
            .await
    }

    async fn list_filtered(&self, filter: &AuditFilter) -> Result<Vec<AuditLog>, sqlx::Error> {
        list_filtered(&self.pool, filter).await
    }

    async fn count_filtered(&self, filter: &AuditFilter) -> Result<i64, sqlx::Error> {
        let search_like = search_like(filter);
        let success_val = filter.success.map(|b| if b { 1i32 } else { 0i32 });
        let row: (i64,) = sqlx::query_as(FILTER_COUNT_SQL)
            .bind(filter.actor_user_id.as_deref())
            .bind(filter.action.as_deref())
            .bind(filter.resource_type.as_deref())
            .bind(filter.resource_id.as_deref())
            .bind(filter.severity.as_deref())
            .bind(filter.since.as_deref())
            .bind(filter.until.as_deref())
            .bind(search_like.as_deref())
            .bind(success_val)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }
}

fn search_like(filter: &AuditFilter) -> Option<String> {
    filter
        .search
        .as_ref()
        .map(|s| format!("%{}%", s.replace('%', "\\%")))
}

async fn list_filtered(
    pool: &SqlitePool,
    filter: &AuditFilter,
) -> Result<Vec<AuditLog>, sqlx::Error> {
    let search_like = search_like(filter);
    let success_val = filter.success.map(|b| if b { 1i32 } else { 0i32 });
    sqlx::query_as::<_, AuditLog>(FILTER_LIST_SQL)
        .bind(filter.actor_user_id.as_deref())
        .bind(filter.action.as_deref())
        .bind(filter.resource_type.as_deref())
        .bind(filter.resource_id.as_deref())
        .bind(filter.severity.as_deref())
        .bind(filter.since.as_deref())
        .bind(filter.until.as_deref())
        .bind(search_like.as_deref())
        .bind(success_val)
        .bind(filter.limit)
        .bind(filter.offset)
        .fetch_all(pool)
        .await
}

const FILTER_LIST_SQL: &str = r#"
    SELECT * FROM audit_logs
    WHERE (?1 IS NULL OR actor_user_id = ?1)
      AND (?2 IS NULL OR action = ?2)
      AND (?3 IS NULL OR resource_type = ?3)
      AND (?4 IS NULL OR resource_id = ?4)
      AND (?5 IS NULL OR severity = ?5)
      AND (?6 IS NULL OR created_at >= ?6)
      AND (?7 IS NULL OR created_at <= ?7)
      AND (?8 IS NULL OR action LIKE ?8 ESCAPE '\' OR resource_type LIKE ?8 ESCAPE '\' OR resource_id LIKE ?8 ESCAPE '\' OR ip_address LIKE ?8 ESCAPE '\' OR metadata_json LIKE ?8 ESCAPE '\')
      AND (?9 IS NULL OR (?9 = 1 AND action NOT LIKE '%fail%' AND action NOT LIKE '%denied%' AND severity != 'critical') OR (?9 = 0 AND (action LIKE '%fail%' OR action LIKE '%denied%' OR severity = 'critical')))
    ORDER BY created_at DESC LIMIT ?10 OFFSET ?11
"#;

const FILTER_COUNT_SQL: &str = r#"
    SELECT COUNT(*) FROM audit_logs
    WHERE (?1 IS NULL OR actor_user_id = ?1)
      AND (?2 IS NULL OR action = ?2)
      AND (?3 IS NULL OR resource_type = ?3)
      AND (?4 IS NULL OR resource_id = ?4)
      AND (?5 IS NULL OR severity = ?5)
      AND (?6 IS NULL OR created_at >= ?6)
      AND (?7 IS NULL OR created_at <= ?7)
      AND (?8 IS NULL OR action LIKE ?8 ESCAPE '\' OR resource_type LIKE ?8 ESCAPE '\' OR resource_id LIKE ?8 ESCAPE '\' OR ip_address LIKE ?8 ESCAPE '\' OR metadata_json LIKE ?8 ESCAPE '\')
      AND (?9 IS NULL OR (?9 = 1 AND action NOT LIKE '%fail%' AND action NOT LIKE '%denied%' AND severity != 'critical') OR (?9 = 0 AND (action LIKE '%fail%' OR action LIKE '%denied%' OR severity = 'critical')))
"#;
