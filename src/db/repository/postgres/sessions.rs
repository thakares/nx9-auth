use crate::db::repository::traits::SessionsRepository;
use async_trait::async_trait;
use sqlx::PgPool;

use crate::db::models::Session;

pub struct PostgresSessionsRepository {
    pub pool: PgPool,
}

#[async_trait]
impl SessionsRepository for PostgresSessionsRepository {
    async fn create(
        &self,
        id: &str,
        user_id: &str,
        token_hash: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        expires_at: &str,
    ) -> Result<Session, sqlx::Error> {
        sqlx::query_as::<_, Session>(
            r#"
        INSERT INTO sessions (id, user_id, token_hash, ip_address, user_agent, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(token_hash)
        .bind(ip_address)
        .bind(user_agent)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
    }

    async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<Session>, sqlx::Error> {
        sqlx::query_as::<_, Session>("SELECT * FROM sessions WHERE token_hash = $1 AND revoked = 0")
            .bind(token_hash)
            .fetch_optional(&self.pool)
            .await
    }

    async fn revoke(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE sessions SET revoked = 1 WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn revoke_all_for_user(&self, user_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE sessions SET revoked = 1 WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_last_seen(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE sessions SET last_seen_at = to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') WHERE id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// List active (non-revoked, non-expired) sessions for a user.
    async fn list_active_for_user(&self, user_id: &str) -> Result<Vec<Session>, sqlx::Error> {
        sqlx::query_as::<_, Session>(
            r#"
        SELECT * FROM sessions
        WHERE user_id = $1
          AND revoked = 0
          AND expires_at >= to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"')
        ORDER BY last_seen_at DESC
        "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn list_all_active(&self) -> Result<Vec<Session>, sqlx::Error> {
        sqlx::query_as::<_, Session>(
            r#"
        SELECT * FROM sessions
        WHERE revoked = 0
          AND expires_at >= to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"')
        ORDER BY last_seen_at DESC
        "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Count active sessions system-wide.
    async fn count_active(&self) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            r#"
        SELECT COUNT(*) FROM sessions
        WHERE revoked = 0
          AND expires_at >= to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"')
        "#,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    /// Delete sessions that are expired or revoked. Called once at startup.
    async fn cleanup_expired(&self) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
        DELETE FROM sessions
        WHERE revoked = 1
           OR expires_at < to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"')
        "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    async fn revoke_others(&self, user_id: &str, except_id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE sessions SET revoked = 1 WHERE user_id = $1 AND id != $2 AND revoked = 0",
        )
        .bind(user_id)
        .bind(except_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }
}
