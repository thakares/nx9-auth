//! HTTP client wrapping the `/api/v1` surface.
//!
//! Authentication:
//! 1. Browser cookies (`fetch_credentials_include`) for the HttpOnly session cookie
//! 2. `Authorization: Bearer <session_token>` from sessionStorage (login body fallback)
//!
//! Frontend permission checks are presentation-only — the backend is authoritative.
//!
//! Note: reqwest on WASM requires **absolute** URLs.

use crate::models::*;
use crate::services::session;
use reqwest::{Client, StatusCode};
use serde::Serialize;
use serde_json::Value;

/// API base path (same-origin).
const API_PREFIX: &str = "/api/v1";

/// Client-side API error.
#[derive(Debug, Clone, PartialEq)]
pub enum ApiError {
    Unauthorized,
    Forbidden,
    NotFound,
    InvalidInput(String),
    Network(String),
    Server(String),
    Other(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthorized => write!(f, "Invalid username or password."),
            Self::Forbidden => write!(f, "You do not have permission to do that"),
            Self::NotFound => write!(f, "Resource not found"),
            Self::InvalidInput(m) => write!(f, "{m}"),
            Self::Network(m) => write!(f, "Network error: {m}"),
            Self::Server(m) => write!(f, "{m}"),
            Self::Other(m) => write!(f, "{m}"),
        }
    }
}

/// Browser origin, e.g. `http://127.0.0.1:8655`.
fn origin() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_default()
}

/// Build an absolute API URL (required by reqwest-wasm).
fn api_url(path: &str) -> String {
    format!("{}{}{}", origin(), API_PREFIX, path)
}

fn client() -> Client {
    Client::new()
}

/// Attach credentials + optional bearer session token.
fn authorize(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    // let builder = builder.fetch_credentials_include();
    if let Some(token) = session::load_access_token() {
        builder.header("Authorization", format!("Bearer {token}"))
    } else {
        builder
    }
}

async fn handle<T: serde::de::DeserializeOwned>(resp: reqwest::Response) -> Result<T, ApiError> {
    let status = resp.status();
    if status == StatusCode::UNAUTHORIZED {
        // Stale client token — drop it so the next login is clean.
        session::clear();
        return Err(ApiError::Unauthorized);
    }
    if status == StatusCode::FORBIDDEN {
        return Err(ApiError::Forbidden);
    }
    if status == StatusCode::NOT_FOUND {
        return Err(ApiError::NotFound);
    }

    let text = resp
        .text()
        .await
        .map_err(|e| ApiError::Network(e.to_string()))?;

    if !status.is_success() {
        if let Ok(body) = serde_json::from_str::<ApiErrorBody>(&text) {
            if status == StatusCode::UNPROCESSABLE_ENTITY {
                return Err(ApiError::InvalidInput(body.error));
            }
            return Err(ApiError::Server(body.error));
        }
        return Err(ApiError::Server(format!("HTTP {status}: {text}")));
    }

    serde_json::from_str(&text).map_err(|e| ApiError::Other(format!("decode error: {e}: {text}")))
}

async fn get<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, ApiError> {
    let url = api_url(path);
    let resp = authorize(client().get(&url))
        .send()
        .await
        .map_err(|e| ApiError::Network(format!("{e} ({url})")))?;
    handle(resp).await
}

async fn post_json<B: Serialize, T: serde::de::DeserializeOwned>(
    path: &str,
    body: &B,
) -> Result<T, ApiError> {
    let url = api_url(path);
    let resp = authorize(client().post(&url).json(body))
        .send()
        .await
        .map_err(|e| ApiError::Network(format!("{e} ({url})")))?;
    handle(resp).await
}

async fn patch_json<B: Serialize, T: serde::de::DeserializeOwned>(
    path: &str,
    body: &B,
) -> Result<T, ApiError> {
    let url = api_url(path);
    let resp = authorize(client().patch(&url).json(body))
        .send()
        .await
        .map_err(|e| ApiError::Network(format!("{e} ({url})")))?;
    handle(resp).await
}

async fn put_json<B: Serialize, T: serde::de::DeserializeOwned>(
    path: &str,
    body: &B,
) -> Result<T, ApiError> {
    let url = api_url(path);
    let resp = authorize(client().put(&url).json(body))
        .send()
        .await
        .map_err(|e| ApiError::Network(format!("{e} ({url})")))?;
    handle(resp).await
}

async fn delete_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, ApiError> {
    let url = api_url(path);
    let resp = authorize(client().delete(&url))
        .send()
        .await
        .map_err(|e| ApiError::Network(format!("{e} ({url})")))?;
    handle(resp).await
}

// ── Auth ──────────────────────────────────────────────────────────────────────

/// Secure login response (POST JSON only — never query parameters).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub user: Option<serde_json::Value>,
}

/// POST /api/v1/auth/login with JSON body.
///
/// Credentials are never placed in the URL, query string, or fragment.
pub async fn login(username: &str, password: &str) -> Result<LoginResponse, ApiError> {
    // Do not send a stale Authorization header on login.
    session::clear();
    let body = serde_json::json!({
        "username": username,
        "password": password,
    });
    let url = api_url("/auth/login");
    let resp = client()
        .post(&url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| ApiError::Network(format!("{e} ({url})")))?;
    let parsed: LoginResponse = handle(resp).await?;
    if !parsed.access_token.is_empty() {
        session::save_access_token(&parsed.access_token);
    }
    if let Some(ref rt) = parsed.refresh_token {
        if !rt.is_empty() {
            session::save_refresh_token(rt);
        }
    }
    Ok(parsed)
}

pub async fn logout() -> Result<(), ApiError> {
    let result = post_json::<_, Value>("/auth/logout", &serde_json::json!({})).await;
    session::clear();
    result.map(|_| ())
}

pub async fn me() -> Result<Option<MeResponse>, ApiError> {
    let url = api_url("/auth/me");
    let resp = authorize(client().get(&url))
        .send()
        .await
        .map_err(|e| ApiError::Network(format!("{e} ({url})")))?;

    if resp.status() == StatusCode::UNAUTHORIZED {
        session::clear();
        return Ok(None);
    }

    let body: MeResponse = handle(resp).await?;
    Ok(Some(body))
}

// ── Dashboard / Profile ───────────────────────────────────────────────────────

pub async fn dashboard() -> Result<DashboardResponse, ApiError> {
    get("/dashboard").await
}

pub async fn get_profile() -> Result<ProfileResponse, ApiError> {
    get("/profile").await
}

pub async fn list_tenants() -> Result<Vec<TenantView>, ApiError> {
    let r: TenantsResponse = get("/tenants").await?;
    Ok(r.tenants)
}

pub async fn update_profile(email: Option<&str>, full_name: Option<&str>) -> Result<Value, ApiError> {
    let body = serde_json::json!({ "email": email, "full_name": full_name });
    patch_json("/profile", &body).await
}

pub async fn change_password(current: &str, new_password: &str) -> Result<(), ApiError> {
    let body = serde_json::json!({
        "current_password": current,
        "new_password": new_password,
    });
    let _: Value = post_json("/profile/password", &body).await?;
    Ok(())
}

// ── Users ─────────────────────────────────────────────────────────────────────

pub async fn list_users() -> Result<Vec<UserView>, ApiError> {
    let r: UsersResponse = get("/users").await?;
    Ok(r.users)
}

pub async fn get_user(id: &str) -> Result<UserView, ApiError> {
    let r: Value = get(&format!("/users/{id}")).await?;
    serde_json::from_value(r.get("user").cloned().unwrap_or(Value::Null))
        .map_err(|e| ApiError::Other(e.to_string()))
}

pub async fn create_user(username: &str, password: &str) -> Result<UserView, ApiError> {
    let body = serde_json::json!({ "username": username, "password": password });
    let r: Value = post_json("/users", &body).await?;
    serde_json::from_value(r.get("user").cloned().unwrap_or(Value::Null))
        .map_err(|e| ApiError::Other(e.to_string()))
}

pub async fn update_user_status(id: &str, status: &str) -> Result<UserView, ApiError> {
    let body = serde_json::json!({ "status": status });
    let r: Value = patch_json(&format!("/users/{id}"), &body).await?;
    serde_json::from_value(r.get("user").cloned().unwrap_or(Value::Null))
        .map_err(|e| ApiError::Other(e.to_string()))
}

pub async fn delete_user(id: &str) -> Result<(), ApiError> {
    let _: Value = delete_json(&format!("/users/{id}")).await?;
    Ok(())
}

pub async fn reset_user_password(id: &str, password: &str) -> Result<(), ApiError> {
    let body = serde_json::json!({ "password": password });
    let _: Value = post_json(&format!("/users/{id}/reset-password"), &body).await?;
    Ok(())
}

pub async fn list_user_roles(id: &str) -> Result<Vec<RoleView>, ApiError> {
    let r: Value = get(&format!("/users/{id}/roles")).await?;
    serde_json::from_value(r.get("roles").cloned().unwrap_or(Value::Array(vec![])))
        .map_err(|e| ApiError::Other(e.to_string()))
}

pub async fn assign_user_role(user_id: &str, role: &str) -> Result<(), ApiError> {
    let body = serde_json::json!({ "role": role });
    let _: Value = post_json(&format!("/users/{user_id}/roles"), &body).await?;
    Ok(())
}

pub async fn remove_user_role(user_id: &str, role: &str) -> Result<(), ApiError> {
    let _: Value = delete_json(&format!("/users/{user_id}/roles/{role}")).await?;
    Ok(())
}

// ── Roles / Permissions ───────────────────────────────────────────────────────

pub async fn list_roles() -> Result<Vec<RoleView>, ApiError> {
    let r: RolesResponse = get("/roles").await?;
    Ok(r.roles)
}

pub async fn create_role(name: &str, description: Option<&str>) -> Result<RoleView, ApiError> {
    let body = serde_json::json!({ "name": name, "description": description });
    let r: Value = post_json("/roles", &body).await?;
    serde_json::from_value(r.get("role").cloned().unwrap_or(Value::Null))
        .map_err(|e| ApiError::Other(e.to_string()))
}

pub async fn update_role(
    id: &str,
    name: &str,
    description: Option<&str>,
) -> Result<RoleView, ApiError> {
    let body = serde_json::json!({ "name": name, "description": description });
    let r: Value = patch_json(&format!("/roles/{id}"), &body).await?;
    serde_json::from_value(r.get("role").cloned().unwrap_or(Value::Null))
        .map_err(|e| ApiError::Other(e.to_string()))
}

pub async fn delete_role(id: &str) -> Result<(), ApiError> {
    let _: Value = delete_json(&format!("/roles/{id}")).await?;
    Ok(())
}

pub async fn set_role_permissions(id: &str, permissions: &[String]) -> Result<(), ApiError> {
    let body = serde_json::json!({ "permissions": permissions });
    let _: Value = put_json(&format!("/roles/{id}/permissions"), &body).await?;
    Ok(())
}

pub async fn list_permissions() -> Result<PermissionsResponse, ApiError> {
    get("/permissions").await
}

// ── Tokens ────────────────────────────────────────────────────────────────────

pub async fn list_tokens() -> Result<Vec<TokenView>, ApiError> {
    let r: TokensResponse = get("/tokens").await?;
    Ok(r.tokens)
}

pub async fn create_token(name: &str) -> Result<CreateTokenResponse, ApiError> {
    let body = serde_json::json!({ "name": name });
    post_json("/tokens", &body).await
}

pub async fn revoke_token(id: &str) -> Result<(), ApiError> {
    let _: Value = delete_json(&format!("/tokens/{id}")).await?;
    Ok(())
}

// ── Applications ──────────────────────────────────────────────────────────────

pub async fn list_applications() -> Result<Vec<ApplicationView>, ApiError> {
    let r: ApplicationsResponse = get("/applications").await?;
    Ok(r.applications)
}

pub async fn create_application(name: &str, slug: &str) -> Result<ApplicationView, ApiError> {
    let body = serde_json::json!({ "name": name, "slug": slug });
    let r: Value = post_json("/applications", &body).await?;
    serde_json::from_value(r.get("application").cloned().unwrap_or(Value::Null))
        .map_err(|e| ApiError::Other(e.to_string()))
}

pub async fn update_application(
    id: &str,
    name: &str,
    slug: &str,
    enabled: bool,
) -> Result<ApplicationView, ApiError> {
    let body = serde_json::json!({ "name": name, "slug": slug, "enabled": enabled });
    let r: Value = patch_json(&format!("/applications/{id}"), &body).await?;
    serde_json::from_value(r.get("application").cloned().unwrap_or(Value::Null))
        .map_err(|e| ApiError::Other(e.to_string()))
}

pub async fn delete_application(id: &str) -> Result<(), ApiError> {
    let _: Value = delete_json(&format!("/applications/{id}")).await?;
    Ok(())
}

// ── Service accounts ──────────────────────────────────────────────────────────

pub async fn list_service_accounts() -> Result<Vec<ServiceAccountView>, ApiError> {
    let r: ServiceAccountsResponse = get("/service-accounts").await?;
    Ok(r.service_accounts)
}

pub async fn create_service_account(
    name: &str,
    description: Option<&str>,
) -> Result<ServiceAccountView, ApiError> {
    let body = serde_json::json!({ "name": name, "description": description });
    let r: Value = post_json("/service-accounts", &body).await?;
    serde_json::from_value(r.get("service_account").cloned().unwrap_or(Value::Null))
        .map_err(|e| ApiError::Other(e.to_string()))
}

pub async fn set_service_account_enabled(id: &str, enabled: bool) -> Result<(), ApiError> {
    let body = serde_json::json!({ "enabled": enabled });
    let _: Value = patch_json(&format!("/service-accounts/{id}"), &body).await?;
    Ok(())
}

pub async fn delete_service_account(id: &str) -> Result<(), ApiError> {
    let _: Value = delete_json(&format!("/service-accounts/{id}")).await?;
    Ok(())
}

pub async fn rotate_service_account_secret(id: &str) -> Result<String, ApiError> {
    let r: Value =
        post_json(&format!("/service-accounts/{id}/secret"), &serde_json::json!({})).await?;
    Ok(r.get("raw_secret")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string())
}

// ── Audit ─────────────────────────────────────────────────────────────────────

pub async fn list_audit(query: &str) -> Result<AuditResponse, ApiError> {
    let path = if query.is_empty() {
        "/audit".to_string()
    } else {
        format!("/audit?{query}")
    };
    get(&path).await
}

// ── Sessions ──────────────────────────────────────────────────────────────────

pub async fn list_sessions() -> Result<SessionsResponse, ApiError> {
    get("/sessions").await
}

pub async fn terminate_session(id: &str) -> Result<(), ApiError> {
    let _: Value = delete_json(&format!("/sessions/{id}")).await?;
    Ok(())
}

pub async fn terminate_other_sessions() -> Result<(), ApiError> {
    let _: Value = delete_json("/sessions/others").await?;
    Ok(())
}

// ── Groups ────────────────────────────────────────────────────────────────────

pub async fn list_groups() -> Result<Vec<GroupView>, ApiError> {
    let r: GroupsResponse = get("/groups").await?;
    Ok(r.groups)
}

pub async fn create_group(name: &str, description: Option<&str>) -> Result<GroupView, ApiError> {
    let body = serde_json::json!({ "name": name, "description": description });
    let r: Value = post_json("/groups", &body).await?;
    serde_json::from_value(r.get("group").cloned().unwrap_or(Value::Null))
        .map_err(|e| ApiError::Other(e.to_string()))
}

pub async fn get_group(id: &str) -> Result<GroupDetailResponse, ApiError> {
    get(&format!("/groups/{id}")).await
}

pub async fn update_group(id: &str, name: &str, description: Option<&str>) -> Result<GroupView, ApiError> {
    let body = serde_json::json!({ "name": name, "description": description });
    let r: Value = patch_json(&format!("/groups/{id}"), &body).await?;
    serde_json::from_value(r.get("group").cloned().unwrap_or(Value::Null))
        .map_err(|e| ApiError::Other(e.to_string()))
}

pub async fn delete_group(id: &str) -> Result<(), ApiError> {
    let _: Value = delete_json(&format!("/groups/{id}")).await?;
    Ok(())
}

pub async fn add_group_member(group_id: &str, user_id: &str) -> Result<(), ApiError> {
    let body = serde_json::json!({ "user_id": user_id });
    let _: Value = post_json(&format!("/groups/{group_id}/members"), &body).await?;
    Ok(())
}

pub async fn remove_group_member(group_id: &str, user_id: &str) -> Result<(), ApiError> {
    let _: Value = delete_json(&format!("/groups/{group_id}/members/{user_id}")).await?;
    Ok(())
}

// ── Tenants (complete) ────────────────────────────────────────────────────────

pub async fn create_tenant(name: &str, slug: Option<&str>) -> Result<TenantView, ApiError> {
    let body = serde_json::json!({ "name": name, "slug": slug });
    let r: Value = post_json("/tenants", &body).await?;
    serde_json::from_value(r.get("tenant").cloned().unwrap_or(Value::Null))
        .map_err(|e| ApiError::Other(e.to_string()))
}

pub async fn update_tenant(id: &str, name: &str, slug: Option<&str>) -> Result<TenantView, ApiError> {
    let body = serde_json::json!({ "name": name, "slug": slug });
    let r: Value = patch_json(&format!("/tenants/{id}"), &body).await?;
    serde_json::from_value(r.get("tenant").cloned().unwrap_or(Value::Null))
        .map_err(|e| ApiError::Other(e.to_string()))
}

pub async fn get_tenant(id: &str) -> Result<TenantView, ApiError> {
    let r: Value = get(&format!("/tenants/{id}")).await?;
    serde_json::from_value(r.get("tenant").cloned().unwrap_or(Value::Null))
        .map_err(|e| ApiError::Other(e.to_string()))
}

pub async fn delete_tenant(id: &str) -> Result<(), ApiError> {
    let _: Value = delete_json(&format!("/tenants/{id}")).await?;
    Ok(())
}
