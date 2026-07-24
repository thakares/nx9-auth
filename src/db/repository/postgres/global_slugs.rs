use crate::db::models::GlobalSlug;
use crate::db::repository::traits::GlobalSlugsRepository;
use sqlx::PgPool;

pub struct PostgresGlobalSlugsRepository {
    pub pool: PgPool,
}

#[async_trait::async_trait]
impl GlobalSlugsRepository for PostgresGlobalSlugsRepository {
    async fn find_by_slug(&self, slug: &str) -> Result<Option<GlobalSlug>, sqlx::Error> {
        sqlx::query_as::<_, GlobalSlug>(
            "SELECT slug, entity_type, entity_id, tenant_id, created_at::text FROM global_slugs WHERE slug = $1"
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
    }
}

pub async fn reserve_slug_postgres(
    conn: &mut sqlx::PgConnection,
    slug: &str,
    entity_type: &str,
    entity_id: &str,
    tenant_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO global_slugs (slug, entity_type, entity_id, tenant_id) VALUES ($1, $2, $3, $4)"
    )
    .bind(slug)
    .bind(entity_type)
    .bind(entity_id)
    .bind(tenant_id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn release_slug_postgres(
    conn: &mut sqlx::PgConnection,
    entity_type: &str,
    entity_id: &str,
) -> Result<u64, sqlx::Error> {
    let res = sqlx::query("DELETE FROM global_slugs WHERE entity_type = $1 AND entity_id = $2")
        .bind(entity_type)
        .bind(entity_id)
        .execute(conn)
        .await?;
    Ok(res.rows_affected())
}

pub async fn release_slug_by_name_postgres(
    conn: &mut sqlx::PgConnection,
    slug: &str,
    entity_type: &str,
    entity_id: &str,
) -> Result<u64, sqlx::Error> {
    let res = sqlx::query(
        "DELETE FROM global_slugs WHERE slug = $1 AND entity_type = $2 AND entity_id = $3",
    )
    .bind(slug)
    .bind(entity_type)
    .bind(entity_id)
    .execute(conn)
    .await?;
    Ok(res.rows_affected())
}
