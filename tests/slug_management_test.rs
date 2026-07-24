#![cfg(feature = "sqlite")]

use nx9_auth::db::{
    self,
    models::Tenant,
    provider::{DatabaseProvider, SqliteProvider},
};
use nx9_auth::identity::{applications, slug};
use std::sync::Arc;

async fn setup_test_provider() -> (Arc<SqliteProvider>, String) {
    let db_id = uuid::Uuid::new_v4().to_string();
    let db_path = format!("target/test_slug_{}.db", db_id);
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
async fn test_canonical_slug_validation_and_policy() {
    // Valid slugs
    assert!(slug::validate_slug("default").is_ok());
    assert!(slug::validate_slug("my-app-1").is_ok());
    assert!(slug::validate_slug("acme-tenant").is_ok());
    assert!(slug::validate_slug("ab").is_ok());

    // Invalid length
    assert!(slug::validate_slug("a").is_err());
    let long_slug = "a".repeat(64);
    assert!(slug::validate_slug(&long_slug).is_err());

    // Invalid formatting
    assert!(slug::validate_slug("-invalid").is_err());
    assert!(slug::validate_slug("invalid-").is_err());
    assert!(slug::validate_slug("in--valid").is_err());
    assert!(slug::validate_slug("Invalid").is_err());
    assert!(slug::validate_slug("in_valid").is_err());

    // Reserved names
    assert!(slug::validate_slug("admin").is_err());
    assert!(slug::validate_slug("api").is_err());
    assert!(slug::validate_slug("system").is_err());
}

#[tokio::test]
async fn test_explicit_invalid_slug_rejection_no_silent_slugify() {
    let (provider, db_path) = setup_test_provider().await;

    // Explicit invalid slug must be rejected directly and NOT silently slugified
    let tenant_id = uuid::Uuid::new_v4().to_string();
    let res = provider
        .tenants()
        .create(&tenant_id, "My Organization", Some("INVALID SLUG!"))
        .await;

    assert!(res.is_err(), "Explicit invalid slug should be rejected");

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_omitted_slug_generation_on_create() {
    let (provider, db_path) = setup_test_provider().await;

    let tenant_id = uuid::Uuid::new_v4().to_string();
    let tenant = provider
        .tenants()
        .create(&tenant_id, "Acme Corporation Inc!", None)
        .await
        .expect("Should derive slug from name when omitted");

    assert_eq!(tenant.slug.as_deref(), Some("acme-corporation-inc"));

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_same_resource_duplicate_rejection() {
    let (provider, db_path) = setup_test_provider().await;

    let id1 = uuid::Uuid::new_v4().to_string();
    provider
        .tenants()
        .create(&id1, "Tenant One", Some("tenant-one"))
        .await
        .expect("First tenant creation should succeed");

    let id2 = uuid::Uuid::new_v4().to_string();
    let res = provider
        .tenants()
        .create(&id2, "Tenant Two", Some("tenant-one"))
        .await;

    assert!(res.is_err(), "Duplicate tenant slug must be rejected");

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_cross_resource_collision_rejection() {
    let (provider, db_path) = setup_test_provider().await;
    let provider_dyn: Arc<dyn nx9_auth::db::provider::DatabaseProvider> = provider.clone();

    // Create a tenant with slug "shared-identifier"
    let t_id = uuid::Uuid::new_v4().to_string();
    provider
        .tenants()
        .create(&t_id, "Shared Tenant", Some("shared-identifier"))
        .await
        .expect("Tenant creation should succeed");

    // Attempting to create an application with the SAME slug "shared-identifier" must fail
    let app_res = applications::create(
        &provider_dyn,
        Tenant::DEFAULT_ID,
        "Colliding Application",
        "shared-identifier",
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await;

    assert!(
        app_res.is_err(),
        "Cross-resource slug collision (app vs tenant) must be rejected"
    );

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_unchanged_slug_update_no_op() {
    let (provider, db_path) = setup_test_provider().await;

    let id = uuid::Uuid::new_v4().to_string();
    provider
        .tenants()
        .create(&id, "Original Name", Some("stable-slug"))
        .await
        .expect("Tenant creation should succeed");

    // Updating tenant name while keeping the exact same slug must succeed (no-op for registry)
    let update_res = provider
        .tenants()
        .update(&id, "Updated Name", Some("stable-slug"))
        .await;

    assert!(
        update_res.is_ok(),
        "Unchanged slug update should succeed as no-op"
    );

    let updated = provider.tenants().find_by_id(&id).await.unwrap().unwrap();
    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.slug.as_deref(), Some("stable-slug"));

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_rename_and_old_slug_release() {
    let (provider, db_path) = setup_test_provider().await;

    let id = uuid::Uuid::new_v4().to_string();
    provider
        .tenants()
        .create(&id, "Alpha Tenant", Some("old-alpha-slug"))
        .await
        .expect("Tenant creation should succeed");

    // Rename to new-alpha-slug
    provider
        .tenants()
        .update(&id, "Alpha Tenant", Some("new-alpha-slug"))
        .await
        .expect("Rename should succeed");

    // Verify old-alpha-slug is released and can be claimed by another resource
    let id2 = uuid::Uuid::new_v4().to_string();
    let claim_res = provider
        .tenants()
        .create(&id2, "Beta Tenant", Some("old-alpha-slug"))
        .await;

    assert!(
        claim_res.is_ok(),
        "Released old slug should be claimable by new resource"
    );

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_slug_release_ownership_verification() {
    let (provider, db_path) = setup_test_provider().await;

    let id1 = uuid::Uuid::new_v4().to_string();
    provider
        .tenants()
        .create(&id1, "Tenant One", Some("tenant-slug-1"))
        .await
        .expect("Tenant 1 creation should succeed");

    // Attempting to release tenant-slug-1 using a wrong entity_id (id2) directly via sqlite helper
    let mut tx = provider.pool.begin().await.unwrap();
    let rows = db::repository::sqlite::global_slugs::release_slug_by_name_sqlite(
        &mut tx,
        "tenant-slug-1",
        "tenant",
        "wrong-entity-id",
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(
        rows, 0,
        "Release with wrong entity_id ownership must affect 0 rows"
    );

    // Verify tenant-slug-1 is still registered in global_slugs
    let existing_slug = provider
        .global_slugs()
        .find_by_slug("tenant-slug-1")
        .await
        .unwrap();
    assert!(
        existing_slug.is_some(),
        "Registration must remain intact when release ownership fails"
    );

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_deletion_slug_release() {
    let (provider, db_path) = setup_test_provider().await;

    let id = uuid::Uuid::new_v4().to_string();
    provider
        .tenants()
        .create(&id, "Temporary Tenant", Some("temp-tenant-slug"))
        .await
        .expect("Tenant creation should succeed");

    // Delete tenant
    provider
        .tenants()
        .delete(&id)
        .await
        .expect("Tenant deletion should succeed");

    // Verify temp-tenant-slug is released from global_slugs
    let found = provider
        .global_slugs()
        .find_by_slug("temp-tenant-slug")
        .await
        .unwrap();
    assert!(
        found.is_none(),
        "Slug must be removed from global_slugs after deletion"
    );

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_application_slug_never_authenticates_as_client_id() {
    let (provider, db_path) = setup_test_provider().await;
    let provider_dyn: Arc<dyn nx9_auth::db::provider::DatabaseProvider> = provider.clone();

    let (app, raw_secret) = applications::create(
        &provider_dyn,
        Tenant::DEFAULT_ID,
        "Auth App",
        "auth-app-slug",
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .expect("App creation should succeed");

    // Authenticating using canonical client_id must succeed
    let auth_ok =
        applications::validate_client_credentials(&provider_dyn, app.get_client_id(), &raw_secret)
            .await;
    assert!(
        auth_ok.is_ok(),
        "Authentication with client_id must succeed"
    );

    // Authenticating using application SLUG as client_id MUST FAIL
    let auth_slug_fail =
        applications::validate_client_credentials(&provider_dyn, "auth-app-slug", &raw_secret)
            .await;
    assert!(
        auth_slug_fail.is_err(),
        "Application slug MUST NEVER authenticate as client_id"
    );

    teardown_test_db(db_path).await;
}

#[tokio::test]
async fn test_dual_migration_paths_sqlite() {
    let db_id = uuid::Uuid::new_v4().to_string();
    let db_path = format!("target/test_migration_path_{}.db", db_id);
    let pool = db::create_pool(&db_path).await.unwrap();

    // Fresh migration
    let res = db::run_migrations(&pool).await;
    assert!(res.is_ok(), "Fresh migration must succeed");

    // Idempotent re-run
    let res2 = db::run_migrations(&pool).await;
    assert!(res2.is_ok(), "Re-running migrations must succeed");

    let _ = std::fs::remove_file(db_path);
}
