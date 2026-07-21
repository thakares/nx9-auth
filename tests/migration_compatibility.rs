use nx9_auth::db::{self, models::Tenant};

async fn setup_test_db() -> (sqlx::SqlitePool, String) {
    let db_id = uuid::Uuid::new_v4().to_string();
    let db_path = format!("target/migration_{}.db", db_id);
    let pool = db::create_pool(&db_path)
        .await
        .expect("Failed to create test pool");
    (pool, db_path)
}

async fn teardown_test_db(path: String) {
    let _ = std::fs::remove_file(path);
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 1: Fresh Database -> Migrate -> Success
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_migration_scenario_1_fresh() {
    let (pool, db_path) = setup_test_db().await;

    // Run all migrations
    let res = db::run_migrations(&pool).await;
    assert!(res.is_ok(), "Fresh migration failed: {:?}", res);

    // Verify default tables exist
    for table in &[
        "tenants",
        "users",
        "roles",
        "permissions",
        "sessions",
        "audit_logs",
        "_sqlx_migrations",
    ] {
        let exists: Option<(String,)> =
            sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' AND name=?")
                .bind(table)
                .fetch_optional(&pool)
                .await
                .unwrap();
        assert!(exists.is_some(), "Table '{}' was not created", table);
    }

    // Verify default tenant and admin/viewer roles exist
    let tenant_exists = sqlx::query("SELECT 1 FROM tenants WHERE id = ?")
        .bind(Tenant::DEFAULT_ID)
        .fetch_optional(&pool)
        .await
        .unwrap()
        .is_some();
    assert!(tenant_exists);

    let provider: std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider> =
        std::sync::Arc::new(nx9_auth::db::provider::SqliteProvider::new(pool.clone()));
    let admin_role = provider.roles().find_by_name("admin").await.unwrap();
    assert!(admin_role.is_some());

    let viewer_role = provider.roles().find_by_name("viewer").await.unwrap();
    assert!(viewer_role.is_some());

    teardown_test_db(db_path).await;
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 2: Database at migration N -> Migrate to N+1 -> Success
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_migration_scenario_2_incremental() {
    let (pool, db_path) = setup_test_db().await;

    let migrator = sqlx::migrate!("src/db/migrations/sqlite");
    let all_migrations = &migrator.migrations;
    assert!(
        all_migrations.len() >= 3,
        "Expected at least 3 migrations to test incremental scenario"
    );

    // 1. Manually create the _sqlx_migrations table
    sqlx::query(
        r#"
        CREATE TABLE _sqlx_migrations (
            version INTEGER PRIMARY KEY,
            description TEXT NOT NULL,
            installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            success BOOLEAN NOT NULL,
            checksum BLOB NOT NULL,
            execution_time INTEGER NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    // 2. Manually apply the first 2 migrations (N = 2)
    for migration in all_migrations.iter().take(2) {
        let sql: &'static str = Box::leak(migration.sql.as_ref().to_string().into_boxed_str());
        // Run SQL query directly
        sqlx::query(sql).execute(&pool).await.unwrap();

        // Record it in _sqlx_migrations so SQLx knows it is applied
        sqlx::query(
            r#"
            INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
            VALUES (?, ?, 1, ?, 0)
            "#,
        )
        .bind(migration.version)
        .bind(migration.description.as_ref())
        .bind(migration.checksum.as_ref())
        .execute(&pool)
        .await
        .unwrap();
    }

    // 3. Now run the SQLx Migrator to migrate to N+1 (and all remaining ones)
    let res = migrator.run(&pool).await;
    assert!(res.is_ok(), "Incremental migration failed: {:?}", res);

    // Verify all tables are successfully created
    for table in &["tenants", "users", "roles", "permissions"] {
        let exists: Option<(String,)> =
            sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' AND name=?")
                .bind(table)
                .fetch_optional(&pool)
                .await
                .unwrap();
        assert!(
            exists.is_some(),
            "Table '{}' was not created incrementally",
            table
        );
    }

    teardown_test_db(db_path).await;
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 3: Run Migrations Twice -> Idempotent
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_migration_scenario_3_idempotence() {
    let (pool, db_path) = setup_test_db().await;

    // First run
    let res1 = db::run_migrations(&pool).await;
    assert!(res1.is_ok());

    // Second run
    let res2 = db::run_migrations(&pool).await;
    assert!(
        res2.is_ok(),
        "Second migration run failed (idempotency issue): {:?}",
        res2
    );

    // Verify default tenant and roles count didn't duplicate
    let tenant_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tenants WHERE id = ?")
        .bind(Tenant::DEFAULT_ID)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(tenant_count.0, 1, "Default tenant was duplicated!");

    let admin_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM roles WHERE name = 'admin'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(admin_count.0, 1, "Admin role was duplicated!");

    let viewer_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM roles WHERE name = 'viewer'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(viewer_count.0, 1, "Viewer role was duplicated!");

    teardown_test_db(db_path).await;
}
