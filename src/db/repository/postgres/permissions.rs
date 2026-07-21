use crate::db::repository::traits::PermissionsRepository;
use async_trait::async_trait;
use sqlx::PgPool;

use crate::db::models::Permission;

pub struct PostgresPermissionsRepository {
    pub pool: PgPool,
}

#[async_trait]
impl PermissionsRepository for PostgresPermissionsRepository {
    /// List all permissions defined in the system.
    async fn list_all(&self) -> Result<Vec<Permission>, sqlx::Error> {
        sqlx::query_as::<_, Permission>("SELECT * FROM permissions ORDER BY name")
            .fetch_all(&self.pool)
            .await
    }

    /// List permissions assigned to a role.
    async fn list_for_role(&self, role_id: &str) -> Result<Vec<Permission>, sqlx::Error> {
        sqlx::query_as::<_, Permission>(
            r#"
        SELECT p.* FROM permissions p
        JOIN role_permissions rp ON rp.permission_id = p.id
        WHERE rp.role_id = $1
        ORDER BY p.name
        "#,
        )
        .bind(role_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Assign a permission to a role (no-op if already assigned).
    async fn assign_to_role(&self, role_id: &str, permission_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT OR IGNORE INTO role_permissions (role_id, permission_id) VALUES ($1, $2)",
        )
        .bind(role_id)
        .bind(permission_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Remove a permission from a role.
    async fn remove_from_role(
        &self,
        role_id: &str,
        permission_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM role_permissions WHERE role_id = $1 AND permission_id = $2")
            .bind(role_id)
            .bind(permission_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Clear all permissions for a role.
    async fn clear_for_role(&self, role_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM role_permissions WHERE role_id = $1")
            .bind(role_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Find a permission by name.
    async fn find_by_name(&self, name: &str) -> Result<Option<Permission>, sqlx::Error> {
        sqlx::query_as::<_, Permission>("SELECT * FROM permissions WHERE name = $1")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
    }

    /// Find a permission by id.
    async fn find_by_id(&self, id: &str) -> Result<Option<Permission>, sqlx::Error> {
        sqlx::query_as::<_, Permission>("SELECT * FROM permissions WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Return all permission names held by a user (via their roles).
    async fn list_for_user(&self, user_id: &str) -> Result<Vec<String>, sqlx::Error> {
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
        SELECT DISTINCT p.name
        FROM permissions p
        JOIN role_permissions rp ON rp.permission_id = p.id
        JOIN user_roles ur       ON ur.role_id = rp.role_id
        WHERE ur.user_id = $1
        ORDER BY p.name
        "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(name,)| name).collect())
    }

    /// Check if a user holds a specific named permission.
    async fn user_has_permission(
        &self,
        user_id: &str,
        permission_name: &str,
    ) -> Result<bool, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            r#"
        SELECT COUNT(*)
        FROM permissions p
        JOIN role_permissions rp ON rp.permission_id = p.id
        JOIN user_roles ur       ON ur.role_id = rp.role_id
        WHERE ur.user_id = $1 AND p.name = $2
        "#,
        )
        .bind(user_id)
        .bind(permission_name)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0 > 0)
    }
}
