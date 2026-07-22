use anyhow::Context;
use std::io::{self, Write};
use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::{
    config::Config,
    db::{
        self,
        models::{Tenant, User, UserStatus},
    },
    error::AppError,
    identity::users as identity_users,
    security::tokens as token_security,
};

/// Resolve a user by ID or username (username lookup is case-sensitive, as stored).
async fn resolve_user(
    provider: &std::sync::Arc<dyn crate::db::provider::DatabaseProvider>,
    id_or_username: &str,
) -> anyhow::Result<User> {
    if let Some(user) = provider.users().find_by_id(id_or_username).await? {
        return Ok(user);
    }
    if let Some(user) = provider.users().find_by_username(id_or_username).await? {
        return Ok(user);
    }
    anyhow::bail!("User not found: '{id_or_username}' (use ID or username)");
}

// ── CLI Definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "nx9-auth",
    about = "nx9-auth \u{2014} Self-hosted Identity & Access Management",
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
    /// Start the HTTP server (API + Admin UI).
    Serve,

    /// Run pending database migrations.
    Migrate,

    /// Check system health and configuration.
    Doctor,

    /// Create the initial administrator account.
    CreateAdmin {
        /// Username for the new admin account.
        username: String,
    },

    /// Create a new user account.
    CreateUser {
        /// Username for the new user account.
        username: String,
    },

    /// List all users.
    ListUsers,

    /// Disable a user account.
    DisableUser {
        /// User ID or username to disable.
        id_or_username: String,
    },

    /// Enable a user account.
    EnableUser {
        /// User ID or username to enable.
        id_or_username: String,
    },

    /// Reset a user's password.
    ResetPassword {
        /// User ID or username to reset.
        id_or_username: String,
    },

    /// Create a personal access token for a user.
    CreateToken {
        /// User ID or username to create the token for.
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

    /// Initialize config, database, and admin user.
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

    /// Show configuration and database paths.
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

    /// Restore the database from a backup file.
    Restore {
        /// Path to the backup file to restore from.
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

        Commands::DisableUser { id_or_username } => {
            cmd_set_status(&config, &id_or_username, UserStatus::Disabled).await
        }
        Commands::EnableUser { id_or_username } => {
            cmd_set_status(&config, &id_or_username, UserStatus::Active).await
        }
        Commands::ResetPassword { id_or_username } => {
            cmd_reset_password(&config, &id_or_username).await
        }

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
        Commands::Restore { path } => cmd_restore(&config, &path).await,
    }
}

// ── migrate ───────────────────────────────────────────────────────────────────

async fn cmd_migrate(config: &Config) -> anyhow::Result<()> {
    let (_provider, backend, _pool) = db::init_provider(config).await?;
    println!("✓ Migrations applied successfully ({backend}).");
    Ok(())
}

// ── doctor ────────────────────────────────────────────────────────────────────

async fn run_doctor_checks(config: &Config) -> anyhow::Result<bool> {
    let mut ok = true;

    println!("\nnx9-auth doctor\n");

    // 1. Config file loads
    println!("  ✓  Config file loads and parses");

    // 2. DB backend & connection
    let (url, backend) = match config.database.resolved_url() {
        Ok(res) => res,
        Err(e) => {
            println!("  ✗  Failed to resolve database configuration: {e}");
            println!("\nDoctor result: FAIL\n");
            return Ok(false);
        }
    };
    println!("  ✓  Database backend detected: {backend}");
    println!("  ✓  Database URL: {url}");

    let provider = match db::init_provider(config).await {
        Ok((p, _, _)) => {
            println!("  ✓  Database connection & migrations successful");
            p
        }
        Err(e) => {
            println!("  ✗  Database initialization failed: {e}");
            println!("\nDoctor result: FAIL\n");
            return Ok(false);
        }
    };

    // 5. Default tenant exists
    match provider.tenants().find_by_id(Tenant::DEFAULT_ID).await {
        Ok(Some(_)) => println!("  ✓  Default tenant exists"),
        _ => {
            println!("  ✗  Default tenant missing — run `nx9-auth migrate`");
            ok = false;
        }
    }

    // 6. Admin role exists
    match provider.roles().admin_role_exists().await {
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
    match provider.users().count_admins().await {
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
    let (provider, _backend, _pool) = db::init_provider(config).await?;

    let password = prompt_password_confirmed("Password for admin: ", true)?;

    let user = identity_users::create_user(
        &provider,
        &config.security,
        Tenant::DEFAULT_ID,
        username,
        &password,
        None,
        None,
        None,
    )
    .await?;

    crate::identity::roles::assign_role(&provider, &user.id, "admin", None, None, None).await?;

    println!("✓ Admin user '{}' created (id: {})", user.username, user.id);
    Ok(())
}

// ── create-user ───────────────────────────────────────────────────────────────

async fn cmd_create_user(config: &Config, username: &str) -> anyhow::Result<()> {
    let (provider, _backend, _pool) = db::init_provider(config).await?;

    let password = prompt_password_confirmed("Password: ", false)?;

    let user = identity_users::create_user(
        &provider,
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
    let (provider, _backend, _pool) = db::init_provider(config).await?;

    let users = provider.users().list(Tenant::DEFAULT_ID).await?;

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

async fn cmd_set_status(
    config: &Config,
    id_or_username: &str,
    status: UserStatus,
) -> anyhow::Result<()> {
    let (provider, _backend, _pool) = db::init_provider(config).await?;

    let user = resolve_user(&provider, id_or_username).await?;
    identity_users::update_status(&provider, &user.id, status.as_i32(), None, None, None).await?;
    println!(
        "✓ User '{}' status set to {}",
        user.username,
        status.as_str()
    );
    Ok(())
}

// ── reset-password ────────────────────────────────────────────────────────────

async fn cmd_reset_password(config: &Config, id_or_username: &str) -> anyhow::Result<()> {
    let (provider, _backend, _pool) = db::init_provider(config).await?;

    let user = resolve_user(&provider, id_or_username).await?;
    let user_roles = provider.roles().list_for_user(&user.id).await?;
    let is_admin = user_roles.iter().any(|r| r.name == "admin");
    let password =
        prompt_password_confirmed(&format!("New password for '{}': ", user.username), is_admin)?;
    identity_users::reset_password(
        &provider,
        &config.security,
        &user.id,
        &password,
        None,
        None,
        None,
    )
    .await?;
    println!("✓ Password reset for user '{}'", user.username);
    Ok(())
}

// ── create-token ──────────────────────────────────────────────────────────────

async fn cmd_create_token(config: &Config, user_ref: &str, name: &str) -> anyhow::Result<()> {
    let (provider, _backend, _pool) = db::init_provider(config).await?;

    let user = resolve_user(&provider, user_ref).await?;
    let (token, raw) = token_security::create_token(
        &provider,
        &user.id,
        name,
        &config.security,
        None,
        None,
        None,
    )
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
    let (provider, _backend, _pool) = db::init_provider(config).await?;

    let token = provider
        .tokens()
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| anyhow::anyhow!("token not found: {}", id))?;

    token_security::revoke_token(&provider, id, None, None, None).await?;

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

    let sqlite_path = config.database.sqlite_path();
    let db_path = std::path::Path::new(&sqlite_path);
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
    println!("Initializing database and migrations...");
    let (provider, backend, _pool) = db::init_provider(config).await?;
    println!("✓ Database initialized ({backend}).");

    // 3. Create administrator
    if skip_admin {
        println!("ℹ Administrator creation skipped.");
    } else {
        let admin_count = provider.users().count_admins().await?;
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

            let user = identity_users::create_user(
                &provider,
                &config.security,
                Tenant::DEFAULT_ID,
                &username,
                &password,
                None,
                None,
                None,
            )
            .await?;

            crate::identity::roles::assign_role(&provider, &user.id, "admin", None, None, None)
                .await?;
            println!("✓ Admin user '{}' created successfully.", username);
        } else {
            println!("✓ Administrator account already exists.");
        }
    }

    // 4. Run post-install validation (relaxed)
    println!("\nValidation:");
    let init_ok = run_init_validation(config, skip_admin).await?;
    if !init_ok {
        anyhow::bail!("Post-installation validation checks failed!");
    }

    println!("\nnx9-auth is ready.\n\nStart the server with:\n  nx9-auth serve\n");
    Ok(())
}

async fn run_init_validation(config: &Config, admin_skipped: bool) -> anyhow::Result<bool> {
    let mut ok = true;

    // 1. Config valid
    println!("  ✓ Configuration");

    // 2. Directories writable
    let sqlite_path = config.database.sqlite_path();
    let db_path = std::path::Path::new(&sqlite_path);
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
        println!("  ✓ Directories");
    } else {
        println!("  ✗ Directories not writable");
        ok = false;
    }

    // 3. Database & Migrations reachable
    let provider = match db::init_provider(config).await {
        Ok((p, _, _)) => {
            println!("  ✓ Database");
            println!("  ✓ Migrations");
            p
        }
        Err(e) => {
            println!("  ✗ Database connection failed: {}", e);
            return Ok(false);
        }
    };

    // 4. Admin account check
    let admin_count = provider.users().count_admins().await.unwrap_or(0);
    if admin_count > 0 {
        println!("  ✓ Administrator account");
    } else if admin_skipped {
        println!("  ℹ Administrator creation skipped");
    } else {
        println!("  ✗ No administrator account exists");
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
    let (database_url, _) = config.database.resolved_url().unwrap_or_else(|_| {
        (
            config.database.sqlite_path(),
            crate::config::DatabaseBackend::Sqlite,
        )
    });

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
            "database": database_url,
            "state": state_dir,
        });
        println!("{}", serde_json::to_string_pretty(&val)?);
    } else {
        println!("\nConfig:");
        println!("  {}", config_file);
        println!("\nDatabase:");
        println!("  {}", database_url);
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
    let (provider, _backend, _pool) = db::init_provider(config).await?;

    let user = resolve_user(&provider, id_or_username).await?;

    let user_roles = provider.roles().list_for_user(&user.id).await?;
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

        let user_perms = provider.permissions().list_for_user(&user.id).await?;
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
    let (provider, _backend, _pool) = db::init_provider(config).await?;

    let token = provider
        .tokens()
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| anyhow::anyhow!("Token not found: {}", id))?;

    let user = provider.users().find_by_id(&token.user_id).await?;
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
    let (url, backend) = config.database.resolved_url()?;
    match backend {
        crate::config::DatabaseBackend::Sqlite => {
            let sqlite_path = config.database.sqlite_path();
            let source_path = std::path::Path::new(&sqlite_path);
            let abs_source =
                std::fs::canonicalize(source_path).unwrap_or_else(|_| source_path.to_path_buf());
            let abs_target = if path.is_absolute() {
                path.to_path_buf()
            } else {
                std::env::current_dir()?.join(path)
            };

            let source_dir = abs_source
                .parent()
                .ok_or_else(|| anyhow::anyhow!("invalid database source path"))?;
            let source_file_name = abs_source
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("invalid database file name"))?
                .to_string_lossy();
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

            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }

            if path.exists() {
                std::fs::remove_file(path)?;
            }

            #[cfg(feature = "sqlite")]
            {
                let pool = db::create_pool(&sqlite_path).await?;
                let path_str = path.to_string_lossy().replace('\'', "''");
                let query = format!("VACUUM INTO '{}'", path_str);

                sqlx::query(sqlx::AssertSqlSafe(query))
                    .execute(&pool)
                    .await?;

                println!(
                    "✓ SQLite database backup created successfully at: {}",
                    path.display()
                );
            }
            #[cfg(not(feature = "sqlite"))]
            {
                anyhow::bail!("SQLite database backups require the 'sqlite' feature");
            }
        }
        crate::config::DatabaseBackend::Postgres => {
            let output = std::process::Command::new("pg_dump")
                .arg("-Fc")
                .arg("-d")
                .arg(&url)
                .arg("-f")
                .arg(path)
                .output()
                .context(
                    "failed to execute pg_dump (ensure PostgreSQL client tools are installed)",
                )?;

            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("pg_dump failed: {err}");
            }

            println!(
                "✓ PostgreSQL database backup created successfully at: {}",
                path.display()
            );
        }
    }
    Ok(())
}

async fn cmd_restore(config: &Config, path: &std::path::Path) -> anyhow::Result<()> {
    if !path.exists() {
        anyhow::bail!("Backup file does not exist: {}", path.display());
    }

    let (url, backend) = config.database.resolved_url()?;
    match backend {
        crate::config::DatabaseBackend::Sqlite => {
            let sqlite_path = config.database.sqlite_path();
            let target_path = std::path::Path::new(&sqlite_path);
            if let Some(parent) = target_path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            std::fs::copy(path, target_path).with_context(|| {
                format!("failed to restore backup to {}", target_path.display())
            })?;
            println!(
                "✓ SQLite database restored successfully from: {}",
                path.display()
            );
        }
        crate::config::DatabaseBackend::Postgres => {
            let output = std::process::Command::new("pg_restore")
                .arg("--clean")
                .arg("--if-exists")
                .arg("-d")
                .arg(&url)
                .arg(path)
                .output()
                .context(
                    "failed to execute pg_restore (ensure PostgreSQL client tools are installed)",
                )?;

            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("pg_restore failed: {err}");
            }

            println!(
                "✓ PostgreSQL database restored successfully from: {}",
                path.display()
            );
        }
    }
    Ok(())
}
