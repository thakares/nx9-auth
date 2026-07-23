use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Application {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub description: Option<String>,
    pub slug: Option<String>,
    pub client_id: String,
    pub enabled: bool,
    pub client_secret_hash: Option<String>,
    pub redirect_uris: Option<String>,
    pub scopes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Application {
    /// Return effective client ID string.
    pub fn get_client_id(&self) -> &str {
        &self.client_id
    }

    /// Parse configured redirect URLs.
    pub fn redirect_urls(&self) -> Vec<String> {
        let Some(raw) = &self.redirect_uris else {
            return Vec::new();
        };
        if let Ok(vec) = serde_json::from_str::<Vec<String>>(raw) {
            return vec;
        }
        raw.split([',', '\n', ' '])
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Parse configured scopes.
    pub fn scopes(&self) -> Vec<String> {
        let Some(raw) = &self.scopes else {
            return Vec::new();
        };
        if let Ok(vec) = serde_json::from_str::<Vec<String>>(raw) {
            return vec;
        }
        raw.split([',', ' '])
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Return true if application has configured client secret credentials.
    pub fn has_credentials(&self) -> bool {
        self.client_secret_hash.is_some()
    }
}
