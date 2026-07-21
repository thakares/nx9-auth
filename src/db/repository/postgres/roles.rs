use crate::db::repository::traits::RolesRepository;
use async_trait::async_trait;
use sqlx::PgPool;

use crate::db::models::Role;

pub struct PostgresRolesRepository {
    pub pool: PgPool,
}

#[async_trait]
impl RolesRepository for PostgresRolesRepository {
    async fn list_all(&self) -> Result<Vec<Role>, sqlx::Error> {
        sqlx::query_as::<_, Role>("SELECT * FROM roles ORDER BY name")
            .fetch_all(&self.pool)
            .await
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Role>, sqlx::Error> {
        sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE name = $1")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Role>, sqlx::Error> {
        sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    async fn list_for_user(&self, user_id: &str) -> Result<Vec<Role>, sqlx::Error> {
        sqlx::query_as::<_, Role>(
            r#"
        SELECT r.* FROM roles r
        JOIN user_roles ur ON ur.role_id = r.id
        WHERE ur.user_id = $1
        ORDER BY r.name
        "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn assign_to_user(&self, user_id: &str, role_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT OR IGNORE INTO user_roles (user_id, role_id) VALUES ($1, $2)")
            .bind(user_id)
            .bind(role_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn remove_from_user(&self, user_id: &str, role_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM user_roles WHERE user_id = $1 AND role_id = $2")
            .bind(user_id)
            .bind(role_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn admin_role_exists(&self) -> Result<bool, sqlx::Error> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM roles WHERE name = 'admin'")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0 > 0)
    }

    /// Create a new role.
    async fn create(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<Role, sqlx::Error> {
        sqlx::query_as::<_, Role>(
            r#"
        INSERT INTO roles (id, name, description)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .fetch_one(&self.pool)
        .await
    }

    /// Update role name/description.
    async fn update(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE roles SET name = $1, description = $2 WHERE id = $3")
            .bind(name)
            .bind(description)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete a role by id.
    async fn delete(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM roles WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// List user ids that hold a given role.
    async fn list_user_ids_for_role(&self, role_id: &str) -> Result<Vec<String>, sqlx::Error> {
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT user_id FROM user_roles WHERE role_id = $1 ORDER BY user_id")
                .bind(role_id)
                .fetch_all(&self.pool)
                .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }
}
