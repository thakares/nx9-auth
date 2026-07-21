use crate::db::repository::traits::TokensRepository;
use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::db::models::ApiToken;

pub struct SqliteTokensRepository {
    pub pool: SqlitePool,
}

#[async_trait]
impl TokensRepository for SqliteTokensRepository {
    async fn create(
        &self,
        id: &str,
        user_id: &str,
        name: &str,
        token_hash: &str,
        expires_at: Option<&str>,
    ) -> Result<ApiToken, sqlx::Error> {
        sqlx::query_as::<_, ApiToken>(
            r#"
        INSERT INTO api_tokens (id, user_id, name, token_hash, expires_at)
        VALUES (?, ?, ?, ?, ?)
        RETURNING *
        "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(name)
        .bind(token_hash)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
    }

    async fn find_by_hash(&self, token_hash: &str) -> Result<Option<ApiToken>, sqlx::Error> {
        sqlx::query_as::<_, ApiToken>(
            "SELECT * FROM api_tokens WHERE token_hash = ? AND revoked = 0",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
    }

    async fn list_for_user(&self, user_id: &str) -> Result<Vec<ApiToken>, sqlx::Error> {
        sqlx::query_as::<_, ApiToken>(
            "SELECT * FROM api_tokens WHERE user_id = ? ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<ApiToken>, sqlx::Error> {
        sqlx::query_as::<_, ApiToken>("SELECT * FROM api_tokens WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    async fn revoke(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE api_tokens SET revoked = 1 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_last_used(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
        "UPDATE api_tokens SET last_used_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
    )
    .bind(id)
    .execute(&self.pool)
    .await?;
        Ok(())
    }
}
