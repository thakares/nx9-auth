use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Application {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub description: Option<String>,
    pub slug: Option<String>,
    pub enabled: bool,
    pub client_secret_hash: Option<String>,
    pub redirect_uris: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
