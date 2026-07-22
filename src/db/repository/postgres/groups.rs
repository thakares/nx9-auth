use crate::db::models::{Group, User};
use crate::db::repository::traits::GroupsRepository;
use async_trait::async_trait;
use sqlx::PgPool;

pub struct PostgresGroupsRepository {
    pub pool: PgPool,
}

#[async_trait]
impl GroupsRepository for PostgresGroupsRepository {
    async fn list(&self, tenant_id: &str) -> Result<Vec<Group>, sqlx::Error> {
        let rows = sqlx::query_as::<_, Group>(
            r#"
            SELECT * FROM groups
            WHERE tenant_id = $1
            ORDER BY name ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Group>, sqlx::Error> {
        sqlx::query_as::<_, Group>("SELECT * FROM groups WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    async fn create(
        &self,
        id: &str,
        tenant_id: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<Group, sqlx::Error> {
        sqlx::query_as::<_, Group>(
            r#"
            INSERT INTO groups (id, tenant_id, name, description)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(name)
        .bind(description)
        .fetch_one(&self.pool)
        .await
    }

    async fn update(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE groups
            SET name = $1, description = $2, updated_at = to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"')
            WHERE id = $3
            "#,
        )
        .bind(name)
        .bind(description)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM groups WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn count_members(&self, group_id: &str) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM user_groups WHERE group_id = $1")
            .bind(group_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }

    async fn list_members(&self, group_id: &str) -> Result<Vec<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT u.*
            FROM users u
            JOIN user_groups ug ON u.id = ug.user_id
            WHERE ug.group_id = $1
            ORDER BY u.username ASC
            "#,
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn add_member(&self, group_id: &str, user_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO user_groups (user_id, group_id)
            VALUES ($1, $2)
            ON CONFLICT (user_id, group_id) DO NOTHING
            "#,
        )
        .bind(user_id)
        .bind(group_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove_member(&self, group_id: &str, user_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM user_groups WHERE user_id = $1 AND group_id = $2")
            .bind(user_id)
            .bind(group_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn count(&self, tenant_id: &str) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM groups WHERE tenant_id = $1")
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }
}
