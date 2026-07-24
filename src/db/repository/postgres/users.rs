use crate::db::repository::traits::UsersRepository;
use async_trait::async_trait;
use sqlx::PgPool;

use crate::db::models::{User, UserProfile};

pub struct PostgresUsersRepository {
    pub pool: PgPool,
}

#[async_trait]
impl UsersRepository for PostgresUsersRepository {
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

    async fn count(&self, tenant_id: &str) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE tenant_id = $1")
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }

    async fn count_by_status(&self, tenant_id: &str, status: i32) -> Result<i64, sqlx::Error> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM users WHERE tenant_id = $1 AND status = $2")
                .bind(tenant_id)
                .bind(status)
                .fetch_one(&self.pool)
                .await?;
        Ok(row.0)
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
            .bind(username)
            .fetch_optional(&self.pool)
            .await
    }

    async fn list(&self, tenant_id: &str) -> Result<Vec<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE tenant_id = $1 ORDER BY created_at DESC",
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
        VALUES ($1, $2, $3, $4, 1)
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
            "UPDATE users SET status = $1, updated_at = to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') WHERE id = $2",
        )
    .bind(status)
    .bind(id)
    .execute(&self.pool)
    .await?;
        Ok(())
    }

    async fn update_user_tenant(&self, id: &str, tenant_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE users SET tenant_id = $1, updated_at = to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') WHERE id = $2",
        )
        .bind(tenant_id)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn reassign_user_tenant_with_audit(
        &self,
        user_id: &str,
        destination_tenant_id: &str,
        actor_id: Option<&str>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        if user.tenant_id == destination_tenant_id {
            tx.commit().await?;
            return Ok(());
        }

        let from_tenant_id = user.tenant_id.clone();

        if from_tenant_id == crate::db::models::Tenant::DEFAULT_ID {
            let admin_rows: Vec<(String,)> = sqlx::query_as(
                "SELECT ur.user_id FROM user_roles ur JOIN roles r ON ur.role_id = r.id WHERE r.name = 'admin' FOR UPDATE",
            )
            .fetch_all(&mut *tx)
            .await?;

            let is_target_admin = admin_rows.iter().any(|r| r.0 == user_id);
            if is_target_admin && admin_rows.len() <= 1 {
                return Err(sqlx::Error::Protocol(
                    "cannot reassign the last system administrator away from default tenant".into(),
                ));
            }
        }

        let dest_exists: Option<(String,)> = sqlx::query_as("SELECT id FROM tenants WHERE id = $1")
            .bind(destination_tenant_id)
            .fetch_optional(&mut *tx)
            .await?;
        if dest_exists.is_none() {
            return Err(sqlx::Error::RowNotFound);
        }

        let collision: Option<(i64,)> =
            sqlx::query_as("SELECT 1 FROM users WHERE tenant_id = $1 AND username = $2")
                .bind(destination_tenant_id)
                .bind(&user.username)
                .fetch_optional(&mut *tx)
                .await?;
        if collision.is_some() {
            return Err(sqlx::Error::Protocol(format!(
                "username '{}' already exists in target tenant",
                user.username
            )));
        }

        let result = sqlx::query(
            "UPDATE users SET tenant_id = $1, updated_at = to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') WHERE id = $2 AND tenant_id = $3",
        )
        .bind(destination_tenant_id)
        .bind(user_id)
        .bind(&from_tenant_id)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() != 1 {
            return Err(sqlx::Error::RowNotFound);
        }

        let metadata = serde_json::json!({
            "user_id": user.id,
            "username": user.username,
            "from_tenant_id": from_tenant_id,
            "to_tenant_id": destination_tenant_id,
        })
        .to_string();

        let audit_id = uuid::Uuid::new_v4().to_string();
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
        .bind(actor_id)
        .bind(Some(&user.id))
        .bind("user.tenant_reassigned")
        .bind("user")
        .bind(Some(&user.id))
        .bind("info")
        .bind(ip_address)
        .bind(user_agent)
        .bind(&metadata)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn update_password_hash(&self, id: &str, password_hash: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE users SET password_hash = $1, updated_at = to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') WHERE id = $2",
        )
    .bind(password_hash)
    .bind(id)
    .execute(&self.pool)
    .await?;
        Ok(())
    }

    async fn set_last_login(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE users SET last_login_at = to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'), updated_at = to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') WHERE id = $1",
        )
    .bind(id)
    .execute(&self.pool)
    .await?;
        Ok(())
    }

    async fn username_exists(&self, tenant_id: &str, username: &str) -> Result<bool, sqlx::Error> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM users WHERE tenant_id = $1 AND username = $2")
                .bind(tenant_id)
                .bind(username)
                .fetch_one(&self.pool)
                .await?;
        Ok(row.0 > 0)
    }

    async fn get_profile(&self, user_id: &str) -> Result<Option<UserProfile>, sqlx::Error> {
        sqlx::query_as::<_, UserProfile>("SELECT * FROM user_profiles WHERE user_id = $1")
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
        VALUES ($1, $2, $3)
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
