use sqlx::SqlitePool;

use crate::{
    db::{models::Application, repository::applications as repo},
    error::AppError,
};

pub async fn create(
    pool: &SqlitePool,
    tenant_id: &str,
    name: &str,
    slug: &str,
) -> Result<Application, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    repo::create(pool, &id, tenant_id, name, slug)
        .await
        .map_err(AppError::Database)
}

pub async fn list(pool: &SqlitePool, tenant_id: &str) -> Result<Vec<Application>, AppError> {
    repo::list(pool, tenant_id)
        .await
        .map_err(AppError::Database)
}

pub async fn find_by_slug(pool: &SqlitePool, slug: &str) -> Result<Application, AppError> {
    repo::find_by_slug(pool, slug)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)
}
