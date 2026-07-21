use crate::db::repository::traits::UsersRepository;
use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::db::models::User;

pub struct SqliteUsersRepository {
    pub pool: SqlitePool,
}

/// User profile fields from `user_profiles`.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct UserProfile {
    pub user_id: String,
    pub email: Option<String>,
    pub full_name: Option<String>,
    pub avatar_url: Option<String>,
    pub metadata_json: Option<String>,
}

#[async_trait]
impl UsersRepository for SqliteUsersRepository {
    /// Count users with a given status in a tenant.
    async fn count_by_status(&self, tenant_id: &str, status: i32) -> Result<i64, sqlx::Error> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM users WHERE tenant_id = ? AND status = ?")
                .bind(tenant_id)
                .bind(status)
                .fetch_one(&self.pool)
                .await?;
        Ok(row.0)
    }

    /// Count all users in a tenant.
    async fn count(&self, tenant_id: &str) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE tenant_id = ?")
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }

    /// Count users that have the admin role.
    async fn count_admins(&self) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            r#"
        SELECT COUNT(DISTINCT ur.user_id)
        FROM user_roles ur
        JOIN roles r ON r.id = ur.role_id
        WHERE r.name = 'admin'
        "#,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }
    async fn find_by_id(&self, id: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(&self.pool)
            .await
    }

    async fn list(&self, tenant_id: &str) -> Result<Vec<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE tenant_id = ? ORDER BY created_at DESC",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn create(
        &self,
        id: &str,
        tenant_id: &str,
        username: &str,
        password_hash: &str,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
        INSERT INTO users (id, tenant_id, username, password_hash, status)
        VALUES (?, ?, ?, ?, 1)
        RETURNING *
        "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(username)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await
    }

    async fn update_status(&self, id: &str, status: i32) -> Result<(), sqlx::Error> {
        sqlx::query(
        "UPDATE users SET status = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
    )
    .bind(status)
    .bind(id)
    .execute(&self.pool)
    .await?;
        Ok(())
    }

    async fn update_password_hash(&self, id: &str, password_hash: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
        "UPDATE users SET password_hash = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
    )
    .bind(password_hash)
    .bind(id)
    .execute(&self.pool)
    .await?;
        Ok(())
    }

    async fn set_last_login(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
        "UPDATE users SET last_login_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
    )
    .bind(id)
    .execute(&self.pool)
    .await?;
        Ok(())
    }

    async fn username_exists(&self, tenant_id: &str, username: &str) -> Result<bool, sqlx::Error> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM users WHERE tenant_id = ? AND username = ?")
                .bind(tenant_id)
                .bind(username)
                .fetch_one(&self.pool)
                .await?;
        Ok(row.0 > 0)
    }

    async fn get_profile(&self, user_id: &str) -> Result<Option<UserProfile>, sqlx::Error> {
        sqlx::query_as::<_, UserProfile>("SELECT * FROM user_profiles WHERE user_id = ?")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
    }

    async fn upsert_profile(
        &self,
        user_id: &str,
        email: Option<&str>,
        full_name: Option<&str>,
    ) -> Result<UserProfile, sqlx::Error> {
        sqlx::query_as::<_, UserProfile>(
            r#"
        INSERT INTO user_profiles (user_id, email, full_name)
        VALUES (?, ?, ?)
        ON CONFLICT(user_id) DO UPDATE SET
            email = excluded.email,
            full_name = excluded.full_name
        RETURNING *
        "#,
        )
        .bind(user_id)
        .bind(email)
        .bind(full_name)
        .fetch_one(&self.pool)
        .await
    }
}
