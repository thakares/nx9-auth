use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl Tenant {
    pub const DEFAULT_ID: &'static str = "00000000-0000-0000-0000-000000000001";
    pub const DEFAULT_SLUG: &'static str = "default";
}
