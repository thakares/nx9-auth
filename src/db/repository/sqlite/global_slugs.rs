use crate::db::models::GlobalSlug;
use crate::db::repository::traits::GlobalSlugsRepository;
use sqlx::SqlitePool;

pub struct SqliteGlobalSlugsRepository {
    pub pool: SqlitePool,
}

#[async_trait::async_trait]
impl GlobalSlugsRepository for SqliteGlobalSlugsRepository {
    async fn find_by_slug(&self, slug: &str) -> Result<Option<GlobalSlug>, sqlx::Error> {
        sqlx::query_as::<_, GlobalSlug>(
            "SELECT slug, entity_type, entity_id, tenant_id, created_at FROM global_slugs WHERE slug = ?"
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
    }
}

pub async fn reserve_slug_sqlite(
    conn: &mut sqlx::SqliteConnection,
    slug: &str,
    entity_type: &str,
    entity_id: &str,
    tenant_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO global_slugs (slug, entity_type, entity_id, tenant_id) VALUES (?, ?, ?, ?)",
    )
    .bind(slug)
    .bind(entity_type)
    .bind(entity_id)
    .bind(tenant_id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn release_slug_sqlite(
    conn: &mut sqlx::SqliteConnection,
    entity_type: &str,
    entity_id: &str,
) -> Result<u64, sqlx::Error> {
    let res = sqlx::query("DELETE FROM global_slugs WHERE entity_type = ? AND entity_id = ?")
        .bind(entity_type)
        .bind(entity_id)
        .execute(conn)
        .await?;
    Ok(res.rows_affected())
}

pub async fn release_slug_by_name_sqlite(
    conn: &mut sqlx::SqliteConnection,
    slug: &str,
    entity_type: &str,
    entity_id: &str,
) -> Result<u64, sqlx::Error> {
    let res = sqlx::query(
        "DELETE FROM global_slugs WHERE slug = ? AND entity_type = ? AND entity_id = ?",
    )
    .bind(slug)
    .bind(entity_type)
    .bind(entity_id)
    .execute(conn)
    .await?;
    Ok(res.rows_affected())
}
