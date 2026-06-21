use sqlx::SqlitePool;

use crate::db::models::Session;

pub async fn create(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    token_hash: &str,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
    expires_at: &str,
) -> Result<Session, sqlx::Error> {
    sqlx::query_as::<_, Session>(
        r#"
        INSERT INTO sessions (id, user_id, token_hash, ip_address, user_agent, expires_at)
        VALUES (?, ?, ?, ?, ?, ?)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(token_hash)
    .bind(ip_address)
    .bind(user_agent)
    .bind(expires_at)
    .fetch_one(pool)
    .await
}

pub async fn find_by_token_hash(
    pool: &SqlitePool,
    token_hash: &str,
) -> Result<Option<Session>, sqlx::Error> {
    sqlx::query_as::<_, Session>("SELECT * FROM sessions WHERE token_hash = ? AND revoked = 0")
        .bind(token_hash)
        .fetch_optional(pool)
        .await
}

pub async fn revoke(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE sessions SET revoked = 1 WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn revoke_all_for_user(pool: &SqlitePool, user_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE sessions SET revoked = 1 WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_last_seen(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE sessions SET last_seen_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Delete sessions that are expired or revoked. Called once at startup.
pub async fn cleanup_expired(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM sessions
        WHERE revoked = 1
           OR expires_at < strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
        "#,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}
