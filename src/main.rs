use std::net::SocketAddr;

use clap::Parser;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use nx9_auth::{
    api,
    cli::{self, Cli, Commands},
    config::Config,
    db,
    state::AppState,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse CLI arguments first (before any logging so --help works cleanly)
    let cli = Cli::parse();

    // Initialize logging based on the command and verbosity
    let is_serve = matches!(cli.command, Commands::Serve);
    if is_serve {
        // Structured JSON logging for production server deployment
        tracing_subscriber::registry()
            .with(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "nx9_auth=info,tower_http=info".parse().unwrap()),
            )
            .with(fmt::layer().json())
            .init();
    } else if cli.verbose {
        // Human-readable compact logging for verbosity in subcommands
        tracing_subscriber::registry()
            .with(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "nx9_auth=debug".parse().unwrap()),
            )
            .with(fmt::layer().compact())
            .init();
    } else {
        // Silence info/debug logging for clean operator CLI commands
        tracing_subscriber::registry()
            .with(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "nx9_auth=warn".parse().unwrap()),
            )
            .with(fmt::layer().compact())
            .init();
    }

    // Load configuration
    let config_opt = if matches!(
        cli.command,
        Commands::Init { .. } | Commands::ConfigPath { .. }
    ) {
        // For init/config-path commands, a missing override config is fine
        if let Some(ref path) = cli.config {
            if path.exists() {
                Some(Config::load(path)?)
            } else {
                let mut cfg = Config {
                    config_path: Some(path.clone()),
                    ..Default::default()
                };
                cfg.resolve_paths();
                Some(cfg)
            }
        } else {
            Config::find_and_load(None)?
        }
    } else {
        Config::find_and_load(cli.config.as_deref())?
    };

    let config = match config_opt {
        Some(cfg) => cfg,
        None => {
            // init and config-path are allowed to run without an existing config file.
            // We use default Config structure for them.
            if matches!(
                cli.command,
                Commands::Init { .. } | Commands::ConfigPath { .. }
            ) {
                let mut cfg = Config::default();
                cfg.resolve_paths();
                cfg
            } else {
                eprintln!(
                    "\nError: No configuration found.\n\nRun:\n\n    nx9-auth init\n\nOr if running in Docker:\n\n    docker exec -it nx9-auth nx9-auth init\n"
                );
                std::process::exit(1);
            }
        }
    };

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        git_commit = env!("GIT_COMMIT"),
        "nx9-auth starting"
    );

    // Dispatch to serve or CLI command
    match cli.command {
        Commands::Serve => run_server(config).await,
        cmd => cli::run(cmd, config).await,
    }
}

/// Start the HTTP server (Milestone B+).
async fn run_server(config: Config) -> anyhow::Result<()> {
    // Refuse insecure production configuration (Secure cookies / HSTS surface).
    config.server.validate_production_security()?;

    // Open DB pool and run migrations
    let pool = db::create_pool(&config.database.path).await?;
    db::run_migrations(&pool).await?;

    let pool_clone = pool.clone();
    tokio::spawn(async move {
        let _ = pool_clone; // TODO: restore session repo cleanup logic using the new provider architecture
    });

    let provider: std::sync::Arc<dyn db::provider::DatabaseProvider> =
        std::sync::Arc::new(db::provider::SqliteProvider::new(pool));

    let state = AppState::new(provider.clone(), config.clone());
    let app = api::router::build(state);
    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port)
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid bind address: {}", e))?;

    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!(
        address = %addr,
        "server listening"
    );

    println!(
        "\nnx9-auth is running\n\n  API + Admin UI : http://{}\n  Health check   : http://{}/health\n",
        addr, addr
    );

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
