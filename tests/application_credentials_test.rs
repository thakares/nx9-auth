#![cfg(feature = "sqlite")]

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

use nx9_auth::{
    api,
    config::{Config, DatabaseConfig, SecurityConfig, ServerConfig},
    db::{self, models::Tenant, provider::SqliteProvider},
    error::AppError,
    identity::{applications, roles, users},
    security::sessions,
    state::AppState,
};

async fn setup_test_db() -> (
    Arc<dyn db::provider::DatabaseProvider>,
    sqlx::SqlitePool,
    String,
) {
    let db_id = uuid::Uuid::new_v4().to_string();
    let db_path = format!("target/test_app_{}.db", db_id);
    let pool = db::create_pool(&db_path)
        .await
        .expect("Failed to create test pool");
    db::run_migrations(&pool)
        .await
        .expect("Failed to run test migrations");
    let provider = Arc::new(SqliteProvider::new(pool.clone()));
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
        argon2_memory: 4096,
        argon2_iterations: 1,
        argon2_parallelism: 1,
    }
}

fn test_config(db_path: String) -> Config {
    Config {
        server: ServerConfig {
            host: "127.0.0.1".into(),
            port: 8655,
            cookie_secure: false,
            production: false,
        },
        database: DatabaseConfig {
            path: Some(db_path),
            ..Default::default()
        },
        security: test_security_config(),
        audit: nx9_auth::config::AuditConfig { enabled: true },
        ..Default::default()
    }
}

async fn setup_app() -> (axum::Router, String, String, String) {
    let (provider, _pool, db_path) = setup_test_db().await;
    let config = test_config(db_path.clone());
    let sec_cfg = config.security.clone();

    let admin = users::create_user(
        &provider,
        &sec_cfg,
        Tenant::DEFAULT_ID,
        "admin_app_user",
        "AdminSecret123!",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    roles::assign_role(&provider, &admin.id, "admin", None, None, None)
        .await
        .unwrap();

    let (_session, raw_token) = sessions::create_session(
        &provider,
        &admin.id,
        Some("127.0.0.1"),
        Some("TestUA"),
        &sec_cfg,
    )
    .await
    .unwrap();

    let state = AppState::new(provider.clone(), config);
    let router = api::router::build(state);
    (router, admin.id, raw_token, db_path)
}

#[tokio::test]
async fn test_application_credential_generation_and_validation() {
    let (provider, _pool, db_path) = setup_test_db().await;

    let client_id = applications::generate_client_id();
    assert!(client_id.starts_with("nx9_app_"));
    assert_eq!(client_id.len(), 40); // nx9_app_ (8) + 32 hex chars = 40

    let client_secret = applications::generate_client_secret();
    assert!(client_secret.starts_with("nx9_secret_"));
    assert_eq!(client_secret.len(), 75); // nx9_secret_ (11) + 64 hex chars = 75

    let (app, raw_secret) = applications::create(
        &provider,
        Tenant::DEFAULT_ID,
        "Test App",
        "test-app",
        Some("Description of Test App"),
        Some(vec!["https://example.com/callback".into()]),
        Some(vec!["openid".into(), "profile".into()]),
        None,
        None,
        None,
    )
    .await
    .unwrap();

    assert!(app.get_client_id().starts_with("nx9_app_"));
    assert!(app.has_credentials());
    assert_eq!(app.redirect_urls(), vec!["https://example.com/callback"]);
    assert_eq!(app.scopes(), vec!["openid", "profile"]);

    // Secret hash in DB must be hex encoded BLAKE3 digest, not plaintext secret
    assert_ne!(app.client_secret_hash.as_ref().unwrap(), &raw_secret);

    // Valid credentials authentication
    let validated =
        applications::validate_client_credentials(&provider, app.get_client_id(), &raw_secret)
            .await
            .unwrap();
    assert_eq!(validated.id, app.id);

    // Invalid secret
    let invalid_sec = applications::validate_client_credentials(
        &provider,
        app.get_client_id(),
        "nx9_secret_invalid",
    )
    .await;
    assert!(matches!(invalid_sec, Err(AppError::Unauthorized)));

    // Unknown client_id
    let unknown_client =
        applications::validate_client_credentials(&provider, "nx9_app_nonexistent", &raw_secret)
            .await;
    assert!(matches!(unknown_client, Err(AppError::Unauthorized)));

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_application_secret_rotation() {
    let (provider, _pool, db_path) = setup_test_db().await;

    let (app, old_secret) = applications::create(
        &provider,
        Tenant::DEFAULT_ID,
        "Rotate App",
        "rotate-app",
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let new_secret = applications::rotate_secret(&provider, &app.id, None, None, None)
        .await
        .unwrap();

    assert_ne!(old_secret, new_secret);

    // Old secret fails
    let old_val =
        applications::validate_client_credentials(&provider, app.get_client_id(), &old_secret)
            .await;
    assert!(matches!(old_val, Err(AppError::Unauthorized)));

    // New secret succeeds
    let new_val =
        applications::validate_client_credentials(&provider, app.get_client_id(), &new_secret)
            .await;
    assert!(new_val.is_ok());

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_application_api_endpoints_and_cache_control() {
    let (app_router, _user_id, token, db_path) = setup_app().await;

    // 1. Create Application API
    let req_body = serde_json::json!({
        "name": "API Test App",
        "slug": "api-test-app",
        "description": "App built for API testing",
        "redirect_urls": ["https://app.test/cb"],
        "scopes": ["openid", "profile"]
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/applications")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, format!("nx9_session={token}"))
        .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
        .unwrap();

    let resp = app_router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers()
            .get(header::CACHE_CONTROL)
            .unwrap()
            .to_str()
            .unwrap(),
        "no-store"
    );

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let create_resp: Value = serde_json::from_slice(&body_bytes).unwrap();
    let app_obj = &create_resp["application"];
    let client_id = app_obj["client_id"].as_str().unwrap().to_string();
    let app_id = app_obj["id"].as_str().unwrap().to_string();
    let client_secret = create_resp["client_secret"].as_str().unwrap().to_string();

    assert!(client_id.starts_with("nx9_app_"));
    assert!(client_secret.starts_with("nx9_secret_"));

    // 2. GET Application API (Must NOT expose secret or secret hash)
    let get_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/applications/{app_id}"))
        .header(header::COOKIE, format!("nx9_session={token}"))
        .body(Body::empty())
        .unwrap();

    let get_resp = app_router.clone().oneshot(get_req).await.unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);
    let get_bytes = get_resp.into_body().collect().await.unwrap().to_bytes();
    let get_json: Value = serde_json::from_slice(&get_bytes).unwrap();
    let get_app = &get_json["application"];

    assert_eq!(get_app["client_id"], client_id);
    assert!(get_app.get("client_secret").is_none());
    assert!(get_app.get("client_secret_hash").is_none());
    assert_eq!(get_app["credentials_configured"], true);

    // 3. PATCH Application containing `client_id` MUST be rejected by `deny_unknown_fields`
    let patch_invalid = serde_json::json!({
        "name": "Updated Name",
        "slug": "api-test-app",
        "client_id": "nx9_app_hack_attempt",
        "enabled": true
    });

    let patch_req = Request::builder()
        .method("PATCH")
        .uri(format!("/api/v1/applications/{app_id}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, format!("nx9_session={token}"))
        .body(Body::from(serde_json::to_vec(&patch_invalid).unwrap()))
        .unwrap();

    let patch_resp = app_router.clone().oneshot(patch_req).await.unwrap();
    assert!(patch_resp.status().is_client_error()); // 400 / 422 Bad Request due to deny_unknown_fields

    // 4. Rotate Secret API
    let rotate_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/applications/{app_id}/secret"))
        .header(header::COOKIE, format!("nx9_session={token}"))
        .body(Body::empty())
        .unwrap();

    let rotate_resp = app_router.clone().oneshot(rotate_req).await.unwrap();
    assert_eq!(rotate_resp.status(), StatusCode::OK);
    assert_eq!(
        rotate_resp
            .headers()
            .get(header::CACHE_CONTROL)
            .unwrap()
            .to_str()
            .unwrap(),
        "no-store"
    );

    let rotate_bytes = rotate_resp.into_body().collect().await.unwrap().to_bytes();
    let rotate_json: Value = serde_json::from_slice(&rotate_bytes).unwrap();
    let new_secret = rotate_json["client_secret"].as_str().unwrap();
    assert!(new_secret.starts_with("nx9_secret_"));
    assert_ne!(new_secret, client_secret);

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_application_database_migration_backfill_and_upgrade() {
    let db_id = uuid::Uuid::new_v4().to_string();
    let db_path = format!("target/test_upgrade_{}.db", db_id);
    let pool = db::create_pool(&db_path).await.unwrap();

    // Execute migrations up to 0016 manually to simulate a v0.3.0 existing database
    sqlx::query(include_str!(
        "../src/db/migrations/sqlite/0001_create_tenants.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(include_str!(
        "../src/db/migrations/sqlite/0002_create_users.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(include_str!(
        "../src/db/migrations/sqlite/0003_create_user_profiles.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(include_str!(
        "../src/db/migrations/sqlite/0004_create_roles.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(include_str!(
        "../src/db/migrations/sqlite/0005_create_permissions.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(include_str!(
        "../src/db/migrations/sqlite/0006_create_role_permissions.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(include_str!(
        "../src/db/migrations/sqlite/0007_create_user_roles.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(include_str!(
        "../src/db/migrations/sqlite/0008_create_sessions.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(include_str!(
        "../src/db/migrations/sqlite/0009_create_api_tokens.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(include_str!(
        "../src/db/migrations/sqlite/0010_create_service_accounts.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(include_str!(
        "../src/db/migrations/sqlite/0011_create_applications.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(include_str!(
        "../src/db/migrations/sqlite/0012_create_audit_logs.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(include_str!(
        "../src/db/migrations/sqlite/0013_seed_default_tenant.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(include_str!(
        "../src/db/migrations/sqlite/0014_seed_roles_and_permissions.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(include_str!(
        "../src/db/migrations/sqlite/0015_create_refresh_tokens.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(include_str!(
        "../src/db/migrations/sqlite/0016_create_groups.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();

    let legacy_id = "30000000-0000-0000-0000-000000000099";
    sqlx::query("INSERT INTO applications (id, tenant_id, name, slug) VALUES (?, '00000000-0000-0000-0000-000000000001', 'Legacy App', 'legacy-app')")
        .bind(legacy_id)
        .execute(&pool)
        .await
        .unwrap();

    // Now run migration 0017 and 0018
    sqlx::query(include_str!(
        "../src/db/migrations/sqlite/0017_update_applications_credentials.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(include_str!(
        "../src/db/migrations/sqlite/0018_harden_application_credentials.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();

    let provider: Arc<dyn db::provider::DatabaseProvider> = Arc::new(SqliteProvider::new(pool));
    let legacy_app = applications::get(&provider, legacy_id).await.unwrap();

    assert_eq!(legacy_app.name, "Legacy App");
    assert_eq!(legacy_app.slug.as_deref(), Some("legacy-app"));
    assert!(legacy_app.get_client_id().starts_with("nx9_app_"));
    assert!(!legacy_app.has_credentials());

    // Administrator performs secret rotation to generate credentials
    let generated_secret = applications::rotate_secret(&provider, &legacy_app.id, None, None, None)
        .await
        .unwrap();
    let updated_legacy = applications::get(&provider, legacy_id).await.unwrap();
    assert!(updated_legacy.has_credentials());

    // Validate generated credentials
    let auth_res = applications::validate_client_credentials(
        &provider,
        updated_legacy.get_client_id(),
        &generated_secret,
    )
    .await;
    assert!(auth_res.is_ok());

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_application_creation_transactional_rollback_on_audit_failure() {
    let (provider, _pool, db_path) = setup_test_db().await;

    // Force audit log foreign-key failure by passing invalid actor_id
    let res = applications::create(
        &provider,
        Tenant::DEFAULT_ID,
        "Rollback App",
        "rollback-app",
        None,
        None,
        None,
        Some("non_existent_actor_id_fk"),
        None,
        None,
    )
    .await;

    assert!(res.is_err());

    // Verify application record was NOT created in DB
    let app_opt = applications::find_by_slug(&provider, "rollback-app").await;
    assert!(matches!(app_opt, Err(AppError::NotFound)));

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_application_secret_rotation_transactional_rollback_on_audit_failure() {
    let (provider, _pool, db_path) = setup_test_db().await;

    let (app, old_secret) = applications::create(
        &provider,
        Tenant::DEFAULT_ID,
        "Rotate Rollback App",
        "rotate-rollback-app",
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let orig_hash = app.client_secret_hash.clone().unwrap();

    // Force audit insertion failure during rotation
    let fail_res = applications::rotate_secret(
        &provider,
        &app.id,
        Some("non_existent_actor_id_fk"),
        None,
        None,
    )
    .await;

    assert!(fail_res.is_err());

    // Assert stored client_secret_hash in DB remains UNCHANGED
    let app_after_failed_rotation = applications::get(&provider, &app.id).await.unwrap();
    assert_eq!(
        app_after_failed_rotation
            .client_secret_hash
            .as_ref()
            .unwrap(),
        &orig_hash
    );

    // Assert original secret STILL authenticates successfully
    let orig_auth =
        applications::validate_client_credentials(&provider, app.get_client_id(), &old_secret)
            .await;
    assert!(orig_auth.is_ok());

    // Perform successful rotation
    let new_secret = applications::rotate_secret(&provider, &app.id, None, None, None)
        .await
        .unwrap();

    // Old secret fails, new secret succeeds
    let old_auth =
        applications::validate_client_credentials(&provider, app.get_client_id(), &old_secret)
            .await;
    assert!(matches!(old_auth, Err(AppError::Unauthorized)));

    let new_auth =
        applications::validate_client_credentials(&provider, app.get_client_id(), &new_secret)
            .await;
    assert!(new_auth.is_ok());

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_application_authentication_slug_rejection() {
    let (provider, _pool, db_path) = setup_test_db().await;

    let (app, secret) = applications::create(
        &provider,
        Tenant::DEFAULT_ID,
        "Slug Reject App",
        "slug-reject-app",
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // Authentication by slug MUST fail
    let slug_auth =
        applications::validate_client_credentials(&provider, "slug-reject-app", &secret).await;
    assert!(matches!(slug_auth, Err(AppError::Unauthorized)));

    // Authentication by client_id MUST succeed
    let client_id_auth =
        applications::validate_client_credentials(&provider, app.get_client_id(), &secret).await;
    assert!(client_id_auth.is_ok());

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_redirect_uri_structural_validation() {
    let (provider, _pool, db_path) = setup_test_db().await;

    // 1. Malformed URI
    let malformed = applications::create(
        &provider,
        Tenant::DEFAULT_ID,
        "App 1",
        "app-1",
        None,
        Some(vec!["not-a-valid-uri".into()]),
        None,
        None,
        None,
        None,
    )
    .await;
    assert!(matches!(malformed, Err(AppError::InvalidInput(_))));

    // 2. Fragment URI
    let fragment = applications::create(
        &provider,
        Tenant::DEFAULT_ID,
        "App 2",
        "app-2",
        None,
        Some(vec!["https://example.com/callback#frag".into()]),
        None,
        None,
        None,
        None,
    )
    .await;
    assert!(matches!(fragment, Err(AppError::InvalidInput(_))));

    // 3. Userinfo URI
    let userinfo = applications::create(
        &provider,
        Tenant::DEFAULT_ID,
        "App 3",
        "app-3",
        None,
        Some(vec!["https://user:pass@example.com/callback".into()]),
        None,
        None,
        None,
        None,
    )
    .await;
    assert!(matches!(userinfo, Err(AppError::InvalidInput(_))));

    // 4. Non-loopback HTTP URI (must be rejected)
    let non_loopback_http = applications::create(
        &provider,
        Tenant::DEFAULT_ID,
        "App 4",
        "app-4",
        None,
        Some(vec!["http://example.com/callback".into()]),
        None,
        None,
        None,
        None,
    )
    .await;
    assert!(matches!(non_loopback_http, Err(AppError::InvalidInput(_))));

    // 5. Custom scheme (must be rejected)
    let custom_scheme = applications::create(
        &provider,
        Tenant::DEFAULT_ID,
        "App 5",
        "app-5",
        None,
        Some(vec!["myapp://callback".into()]),
        None,
        None,
        None,
        None,
    )
    .await;
    assert!(matches!(custom_scheme, Err(AppError::InvalidInput(_))));

    // 6. >10 URIs
    let too_many_uris: Vec<String> = (0..11)
        .map(|i| format!("https://example{i}.com/cb"))
        .collect();
    let too_many = applications::create(
        &provider,
        Tenant::DEFAULT_ID,
        "App 6",
        "app-6",
        None,
        Some(too_many_uris),
        None,
        None,
        None,
        None,
    )
    .await;
    assert!(matches!(too_many, Err(AppError::InvalidInput(_))));

    // 7. Valid URIs (https and http loopback)
    let valid = applications::create(
        &provider,
        Tenant::DEFAULT_ID,
        "Valid App",
        "valid-app",
        None,
        Some(vec![
            "https://app.example.com/callback".into(),
            "http://127.0.0.1:8080/callback".into(),
            "http://localhost:3000/callback".into(),
        ]),
        None,
        None,
        None,
        None,
    )
    .await;
    assert!(valid.is_ok());

    teardown_test_db(db_path).await;
}
