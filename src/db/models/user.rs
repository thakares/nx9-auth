use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// User account status.
///
/// Stored as INTEGER in SQLite: 1 = Active, 2 = Disabled, 3 = Locked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    Active = 1,
    Disabled = 2,
    Locked = 3,
}

impl UserStatus {
    pub fn from_i32(v: i32) -> Self {
        match v {
            2 => Self::Disabled,
            3 => Self::Locked,
            _ => Self::Active,
        }
    }

    pub fn as_i32(self) -> i32 {
        self as i32
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Disabled => "disabled",
            Self::Locked => "locked",
        }
    }
}

impl std::fmt::Display for UserStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A user account row from the `users` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub slug: Option<String>,
    pub id: String,
    pub tenant_id: String,
    pub username: String,
    /// Argon2id PHC string — never expose in API responses.
    #[serde(skip_serializing)]
    pub password_hash: String,
    /// Raw integer status — use `status()` for the typed enum.
    pub status: i32,
    pub last_login_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl User {
    /// Typed status accessor.
    pub fn status(&self) -> UserStatus {
        UserStatus::from_i32(self.status)
    }

    pub fn is_active(&self) -> bool {
        self.status() == UserStatus::Active
    }
}
