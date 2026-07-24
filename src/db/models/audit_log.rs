use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Audit event severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditSeverity {
    Info,
    Warning,
    Critical,
}

impl AuditSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }
}

impl std::str::FromStr for AuditSeverity {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "warning" => Ok(Self::Warning),
            "critical" => Ok(Self::Critical),
            _ => Ok(Self::Info),
        }
    }
}

impl std::fmt::Display for AuditSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Filtered audit log query. All filters are optional.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AuditFilter {
    pub actor_user_id: Option<String>,
    pub action: Option<String>,
    pub resource_type: Option<String>,
    pub severity: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub search: Option<String>,
    pub success: Option<bool>,
    pub limit: i64,
    pub offset: i64,
}

/// A row from the `audit_logs` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLog {
    pub id: String,
    pub actor_user_id: Option<String>,
    pub target_user_id: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub severity: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: String,
}
