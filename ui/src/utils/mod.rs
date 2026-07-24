//! Small UI helpers.

/// Initials from a username (up to 2 chars).
pub fn initials(name: &str) -> String {
    let parts: Vec<&str> = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
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

/// Check if location search contains exact `create=1` query parameter, and clear `create=1` from history URL while preserving other parameters.
pub fn check_and_clear_create_intent() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(search) = window.location().search() {
                let query_str = search.trim_start_matches('?');
                let mut has_create = false;
                let mut remaining_params = Vec::new();

                for part in query_str.split('&') {
                    if part.is_empty() {
                        continue;
                    }
                    let mut key_val = part.splitn(2, '=');
                    let key = key_val.next().unwrap_or("");
                    let val = key_val.next().unwrap_or("");
                    if key == "create" && val == "1" {
                        has_create = true;
                    } else {
                        remaining_params.push(part);
                    }
                }

                if has_create {
                    if let Ok(pathname) = window.location().pathname() {
                        let new_search = if remaining_params.is_empty() {
                            String::new()
                        } else {
                            format!("?{}", remaining_params.join("&"))
                        };
                        let new_url = format!("{pathname}{new_search}");
                        let _ = window.history().and_then(|h| {
                            h.replace_state_with_url(
                                &wasm_bindgen::JsValue::NULL,
                                "",
                                Some(&new_url),
                            )
                        });
                    }
                    return true;
                }
            }
        }
    }
    false
}
