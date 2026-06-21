use sqlx::SqlitePool;

/// Return all permission names held by a user (via their roles).
pub async fn list_for_user(pool: &SqlitePool, user_id: &str) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT DISTINCT p.name
        FROM permissions p
        JOIN role_permissions rp ON rp.permission_id = p.id
        JOIN user_roles ur       ON ur.role_id = rp.role_id
        WHERE ur.user_id = ?
        ORDER BY p.name
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|(name,)| name).collect())
}

/// Check if a user holds a specific named permission.
pub async fn user_has_permission(
    pool: &SqlitePool,
    user_id: &str,
    permission_name: &str,
) -> Result<bool, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM permissions p
        JOIN role_permissions rp ON rp.permission_id = p.id
        JOIN user_roles ur       ON ur.role_id = rp.role_id
        WHERE ur.user_id = ? AND p.name = ?
        "#,
    )
    .bind(user_id)
    .bind(permission_name)
    .fetch_one(pool)
    .await?;
    Ok(row.0 > 0)
}
