#![cfg(feature = "sqlite")]

use nx9_auth::db::{
    self,
    models::Tenant,
    provider::{DatabaseProvider, SqliteProvider},
};
use nx9_auth::identity::{application_members, applications, users};
use std::sync::Arc;

async fn setup_test_provider() -> (Arc<SqliteProvider>, String) {
    let db_id = uuid::Uuid::new_v4().to_string();
    let db_path = format!("target/test_app_members_{}.db", db_id);
    let pool = db::create_pool(&db_path)
        .await
        .expect("Failed to create test pool");
    db::run_migrations(&pool)
        .await
        .expect("Failed to run migrations");
    (Arc::new(SqliteProvider::new(pool)), db_path)
}

async fn teardown_test_db(path: String) {
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn test_application_membership_add_update_remove_transactions() {
    let (provider, db_path) = setup_test_provider().await;
    let provider_dyn: Arc<dyn DatabaseProvider> = provider.clone();
    let dummy_cfg = nx9_auth::config::SecurityConfig::default();

    // Create user and application in Default Tenant
    let user = users::create_user(
        &provider_dyn,
        &dummy_cfg,
        Tenant::DEFAULT_ID,
        "app_user_1",
        "X9#mK$9qL!2zP0",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let (app, _) = applications::create(
        &provider_dyn,
        Tenant::DEFAULT_ID,
        "Portal App",
        "portal-app",
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // 1. Add membership atomically with audit log
    let member = application_members::add(
        &provider_dyn,
        &app.id,
        &user.id,
        Some("member"),
        Some(&user.id),
        Some("127.0.0.1"),
        Some("TestRunner"),
    )
    .await
    .unwrap();

    assert_eq!(member.application_id, app.id);
    assert_eq!(member.user_id, user.id);
    assert_eq!(member.role, "member");

    let audit_add = provider
        .audit()
        .list_filtered(&nx9_auth::db::models::AuditFilter {
            resource_type: Some("application".to_string()),
            limit: 10,
            ..Default::default()
        })
        .await
        .unwrap();

    let add_event = audit_add
        .into_iter()
        .find(|e| e.action == "application.member_added")
        .expect("member_added audit record must exist");
    assert_eq!(add_event.target_user_id.as_deref(), Some(user.id.as_str()));

    // 2. Update membership role atomically with audit log
    let updated_member = application_members::update(
        &provider_dyn,
        &app.id,
        &user.id,
        Some("admin"),
        None,
        Some(&user.id),
        Some("127.0.0.1"),
        Some("TestRunner"),
    )
    .await
    .unwrap();

    assert_eq!(updated_member.role, "admin");

    // 3. Remove membership atomically with audit log
    application_members::remove(
        &provider_dyn,
        &app.id,
        &user.id,
        Some(&user.id),
        Some("127.0.0.1"),
        Some("TestRunner"),
    )
    .await
    .unwrap();

    let remaining_members = provider
        .application_members()
        .list_by_application(&app.id)
        .await
        .unwrap();
    assert_eq!(remaining_members.len(), 0);

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_application_membership_same_tenant_isolation() {
    let (provider, db_path) = setup_test_provider().await;
    let provider_dyn: Arc<dyn DatabaseProvider> = provider.clone();
    let dummy_cfg = nx9_auth::config::SecurityConfig::default();

    let tenant_b = uuid::Uuid::new_v4().to_string();
    provider
        .tenants()
        .create(&tenant_b, "Tenant B", Some("tenant-b-app"))
        .await
        .unwrap();

    // Create user in Default Tenant
    let user_a = users::create_user(
        &provider_dyn,
        &dummy_cfg,
        Tenant::DEFAULT_ID,
        "user_tenant_a",
        "X9#mK$9qL!2zP0",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // Create application in Tenant B
    let (app_b, _) = applications::create(
        &provider_dyn,
        &tenant_b,
        "App Tenant B",
        "app-tenant-b",
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // Attempting to assign user_a (Tenant Default) to app_b (Tenant B) MUST be rejected
    let res = application_members::add(
        &provider_dyn,
        &app_b.id,
        &user_a.id,
        Some("member"),
        None,
        None,
        None,
    )
    .await;

    assert!(
        res.is_err(),
        "Cross-tenant application membership assignment MUST be rejected"
    );

    teardown_test_db(db_path).await;
}
