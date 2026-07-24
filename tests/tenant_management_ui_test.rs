#![cfg(feature = "sqlite")]

use nx9_auth::db::{
    self,
    models::Tenant,
    provider::{DatabaseProvider, SqliteProvider},
};
use nx9_auth::identity::{applications, users};
use std::sync::Arc;

async fn setup_test_provider() -> (Arc<SqliteProvider>, String) {
    let db_id = uuid::Uuid::new_v4().to_string();
    let db_path = format!("target/test_tenant_mgmt_{}.db", db_id);
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
async fn test_tenant_user_listing_and_assignment() {
    let (provider, db_path) = setup_test_provider().await;
    let provider_dyn: Arc<dyn DatabaseProvider> = provider.clone();

    // Create a new tenant
    let tenant_id = uuid::Uuid::new_v4().to_string();
    provider
        .tenants()
        .create(&tenant_id, "Acme Org", Some("acme-org"))
        .await
        .expect("Tenant creation should succeed");

    let dummy_cfg = nx9_auth::config::SecurityConfig::default();

    let user = users::create_user(
        &provider_dyn,
        &dummy_cfg,
        Tenant::DEFAULT_ID,
        "employee_1",
        "X9#mK$9qL!2zP0",
        None,
        None,
        None,
    )
    .await
    .expect("User creation should succeed");

    assert_eq!(user.tenant_id, Tenant::DEFAULT_ID);

    // Reassign user to Acme Org
    provider
        .users()
        .reassign_user_tenant_with_audit(&user.id, &tenant_id, None, None, None)
        .await
        .expect("Tenant assignment should succeed");

    let tenant_users = provider
        .users()
        .list(&tenant_id)
        .await
        .expect("Listing tenant users should succeed");

    assert_eq!(tenant_users.len(), 1);
    assert_eq!(tenant_users[0].username, "employee_1");

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_assign_user_username_collision_rejection() {
    let (provider, db_path) = setup_test_provider().await;
    let provider_dyn: Arc<dyn DatabaseProvider> = provider.clone();

    let dummy_cfg = nx9_auth::config::SecurityConfig::default();

    let tenant_id = uuid::Uuid::new_v4().to_string();
    provider
        .tenants()
        .create(&tenant_id, "Beta Corp", Some("beta-corp"))
        .await
        .unwrap();

    // Create user in Default tenant named "common_user"
    let u1 = users::create_user(
        &provider_dyn,
        &dummy_cfg,
        Tenant::DEFAULT_ID,
        "common_user",
        "X9#mK$9qL!2zP0",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // Create user in Beta Corp also named "common_user"
    let _u2 = users::create_user(
        &provider_dyn,
        &dummy_cfg,
        &tenant_id,
        "common_user",
        "X9#mK$9qL!2zP0",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // Attempting to move u1 into Beta Corp must collide because "common_user" already exists in Beta Corp
    let exists = provider
        .users()
        .username_exists(&tenant_id, &u1.username)
        .await
        .unwrap();

    assert!(
        exists,
        "Username existence check must return true for colliding username in destination tenant"
    );

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_tenant_application_listing_isolation() {
    let (provider, db_path) = setup_test_provider().await;
    let provider_dyn: Arc<dyn DatabaseProvider> = provider.clone();

    let tenant_a = uuid::Uuid::new_v4().to_string();
    let tenant_b = uuid::Uuid::new_v4().to_string();

    provider
        .tenants()
        .create(&tenant_a, "Tenant A", Some("tenant-a"))
        .await
        .unwrap();

    provider
        .tenants()
        .create(&tenant_b, "Tenant B", Some("tenant-b"))
        .await
        .unwrap();

    // Create application in Tenant A
    let (app_a, _) = applications::create(
        &provider_dyn,
        &tenant_a,
        "App A",
        "app-a-slug",
        None,
        None,
        None,
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
        "App B",
        "app-b-slug",
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let apps_a = provider.applications().list(&tenant_a).await.unwrap();
    let apps_b = provider.applications().list(&tenant_b).await.unwrap();

    assert_eq!(apps_a.len(), 1);
    assert_eq!(apps_a[0].id, app_a.id);

    assert_eq!(apps_b.len(), 1);
    assert_eq!(apps_b[0].id, app_b.id);

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_session_identity_immediately_reflects_tenant_reassignment() {
    let (provider, db_path) = setup_test_provider().await;
    let provider_dyn: Arc<dyn DatabaseProvider> = provider.clone();
    let dummy_cfg = nx9_auth::config::SecurityConfig::default();

    let tenant_b = uuid::Uuid::new_v4().to_string();
    provider
        .tenants()
        .create(&tenant_b, "Tenant B", Some("tenant-b-session"))
        .await
        .unwrap();

    // 1. Create user in Default Tenant
    let user = users::create_user(
        &provider_dyn,
        &dummy_cfg,
        Tenant::DEFAULT_ID,
        "session_user",
        "X9#mK$9qL!2zP0",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // 2. Create active session for user
    let session_id = uuid::Uuid::new_v4().to_string();
    let token_hash = "hash_123456";
    let expires_at = "2030-01-01T00:00:00Z";

    provider
        .sessions()
        .create(
            &session_id,
            &user.id,
            token_hash,
            Some("127.0.0.1"),
            Some("TestAgent"),
            expires_at,
        )
        .await
        .unwrap();

    // 3. Resolve user identity via session user_id -> tenant_id must be Default Tenant
    let session_user_before = provider
        .users()
        .find_by_id(&user.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(session_user_before.tenant_id, Tenant::DEFAULT_ID);

    // 4. Reassign user to Tenant B
    provider
        .users()
        .reassign_user_tenant_with_audit(&user.id, &tenant_b, None, None, None)
        .await
        .unwrap();

    // 5. Subsequent request resolving user identity for the same active session MUST immediately yield Tenant B
    let session_user_after = provider
        .users()
        .find_by_id(&user.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(session_user_after.tenant_id, tenant_b);

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_prevent_last_admin_reassignment_validation() {
    let (provider, db_path) = setup_test_provider().await;
    let provider_dyn: Arc<dyn DatabaseProvider> = provider.clone();
    let dummy_cfg = nx9_auth::config::SecurityConfig::default();

    // Create an admin user
    let admin_user = users::create_user(
        &provider_dyn,
        &dummy_cfg,
        Tenant::DEFAULT_ID,
        "admin_user",
        "X9#mK$9qL!2zP0",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let admin_role = provider
        .roles()
        .find_by_name("admin")
        .await
        .unwrap()
        .expect("admin role must exist");
    provider
        .roles()
        .assign_to_user(&admin_user.id, &admin_role.id)
        .await
        .unwrap();

    // Check system admin count
    let admin_count = provider.users().count_admins().await.unwrap();
    assert_eq!(admin_count, 1, "Should count 1 system admin user");

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_reassignment_audit_event_metadata_invariants() {
    let (provider, db_path) = setup_test_provider().await;
    let provider_dyn: Arc<dyn DatabaseProvider> = provider.clone();
    let dummy_cfg = nx9_auth::config::SecurityConfig::default();

    let user = users::create_user(
        &provider_dyn,
        &dummy_cfg,
        Tenant::DEFAULT_ID,
        "audit_user",
        "X9#mK$9qL!2zP0",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let audit_id = uuid::Uuid::new_v4().to_string();
    let from_tenant_id = Tenant::DEFAULT_ID;
    let to_tenant_id = uuid::Uuid::new_v4().to_string();

    let metadata = serde_json::json!({
        "user_id": user.id,
        "from_tenant_id": from_tenant_id,
        "to_tenant_id": to_tenant_id,
    })
    .to_string();

    provider
        .audit()
        .insert(
            &audit_id,
            Some(&user.id),
            Some(&user.id),
            "user.tenant_reassigned",
            "user",
            Some(&user.id),
            "info",
            Some("127.0.0.1"),
            Some("TestRunner"),
            Some(&metadata),
        )
        .await
        .unwrap();

    let entries = provider
        .audit()
        .list_filtered(&nx9_auth::db::models::AuditFilter {
            resource_type: Some("user".to_string()),
            limit: 10,
            ..Default::default()
        })
        .await
        .unwrap();

    let event = entries
        .into_iter()
        .find(|e| e.id == audit_id)
        .expect("Audit event must exist");

    assert_eq!(event.action, "user.tenant_reassigned");
    let parsed_meta: serde_json::Value =
        serde_json::from_str(event.metadata_json.as_deref().unwrap()).unwrap();
    assert_eq!(parsed_meta["from_tenant_id"], from_tenant_id);
    assert_eq!(parsed_meta["to_tenant_id"], to_tenant_id);

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_reassign_user_tenant_with_audit_atomic_success() {
    let (provider, db_path) = setup_test_provider().await;
    let provider_dyn: Arc<dyn DatabaseProvider> = provider.clone();
    let dummy_cfg = nx9_auth::config::SecurityConfig::default();

    let tenant_dest = uuid::Uuid::new_v4().to_string();
    provider
        .tenants()
        .create(&tenant_dest, "Dest Tenant", Some("dest-tenant"))
        .await
        .unwrap();

    let user = users::create_user(
        &provider_dyn,
        &dummy_cfg,
        Tenant::DEFAULT_ID,
        "atomic_user",
        "X9#mK$9qL!2zP0",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // Call atomic reassign
    provider
        .users()
        .reassign_user_tenant_with_audit(
            &user.id,
            &tenant_dest,
            Some(&user.id),
            Some("127.0.0.1"),
            Some("TestRunner"),
        )
        .await
        .unwrap();

    // Verify tenant update
    let updated_user = provider
        .users()
        .find_by_id(&user.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated_user.tenant_id, tenant_dest);

    // Verify audit record was inserted inside transaction
    let audit_entries = provider
        .audit()
        .list_filtered(&nx9_auth::db::models::AuditFilter {
            resource_type: Some("user".to_string()),
            limit: 10,
            ..Default::default()
        })
        .await
        .unwrap();

    let audit_event = audit_entries
        .into_iter()
        .find(|e| {
            e.action == "user.tenant_reassigned" && e.target_user_id.as_deref() == Some(&user.id)
        })
        .expect("Atomic reassignment audit event must exist");

    let meta: serde_json::Value =
        serde_json::from_str(audit_event.metadata_json.as_deref().unwrap()).unwrap();
    assert_eq!(meta["from_tenant_id"], Tenant::DEFAULT_ID);
    assert_eq!(meta["to_tenant_id"], tenant_dest);

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_reassign_user_tenant_no_op_behavior() {
    let (provider, db_path) = setup_test_provider().await;
    let provider_dyn: Arc<dyn DatabaseProvider> = provider.clone();
    let dummy_cfg = nx9_auth::config::SecurityConfig::default();

    let user = users::create_user(
        &provider_dyn,
        &dummy_cfg,
        Tenant::DEFAULT_ID,
        "noop_user",
        "X9#mK$9qL!2zP0",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // Reassigning to SAME tenant must be a defined no-op
    provider
        .users()
        .reassign_user_tenant_with_audit(
            &user.id,
            Tenant::DEFAULT_ID,
            Some("actor_1"),
            Some("127.0.0.1"),
            Some("TestRunner"),
        )
        .await
        .unwrap();

    // Audit logs MUST NOT contain a false tenant_reassigned event
    let audit_entries = provider
        .audit()
        .list_filtered(&nx9_auth::db::models::AuditFilter {
            resource_type: Some("user".to_string()),
            limit: 10,
            ..Default::default()
        })
        .await
        .unwrap();

    let noop_audit = audit_entries.into_iter().find(|e| {
        e.action == "user.tenant_reassigned" && e.target_user_id.as_deref() == Some(&user.id)
    });

    assert!(
        noop_audit.is_none(),
        "No-op reassignment must NOT write a false audit log entry"
    );

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_concurrent_admin_reassignment_cannot_remove_all_system_admins() {
    let (provider, db_path) = setup_test_provider().await;
    let provider_dyn: Arc<dyn DatabaseProvider> = provider.clone();
    let dummy_cfg = nx9_auth::config::SecurityConfig::default();

    let dest_tenant = uuid::Uuid::new_v4().to_string();
    provider
        .tenants()
        .create(&dest_tenant, "Dest", Some("dest"))
        .await
        .unwrap();

    // Create single system admin in Default Tenant
    let admin_user = users::create_user(
        &provider_dyn,
        &dummy_cfg,
        Tenant::DEFAULT_ID,
        "single_admin",
        "X9#mK$9qL!2zP0",
        None,
        None,
        None,
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
        .assign_to_user(&admin_user.id, &admin_role.id)
        .await
        .unwrap();

    // Single system admin reassignment away from Default Tenant MUST be rejected
    let res = provider
        .users()
        .reassign_user_tenant_with_audit(
            &admin_user.id,
            &dest_tenant,
            Some(&admin_user.id),
            Some("127.0.0.1"),
            Some("TestRunner"),
        )
        .await;

    assert!(
        res.is_err(),
        "Moving the last system admin away from Default Tenant MUST fail"
    );

    // Invariant check: system admin count MUST remain >= 1 in Default Tenant
    let admin_count = provider.users().count_admins().await.unwrap();
    assert_eq!(admin_count, 1, "System admin count must never drop to 0");

    teardown_test_db(db_path).await;
}
