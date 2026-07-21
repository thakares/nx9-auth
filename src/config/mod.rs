use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Root configuration loaded from config.toml
#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(skip)]
    pub config_path: Option<PathBuf>,

    #[serde(default)]
    pub server: ServerConfig,

    #[serde(default)]
    pub database: DatabaseConfig,

    #[serde(default)]
    pub security: SecurityConfig,

    #[serde(default)]
    pub audit: AuditConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    /// Interface to listen on.
    pub host: String,
    /// Port to listen on.
    pub port: u16,
    /// Whether the session cookie should set the `Secure` flag.
    ///
    /// Must be `true` when the UI is served over HTTPS (or behind a TLS
    /// reverse proxy). Leave `false` for plain-HTTP self-hosted installs —
    /// browsers reject `Secure` cookies on `http://` and authentication breaks.
    #[serde(default)]
    pub cookie_secure: bool,
    /// Production mode: enables HSTS, requires secure cookies, and refuses
    /// known-insecure bind configurations.
    #[serde(default)]
    pub production: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    /// Path to the SQLite database file (supports ~ prefix).
    pub path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SecurityConfig {
    /// Session idle timeout in hours.
    pub session_ttl_hours: u32,
    /// Session absolute lifetime in days.
    pub session_absolute_ttl_days: u32,
    /// Default API token lifetime in days.
    pub token_ttl_days: u32,
    /// Argon2id memory cost (KiB).
    pub argon2_memory: u32,
    /// Argon2id iteration count.
    pub argon2_iterations: u32,
    /// Argon2id parallelism.
    pub argon2_parallelism: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuditConfig {
    /// Whether to write events to the audit_logs table.
    pub enabled: bool,
}

// ── Defaults ────────────────────────────────────────────────────────────────

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(), // Default to loopback for user mode safety
            port: 8655,
            // Safe default for local/self-hosted HTTP. Enable for HTTPS production.
            cookie_secure: false,
            production: false,
        }
    }
}

impl ServerConfig {
    /// Refuse insecure production deployments.
    ///
    /// TLS is typically terminated at a reverse proxy; this enforces that
    /// cookies/HSTS are configured as if the external surface is HTTPS.
    pub fn validate_production_security(&self) -> anyhow::Result<()> {
        if !self.production {
            return Ok(());
        }
        if !self.cookie_secure {
            anyhow::bail!(
                "production mode requires server.cookie_secure = true \
                 (session cookies must be Secure for HTTPS deployments)"
            );
        }
        Ok(())
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        let default_db_path = if let Ok(home) = std::env::var("HOME") {
            Path::new(&home)
                .join(".local/share/nx9-auth/auth.db")
                .to_string_lossy()
                .into_owned()
        } else {
            "/var/lib/nx9-auth/auth.db".to_string()
        };
        Self {
            path: default_db_path,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            session_ttl_hours: 24,
            session_absolute_ttl_days: 30,
            token_ttl_days: 365,
            argon2_memory: 65536,
            argon2_iterations: 3,
            argon2_parallelism: 1,
        }
    }
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn resolve_home_path(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return Path::new(&home)
                .join(stripped)
                .to_string_lossy()
                .into_owned();
        }
    }
    path.to_string()
}

// ── Loading ──────────────────────────────────────────────────────────────────

impl Config {
    /// Resolve path prefixes such as ~ to actual home directories.
    pub fn resolve_paths(&mut self) {
        self.database.path = resolve_home_path(&self.database.path);
    }

    /// Load and parse config from a TOML file.
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let mut config: Config = toml::from_str(&content)
            .with_context(|| format!("failed to parse config file: {}", path.display()))?;
        config.config_path = Some(path.to_path_buf());
        config.resolve_paths();
        Ok(config)
    }

    /// Load config, falling back to defaults if the file doesn't exist.
    /// Errors on malformed files.
    pub fn load_or_default(path: &Path) -> Result<Self> {
        let mut config = if path.exists() {
            Self::load(path)?
        } else {
            let mut cfg = Self::default();
            cfg.resolve_paths();
            cfg
        };
        config.config_path = Some(path.to_path_buf());
        Ok(config)
    }

    /// Canonical config path candidates in priority order:
    /// 1. ./config.toml (Current directory override)
    /// 2. $XDG_CONFIG_HOME/nx9-auth/config.toml or ~/.config/nx9-auth/config.toml
    /// 3. /etc/nx9-auth/config.toml (System-wide default)
    pub fn search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // 1. Current directory override
        paths.push(PathBuf::from("./config.toml"));

        // 2. ~/.config/nx9-auth/config.toml (or XDG_CONFIG_HOME)
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            if !xdg.is_empty() {
                paths.push(PathBuf::from(xdg).join("nx9-auth/config.toml"));
            }
        } else if let Ok(home) = std::env::var("HOME") {
            paths.push(PathBuf::from(home).join(".config/nx9-auth/config.toml"));
        }

        // 3. System-wide default
        paths.push(PathBuf::from("/etc/nx9-auth/config.toml"));

        paths
    }

    /// Default user configuration path (~/.config/nx9-auth/config.toml)
    pub fn default_user_config_path() -> Option<PathBuf> {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            if !xdg.is_empty() {
                return Some(PathBuf::from(xdg).join("nx9-auth/config.toml"));
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            return Some(PathBuf::from(home).join(".config/nx9-auth/config.toml"));
        }
        None
    }

    /// Find and load the first existing config file from the search path.
    /// Returns Ok(None) if no configuration file is found in any search path.
    pub fn find_and_load(override_path: Option<&Path>) -> Result<Option<Self>> {
        if let Some(p) = override_path {
            let mut config = Self::load(p)?;
            config.config_path = Some(p.to_path_buf());
            return Ok(Some(config));
        }
        for path in Self::search_paths() {
            if path.exists() {
                let mut config = Self::load(&path)?;
                config.config_path = Some(path);
                return Ok(Some(config));
            }
        }
        Ok(None)
    }

    /// Generate default TOML content for the `init` command
    pub fn generate_default_toml() -> &'static str {
        r#"# nx9-auth configuration file

[server]
# Interface to bind on. Use 127.0.0.1 for local/user mode.
host = "127.0.0.1"
port = 8655
# Session cookie Secure flag (true only when serving over HTTPS).
cookie_secure = false
# Production mode: requires cookie_secure and enables HSTS.
production = false

[database]
# Absolute or home-relative path to the SQLite database file.
path = "~/.local/share/nx9-auth/auth.db"

[security]
# Session idle timeout in hours.
session_ttl_hours = 24
# Session absolute lifetime in days.
session_absolute_ttl_days = 30
# Default API token lifetime in days.
token_ttl_days = 365

# Argon2id verification parameters (production strength recommended).
argon2_memory = 65536
argon2_iterations = 3
argon2_parallelism = 1

[audit]
# Enable structured audit logging to the database.
enabled = true
"#
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let cfg = Config::default();
        assert_eq!(cfg.server.port, 8655);
        assert_eq!(cfg.server.host, "127.0.0.1");
        assert!(!cfg.server.cookie_secure);
        assert!(!cfg.server.production);
        if std::env::var("HOME").is_ok() {
            assert!(cfg.database.path.contains(".local/share/nx9-auth/auth.db"));
        } else {
            assert_eq!(cfg.database.path, "/var/lib/nx9-auth/auth.db");
        }
        assert_eq!(cfg.security.session_ttl_hours, 24);
        assert_eq!(cfg.security.session_absolute_ttl_days, 30);
        assert_eq!(cfg.security.token_ttl_days, 365);
        assert!(cfg.audit.enabled);
    }

    #[test]
    fn test_search_paths() {
        let paths = Config::search_paths();
        assert!(paths.iter().any(|p| p.to_str().unwrap() == "./config.toml"));
        assert!(
            paths
                .iter()
                .any(|p| p.to_str().unwrap() == "/etc/nx9-auth/config.toml")
        );
    }
}
