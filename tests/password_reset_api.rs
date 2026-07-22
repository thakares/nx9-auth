#![cfg(feature = "sqlite")]

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
    identity::users as identity_users,
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

async fn setup() -> (AppState, String, String) {
    let db_id = uuid::Uuid::new_v4().to_string();
    let db_path = format!("target/test_pwdreset_{}.db", db_id);
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
    let state = AppState::new(provider.clone(), config);
    let admin = identity_users::create_user(
        &state.provider,
        &test_security_config(),
        Tenant::DEFAULT_ID,
        "admin_pw",
        "S3cur3#P@ssw0rd$N0S3qu3nc3!",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let admin_role = state
        .provider
        .roles()
        .find_by_name("admin")
        .await
        .unwrap()
        .unwrap();
    state
        .provider
        .roles()
        .assign_to_user(&admin.id, &admin_role.id)
        .await
        .unwrap();

    (state, db_path, admin.id)
}

async fn login_cookie(app: axum::Router, user: &str, pass: &str) -> String {
    let app = app;
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"username":"{user}","password":"{pass}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "login failed");
    let set_cookie = res
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|c| c.starts_with("nx9_session="))
        .expect("session cookie")
        .to_string();
    set_cookie.split(';').next().unwrap().to_string()
}

#[tokio::test]
async fn test_api_profile_change_password() {
    let (state, db_path, _admin_id) = setup().await;
    let app = api::router::build(state.clone());
    let cookie = login_cookie(app.clone(), "admin_pw", "S3cur3#P@ssw0rd$N0S3qu3nc3!").await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/profile/password")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(
                    r#"{"current_password":"S3cur3#P@ssw0rd$N0S3qu3nc3!","new_password":"brand_new_admin_pass_456"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8_lossy(&body);
    assert_eq!(status, StatusCode::OK, "change password failed: {text}");

    // login with new password
    let app = api::router::build(state);
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"admin_pw","password":"brand_new_admin_pass_456"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_api_admin_reset_password() {
    let (state, db_path, _admin_id) = setup().await;
    let _pool = state.provider.clone();
    let target = identity_users::create_user(
        &state.provider,
        &test_security_config(),
        Tenant::DEFAULT_ID,
        "target_user",
        "super_secure_passphrase_123",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let app = api::router::build(state.clone());
    let cookie = login_cookie(app.clone(), "admin_pw", "S3cur3#P@ssw0rd$N0S3qu3nc3!").await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/users/{}/reset-password", target.id))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(r#"{"password":"reset_to_new_secure_phrase"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8_lossy(&body);
    assert_eq!(status, StatusCode::OK, "reset password failed: {text}");

    // target can login with new password
    let app = api::router::build(state);
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"target_user","password":"reset_to_new_secure_phrase"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_api_reset_password_weak_returns_422() {
    let (state, db_path, _admin_id) = setup().await;
    let _pool = state.provider.clone();
    let target = identity_users::create_user(
        &state.provider,
        &test_security_config(),
        Tenant::DEFAULT_ID,
        "target_weak",
        "super_secure_passphrase_123",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let app = api::router::build(state);
    let cookie = login_cookie(app.clone(), "admin_pw", "S3cur3#P@ssw0rd$N0S3qu3nc3!").await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/users/{}/reset-password", target.id))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(r#"{"password":"password12345"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(
        json["error"].as_str().unwrap().contains("weak")
            || json["error"].as_str().unwrap().contains("password")
    );
    let _ = std::fs::remove_file(db_path);
}
