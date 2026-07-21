use nx9_auth::cli::{Commands, run};
use nx9_auth::config::Config;
use std::fs;
use std::path::{Path, PathBuf};

fn setup_test_db(db_path: &str) {
    let _ = fs::remove_file(db_path);
}

fn teardown_test_db(db_path: &str) {
    let _ = fs::remove_file(db_path);
    let _ = fs::remove_file(format!("{}-wal", db_path));
    let _ = fs::remove_file(format!("{}-shm", db_path));
}

#[tokio::test]
async fn test_path_expansion() {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());

    let mut config = Config::default();
    config.database.path = "~/test_subdir/test.db".to_string();
    config.resolve_paths();

    let expected = Path::new(&home).join("test_subdir/test.db");
    assert_eq!(config.database.path, expected.to_string_lossy().to_string());
}

#[tokio::test]
async fn test_backup_validation_and_integrity() {
    let db_path = "test_cli_backup.db";
    setup_test_db(db_path);

    let mut config = Config::default();
    config.database.path = db_path.to_string();

    // 1. Initialize DB and run migrations
    let pool = nx9_auth::db::create_pool(db_path).await.unwrap();
    nx9_auth::db::run_migrations(&pool).await.unwrap();

    // 2. Validate backup safety rejects active DB
    let res = run(
        Commands::Backup {
            path: PathBuf::from(db_path),
        },
        config.clone(),
    )
    .await;
    assert!(res.is_err());
    assert!(
        res.unwrap_err()
            .to_string()
            .contains("Backup destination cannot be the active database file")
    );

    // Reject WAL file
    let wal_path = format!("{}-wal", db_path);
    let res = run(
        Commands::Backup {
            path: PathBuf::from(&wal_path),
        },
        config.clone(),
    )
    .await;
    assert!(res.is_err());
    assert!(
        res.unwrap_err()
            .to_string()
            .contains("Backup destination cannot be the active WAL file")
    );

    // Reject SHM file
    let shm_path = format!("{}-shm", db_path);
    let res = run(
        Commands::Backup {
            path: PathBuf::from(&shm_path),
        },
        config.clone(),
    )
    .await;
    assert!(res.is_err());
    assert!(
        res.unwrap_err()
            .to_string()
            .contains("Backup destination cannot be the active SHM file")
    );

    // 3. Test successful backup
    let backup_path = "test_cli_backup_dest.db";
    let _ = fs::remove_file(backup_path);

    let res = run(
        Commands::Backup {
            path: PathBuf::from(backup_path),
        },
        config.clone(),
    )
    .await;
    assert!(res.is_ok());
    assert!(Path::new(backup_path).exists());

    // 4. Verify integrity of the backup database
    let backup_pool = nx9_auth::db::create_pool(backup_path).await.unwrap();
    let integrity: (String,) = sqlx::query_as("PRAGMA integrity_check")
        .fetch_one(&backup_pool)
        .await
        .unwrap();
    assert_eq!(integrity.0, "ok");

    // Clean up
    teardown_test_db(db_path);
    teardown_test_db(backup_path);
}

#[tokio::test]
async fn test_cli_config_path_json() {
    let mut config = Config::default();
    config.database.path = "test.db".to_string();

    let res = run(Commands::ConfigPath { json: true }, config.clone()).await;
    assert!(res.is_ok());

    let res = run(Commands::ConfigPath { json: false }, config).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_cli_init_non_interactive() {
    let db_path = "test_cli_init.db";

    // Clean up
    let _ = fs::remove_file(db_path);

    let mut config = Config::default();
    config.database.path = db_path.to_string();

    // Run init command in non-interactive mode
    let res = run(
        Commands::Init {
            non_interactive: true,
            skip_admin: false,
            force: false,
            admin_user: Some("init_admin".to_string()),
            admin_password: Some("S3cur3#P@ssw0rd$N0S3qu3nc3!".to_string()),
        },
        config.clone(),
    )
    .await;

    if let Err(ref e) = res {
        println!("INIT ERROR: {:?}", e);
    }
    assert!(res.is_ok());

    // Verify DB exists
    assert!(Path::new(db_path).exists());

    // Verify admin user is created in database
    let pool = nx9_auth::db::create_pool(db_path).await.unwrap();
    let provider: std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider> =
        std::sync::Arc::new(nx9_auth::db::provider::SqliteProvider::new(pool.clone()));
    let admin_exists = provider
        .users()
        .username_exists(nx9_auth::db::models::Tenant::DEFAULT_ID, "init_admin")
        .await
        .unwrap();
    assert!(admin_exists);

    // Clean up
    teardown_test_db(db_path);
}

#[tokio::test]
async fn test_cli_init_skip_admin() {
    let db_path = "test_cli_init_skip_admin.db";

    // Clean up
    let _ = fs::remove_file(db_path);

    let mut config = Config::default();
    config.database.path = db_path.to_string();

    // Run init command with skip_admin
    let res = run(
        Commands::Init {
            non_interactive: true,
            skip_admin: true,
            force: false,
            admin_user: None,
            admin_password: None,
        },
        config.clone(),
    )
    .await;

    assert!(res.is_ok());

    // Verify DB exists
    assert!(Path::new(db_path).exists());

    // Verify no admin users exist
    let pool = nx9_auth::db::create_pool(db_path).await.unwrap();
    let provider: std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider> =
        std::sync::Arc::new(nx9_auth::db::provider::SqliteProvider::new(pool));
    let admin_count = provider.users().count_admins().await.unwrap();
    assert_eq!(admin_count, 0);

    // Clean up
    teardown_test_db(db_path);
}

#[tokio::test]
async fn test_cli_show_user_and_token() {
    let db_path = "test_cli_show.db";
    setup_test_db(db_path);

    let mut config = Config::default();
    config.database.path = db_path.to_string();

    // 1. Init DB and seed user
    let pool = nx9_auth::db::create_pool(db_path).await.unwrap();
    nx9_auth::db::run_migrations(&pool).await.unwrap();
    let provider: std::sync::Arc<dyn nx9_auth::db::provider::DatabaseProvider> =
        std::sync::Arc::new(nx9_auth::db::provider::SqliteProvider::new(pool));

    let user = nx9_auth::identity::users::create_user(
        &provider,
        &config.security,
        nx9_auth::db::models::Tenant::DEFAULT_ID,
        "show_test_user",
        "S3cur3#P@ssw0rd$N0S3qu3nc3!",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // Assign role
    nx9_auth::identity::roles::assign_role(&provider, &user.id, "viewer", None, None, None)
        .await
        .unwrap();

    // Create a token
    let (token, _raw) = nx9_auth::security::tokens::create_token(
        &provider,
        &user.id,
        "test-token",
        &config.security,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // 2. Run show-user command
    let res = run(
        Commands::ShowUser {
            id_or_username: "show_test_user".to_string(),
            permissions: true,
        },
        config.clone(),
    )
    .await;
    assert!(res.is_ok());

    let res = run(
        Commands::ShowUser {
            id_or_username: user.id.clone(),
            permissions: false,
        },
        config.clone(),
    )
    .await;
    assert!(res.is_ok());

    // 3. Run show-token command
    let res = run(
        Commands::ShowToken {
            id: token.id.clone(),
        },
        config.clone(),
    )
    .await;
    assert!(res.is_ok());

    // Clean up
    teardown_test_db(db_path);
}
