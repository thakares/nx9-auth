//! User and admin dashboards.

use crate::components::feedback::{ErrorState, LoadingSpinner};
use crate::components::navigation::Breadcrumb;
use crate::components::widgets::StatCard;
use crate::routes::Route;
use crate::services::api;
use crate::utils::format_datetime;
use dioxus::prelude::*;
use serde_json::Value;

#[component]
pub fn DashboardPage() -> Element {
    let mut data = use_signal(|| Option::<crate::models::DashboardResponse>::None);
    let mut error = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| true);

    let load = use_callback(move |_: ()| {
        loading.set(true);
        error.set(None);
        spawn(async move {
            match api::dashboard().await {
                Ok(d) => {
                    data.set(Some(d));
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    use_effect(move || { load.call(()); });

    rsx! {
        Breadcrumb { items: vec![("Dashboard".to_string(), None)] }

        div { class: "page-header",
            div {
                h1 { "Dashboard" }
                p { class: "desc", "Overview of your identity workspace" }
            }
            button {
                class: "btn btn-outline",
                r#type: "button",
                onclick: move |_| load.call(()),
                "Refresh"
            }
        }

        if loading() {
            LoadingSpinner { label: "Loading dashboard…" }
        } else if let Some(err) = error() {
            ErrorState {
                message: err,
                on_retry: move |_| load.call(())
            }
        } else if let Some(d) = data() {
            {
                let personal = &d.personal;
                let username = personal
                    .pointer("/user/username")
                    .and_then(|v| v.as_str())
                    .unwrap_or("user");
                let roles = personal
                    .get("roles")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                let sessions = personal
                    .get("sessions")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                let tokens = personal
                    .get("tokens")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                let apps = personal
                    .get("applications")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                let recent = personal
                    .get("recent_audit")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();

                rsx! {
                    // Personal welcome
                    div { class: "card mb-2",
                        div { class: "card-body",
                            h2 { "Welcome, {username}" }
                            p { class: "text-secondary",
                                "Roles: "
                                for (i, r) in roles.iter().enumerate() {
                                    if i > 0 { span { ", " } }
                                    span { class: "badge badge-accent", "{r.as_str().unwrap_or(\"\")}" }
                                }
                                if roles.is_empty() {
                                    span { class: "text-muted", "none" }
                                }
                            }
                        }
                    }

                    if d.is_admin {
                        if let Some(admin) = &d.admin {
                            AdminSummary { admin: admin.clone() }
                        }
                    }

                    div { class: "grid-2",
                        div { class: "card",
                            div { class: "card-header", h3 { "Active sessions" } }
                            div { class: "card-body",
                                if sessions.is_empty() {
                                    p { class: "text-muted", "No active sessions." }
                                } else {
                                    div { class: "table-wrap",
                                        table { class: "data-table",
                                            thead {
                                                tr {
                                                    th { "IP" }
                                                    th { "Last seen" }
                                                    th { "Expires" }
                                                }
                                            }
                                            tbody {
                                                for s in sessions.iter().take(5) {
                                                    tr {
                                                        td { class: "mono",
                                                            "{s.get(\"ip_address\").and_then(|v| v.as_str()).unwrap_or(\"—\")}"
                                                        }
                                                        td {
                                                            "{format_datetime(s.get(\"last_seen_at\").and_then(|v| v.as_str()).unwrap_or(\"\"))}"
                                                        }
                                                        td {
                                                            "{format_datetime(s.get(\"expires_at\").and_then(|v| v.as_str()).unwrap_or(\"\"))}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "card",
                            div { class: "card-header",
                                h3 { "API tokens" }
                                Link { class: "btn btn-sm btn-outline", to: Route::TokensPage {}, "Manage" }
                            }
                            div { class: "card-body",
                                if tokens.is_empty() {
                                    p { class: "text-muted", "No personal tokens yet." }
                                } else {
                                    ul { style: "margin:0;padding-left:1.1rem;",
                                        for t in tokens.iter().take(5) {
                                            li {
                                                strong { "{t.get(\"name\").and_then(|v| v.as_str()).unwrap_or(\"token\")}" }
                                                span { class: "text-muted",
                                                    " · created {format_datetime(t.get(\"created_at\").and_then(|v| v.as_str()).unwrap_or(\"\"))}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "card",
                            div { class: "card-header", h3 { "Applications" } }
                            div { class: "card-body",
                                if apps.is_empty() {
                                    p { class: "text-muted", "No applications registered." }
                                } else {
                                    div { class: "row", style: "flex-wrap:wrap;gap:0.5rem;",
                                        for a in apps {
                                            span { class: "badge badge-info",
                                                "{a.get(\"name\").and_then(|v| v.as_str()).unwrap_or(\"app\")}"
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "card",
                            div { class: "card-header", h3 { "Recent activity" } }
                            div { class: "card-body",
                                if recent.is_empty() {
                                    p { class: "text-muted", "No recent events." }
                                } else {
                                    div { class: "stack",
                                        for e in recent.iter().take(8) {
                                            div { style: "display:flex;justify-content:space-between;gap:0.5rem;font-size:13px;",
                                                span {
                                                    code { "{e.get(\"action\").and_then(|v| v.as_str()).unwrap_or(\"?\")}" }
                                                    span { class: "text-muted",
                                                        " on {e.get(\"resource_type\").and_then(|v| v.as_str()).unwrap_or(\"?\")}"
                                                    }
                                                }
                                                span { class: "text-muted",
                                                    "{format_datetime(e.get(\"created_at\").and_then(|v| v.as_str()).unwrap_or(\"\"))}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn AdminSummary(admin: Value) -> Element {
    let summary = admin.get("summary").cloned().unwrap_or(Value::Null);
    let num = |k: &str| -> String {
        summary
            .get(k)
            .and_then(|v| v.as_i64().or_else(|| v.as_u64().map(|u| u as i64)))
            .map(|n| n.to_string())
            .unwrap_or_else(|| "—".to_string())
    };

    let recent_logins = admin
        .get("recent_logins")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let recent_users = admin
        .get("recent_users")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let health = admin
        .get("system_health")
        .and_then(|v| v.get("status"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    rsx! {
        div { class: "mb-2",
            h2 { style: "margin-bottom: 0.75rem;", "Administrator overview" }
            div { class: "stat-grid",
                StatCard { label: "Tenants", value: num("tenants"), hint: "".to_string() }
                StatCard { label: "Users", value: num("total_users"), hint: "".to_string() }
                StatCard { label: "Sessions", value: num("active_sessions"), hint: "".to_string() }
                StatCard { label: "Roles", value: num("roles"), hint: "".to_string() }
                StatCard { label: "Permissions", value: num("permissions"), hint: "".to_string() }
                StatCard { label: "Applications", value: num("applications"), hint: "".to_string() }
                StatCard { label: "OAuth Clients", value: num("oauth_clients"), hint: "".to_string() }
                StatCard { label: "Service accounts", value: num("service_accounts"), hint: "".to_string() }
                StatCard { label: "Audit events", value: num("audit_events"), hint: "".to_string() }
                StatCard { label: "Security Alerts", value: num("security_alerts"), hint: "".to_string() }
            }

            div { class: "grid-2",
                div { class: "card",
                    div { class: "card-header", h3 { "Recent logins" } }
                    div { class: "card-body",
                        if recent_logins.is_empty() {
                            p { class: "text-muted", "No recent logins." }
                        } else {
                            for e in recent_logins.iter().take(6) {
                                div { style: "display:flex;justify-content:space-between;font-size:13px;margin-bottom:0.4rem;",
                                    span { class: "mono",
                                        "{e.get(\"actor_user_id\").and_then(|v| v.as_str()).unwrap_or(\"?\")[..8.min(e.get(\"actor_user_id\").and_then(|v| v.as_str()).unwrap_or(\"\").len())].to_string()}"
                                    }
                                    span { class: "text-muted",
                                        "{format_datetime(e.get(\"created_at\").and_then(|v| v.as_str()).unwrap_or(\"\"))}"
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "card",
                    div { class: "card-header", h3 { "Recent users" } }
                    div { class: "card-body",
                        if recent_users.is_empty() {
                            p { class: "text-muted", "No users." }
                        } else {
                            for u in recent_users.iter().take(6) {
                                div { style: "display:flex;justify-content:space-between;font-size:13px;margin-bottom:0.4rem;",
                                    span { "{u.get(\"username\").and_then(|v| v.as_str()).unwrap_or(\"?\")}" }
                                    span { class: "badge badge-success",
                                        "{u.get(\"status\").and_then(|v| v.as_str()).unwrap_or(\"\")}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
