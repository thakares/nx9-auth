#![allow(clippy::needless_borrow)]
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use nx9_auth::{
    api,
    config::{Config, SecurityConfig},
    db::{
        self,
        models::{ApiToken, Role, Tenant, User, UserStatus},
    },
    error::AppError,
    identity::{
        permissions as identity_perms, roles as identity_roles_real, users as identity_users_real,
    },
    security::{sessions, tokens as tokens_real},
    state::AppState,
};

#[allow(dead_code)]
mod identity_users {
    use super::AppError;
    use super::SecurityConfig;
    use super::User;
    use super::identity_users_real;

    pub async fn create_user(
        provider: &std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider>,
        cfg: &SecurityConfig,
        tenant_id: &str,
        username: &str,
        password: &str,
    ) -> Result<User, AppError> {
        identity_users_real::create_user(
            &provider, cfg, tenant_id, username, password, None, None, None,
        )
        .await
    }

    pub async fn get_user(
        provider: &std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider>,
        id: &str,
    ) -> Result<User, AppError> {
        identity_users_real::get_user(&provider, id).await
    }

    pub async fn get_user_by_username(
        provider: &std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider>,
        username: &str,
    ) -> Result<User, AppError> {
        identity_users_real::get_user_by_username(&provider, username).await
    }

    pub async fn list_users(
        provider: &std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider>,
        tenant_id: &str,
    ) -> Result<Vec<User>, AppError> {
        identity_users_real::list_users(&provider, tenant_id).await
    }

    pub async fn update_status(
        provider: &std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider>,
        user_id: &str,
        status: i32,
    ) -> Result<(), AppError> {
        identity_users_real::update_status(&provider, user_id, status, None, None, None).await
    }

    pub async fn reset_password(
        provider: &std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider>,
        cfg: &SecurityConfig,
        user_id: &str,
        new_password: &str,
    ) -> Result<(), AppError> {
        identity_users_real::reset_password(&provider, cfg, user_id, new_password, None, None, None)
            .await
    }
}

#[allow(dead_code)]
mod identity_roles {
    use super::AppError;
    use super::Role;
    use super::identity_roles_real;

    pub async fn assign_role(
        provider: &std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider>,
        user_id: &str,
        role_name: &str,
    ) -> Result<(), AppError> {
        identity_roles_real::assign_role(&provider, user_id, role_name, None, None, None).await
    }

    pub async fn list_roles(
        provider: &std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider>,
    ) -> Result<Vec<Role>, AppError> {
        identity_roles_real::list_roles(provider).await
    }

    pub async fn list_user_roles(
        provider: &std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider>,
        user_id: &str,
    ) -> Result<Vec<Role>, AppError> {
        identity_roles_real::list_user_roles(&provider, user_id).await
    }
}

#[allow(dead_code)]
mod tokens {
    use super::ApiToken;
    use super::AppError;
    use super::SecurityConfig;
    use super::tokens_real;

    pub fn generate_pat() -> String {
        tokens_real::generate_pat()
    }

    pub fn hash_token(raw: &str) -> String {
        tokens_real::hash_token(raw)
    }

    pub async fn create_token(
        provider: &std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider>,
        user_id: &str,
        name: &str,
        cfg: &SecurityConfig,
    ) -> Result<(ApiToken, String), AppError> {
        tokens_real::create_token(&provider, user_id, name, cfg, None, None, None).await
    }

    pub async fn validate_token(
        provider: &std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider>,
        raw: &str,
    ) -> Result<Option<ApiToken>, AppError> {
        tokens_real::validate_token(&provider, raw).await
    }
}

async fn setup_test_db() -> (
    std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider>,
    sqlx::SqlitePool,
    String,
) {
    let db_id = uuid::Uuid::new_v4().to_string();
    let db_path = format!("target/test_{}.db", db_id);
    let pool = db::create_pool(&db_path)
        .await
        .expect("Failed to create test pool");
    nx9_auth::db::run_migrations(&pool)
        .await
        .expect("Failed to run test migrations");
    let provider = std::sync::Arc::new(nx9_auth::db::provider::SqliteProvider::new(pool.clone()));
    (provider, pool, db_path)
}

async fn teardown_test_db(path: String) {
    let _ = std::fs::remove_file(path);
}

fn test_security_config() -> SecurityConfig {
    SecurityConfig {
        session_ttl_hours: 24,
        session_absolute_ttl_days: 30,
        token_ttl_days: 365,
        argon2_memory: 4096, // low cost for fast tests
        argon2_iterations: 1,
        argon2_parallelism: 1,
    }
}

fn test_config(db_path: String) -> Config {
    Config {
        server: nx9_auth::config::ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8655,
            cookie_secure: false,
            production: false,
        },
        database: nx9_auth::config::DatabaseConfig { path: db_path },
        security: test_security_config(),
        audit: nx9_auth::config::AuditConfig { enabled: true },
        ..Default::default()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Database & Migration Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_db_migration_creates_default_tenant() {
    let (_provider, pool, db_path) = setup_test_db().await;
    let exists = sqlx::query("SELECT 1 FROM tenants WHERE id = ?")
        .bind(Tenant::DEFAULT_ID)
        .fetch_optional(&pool)
        .await
        .unwrap()
        .is_some();
    assert!(exists);
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_db_migration_seeds_admin_role() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let role = provider.roles().find_by_name("admin").await.unwrap();
    assert!(role.is_some());
    assert_eq!(role.unwrap().name, "admin");
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_db_migration_seeds_viewer_role() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let role = provider.roles().find_by_name("viewer").await.unwrap();
    assert!(role.is_some());
    assert_eq!(role.unwrap().name, "viewer");
    teardown_test_db(db_path).await;
}

// ─────────────────────────────────────────────────────────────────────────────
// User Repository Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_repo_create_user_success() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();
    let user = identity_users::create_user(
        &provider,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "repo_user_1",
        "super_secure_passphrase_123",
    )
    .await
    .unwrap();
    assert_eq!(user.username, "repo_user_1");
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_repo_create_user_empty_username() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();
    let res = identity_users::create_user(
        &provider,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "   ",
        "super_secure_passphrase_123",
    )
    .await;
    assert!(res.is_err());
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_repo_create_user_conflict() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();
    let _ = identity_users::create_user(
        &provider,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "repo_user_conflict",
        "super_secure_passphrase_123",
    )
    .await
    .unwrap();
    let res = identity_users::create_user(
        &provider,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "repo_user_conflict",
        "super_secure_passphrase_123",
    )
    .await;
    assert!(res.is_err());
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_repo_find_user_by_id() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();
    let user = identity_users::create_user(
        &provider,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "find_by_id_user",
        "super_secure_passphrase_123",
    )
    .await
    .unwrap();
    let found = identity_users::get_user(&provider, &user.id).await.unwrap();
    assert_eq!(found.username, "find_by_id_user");
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_repo_find_user_by_username() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();
    let _ = identity_users::create_user(
        &provider,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "find_by_username_user",
        "super_secure_passphrase_123",
    )
    .await
    .unwrap();
    let found = identity_users::get_user_by_username(&provider, "find_by_username_user")
        .await
        .unwrap();
    assert_eq!(found.username, "find_by_username_user");
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_repo_update_status() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();
    let user = identity_users::create_user(
        &provider,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "status_user",
        "super_secure_passphrase_123",
    )
    .await
    .unwrap();
    identity_users::update_status(&provider, &user.id, UserStatus::Disabled as i32)
        .await
        .unwrap();
    let updated = identity_users::get_user(&provider, &user.id).await.unwrap();
    assert_eq!(updated.status, UserStatus::Disabled as i32);
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_repo_reset_password_strength_standard() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();
    let user = identity_users::create_user(
        &provider,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "pwd_reset_user",
        "super_secure_passphrase_123",
    )
    .await
    .unwrap();

    // Standard user password reset fails with too short
    assert!(
        identity_users::reset_password(&provider, &sec_cfg, &user.id, "short")
            .await
            .is_err()
    );
    // Fails with weak password
    assert!(
        identity_users::reset_password(&provider, &sec_cfg, &user.id, "password12345")
            .await
            .is_err()
    );
    // Succeeds with valid
    assert!(
        identity_users::reset_password(
            &provider,
            &sec_cfg,
            &user.id,
            "super_secure_new_phrase_123"
        )
        .await
        .is_ok()
    );

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_repo_reset_password_strength_admin() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();
    let user = identity_users::create_user(
        &provider,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "pwd_reset_admin",
        "super_secure_admin_passphrase_123",
    )
    .await
    .unwrap();
    identity_roles::assign_role(&provider, &user.id, "admin")
        .await
        .unwrap();

    // Admin reset fails with 8 characters (requires 12)
    assert!(
        identity_users::reset_password(&provider, &sec_cfg, &user.id, "short_pwd")
            .await
            .is_err()
    );
    // Succeeds with >= 12 chars
    assert!(
        identity_users::reset_password(
            &provider,
            &sec_cfg,
            &user.id,
            "super_secure_admin_new_phrase_123"
        )
        .await
        .is_ok()
    );

    teardown_test_db(db_path).await;
}

// ─────────────────────────────────────────────────────────────────────────────
// Role & Permission Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_role_assignment() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();
    let user = identity_users::create_user(
        &provider,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "role_user",
        "super_secure_passphrase_123",
    )
    .await
    .unwrap();

    identity_roles::assign_role(&provider, &user.id, "viewer")
        .await
        .unwrap();
    let user_roles = nx9_auth::identity::roles::list_user_roles(&provider, &user.id)
        .await
        .unwrap();
    assert!(user_roles.iter().any(|r| r.name == "viewer"));
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_role_removal() {
    let (provider, pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();
    let user = identity_users::create_user(
        &provider,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "role_rm_user",
        "super_secure_passphrase_123",
    )
    .await
    .unwrap();

    identity_roles::assign_role(&provider, &user.id, "viewer")
        .await
        .unwrap();
    let role = provider
        .roles()
        .find_by_name("viewer")
        .await
        .unwrap()
        .unwrap();
    let tx = pool.begin().await.unwrap();
    provider
        .roles()
        .remove_from_user(&user.id, &role.id)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let user_roles = nx9_auth::identity::roles::list_user_roles(&provider, &user.id)
        .await
        .unwrap();
    assert!(user_roles.is_empty());
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_permission_listing_admin() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();
    let user = identity_users::create_user(
        &provider,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "perm_admin",
        "super_secure_admin_passphrase_123",
    )
    .await
    .unwrap();
    identity_roles::assign_role(&provider, &user.id, "admin")
        .await
        .unwrap();

    let perms = identity_perms::list_user_permissions(&provider, &user.id)
        .await
        .unwrap();
    assert!(perms.contains(&"users:create".to_string()));
    assert!(perms.contains(&"users:delete".to_string()));
    teardown_test_db(db_path).await;
}

// ─────────────────────────────────────────────────────────────────────────────
// Session Lifecycle Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_session_creation_success() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();
    let user = identity_users::create_user(
        &provider,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "sess_create_user",
        "super_secure_passphrase_123",
    )
    .await
    .unwrap();

    let (session, raw_token) =
        sessions::create_session(&provider, &user.id, Some("127.0.0.1"), None, &sec_cfg)
            .await
            .unwrap();
    assert_eq!(session.user_id, user.id);
    assert_eq!(raw_token.len(), 64);
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_session_validation_valid_token() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();
    let user = identity_users::create_user(
        &provider,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "sess_val_user",
        "super_secure_passphrase_123",
    )
    .await
    .unwrap();

    let (_, raw_token) =
        sessions::create_session(&provider, &user.id, Some("127.0.0.1"), None, &sec_cfg)
            .await
            .unwrap();
    let validated = sessions::validate_session(&provider, &raw_token, &sec_cfg)
        .await
        .unwrap();
    assert!(validated.is_some());
    assert_eq!(validated.unwrap().user_id, user.id);
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_session_validation_revoked_token() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();
    let user = identity_users::create_user(
        &provider,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "sess_rev_user",
        "super_secure_passphrase_123",
    )
    .await
    .unwrap();

    let (session, raw_token) =
        sessions::create_session(&provider, &user.id, Some("127.0.0.1"), None, &sec_cfg)
            .await
            .unwrap();
    sessions::revoke_session(&provider, &session.id)
        .await
        .unwrap();
    let validated = sessions::validate_session(&provider, &raw_token, &sec_cfg)
        .await
        .unwrap();
    assert!(validated.is_none());
    teardown_test_db(db_path).await;
}

// ─────────────────────────────────────────────────────────────────────────────
// PAT Token Lifecycle Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_pat_creation_and_validation() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();
    let user = identity_users::create_user(
        &provider,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "pat_user",
        "super_secure_passphrase_123",
    )
    .await
    .unwrap();

    let (token, raw_pat) = tokens::create_token(&provider, &user.id, "my-token", &sec_cfg)
        .await
        .unwrap();
    assert!(raw_pat.starts_with("nx9_pat_"));

    let validated = tokens::validate_token(&provider, &raw_pat)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(validated.id, token.id);
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_pat_revocation() {
    let (provider, pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();
    let user = identity_users::create_user(
        &provider,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "pat_rev_user",
        "super_secure_passphrase_123",
    )
    .await
    .unwrap();

    let (token, raw_pat) = tokens::create_token(&provider, &user.id, "my-token", &sec_cfg)
        .await
        .unwrap();
    let tx = pool.begin().await.unwrap();
    provider.tokens().revoke(&token.id).await.unwrap();
    tx.commit().await.unwrap();

    let validated = tokens::validate_token(&provider, &raw_pat).await.unwrap();
    assert!(validated.is_none());
    teardown_test_db(db_path).await;
}

// ─────────────────────────────────────────────────────────────────────────────
// API Endpoints Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_api_health_endpoint() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let config = test_config(db_path.clone());
    let state = AppState::new(provider.clone(), config);
    let app = api::router::build(state);

    let req = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_api_version_endpoint() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let config = test_config(db_path.clone());
    let state = AppState::new(provider.clone(), config);
    let app = api::router::build(state);

    let req = Request::builder()
        .uri("/version")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_api_login_success() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let config = test_config(db_path.clone());
    let state = AppState::new(provider.clone(), config.clone());
    let app = api::router::build(state);

    let password = "super_secure_passphrase_123";
    let _ = identity_users::create_user(
        &provider,
        &config.security,
        Tenant::DEFAULT_ID,
        "login_ok_user",
        password,
    )
    .await
    .unwrap();

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"username":"login_ok_user","password":"super_secure_passphrase_123"}"#,
        ))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let cookie = res
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(cookie.contains("nx9_session="));
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_api_login_invalid_password() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let config = test_config(db_path.clone());
    let state = AppState::new(provider.clone(), config.clone());
    let app = api::router::build(state);

    let _ = identity_users::create_user(
        &provider,
        &config.security,
        Tenant::DEFAULT_ID,
        "login_err_user",
        "super_secure_passphrase_123",
    )
    .await
    .unwrap();

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"username":"login_err_user","password":"wrong_password"}"#,
        ))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_api_login_invalid_user() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let config = test_config(db_path.clone());
    let state = AppState::new(provider.clone(), config.clone());
    let app = api::router::build(state);

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"username":"does_not_exist","password":"some_password"}"#,
        ))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_api_me_authenticated() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let config = test_config(db_path.clone());
    let state = AppState::new(provider.clone(), config.clone());
    let app = api::router::build(state);

    let _ = identity_users::create_user(
        &provider,
        &config.security,
        Tenant::DEFAULT_ID,
        "me_user",
        "super_secure_passphrase_123",
    )
    .await
    .unwrap();

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"username":"me_user","password":"super_secure_passphrase_123"}"#,
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let cookie = res
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();

    let req = Request::builder()
        .uri("/api/v1/auth/me")
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_api_me_unauthenticated() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let config = test_config(db_path.clone());
    let state = AppState::new(provider.clone(), config);
    let app = api::router::build(state);

    let req = Request::builder()
        .uri("/api/v1/auth/me")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_api_logout_success() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let config = test_config(db_path.clone());
    let state = AppState::new(provider.clone(), config.clone());
    let app = api::router::build(state);

    let _ = identity_users::create_user(
        &provider,
        &config.security,
        Tenant::DEFAULT_ID,
        "logout_user",
        "super_secure_passphrase_123",
    )
    .await
    .unwrap();

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"username":"logout_user","password":"super_secure_passphrase_123"}"#,
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let cookie = res
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/logout")
        .header(header::COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Profile should now fail
    let req = Request::builder()
        .uri("/api/v1/auth/me")
        .header(header::COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_api_list_users_viewer_forbidden() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let config = test_config(db_path.clone());
    let state = AppState::new(provider.clone(), config.clone());
    let app = api::router::build(state);

    // Create a viewer user
    let viewer = identity_users::create_user(
        &provider,
        &config.security,
        Tenant::DEFAULT_ID,
        "api_viewer",
        "super_secure_passphrase_123",
    )
    .await
    .unwrap();
    identity_roles::assign_role(&provider, &viewer.id, "viewer")
        .await
        .unwrap();

    // Login as viewer
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"username":"api_viewer","password":"super_secure_passphrase_123"}"#,
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let cookie = res
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();

    // Attempt list users (requires users:create)
    let req = Request::builder()
        .uri("/api/v1/users")
        .header(header::COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_api_list_users_admin_allowed() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let config = test_config(db_path.clone());
    let state = AppState::new(provider.clone(), config.clone());
    let app = api::router::build(state);

    // Create an admin user
    let admin = identity_users::create_user(
        &provider,
        &config.security,
        Tenant::DEFAULT_ID,
        "api_admin_list",
        "super_secure_admin_passphrase_123",
    )
    .await
    .unwrap();
    identity_roles::assign_role(&provider, &admin.id, "admin")
        .await
        .unwrap();

    // Login as admin
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"username":"api_admin_list","password":"super_secure_admin_passphrase_123"}"#,
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let cookie = res
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();

    // List users (requires users:create, which admin has)
    let req = Request::builder()
        .uri("/api/v1/users")
        .header(header::COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_api_create_user_unauthorized() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let config = test_config(db_path.clone());
    let state = AppState::new(provider.clone(), config);
    let app = api::router::build(state);

    // Call user creation without cookie
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/users")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"username":"new_user","password":"some_password"}"#,
        ))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_api_create_user_authorized() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let config = test_config(db_path.clone());
    let state = AppState::new(provider.clone(), config.clone());
    let app = api::router::build(state);

    // Create admin
    let admin = identity_users::create_user(
        &provider,
        &config.security,
        Tenant::DEFAULT_ID,
        "api_admin_creator",
        "super_secure_admin_passphrase_123",
    )
    .await
    .unwrap();
    identity_roles::assign_role(&provider, &admin.id, "admin")
        .await
        .unwrap();

    // Login as admin
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"username":"api_admin_creator","password":"super_secure_admin_passphrase_123"}"#,
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let cookie = res
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();

    // Create user via API
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/users")
        .header(header::COOKIE, &cookie)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"username":"api_created_user","password":"super_secure_passphrase_123"}"#,
        ))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_api_delete_user_self_forbidden() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let config = test_config(db_path.clone());
    let state = AppState::new(provider.clone(), config.clone());
    let app = api::router::build(state);

    // Create admin
    let admin = identity_users::create_user(
        &provider,
        &config.security,
        Tenant::DEFAULT_ID,
        "api_admin_del_self",
        "super_secure_admin_passphrase_123",
    )
    .await
    .unwrap();
    identity_roles::assign_role(&provider, &admin.id, "admin")
        .await
        .unwrap();

    // Login as admin
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"username":"api_admin_del_self","password":"super_secure_admin_passphrase_123"}"#,
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let cookie = res
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();

    // Delete self (should fail)
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/users/{}", admin.id))
        .header(header::COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_api_delete_user_success() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let config = test_config(db_path.clone());
    let state = AppState::new(provider.clone(), config.clone());
    let app = api::router::build(state);

    // Create admin
    let admin = identity_users::create_user(
        &provider,
        &config.security,
        Tenant::DEFAULT_ID,
        "api_admin_deleter",
        "super_secure_admin_passphrase_123",
    )
    .await
    .unwrap();
    identity_roles::assign_role(&provider, &admin.id, "admin")
        .await
        .unwrap();

    // Create standard user to delete
    let target = identity_users::create_user(
        &provider,
        &config.security,
        Tenant::DEFAULT_ID,
        "delete_target",
        "super_secure_passphrase_123",
    )
    .await
    .unwrap();

    // Login as admin
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"username":"api_admin_deleter","password":"super_secure_admin_passphrase_123"}"#,
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let cookie = res
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();

    // Delete target user
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/users/{}", target.id))
        .header(header::COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_api_token_creation_and_listing() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let config = test_config(db_path.clone());
    let state = AppState::new(provider.clone(), config.clone());
    let app = api::router::build(state);

    // Create admin
    let admin = identity_users::create_user(
        &provider,
        &config.security,
        Tenant::DEFAULT_ID,
        "api_admin_token",
        "super_secure_admin_passphrase_123",
    )
    .await
    .unwrap();
    identity_roles::assign_role(&provider, &admin.id, "admin")
        .await
        .unwrap();

    // Login as admin
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"username":"api_admin_token","password":"super_secure_admin_passphrase_123"}"#,
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let cookie = res
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();

    // Create token
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/tokens")
        .header(header::COOKIE, &cookie)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"name":"test-api-token"}"#))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // List tokens
    let req = Request::builder()
        .uri("/api/v1/tokens")
        .header(header::COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_api_token_revocation() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let config = test_config(db_path.clone());
    let state = AppState::new(provider.clone(), config.clone());
    let app = api::router::build(state);

    // Create admin
    let admin = identity_users::create_user(
        &provider,
        &config.security,
        Tenant::DEFAULT_ID,
        "api_admin_tok_rev",
        "super_secure_admin_passphrase_123",
    )
    .await
    .unwrap();
    identity_roles::assign_role(&provider, &admin.id, "admin")
        .await
        .unwrap();

    // Login as admin
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"username":"api_admin_tok_rev","password":"super_secure_admin_passphrase_123"}"#,
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let cookie = res
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();

    // Create token
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/tokens")
        .header(header::COOKIE, &cookie)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"name":"test-rev-token"}"#))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body: Value =
        serde_json::from_slice(&res.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let token_id = body
        .get("token")
        .unwrap()
        .get("id")
        .unwrap()
        .as_str()
        .unwrap();

    // Revoke token
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/tokens/{}", token_id))
        .header(header::COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_api_dashboard_success() {
    let (provider, _pool, db_path) = setup_test_db().await;
    let config = test_config(db_path.clone());
    let state = AppState::new(provider.clone(), config.clone());

    // Create an admin user
    let admin = identity_users::create_user(
        &provider,
        &config.security,
        Tenant::DEFAULT_ID,
        "admin_dashboard",
        "S3cur3#P@ssw0rd!",
    )
    .await
    .unwrap();

    let admin_role = provider
        .roles()
        .find_by_name("admin")
        .await
        .unwrap()
        .unwrap();
    provider
        .roles()
        .assign_to_user(&admin.id, &admin_role.id)
        .await
        .unwrap();

    let app = api::router::build(state.clone());

    // Login to get cookie
    let login_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"username":"admin_dashboard","password":"S3cur3#P@ssw0rd!"}"#,
        ))
        .unwrap();
    let login_res = app.clone().oneshot(login_req).await.unwrap();
    assert_eq!(login_res.status(), StatusCode::OK);
    let set_cookie = login_res
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap();
    let cookie = set_cookie.split(';').next().unwrap().to_string();

    // Call dashboard
    let dash_req = Request::builder()
        .method("GET")
        .uri("/api/v1/dashboard")
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap();

    let dash_res = app.oneshot(dash_req).await.unwrap();
    let status = dash_res.status();
    let body_bytes = dash_res.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

    assert_eq!(status, StatusCode::OK, "dashboard failed: {}", body_str);

    teardown_test_db(db_path).await;
}
