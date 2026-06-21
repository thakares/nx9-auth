use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Application {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub slug: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}
