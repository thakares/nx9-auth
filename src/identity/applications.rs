use crate::{db::models::Application, error::AppError};

pub async fn create(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    tenant_id: &str,
    name: &str,
    slug: &str,
) -> Result<Application, AppError> {
    let name = name.trim();
    let slug = slug.trim();
    if name.is_empty() || slug.is_empty() {
        return Err(AppError::InvalidInput(
            "name and slug cannot be empty".into(),
        ));
    }
    if provider
        .applications()
        .find_by_slug(slug)
        .await
        .map_err(AppError::Database)?
        .is_some()
    {
        return Err(AppError::Conflict(format!("slug '{slug}' already exists")));
    }
    let id = uuid::Uuid::new_v4().to_string();
    provider
        .applications()
        .create(&id, tenant_id, name, slug)
        .await
        .map_err(AppError::Database)
}

pub async fn list(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    tenant_id: &str,
) -> Result<Vec<Application>, AppError> {
    provider
        .applications()
        .list(tenant_id)
        .await
        .map_err(AppError::Database)
}

pub async fn get(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    id: &str,
) -> Result<Application, AppError> {
    provider
        .applications()
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)
}

pub async fn find_by_slug(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    slug: &str,
) -> Result<Application, AppError> {
    provider
        .applications()
        .find_by_slug(slug)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)
}

pub async fn update(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    id: &str,
    name: &str,
    slug: &str,
    enabled: bool,
) -> Result<Application, AppError> {
    let _ = provider.applications().find_by_id(id).await?;
    let name = name.trim();
    let slug = slug.trim();
    if name.is_empty() || slug.is_empty() {
        return Err(AppError::InvalidInput(
            "name and slug cannot be empty".into(),
        ));
    }
    if let Some(other) = provider
        .applications()
        .find_by_slug(slug)
        .await
        .map_err(AppError::Database)?
    {
        if other.id != id {
            return Err(AppError::Conflict(format!("slug '{slug}' already exists")));
        }
    }
    provider
        .applications()
        .update(id, name, slug, enabled)
        .await
        .map_err(AppError::Database)?;
    provider
        .applications()
        .find_by_id(id)
        .await
        .map_err(crate::error::AppError::Database)?
        .ok_or_else(|| crate::error::AppError::NotFound)
}

pub async fn set_enabled(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    id: &str,
    enabled: bool,
) -> Result<(), AppError> {
    let _ = provider.applications().find_by_id(id).await?;
    provider
        .applications()
        .set_enabled(id, enabled)
        .await
        .map_err(AppError::Database)
}

pub async fn delete(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    id: &str,
) -> Result<(), AppError> {
    let _ = provider.applications().find_by_id(id).await?;
    provider
        .applications()
        .delete(id)
        .await
        .map_err(AppError::Database)
}
