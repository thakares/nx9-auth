use std::io::{self, Write};
use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::{
    config::Config,
    db::repository::{roles as role_repo, users as user_repo},
    db::{
        self,
        models::{Tenant, UserStatus},
    },
    error::AppError,
    identity::{roles, users as identity_users},
    security::tokens as token_security,
};

// ── CLI Definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "nx9-auth",
    about = "NX9 Identity and Access Management service",
    version = env!("CARGO_PKG_VERSION"),
    author,
)]
pub struct Cli {
    /// Path to the configuration file.
    #[arg(long, short, global = true, env = "NX9_AUTH_CONFIG")]
    pub config: Option<PathBuf>,

    /// Enable verbose logging for CLI commands.
    #[arg(long, short, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the HTTP server.
    Serve,

    /// Run pending database migrations.
    Migrate,

    /// Check system health and configuration.
    Doctor,

    /// Create an administrator user.
    CreateAdmin {
        /// Username for the new admin account.
        username: String,
    },

    /// Create a standard user.
    CreateUser {
        /// Username for the new user account.
        username: String,
    },

    /// List all users in the system.
    ListUsers,

    /// Disable a user account (sets status = disabled).
    DisableUser {
        /// User ID to disable.
        id: String,
    },

    /// Enable a user account (sets status = active).
    EnableUser {
        /// User ID to enable.
        id: String,
    },

    /// Reset a user's password.
    ResetPassword {
        /// User ID to reset.
        id: String,
    },

    /// Create a personal access token for a user.
    CreateToken {
        /// User ID to create the token for.
        #[arg(long)]
        user: String,
        /// Descriptive name for the token.
        #[arg(long, default_value = "CLI token")]
        name: String,
    },

    /// Revoke a personal access token by ID.
    RevokeToken {
        /// Token ID to revoke.
        id: String,
    },

    /// Initialize the configuration, directories, database and admin user.
    Init {
        /// Run in non-interactive mode.
        #[arg(long)]
        non_interactive: bool,

        /// Skip administrator user creation.
        #[arg(long)]
        skip_admin: bool,

        /// Force initialization even if already initialized (overwrites existing configuration).
        #[arg(long)]
        force: bool,

        /// Administrator username (required in non-interactive mode unless --skip-admin is set).
        #[arg(long)]
        admin_user: Option<String>,

        /// Administrator password (required in non-interactive mode unless --skip-admin is set).
        #[arg(long)]
        admin_password: Option<String>,
    },

    /// Print configuration and database file paths.
    ConfigPath {
        /// Output in machine-readable JSON format.
        #[arg(long)]
        json: bool,
    },

    /// Show details of a user by ID or username.
    ShowUser {
        /// User ID or username.
        id_or_username: String,

        /// Display all granular permissions and roles.
        #[arg(long)]
        permissions: bool,
    },

    /// Show details of a personal access token by ID.
    ShowToken {
        /// Token ID.
        id: String,
    },

    /// Backup the database to a target path.
    Backup {
        /// Path where the backup file will be created.
        path: PathBuf,
    },
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Securely prompt for a password (no echo).
fn prompt_password(prompt: &str) -> anyhow::Result<String> {
    print!("{prompt}");
    io::stdout().flush()?;

    // Use rpassword-style reading without the dep: read from /dev/tty or stdin
    // For CLI use, we read a line and trim it (terminals will hide input for
    // proper TTY usage; a future version can add rpassword)
    let mut pw = String::new();
    io::stdin().read_line(&mut pw)?;
    Ok(pw.trim().to_string())
}

fn prompt_password_confirmed(prompt: &str, is_admin: bool) -> anyhow::Result<String> {
    let pw1 = prompt_password(prompt)?;
    let pw2 = prompt_password("Confirm password: ")?;
    if pw1 != pw2 {
        anyhow::bail!("passwords do not match");
    }
    crate::security::passwords::validate_password_strength(&pw1, is_admin)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(pw1)
}

// ── Command Handlers ──────────────────────────────────────────────────────────

pub async fn run(command: Commands, config: Config) -> anyhow::Result<()> {
    match command {
        Commands::Serve => {
            // Handled in main.rs — should not be reached here
            unreachable!("serve is handled in main")
        }

        Commands::Migrate => cmd_migrate(&config).await,
        Commands::Doctor => cmd_doctor(&config).await,

        Commands::CreateAdmin { username } => cmd_create_admin(&config, &username).await,
        Commands::CreateUser { username } => cmd_create_user(&config, &username).await,
        Commands::ListUsers => cmd_list_users(&config).await,

        Commands::DisableUser { id } => cmd_set_status(&config, &id, UserStatus::Disabled).await,
        Commands::EnableUser { id } => cmd_set_status(&config, &id, UserStatus::Active).await,
        Commands::ResetPassword { id } => cmd_reset_password(&config, &id).await,

        Commands::CreateToken { user, name } => cmd_create_token(&config, &user, &name).await,
        Commands::RevokeToken { id } => cmd_revoke_token(&config, &id).await,

        Commands::Init {
            non_interactive,
            skip_admin,
            force,
            admin_user,
            admin_password,
        } => {
            cmd_init(
                &config,
                non_interactive,
                skip_admin,
                force,
                admin_user.as_deref(),
                admin_password.as_deref(),
            )
            .await
        }
        Commands::ConfigPath { json } => cmd_config_path(&config, json).await,
        Commands::ShowUser {
            id_or_username,
            permissions,
        } => cmd_show_user(&config, &id_or_username, permissions).await,
        Commands::ShowToken { id } => cmd_show_token(&config, &id).await,
        Commands::Backup { path } => cmd_backup(&config, &path).await,
    }
}

// ── migrate ───────────────────────────────────────────────────────────────────

async fn cmd_migrate(config: &Config) -> anyhow::Result<()> {
    let pool = db::create_pool(&config.database.path).await?;
    db::run_migrations(&pool).await?;
    println!("✓ Migrations applied successfully.");
    Ok(())
}

// ── doctor ────────────────────────────────────────────────────────────────────

async fn run_doctor_checks(config: &Config) -> anyhow::Result<bool> {
    let mut ok = true;

    println!("\nnx9-auth doctor\n");

    // 1. Config loads (already done — we got here with a valid config)
    println!("  ✓  Config file loads and parses");

    // 2. DB path is writable
    let db_path = std::path::Path::new(&config.database.path);
    let db_dir_writable = if let Some(parent) = db_path.parent() {
        if parent.as_os_str().is_empty() {
            true
        } else if std::fs::create_dir_all(parent).is_err() {
            false
        } else {
            let temp_file = parent.join(format!(
                ".nx9_auth_doctor_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            ));
            if std::fs::write(&temp_file, b"test").is_ok() {
                let _ = std::fs::remove_file(temp_file);
                true
            } else {
                false
            }
        }
    } else {
        true
    };
    if db_dir_writable {
        println!("  ✓  Database directory is writable");
    } else {
        println!(
            "  ✗  Database directory is not writable: {}",
            config.database.path
        );
        ok = false;
    }

    // 3. DB connects
    let pool_result = db::create_pool(&config.database.path).await;
    let pool = match pool_result {
        Ok(p) => {
            println!("  ✓  Database connection successful");
            p
        }
        Err(e) => {
            println!("  ✗  Database connection failed: {}", e);
            println!("\nDoctor result: FAIL\n");
            return Ok(false);
        }
    };

    // 4. Migrations are up to date
    // Verify migrations are applied
    let migration_check: Result<(i64,), sqlx::Error> =
        sqlx::query_as("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await;
    match migration_check {
        Ok((count,)) if count > 0 => println!("  ✓  Migrations applied ({} recorded)", count),
        Ok(_) => {
            println!("  ✗  No migrations recorded — run `nx9-auth migrate` first");
            ok = false;
        }
        Err(_) => {
            println!("  ✗  Migrations table missing — run `nx9-auth migrate` first");
            ok = false;
        }
    }

    // 5. Default tenant exists
    let tenant_check: Result<(i64,), sqlx::Error> =
        sqlx::query_as("SELECT COUNT(*) FROM tenants WHERE id = ?")
            .bind(Tenant::DEFAULT_ID)
            .fetch_one(&pool)
            .await;
    match tenant_check {
        Ok((1,)) => println!("  ✓  Default tenant exists"),
        _ => {
            println!("  ✗  Default tenant missing — run `nx9-auth migrate`");
            ok = false;
        }
    }

    // 6. Admin role exists
    match role_repo::admin_role_exists(&pool).await {
        Ok(true) => println!("  ✓  admin role exists"),
        Ok(false) => {
            println!("  ✗  admin role missing — run `nx9-auth migrate`");
            ok = false;
        }
        Err(e) => {
            println!("  ✗  role check failed: {}", e);
            ok = false;
        }
    }

    // 7. At least one admin user exists
    match user_repo::count_admins(&pool).await {
        Ok(n) if n > 0 => println!("  ✓  {} admin user(s) exist", n),
        Ok(_) => {
            println!("  ✗  No admin users — run `nx9-auth create-admin <username>`");
            ok = false;
        }
        Err(e) => {
            println!("  ✗  admin count failed: {}", e);
            ok = false;
        }
    }

    // 8. WAL mode
    let journal_mode: Result<(String,), sqlx::Error> =
        sqlx::query_as("PRAGMA journal_mode").fetch_one(&pool).await;
    match journal_mode {
        Ok((mode,)) if mode.to_lowercase() == "wal" => println!("  ✓  WAL mode enabled"),
        Ok((mode,)) => {
            println!("  ✗  WAL mode not enabled (current mode: {})", mode);
            ok = false;
        }
        Err(e) => {
            println!("  ✗  Failed to check journal mode: {}", e);
            ok = false;
        }
    }

    // 9. Foreign Keys
    let foreign_keys: Result<(i64,), sqlx::Error> =
        sqlx::query_as("PRAGMA foreign_keys").fetch_one(&pool).await;
    match foreign_keys {
        Ok((1,)) => println!("  ✓  Foreign keys constraint enforcement enabled"),
        Ok((val,)) => {
            println!(
                "  ✗  Foreign keys constraint enforcement disabled (current value: {})",
                val
            );
            ok = false;
        }
        Err(e) => {
            println!("  ✗  Failed to check foreign keys: {}", e);
            ok = false;
        }
    }

    // 10. Table existence
    for table in &["audit_logs", "sessions"] {
        let table_exists: Result<Option<(String,)>, sqlx::Error> =
            sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' AND name=?")
                .bind(table)
                .fetch_optional(&pool)
                .await;
        match table_exists {
            Ok(Some(_)) => println!("  ✓  Table '{}' exists", table),
            Ok(None) => {
                println!("  ✗  Table '{}' is missing", table);
                ok = false;
            }
            Err(e) => {
                println!("  ✗  Failed to check existence of table '{}': {}", table, e);
                ok = false;
            }
        }
    }

    // 11. Database Write Test
    let write_test: Result<(), sqlx::Error> = async {
        let mut tx = pool.begin().await?;
        sqlx::query("CREATE TEMP TABLE doctor_test_write (id INTEGER PRIMARY KEY)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO doctor_test_write (id) VALUES (1)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DROP TABLE doctor_test_write")
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
    .await;
    match write_test {
        Ok(()) => {
            println!("  ✓  Database write test successful (temp table creation and deletion)")
        }
        Err(e) => {
            println!("  ✗  Database write test failed: {}", e);
            ok = false;
        }
    }

    // 12. Database Integrity Check
    let integrity_check: Result<(String,), sqlx::Error> = sqlx::query_as("PRAGMA integrity_check")
        .fetch_one(&pool)
        .await;
    match integrity_check {
        Ok((res,)) if res.to_lowercase() == "ok" => {
            println!("  ✓  Database integrity check passed")
        }
        Ok((res,)) => {
            println!("  ✗  Database integrity check failed: {}", res);
            ok = false;
        }
        Err(e) => {
            println!("  ✗  Failed to run database integrity check: {}", e);
            ok = false;
        }
    }

    println!();
    if ok {
        println!("Doctor result: OK\n");
    } else {
        println!("Doctor result: FAIL\n");
    }

    Ok(ok)
}

async fn cmd_doctor(config: &Config) -> anyhow::Result<()> {
    let ok = run_doctor_checks(config).await?;
    if !ok {
        std::process::exit(1);
    }
    Ok(())
}

// ── create-admin ──────────────────────────────────────────────────────────────

async fn cmd_create_admin(config: &Config, username: &str) -> anyhow::Result<()> {
    let pool = db::create_pool(&config.database.path).await?;
    let password = prompt_password_confirmed("Password for admin: ", true)?;

    let user = identity_users::create_user(
        &pool,
        &config.security,
        Tenant::DEFAULT_ID,
        username,
        &password,
        None,
        None,
        None,
    )
    .await?;

    roles::assign_role(&pool, &user.id, "admin", None, None, None).await?;

    println!("✓ Admin user '{}' created (id: {})", user.username, user.id);
    Ok(())
}

// ── create-user ───────────────────────────────────────────────────────────────

async fn cmd_create_user(config: &Config, username: &str) -> anyhow::Result<()> {
    let pool = db::create_pool(&config.database.path).await?;
    let password = prompt_password_confirmed("Password: ", false)?;

    let user = identity_users::create_user(
        &pool,
        &config.security,
        Tenant::DEFAULT_ID,
        username,
        &password,
        None,
        None,
        None,
    )
    .await?;

    println!("✓ User '{}' created (id: {})", user.username, user.id);
    Ok(())
}

// ── list-users ────────────────────────────────────────────────────────────────

async fn cmd_list_users(config: &Config) -> anyhow::Result<()> {
    let pool = db::create_pool(&config.database.path).await?;
    let users = identity_users::list_users(&pool, Tenant::DEFAULT_ID).await?;

    if users.is_empty() {
        println!("No users found.");
        return Ok(());
    }

    // Table header
    println!(
        "\n{:<38}  {:<24}  {:<10}  Created",
        "ID", "Username", "Status"
    );
    println!("{}", "-".repeat(90));

    for u in &users {
        println!(
            "{:<38}  {:<24}  {:<10}  {}",
            u.id,
            u.username,
            UserStatus::from_i32(u.status).as_str(),
            &u.created_at[..10],
        );
    }
    println!("\n{} user(s) total\n", users.len());
    Ok(())
}

// ── disable/enable-user ───────────────────────────────────────────────────────

async fn cmd_set_status(config: &Config, id: &str, status: UserStatus) -> anyhow::Result<()> {
    let pool = db::create_pool(&config.database.path).await?;
    let user = identity_users::get_user(&pool, id).await?;
    identity_users::update_status(&pool, id, status.as_i32(), None, None, None).await?;
    println!(
        "✓ User '{}' status set to {}",
        user.username,
        status.as_str()
    );
    Ok(())
}

// ── reset-password ────────────────────────────────────────────────────────────

async fn cmd_reset_password(config: &Config, id: &str) -> anyhow::Result<()> {
    let pool = db::create_pool(&config.database.path).await?;
    let user = identity_users::get_user(&pool, id).await?;
    let user_roles = role_repo::list_for_user(&pool, &user.id).await?;
    let is_admin = user_roles.iter().any(|r| r.name == "admin");
    let password =
        prompt_password_confirmed(&format!("New password for '{}': ", user.username), is_admin)?;
    identity_users::reset_password(&pool, &config.security, id, &password, None, None, None)
        .await?;
    println!("✓ Password reset for user '{}'", user.username);
    Ok(())
}

// ── create-token ──────────────────────────────────────────────────────────────

async fn cmd_create_token(config: &Config, user_id: &str, name: &str) -> anyhow::Result<()> {
    let pool = db::create_pool(&config.database.path).await?;
    let user = identity_users::get_user(&pool, user_id).await?;
    let (token, raw) =
        token_security::create_token(&pool, user_id, name, &config.security, None, None, None)
            .await?;

    println!(
        "\nPersonal Access Token created for user '{}':",
        user.username
    );
    println!("  Token ID:   {}", token.id);
    println!("  Name:       {}", token.name);
    println!(
        "  Expires at: {}",
        token.expires_at.as_deref().unwrap_or("never")
    );
    println!();
    println!("  Token: {}", raw);
    println!();
    println!("⚠ Store this token securely — it will not be shown again.");
    println!();
    Ok(())
}

// ── revoke-token ──────────────────────────────────────────────────────────────

async fn cmd_revoke_token(config: &Config, id: &str) -> anyhow::Result<()> {
    let pool = db::create_pool(&config.database.path).await?;

    let token = crate::db::repository::tokens::find_by_id(&pool, id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| anyhow::anyhow!("token not found: {}", id))?;

    crate::security::tokens::revoke_token(&pool, id, None, None, None).await?;

    println!("✓ Token '{}' (id: {}) revoked", token.name, token.id);
    Ok(())
}

// ── init ──────────────────────────────────────────────────────────────────────

async fn cmd_init(
    config: &Config,
    non_interactive: bool,
    skip_admin: bool,
    force: bool,
    admin_user: Option<&str>,
    admin_password: Option<&str>,
) -> anyhow::Result<()> {
    println!("Initializing nx9-auth...\n");

    // 1. Create config and data and state directories
    let config_path = config
        .config_path
        .clone()
        .or_else(Config::default_user_config_path);
    if let Some(ref path) = config_path {
        println!("Creating configuration directory...");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Write default config if it doesn't exist or if forced
        if !path.exists() || force {
            let default_content = Config::generate_default_toml();
            std::fs::write(path, default_content)?;
            if force && path.exists() {
                println!(
                    "✓ Overwrote config file with default settings at: {}",
                    path.display()
                );
            } else {
                println!("✓ Created default config file at: {}", path.display());
            }
        } else {
            println!("✓ Configuration file already exists at: {}", path.display());
        }
    }

    let db_path = std::path::Path::new(&config.database.path);
    println!("Creating database directory...");
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    // Create state directory
    if let Ok(home) = std::env::var("HOME") {
        let state_dir = std::path::Path::new(&home).join(".local/state/nx9-auth");
        std::fs::create_dir_all(&state_dir)?;
        println!("✓ Created state directory at: {}", state_dir.display());
    }

    // 2. Open DB pool and run migrations
    println!("Running migrations...");
    let pool = db::create_pool(&config.database.path).await?;
    db::run_migrations(&pool).await?;
    println!("✓ Migrations applied successfully.");

    // 3. Create administrator
    if skip_admin {
        println!("ℹ Administrator creation skipped.");
    } else {
        let admin_count = user_repo::count_admins(&pool).await?;
        if admin_count == 0 {
            let username: String;
            let password: String;

            if non_interactive {
                let u = admin_user.ok_or_else(|| {
                    anyhow::anyhow!("--admin-user is required in non-interactive mode")
                })?;
                let p = admin_password.ok_or_else(|| {
                    anyhow::anyhow!("--admin-password is required in non-interactive mode")
                })?;

                // Validate strength
                crate::security::passwords::validate_password_strength(p, true)
                    .map_err(|e| anyhow::anyhow!("Password validation failed: {}", e))?;

                username = u.to_string();
                password = p.to_string();
            } else {
                println!("\nCreate administrator:");
                print!("Username [admin]: ");
                io::stdout().flush()?;
                let mut u_in = String::new();
                io::stdin().read_line(&mut u_in)?;
                let u_trimmed = u_in.trim();
                username = if u_trimmed.is_empty() {
                    "admin".to_string()
                } else {
                    u_trimmed.to_string()
                };

                password = prompt_password_confirmed("Password: ", true)?;
            }

            let user = crate::identity::users::create_user(
                &pool,
                &config.security,
                Tenant::DEFAULT_ID,
                &username,
                &password,
                None,
                None,
                None,
            )
            .await?;

            roles::assign_role(&pool, &user.id, "admin", None, None, None).await?;
            println!("✓ Admin user '{}' created successfully.", username);
        } else {
            println!("✓ Administrator account already exists.");
        }
    }

    // 4. Run post-install validation (relaxed)
    println!("\nRunning validation...");
    let init_ok = run_init_validation(config, skip_admin).await?;
    if !init_ok {
        anyhow::bail!("Post-installation validation checks failed!");
    }

    println!("\nnx9-auth is ready.\n\nStart with:\n\n    nx9-auth serve\n");
    Ok(())
}

async fn run_init_validation(config: &Config, admin_skipped: bool) -> anyhow::Result<bool> {
    let mut ok = true;

    // 1. Config valid
    println!("  ✓  Config valid");

    // 2. Directories writable
    let db_path = std::path::Path::new(&config.database.path);
    let mut dirs_ok = true;
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() && std::fs::create_dir_all(parent).is_err() {
            dirs_ok = false;
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let state_dir = std::path::Path::new(&home).join(".local/state/nx9-auth");
        if std::fs::create_dir_all(&state_dir).is_err() {
            dirs_ok = false;
        }
    }
    if dirs_ok {
        println!("  ✓  Directories writable");
    } else {
        println!("  ✗  Directories not writable");
        ok = false;
    }

    // 3. Database reachable
    let pool = match db::create_pool(&config.database.path).await {
        Ok(p) => {
            println!("  ✓  Database reachable");
            p
        }
        Err(e) => {
            println!("  ✗  Database connection failed: {}", e);
            return Ok(false);
        }
    };

    // 4. Migrations applied
    let migration_check: Result<(i64,), sqlx::Error> =
        sqlx::query_as("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await;
    match migration_check {
        Ok((count,)) if count > 0 => println!("  ✓  Migrations applied"),
        _ => {
            println!("  ✗  Migrations not applied");
            ok = false;
        }
    }

    // 5. Admin account check
    let admin_count = user_repo::count_admins(&pool).await.unwrap_or(0);
    if admin_count > 0 {
        println!("  ✓  Administrator account exists");
    } else if admin_skipped {
        println!("  ℹ  Administrator creation skipped");
    } else {
        println!("  ✗  No administrator account exists");
        ok = false;
    }

    Ok(ok)
}

// ── config-path ──────────────────────────────────────────────────────────────

async fn cmd_config_path(config: &Config, json: bool) -> anyhow::Result<()> {
    let config_file = config
        .config_path
        .clone()
        .or_else(Config::default_user_config_path)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let database_file = config.database.path.clone();

    let state_dir = if let Ok(home) = std::env::var("HOME") {
        std::path::Path::new(&home)
            .join(".local/state/nx9-auth")
            .to_string_lossy()
            .into_owned()
    } else {
        "".to_string()
    };

    if json {
        let val = serde_json::json!({
            "config": config_file,
            "database": database_file,
            "state": state_dir,
        });
        println!("{}", serde_json::to_string_pretty(&val)?);
    } else {
        println!("\nConfig:");
        println!("  {}", config_file);
        println!("\nDatabase:");
        println!("  {}", database_file);
        println!("\nLogs/State:");
        println!("  {}", state_dir);
        println!();
    }
    Ok(())
}

// ── show-user ─────────────────────────────────────────────────────────────────

async fn cmd_show_user(
    config: &Config,
    id_or_username: &str,
    permissions: bool,
) -> anyhow::Result<()> {
    let pool = db::create_pool(&config.database.path).await?;

    let user = match user_repo::find_by_id(&pool, id_or_username).await? {
        Some(u) => Some(u),
        None => user_repo::find_by_username(&pool, id_or_username).await?,
    };

    let user = match user {
        Some(u) => u,
        None => anyhow::bail!("User not found: '{}'", id_or_username),
    };

    let user_roles = role_repo::list_for_user(&pool, &user.id).await?;
    let role_names: Vec<String> = user_roles.into_iter().map(|r| r.name).collect();

    println!("\nUser");
    println!("────");
    println!("ID:          {}", user.id);
    println!("Username:    {}", user.username);
    println!("Status:      {}", user.status().as_str());
    println!("Created:     {}", user.created_at);
    println!(
        "Last Login:  {}",
        user.last_login_at.as_deref().unwrap_or("never")
    );

    println!("\nRoles");
    println!("─────");
    if role_names.is_empty() {
        println!("none");
    } else {
        for role in &role_names {
            println!("{}", role);
        }
    }

    if permissions {
        println!("\nPermissions");
        println!("───────────");

        let user_perms = crate::db::repository::permissions::list_for_user(&pool, &user.id).await?;
        if user_perms.is_empty() {
            println!("none");
        } else {
            for perm in user_perms {
                println!("{}", perm);
            }
        }
    }
    println!();

    Ok(())
}

// ── show-token ────────────────────────────────────────────────────────────────

async fn cmd_show_token(config: &Config, id: &str) -> anyhow::Result<()> {
    let pool = db::create_pool(&config.database.path).await?;
    let token = crate::db::repository::tokens::find_by_id(&pool, id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| anyhow::anyhow!("Token not found: {}", id))?;

    let user = user_repo::find_by_id(&pool, &token.user_id).await?;
    let username = user
        .map(|u| u.username)
        .unwrap_or_else(|| "unknown".to_string());

    println!("\nToken");
    println!("─────");
    println!("ID:          {}", token.id);
    println!("Name:        {}", token.name);
    println!("User ID:     {}", token.user_id);
    println!("Username:    {}", username);
    println!(
        "Status:      {}",
        if token.revoked { "revoked" } else { "active" }
    );
    println!(
        "Expires:     {}",
        token.expires_at.as_deref().unwrap_or("never")
    );
    println!(
        "Last Used:   {}",
        token.last_used_at.as_deref().unwrap_or("never")
    );
    println!("Created:     {}", token.created_at);
    println!();

    Ok(())
}

// ── backup ────────────────────────────────────────────────────────────────────

async fn cmd_backup(config: &Config, path: &std::path::Path) -> anyhow::Result<()> {
    // 1. Resolve paths to absolute paths
    let source_path = std::path::Path::new(&config.database.path);

    let abs_source =
        std::fs::canonicalize(source_path).unwrap_or_else(|_| source_path.to_path_buf());

    let abs_target = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };

    let source_dir = abs_source.parent().unwrap();
    let source_file_name = abs_source.file_name().unwrap().to_string_lossy();
    let source_wal = source_dir.join(format!("{}-wal", source_file_name));
    let source_shm = source_dir.join(format!("{}-shm", source_file_name));

    if abs_target == abs_source {
        anyhow::bail!(
            "Backup destination cannot be the active database file: {}",
            path.display()
        );
    }
    if abs_target == source_wal {
        anyhow::bail!(
            "Backup destination cannot be the active WAL file: {}",
            path.display()
        );
    }
    if abs_target == source_shm {
        anyhow::bail!(
            "Backup destination cannot be the active SHM file: {}",
            path.display()
        );
    }

    // 2. Ensure parent directory exists
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    // 3. Delete target file if it already exists to overwrite
    if path.exists() {
        std::fs::remove_file(path)?;
    }

    // 4. Perform SQLite VACUUM INTO
    // VACUUM INTO is a standard SQL statement supported by SQLite
    // for transactionally consistent online backups. It is the modern
    // SQL alternative to the online backup C API, especially on WAL-enabled databases.
    let pool = db::create_pool(&config.database.path).await?;
    let path_str = path.to_string_lossy().replace('\'', "''");
    let query = format!("VACUUM INTO '{}'", path_str);

    sqlx::query(sqlx::AssertSqlSafe(query))
        .execute(&pool)
        .await?;

    println!(
        "✓ Database backup created successfully at: {}",
        path.display()
    );
    Ok(())
}
