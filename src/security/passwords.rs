use argon2::{
    Argon2, Params,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

use crate::{config::SecurityConfig, error::AppError};

/// Hash a plaintext password using Argon2id with configurable cost parameters.
///
/// Returns a PHC-format string (e.g. `$argon2id$v=19$...`) that includes the
/// salt and all parameters. This string is safe to store directly in the DB.
pub fn hash_password(password: &str, cfg: &SecurityConfig) -> Result<String, AppError> {
    let params = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        Params::new(
            cfg.argon2_memory,
            cfg.argon2_iterations,
            cfg.argon2_parallelism,
            None,
        )
        .map_err(|e| {
            tracing::error!(error = %e, "invalid argon2 params");
            AppError::Internal
        })?,
    );

    let salt = SaltString::generate(&mut OsRng);
    let hash = params
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| {
            tracing::error!(error = %e, "argon2 hashing failed");
            AppError::Internal
        })?;

    Ok(hash.to_string())
}

/// Verify a plaintext password against a stored Argon2id PHC hash.
///
/// Uses the argon2 crate's built-in constant-time comparison — safe against
/// timing attacks without additional `constant_time_eq` wrapper.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let parsed = PasswordHash::new(hash).map_err(|e| {
        tracing::error!(error = %e, "failed to parse password hash");
        AppError::Internal
    })?;

    match Argon2::default().verify_password(password.as_bytes(), &parsed) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => {
            tracing::error!(error = %e, "argon2 verification error");
            Err(AppError::Internal)
        }
    }
}

/// Execute a dummy Argon2id hash with the currently configured parameters.
///
/// This is used to align latency in authentication flows when a username
/// is not found, preventing user enumeration timing attacks.
pub fn verify_dummy(cfg: &SecurityConfig) -> Result<(), AppError> {
    let _ = hash_password("dummy_password_for_timing_attacks", cfg)?;
    Ok(())
}

/// Validate password strength against common patterns and minimum length.
///
/// For admin accounts (is_admin = true), enforces 12-char minimum.
/// For standard accounts, enforces 8-char minimum.
/// Both reject common passwords like "password", "admin123", "qwerty", "12345678".
pub fn validate_password_strength(password: &str, is_admin: bool) -> Result<(), AppError> {
    let min_len = if is_admin { 12 } else { 8 };
    if password.len() < min_len {
        return Err(AppError::InvalidInput(format!(
            "password must be at least {min_len} characters long"
        )));
    }

    let normalized = password.to_lowercase();
    let weak_list = [
        "password",
        "admin123",
        "qwerty",
        "12345678",
        "123456789",
        "administrator",
        "nx9-auth",
        "nx9auth",
    ];

    for weak in &weak_list {
        if normalized.contains(weak) {
            return Err(AppError::InvalidInput(
                "password contains a weak or common sequence".to_string(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SecurityConfig;

    fn test_cfg() -> SecurityConfig {
        SecurityConfig {
            session_ttl_hours: 24,
            session_absolute_ttl_days: 30,
            token_ttl_days: 365,
            argon2_memory: 4096, // low cost for tests
            argon2_iterations: 1,
            argon2_parallelism: 1,
        }
    }

    #[test]
    fn test_hash_and_verify() {
        let cfg = test_cfg();
        let pass = "correct_password_123";
        let hash = hash_password(pass, &cfg).unwrap();
        assert!(verify_password(pass, &hash).unwrap());
        assert!(!verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_strength_validation() {
        // Standard user length
        assert!(validate_password_strength("super_secure_passphrase_123", false).is_ok());
        assert!(validate_password_strength("short", false).is_err());

        // Admin length
        assert!(validate_password_strength("super_secure_admin_passphrase_123", true).is_ok());
        assert!(validate_password_strength("short_admin", true).is_err());

        // Weak password checks
        assert!(validate_password_strength("my-password-is-weak", false).is_err());
        assert!(validate_password_strength("admin1234567", false).is_err());
    }
}
