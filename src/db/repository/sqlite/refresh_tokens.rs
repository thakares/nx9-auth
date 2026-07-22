use crate::db::repository::traits::RefreshTokensRepository;
use async_trait::async_trait;
use sqlx::SqlitePool;

pub struct SqliteRefreshTokensRepository {
    pub pool: SqlitePool,
}

use crate::db::models::RefreshToken;

#[async_trait]
impl RefreshTokensRepository for SqliteRefreshTokensRepository {
    async fn create(
        &self,
        id: &str,
        user_id: &str,
        token_hash: &str,
        expires_at: &str,
    ) -> Result<RefreshToken, sqlx::Error> {
        sqlx::query_as::<_, RefreshToken>(
            r#"
        INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at)
        VALUES (?, ?, ?, ?)
        RETURNING *
        "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
    }

    async fn find_by_hash(&self, token_hash: &str) -> Result<Option<RefreshToken>, sqlx::Error> {
        sqlx::query_as::<_, RefreshToken>(
            "SELECT * FROM refresh_tokens WHERE token_hash = ? AND revoked = 0",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
    }

    async fn revoke(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE refresh_tokens SET revoked = 1 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn revoke_all_for_user(&self, user_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE refresh_tokens SET revoked = 1 WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
