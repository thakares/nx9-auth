use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Allowed application membership roles (lightweight metadata only).
///
/// These do **not** grant global NX9-Auth RBAC permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApplicationMembershipRole {
    Owner,
    Admin,
    #[default]
    Member,
}

impl ApplicationMembershipRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Member => "member",
        }
    }

    /// Parse a role string. Returns `None` for invalid values.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "owner" => Some(Self::Owner),
            "admin" => Some(Self::Admin),
            "member" => Some(Self::Member),
            _ => None,
        }
    }
}

impl std::fmt::Display for ApplicationMembershipRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Assignment of an existing NX9-Auth user to a registered application.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApplicationMember {
    pub id: String,
    pub application_id: String,
    pub user_id: String,
    pub role: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl ApplicationMember {
    pub fn membership_role(&self) -> Option<ApplicationMembershipRole> {
        ApplicationMembershipRole::parse(&self.role)
    }
}
