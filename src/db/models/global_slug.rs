use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GlobalSlug {
    pub slug: String,
    pub entity_type: String,
    pub entity_id: String,
    pub tenant_id: String,
    pub created_at: String,
}
