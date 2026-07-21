//! Small UI helpers.

/// Initials from a username (up to 2 chars).
pub fn initials(name: &str) -> String {
    let parts: Vec<&str> = name.split(|c: char| !c.is_alphanumeric()).filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return "?".to_string();
    }
    if parts.len() == 1 {
        return parts[0].chars().take(2).collect::<String>().to_uppercase();
    }
    format!(
        "{}{}",
        parts[0].chars().next().unwrap_or('?'),
        parts[1].chars().next().unwrap_or('?')
    )
    .to_uppercase()
}

/// Relative-ish short date display (pass through ISO for now).
pub fn format_datetime(s: &str) -> String {
    if s.is_empty() {
        return "—".to_string();
    }
    // Prefer "YYYY-MM-DD HH:MM" from ISO-ish strings
    let s = s.replace('T', " ");
    if s.len() >= 16 {
        s[..16].to_string()
    } else {
        s
    }
}

/// Status → CSS badge class.
pub fn status_badge_class(status: &str) -> &'static str {
    match status {
        "active" | "enabled" => "badge badge-success",
        "disabled" => "badge badge-danger",
        "locked" | "revoked" => "badge badge-warning",
        _ => "badge",
    }
}

/// Severity → CSS badge class.
pub fn severity_badge_class(sev: &str) -> &'static str {
    match sev {
        "info" => "badge badge-info",
        "warning" => "badge badge-warning",
        "critical" => "badge badge-danger",
        _ => "badge",
    }
}

/// Client-side filter helper.
pub fn matches_query(haystack: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    haystack.to_lowercase().contains(&query.to_lowercase())
}

/// Simple slugify for application slugs.
pub fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
