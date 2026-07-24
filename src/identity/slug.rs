use crate::error::AppError;

pub const RESERVED_SLUGS: &[&str] = &[
    "admin",
    "api",
    "system",
    "auth",
    "login",
    "logout",
    "dashboard",
    "health",
    "metrics",
    "root",
    "public",
    "private",
    "null",
    "undefined",
    "config",
    "settings",
    "account",
    "accounts",
    "role",
    "roles",
    "permission",
    "permissions",
    "group",
    "groups",
    "service-account",
    "service-accounts",
];

/// Validates an explicit or derived slug string according to server-side policy:
/// - Must be 2..=63 characters in length.
/// - Must consist only of lowercase ASCII alphanumeric characters ('a'..='z', '0'..='9') and hyphens ('-').
/// - Cannot start or end with a hyphen.
/// - Cannot contain consecutive hyphens ("--").
/// - Cannot be one of the reserved slug names (except "default" which is preserved for built-in tenant).
pub fn validate_slug(slug: &str) -> Result<(), AppError> {
    let s = slug.trim();
    if s.is_empty() {
        return Err(AppError::InvalidInput("slug cannot be empty".into()));
    }
    if s.len() < 2 || s.len() > 63 {
        return Err(AppError::InvalidInput(format!(
            "slug length must be between 2 and 63 characters, got {}",
            s.len()
        )));
    }
    if s.starts_with('-') || s.ends_with('-') {
        return Err(AppError::InvalidInput(
            "slug cannot start or end with a hyphen".into(),
        ));
    }
    if s.contains("--") {
        return Err(AppError::InvalidInput(
            "slug cannot contain consecutive hyphens".into(),
        ));
    }
    for ch in s.chars() {
        if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '-' {
            return Err(AppError::InvalidInput(format!(
                "slug contains invalid character '{ch}'; only lowercase alphanumeric characters and hyphens are allowed"
            )));
        }
    }
    if RESERVED_SLUGS.contains(&s) {
        return Err(AppError::InvalidInput(format!(
            "slug '{s}' is reserved by system"
        )));
    }
    Ok(())
}

/// Slugifies a display name when CREATE omits an explicit slug.
/// Converts non-alphanumeric characters to hyphens, lowercases the string,
/// collapses repeated hyphens, and validates the result.
pub fn slugify(input: &str) -> Result<String, AppError> {
    let mut slug = String::with_capacity(input.len());
    let mut prev_hyphen = false;

    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            prev_hyphen = false;
        } else if !prev_hyphen && !slug.is_empty() {
            slug.push('-');
            prev_hyphen = true;
        }
    }

    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput(
            "unable to generate valid slug from provided name".into(),
        ));
    }

    validate_slug(trimmed)?;
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_slugs() {
        assert!(validate_slug("default").is_ok());
        assert!(validate_slug("my-app-1").is_ok());
        assert!(validate_slug("acme-corp").is_ok());
        assert!(validate_slug("xy").is_ok());
    }

    #[test]
    fn test_invalid_slugs() {
        assert!(validate_slug("").is_err());
        assert!(validate_slug("a").is_err());
        assert!(validate_slug("-app").is_err());
        assert!(validate_slug("app-").is_err());
        assert!(validate_slug("my--app").is_err());
        assert!(validate_slug("My-App").is_err());
        assert!(validate_slug("my_app").is_err());
        assert!(validate_slug("admin").is_err());
        assert!(validate_slug("api").is_err());
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Acme Corp!").unwrap(), "acme-corp");
        assert_eq!(slugify("My   App 123").unwrap(), "my-app-123");
        assert!(slugify("!!!").is_err());
    }
}
