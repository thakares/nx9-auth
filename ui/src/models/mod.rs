//! Shared API response / request types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TenantView {
    pub id: String,
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TenantsResponse {
    #[serde(default)]
    pub tenants: Vec<TenantView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UserView {
    pub id: String,
    pub username: String,
    pub status: String,
    #[serde(default)]
    pub last_login_at: Option<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MeResponse {
    pub user: UserView,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UsersResponse {
    #[serde(default)]
    pub users: Vec<UserView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TokenView {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub last_used_at: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub revoked: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TokensResponse {
    #[serde(default)]
    pub tokens: Vec<TokenView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CreateTokenResponse {
    pub token: TokenView,
    pub raw_token: String,
    #[serde(default)]
    pub warning: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RoleView {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub user_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RolesResponse {
    #[serde(default)]
    pub roles: Vec<RoleView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PermissionView {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub group: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PermissionGroup {
    pub group: String,
    #[serde(default)]
    pub permissions: Vec<PermissionView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PermissionsResponse {
    #[serde(default)]
    pub permissions: Vec<PermissionView>,
    #[serde(default)]
    pub groups: Vec<PermissionGroup>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ApplicationView {
    pub id: String,
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub credentials_configured: bool,
    #[serde(default)]
    pub redirect_urls: Vec<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ApplicationsResponse {
    #[serde(default)]
    pub applications: Vec<ApplicationView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CreateApplicationResponse {
    pub application: ApplicationView,
    pub client_secret: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RotateSecretResponse {
    pub client_secret: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ServiceAccountView {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ServiceAccountsResponse {
    #[serde(default)]
    pub service_accounts: Vec<ServiceAccountView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AuditEntry {
    pub id: String,
    #[serde(default)]
    pub actor_user_id: Option<String>,
    #[serde(default)]
    pub target_user_id: Option<String>,
    pub action: String,
    pub resource_type: String,
    #[serde(default)]
    pub resource_id: Option<String>,
    pub severity: String,
    #[serde(default)]
    pub ip_address: Option<String>,
    #[serde(default)]
    pub user_agent: Option<String>,
    #[serde(default)]
    pub metadata_json: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AuditResponse {
    #[serde(default)]
    pub entries: Vec<AuditEntry>,
    #[serde(default)]
    pub total: i64,
    #[serde(default)]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DashboardResponse {
    #[serde(default)]
    pub personal: serde_json::Value,
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub admin: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProfileResponse {
    #[serde(default)]
    pub user: UserView,
    #[serde(default)]
    pub profile: ProfileFields,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub sessions: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProfileFields {
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub full_name: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiErrorBody {
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SessionView {
    pub id: String,
    pub user_id: String,
    #[serde(default)]
    pub ip_address: Option<String>,
    #[serde(default)]
    pub user_agent: Option<String>,
    pub created_at: String,
    pub last_seen_at: String,
    pub expires_at: String,
    #[serde(default)]
    pub is_current: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SessionsResponse {
    #[serde(default)]
    pub sessions: Vec<SessionView>,
    #[serde(default)]
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GroupView {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub member_count: usize,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GroupMemberView {
    pub id: String,
    pub username: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GroupDetailResponse {
    pub group: GroupView,
    #[serde(default)]
    pub members: Vec<GroupMemberView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GroupsResponse {
    #[serde(default)]
    pub groups: Vec<GroupView>,
}
