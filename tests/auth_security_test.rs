//! Authentication security tests (OWASP-oriented).

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
    db::models::Tenant,
    identity::{roles as identity_roles, users as identity_users},
    state::AppState,
};

fn test_security_config() -> SecurityConfig {
    SecurityConfig {
        session_ttl_hours: 24,
        session_absolute_ttl_days: 30,
        token_ttl_days: 365,
        argon2_memory: 4096,
        argon2_iterations: 1,
        argon2_parallelism: 1,
    }
}

async fn setup() -> (AppState, String) {
    let db_id = uuid::Uuid::new_v4().to_string();
    let db_path = format!("target/test_authsec_{}.db", db_id);
    let pool = nx9_auth::db::create_pool(&db_path).await.unwrap();
    nx9_auth::db::run_migrations(&pool).await.unwrap();
    let provider: std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider> =
        std::sync::Arc::new(nx9_auth::db::provider::SqliteProvider::new(pool));
    let mut config = Config {
        security: test_security_config(),
        ..Default::default()
    };
    config.server.host = "127.0.0.1".into();
    config.server.port = 8655;
    config.server.cookie_secure = false;
    config.server.production = false;
    let state = AppState::new(provider.clone(), config);
    let admin = identity_users::create_user(
        &provider,
        &test_security_config(),
        Tenant::DEFAULT_ID,
        "sec_admin",
        "super_secure_admin_passphrase_123",
        None,
        None,
        None,
    )
    .await
    .unwrap();
    identity_roles::assign_role(&provider, &admin.id, "admin", None, None, None)
        .await
        .unwrap();
    (state, db_path)
}

#[tokio::test]
async fn test_login_is_post_only() {
    let (state, db_path) = setup().await;
    let app = api::router::build(state);

    // GET must not authenticate and must not be a login handler (405 or 404).
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/login?username=sec_admin&password=super_secure_admin_passphrase_123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        res.status() == StatusCode::METHOD_NOT_ALLOWED
            || res.status() == StatusCode::NOT_FOUND
            || res.status() == StatusCode::UNAUTHORIZED,
        "GET login must not succeed: {}",
        res.status()
    );

    // POST with JSON succeeds and returns access_token.
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::ACCEPT, "application/json")
                .body(Body::from(
                    r#"{"username":"sec_admin","password":"super_secure_admin_passphrase_123"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("access_token").and_then(|v| v.as_str()).is_some());
    assert!(json.get("refresh_token").and_then(|v| v.as_str()).is_some());
    assert!(json.get("expires_in").and_then(|v| v.as_u64()).is_some());
    assert_eq!(
        json.get("token_type").and_then(|v| v.as_str()),
        Some("Bearer")
    );
    assert!(json.pointer("/user/username").and_then(|v| v.as_str()) == Some("sec_admin"));
    // Password must never appear in response
    let text = String::from_utf8_lossy(&body);
    assert!(!text.contains("super_secure_admin_passphrase_123"));

    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_login_invalid_credentials_non_enumerating() {
    let (state, db_path) = setup().await;
    let app = api::router::build(state);

    for body in [
        r#"{"username":"no_such_user","password":"whatever_password_xx"}"#,
        r#"{"username":"sec_admin","password":"wrong_password_xx"}"#,
    ] {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        let err = json["error"].as_str().unwrap_or("");
        assert_eq!(err, "Invalid username or password.");
        // Must not reveal which field failed
        assert!(!err.to_lowercase().contains("unknown user"));
        assert!(!err.to_lowercase().contains("incorrect password"));
    }

    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_login_bearer_access_token_works() {
    let (state, db_path) = setup().await;
    let app = api::router::build(state);

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"sec_admin","password":"super_secure_admin_passphrase_123"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let token = json["access_token"].as_str().unwrap();

    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/me")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_security_headers_present() {
    let (state, db_path) = setup().await;
    let app = api::router::build(state);
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let h = res.headers();
    assert!(h.get("x-content-type-options").is_some());
    assert!(h.get("x-frame-options").is_some());
    assert!(h.get("referrer-policy").is_some());
    assert!(h.get("content-security-policy").is_some());
    assert!(h.get("permissions-policy").is_some());

    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_production_requires_cookie_secure() {
    let mut cfg = Config::default();
    cfg.server.production = true;
    cfg.server.cookie_secure = false;
    assert!(cfg.server.validate_production_security().is_err());

    cfg.server.cookie_secure = true;
    assert!(cfg.server.validate_production_security().is_ok());
}

#[tokio::test]
async fn test_argon2id_hash_format() {
    use nx9_auth::security::passwords;
    let cfg = test_security_config();
    let hash = passwords::hash_password("super_secure_passphrase_123", &cfg).unwrap();
    assert!(hash.starts_with("$argon2id$"), "hash={hash}");
    assert!(passwords::verify_password("super_secure_passphrase_123", &hash).unwrap());
    assert!(!passwords::verify_password("wrong", &hash).unwrap());
}
