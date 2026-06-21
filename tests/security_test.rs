use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use nx9_auth::{
    api,
    config::{Config, SecurityConfig},
    db::{
        self,
        models::Tenant,
        repository::{roles as role_repo, tokens as token_repo, users as user_repo},
    },
    identity::{roles as identity_roles, users as identity_users},
    security::{passwords, sessions, tokens},
    state::AppState,
};
use serde_json::Value;
use tower::ServiceExt;

async fn setup_test_db() -> (sqlx::SqlitePool, String) {
    let db_id = uuid::Uuid::new_v4().to_string();
    let db_path = format!("target/security_{}.db", db_id);
    let pool = db::create_pool(&db_path)
        .await
        .expect("Failed to create test pool");
    db::run_migrations(&pool)
        .await
        .expect("Failed to run test migrations");
    (pool, db_path)
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
            port: 8656,
        },
        database: nx9_auth::config::DatabaseConfig { path: db_path },
        security: test_security_config(),
        audit: nx9_auth::config::AuditConfig { enabled: true },
        ..Default::default()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Password & Token Leakage Verification
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_security_no_plaintext_passwords_in_db() {
    let (pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();
    let password = "super_secret_special_pass_123456";

    let user = identity_users::create_user(
        &pool,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "leak_test_user",
        password,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // Query the raw database row and verify the plaintext password is not in the row
    let row: (String,) = sqlx::query_as("SELECT password_hash FROM users WHERE id = ?")
        .bind(&user.id)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert!(!row.0.contains(password));
    assert_ne!(row.0, password);

    // Grep/search the entire users table for the plaintext password string
    let matches: Vec<(String,)> =
        sqlx::query_as("SELECT id FROM users WHERE password_hash LIKE ? OR username LIKE ?")
            .bind(format!("%{}%", password))
            .bind(format!("%{}%", password))
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(matches.is_empty());

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_security_no_plaintext_tokens_in_db() {
    let (pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();
    let user = identity_users::create_user(
        &pool,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "token_leak_user",
        "super_secure_passphrase_123",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let (token, raw_pat) =
        tokens::create_token(&pool, &user.id, "my_pat", &sec_cfg, None, None, None)
            .await
            .unwrap();

    // Check token_hash in db
    let row: (String,) = sqlx::query_as("SELECT token_hash FROM api_tokens WHERE id = ?")
        .bind(&token.id)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert!(!raw_pat.is_empty());
    assert!(!row.0.contains(&raw_pat));
    assert_ne!(row.0, raw_pat);

    // Search table
    let matches: Vec<(String,)> =
        sqlx::query_as("SELECT id FROM api_tokens WHERE token_hash LIKE ? OR name LIKE ?")
            .bind(format!("%{}%", raw_pat))
            .bind(format!("%{}%", raw_pat))
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(matches.is_empty());

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_security_no_plaintext_sessions_in_db() {
    let (pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();
    let user = identity_users::create_user(
        &pool,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "session_leak_user",
        "super_secure_passphrase_123",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let (session, raw_token) =
        sessions::create_session(&pool, &user.id, Some("127.0.0.1"), Some("UA"), &sec_cfg)
            .await
            .unwrap();

    let row: (String,) = sqlx::query_as("SELECT token_hash FROM sessions WHERE id = ?")
        .bind(&session.id)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert!(!raw_token.is_empty());
    assert!(!row.0.contains(&raw_token));
    assert_ne!(row.0, raw_token);

    teardown_test_db(db_path).await;
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. User Enumeration Protection
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_security_user_enumeration_payload_match() {
    let (pool, db_path) = setup_test_db().await;
    let config = test_config(db_path.clone());
    let state = AppState::new(pool.clone(), config);
    let app = api::router::build(state);

    // Scenario A: Non-existent user
    let req_non_existent = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"username": "non_existent_user_123", "password": "some_random_password"}"#,
        ))
        .unwrap();
    let res_non_existent = app.clone().oneshot(req_non_existent).await.unwrap();
    assert_eq!(res_non_existent.status(), StatusCode::UNAUTHORIZED);

    let body_bytes = axum::body::to_bytes(res_non_existent.into_body(), 2048)
        .await
        .unwrap();
    let json_non_existent: Value = serde_json::from_slice(&body_bytes).unwrap();

    // Scenario B: Existent user, wrong password
    let sec_cfg = test_security_config();
    let _user = identity_users::create_user(
        &pool,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "existent_user",
        "super_secure_passphrase_123",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let req_wrong_password = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"username": "existent_user", "password": "wrong_password_abc"}"#,
        ))
        .unwrap();
    let res_wrong_password = app.oneshot(req_wrong_password).await.unwrap();
    assert_eq!(res_wrong_password.status(), StatusCode::UNAUTHORIZED);

    let body_bytes_wrong = axum::body::to_bytes(res_wrong_password.into_body(), 2048)
        .await
        .unwrap();
    let json_wrong_password: Value = serde_json::from_slice(&body_bytes_wrong).unwrap();

    // Compare JSON outputs and check format
    let expected = serde_json::json!({
        "error": "invalid credentials",
        "code": "unauthorized"
    });

    assert_eq!(json_non_existent, expected);
    assert_eq!(json_wrong_password, expected);

    teardown_test_db(db_path).await;
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Session Revocation
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_security_session_revocation_lifecycle() {
    let (pool, db_path) = setup_test_db().await;
    let config = test_config(db_path.clone());
    let state = AppState::new(pool.clone(), config);
    let app = api::router::build(state);

    let sec_cfg = test_security_config();
    let _user = identity_users::create_user(
        &pool,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "session_lifecycle_user",
        "super_secure_passphrase_123",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // 1. Login to get cookie
    let req_login = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"username": "session_lifecycle_user", "password": "super_secure_passphrase_123"}"#,
        ))
        .unwrap();
    let res_login = app.clone().oneshot(req_login).await.unwrap();
    assert_eq!(res_login.status(), StatusCode::OK);

    let cookie_header = res_login
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap();
    let cookie_value = cookie_header.split(';').next().unwrap(); // e.g. nx9_session=abc...

    // 2. Validate GET /api/v1/auth/me works
    let req_me = Request::builder()
        .method("GET")
        .uri("/api/v1/auth/me")
        .header(header::COOKIE, cookie_value)
        .body(Body::empty())
        .unwrap();
    let res_me = app.clone().oneshot(req_me).await.unwrap();
    assert_eq!(res_me.status(), StatusCode::OK);

    // 3. Logout to revoke session
    let req_logout = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/logout")
        .header(header::COOKIE, cookie_value)
        .body(Body::empty())
        .unwrap();
    let res_logout = app.clone().oneshot(req_logout).await.unwrap();
    assert_eq!(res_logout.status(), StatusCode::OK);

    // 4. Try reuse session cookie -> must get 401
    let req_me_revoked = Request::builder()
        .method("GET")
        .uri("/api/v1/auth/me")
        .header(header::COOKIE, cookie_value)
        .body(Body::empty())
        .unwrap();
    let res_me_revoked = app.oneshot(req_me_revoked).await.unwrap();
    assert_eq!(res_me_revoked.status(), StatusCode::UNAUTHORIZED);

    teardown_test_db(db_path).await;
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Transaction Rollback Verification
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_security_transaction_rollback_on_audit_failure_create_user() {
    let (pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();

    // Trigger Foreign Key constraint violation by passing non-existent audit actor ID
    let bad_actor_id = "non_existent_user_id_trigger_rollback";
    let res = identity_users::create_user(
        &pool,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "rollback_user",
        "super_secure_passphrase_123",
        Some(bad_actor_id),
        None,
        None,
    )
    .await;

    // Must return Database/Constraint error
    assert!(res.is_err());

    // Verify user was NOT created in the database due to transaction rollback
    let user_in_db = user_repo::find_by_username(&pool, "rollback_user")
        .await
        .unwrap();
    assert!(user_in_db.is_none());

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_security_transaction_rollback_on_audit_failure_reset_password() {
    let (pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();

    // Create user successfully
    let user = identity_users::create_user(
        &pool,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "rollback_pwd_user",
        "super_secure_passphrase_123",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let original_hash = user.password_hash.clone();

    // Try resetting password but with a bad audit actor id to trigger FK violation
    let bad_actor_id = "non_existent_actor_id";
    let res = identity_users::reset_password(
        &pool,
        &sec_cfg,
        &user.id,
        "new_super_secure_passphrase_123456",
        Some(bad_actor_id),
        None,
        None,
    )
    .await;

    assert!(res.is_err());

    // Verify password hash in db is still the original one (rolled back)
    let user_after = user_repo::find_by_id(&pool, &user.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(user_after.password_hash, original_hash);

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_security_transaction_rollback_on_audit_failure_assign_role() {
    let (pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();

    let user = identity_users::create_user(
        &pool,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "rollback_role_user",
        "super_secure_passphrase_123",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // Try to assign admin role but fail on audit step
    let bad_actor_id = "non_existent_actor_id";
    let res =
        identity_roles::assign_role(&pool, &user.id, "admin", Some(bad_actor_id), None, None).await;

    assert!(res.is_err());

    // Verify role was not assigned
    let user_roles = role_repo::list_for_user(&pool, &user.id).await.unwrap();
    assert!(user_roles.is_empty());

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_security_transaction_rollback_on_audit_failure_create_token() {
    let (pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();

    let user = identity_users::create_user(
        &pool,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "rollback_tok_user",
        "super_secure_passphrase_123",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // Try to create token but fail on audit log FK violation
    let bad_actor_id = "non_existent_actor_id";
    let res = tokens::create_token(
        &pool,
        &user.id,
        "my-pat-token",
        &sec_cfg,
        Some(bad_actor_id),
        None,
        None,
    )
    .await;

    assert!(res.is_err());

    // Verify no tokens were created for the user
    let user_tokens = token_repo::list_for_user(&pool, &user.id).await.unwrap();
    assert!(user_tokens.is_empty());

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_security_assign_non_existent_role_fails() {
    let (pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();
    let user = identity_users::create_user(
        &pool,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "no_role_user",
        "super_secure_passphrase_123",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let res =
        identity_roles::assign_role(&pool, &user.id, "non_existent_role_name", None, None, None)
            .await;
    assert!(res.is_err());
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_security_create_token_non_existent_user_fails() {
    let (pool, db_path) = setup_test_db().await;
    let sec_cfg = test_security_config();
    let res = tokens::create_token(
        &pool,
        "non_existent_user_id",
        "my-token",
        &sec_cfg,
        None,
        None,
        None,
    )
    .await;
    assert!(res.is_err());
    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_security_service_account_audit_lifecycle() {
    let (pool, db_path) = setup_test_db().await;
    let name = "my-service-account";
    let desc = Some("A test description");

    // 1. Create service account
    let sa = nx9_auth::identity::service_accounts::create(
        &pool,
        Tenant::DEFAULT_ID,
        name,
        desc,
        None,
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(sa.name, name);
    assert_eq!(sa.description.as_deref(), desc);
    assert!(sa.enabled);

    // 2. Disable service account
    let res_disable =
        nx9_auth::identity::service_accounts::set_enabled(&pool, &sa.id, false, None, None, None)
            .await;
    assert!(res_disable.is_ok());

    let sa_disabled = nx9_auth::identity::service_accounts::list(&pool, Tenant::DEFAULT_ID)
        .await
        .unwrap()
        .into_iter()
        .find(|x| x.id == sa.id)
        .unwrap();
    assert!(!sa_disabled.enabled);

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_security_verify_dummy_execution() {
    let sec_cfg = test_security_config();
    let res = passwords::verify_dummy(&sec_cfg);
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_security_invalid_password_strength_admin() {
    // Admin password needs to be at least 12 characters
    let res = passwords::validate_password_strength("too_short_1", true);
    assert!(res.is_err());

    let res_ok = passwords::validate_password_strength("long_enough_admin_pass_123", true);
    assert!(res_ok.is_ok());
}
