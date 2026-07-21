use crate::db::models::{Group, User};
use crate::db::repository::traits::GroupsRepository;
use async_trait::async_trait;
use sqlx::SqlitePool;

pub struct SqliteGroupsRepository {
    pub pool: SqlitePool,
}

#[async_trait]
impl GroupsRepository for SqliteGroupsRepository {
    async fn list(&self, tenant_id: &str) -> Result<Vec<Group>, sqlx::Error> {
        let rows = sqlx::query_as::<_, Group>(
            r#"
            SELECT * FROM groups
            WHERE tenant_id = ?
            ORDER BY name ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Group>, sqlx::Error> {
        sqlx::query_as::<_, Group>("SELECT * FROM groups WHERE id = ?")
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
            VALUES (?, ?, ?, ?)
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
            SET name = ?, description = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
            WHERE id = ?
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
        sqlx::query("DELETE FROM groups WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn count_members(&self, group_id: &str) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM user_groups WHERE group_id = ?")
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
            WHERE ug.group_id = ?
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
            VALUES (?, ?)
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
        sqlx::query("DELETE FROM user_groups WHERE user_id = ? AND group_id = ?")
            .bind(user_id)
            .bind(group_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn count(&self, tenant_id: &str) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM groups WHERE tenant_id = ?")
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }
}
