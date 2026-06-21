use sqlx::SqlitePool;

use crate::db::models::User;

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn find_by_username(
    pool: &SqlitePool,
    username: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await
}

pub async fn list(pool: &SqlitePool, tenant_id: &str) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE tenant_id = ? ORDER BY created_at DESC")
        .bind(tenant_id)
        .fetch_all(pool)
        .await
}

pub async fn create(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
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
    .fetch_one(&mut **tx)
    .await
}

pub async fn update_status(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: &str,
    status: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE users SET status = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
    )
    .bind(status)
    .bind(id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn update_password_hash(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: &str,
    password_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE users SET password_hash = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
    )
    .bind(password_hash)
    .bind(id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn set_last_login(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE users SET last_login_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
    )
    .bind(id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn username_exists(
    pool: &SqlitePool,
    tenant_id: &str,
    username: &str,
) -> Result<bool, sqlx::Error> {
    let row: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM users WHERE tenant_id = ? AND username = ?")
            .bind(tenant_id)
            .bind(username)
            .fetch_one(pool)
            .await?;
    Ok(row.0 > 0)
}

/// Count users that have the admin role.
pub async fn count_admins(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(DISTINCT ur.user_id)
        FROM user_roles ur
        JOIN roles r ON r.id = ur.role_id
        WHERE r.name = 'admin'
        "#,
    )
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}
