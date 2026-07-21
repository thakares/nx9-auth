use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Permission {
    pub slug: Option<String>,
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}
