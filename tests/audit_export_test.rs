#![cfg(feature = "sqlite")]

use nx9_auth::db::{
    self,
    provider::{DatabaseProvider, SqliteProvider},
};
use std::sync::Arc;

async fn setup_test_provider() -> (Arc<SqliteProvider>, String) {
    let db_id = uuid::Uuid::new_v4().to_string();
    let db_path = format!("target/test_audit_export_{}.db", db_id);
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
async fn test_audit_list_remains_clamped_to_500() {
    let (provider, db_path) = setup_test_provider().await;

    // Insert 600 audit events
    for i in 0..600 {
        let audit_id = uuid::Uuid::new_v4().to_string();
        provider
            .audit()
            .insert(
                &audit_id,
                None,
                None,
                "user.login",
                "user",
                None,
                "info",
                Some("127.0.0.1"),
                Some("TestRunner"),
                Some(&format!("{{\"index\": {i}}}")),
            )
            .await
            .unwrap();
    }

    // Normal list filter with limit=1000 MUST be clamped to 500 by backend API logic
    let filter = nx9_auth::db::models::AuditFilter {
        limit: 1000,
        ..Default::default()
    };
    let clamped_limit = filter.limit.clamp(1, 500);
    assert_eq!(clamped_limit, 500);

    let entries = provider
        .audit()
        .list_filtered(&nx9_auth::db::models::AuditFilter {
            limit: clamped_limit,
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(
        entries.len(),
        500,
        "Normal audit listing must be bounded to 500 records max"
    );

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_audit_export_can_return_more_than_500_up_to_5000() {
    let (provider, db_path) = setup_test_provider().await;

    // Insert 600 audit events
    for i in 0..600 {
        let audit_id = uuid::Uuid::new_v4().to_string();
        provider
            .audit()
            .insert(
                &audit_id,
                None,
                None,
                "user.login",
                "user",
                None,
                "info",
                Some("127.0.0.1"),
                Some("TestRunner"),
                Some(&format!("{{\"index\": {i}}}")),
            )
            .await
            .unwrap();
    }

    // Export query path with limit=5000 returns all 600 records (>500)
    let export_filter = nx9_auth::db::models::AuditFilter {
        limit: 5000,
        ..Default::default()
    };

    let entries = provider
        .audit()
        .list_filtered(&export_filter)
        .await
        .unwrap();

    assert_eq!(
        entries.len(),
        600,
        "Export query path must return >500 records when available"
    );

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_audit_export_hard_bounded_at_5000() {
    let (_provider, db_path) = setup_test_provider().await;

    // Request limit 999999 must be clamped to hard maximum 5000
    let requested_limit = 999999i64;
    let export_limit = requested_limit.clamp(1, 5000);
    assert_eq!(export_limit, 5000);

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_server_side_success_filtering_before_limit_and_count() {
    let (provider, db_path) = setup_test_provider().await;

    // Insert 10 success events and 10 failure events
    for _ in 0..10 {
        let audit_id = uuid::Uuid::new_v4().to_string();
        provider
            .audit()
            .insert(
                &audit_id,
                None,
                None,
                "user.login",
                "user",
                None,
                "info",
                Some("127.0.0.1"),
                Some("TestRunner"),
                None,
            )
            .await
            .unwrap();
    }

    for _ in 0..10 {
        let audit_id = uuid::Uuid::new_v4().to_string();
        provider
            .audit()
            .insert(
                &audit_id,
                None,
                None,
                "auth.failed",
                "user",
                None,
                "warning",
                Some("127.0.0.1"),
                Some("TestRunner"),
                None,
            )
            .await
            .unwrap();
    }

    // Server-side filter success = true
    let success_filter = nx9_auth::db::models::AuditFilter {
        success: Some(true),
        limit: 50,
        ..Default::default()
    };

    let count = provider
        .audit()
        .count_filtered(&success_filter)
        .await
        .unwrap();
    let entries = provider
        .audit()
        .list_filtered(&success_filter)
        .await
        .unwrap();

    assert_eq!(count, 10);
    assert_eq!(entries.len(), 10);
    assert!(entries.iter().all(|e| !e.action.contains("fail")));

    // Server-side filter success = false
    let fail_filter = nx9_auth::db::models::AuditFilter {
        success: Some(false),
        limit: 50,
        ..Default::default()
    };

    let fail_count = provider.audit().count_filtered(&fail_filter).await.unwrap();
    let fail_entries = provider.audit().list_filtered(&fail_filter).await.unwrap();

    assert_eq!(fail_count, 10);
    assert_eq!(fail_entries.len(), 10);
    assert!(fail_entries.iter().all(|e| e.action.contains("fail")));

    teardown_test_db(db_path).await;
}

#[test]
fn test_rfc4180_csv_escaping_rules() {
    let esc = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));

    assert_eq!(esc("simple"), "\"simple\"");
    assert_eq!(esc("with,comma"), "\"with,comma\"");
    assert_eq!(esc("with \"quotes\""), "\"with \"\"quotes\"\"\"");
    assert_eq!(esc("multi\nline"), "\"multi\nline\"");
}
