#![cfg(feature = "postgres")]

use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

#[tokio::test]
async fn test_postgres_fresh_migration_from_0001_to_latest() {
    let database_url = "postgres://postgres@127.0.0.1:5433/nx9_auth_test_fresh";

    let pool = match PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(1))
        .connect(database_url)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            println!("Skipping PostgreSQL live connection test (server not running): {e}");
            return;
        }
    };

    let migrator = sqlx::migrate!("src/db/migrations/postgres");
    let res = migrator.run(&pool).await;

    assert!(res.is_ok(), "Fresh PostgreSQL migration failed: {:?}", res);

    // Test idempotency (re-running on migrated database)
    let res_idempotent = migrator.run(&pool).await;
    assert!(
        res_idempotent.is_ok(),
        "Re-running PostgreSQL migrations failed: {:?}",
        res_idempotent
    );
}
