use crate::components::feedback::{ConfirmDialog, EmptyState, ErrorState, LoadingSpinner};
use crate::components::navigation::Breadcrumb;
use crate::components::tables::{ColumnDef, DataTable};
use crate::models::SessionView;
use crate::routes::Route;
use crate::services::api;
use crate::state::{AppState, ToastKind};
use crate::utils::{format_datetime, matches_query};
use dioxus::prelude::*;

fn parse_browser(ua: &str) -> String {
    if ua.contains("Chrome") && !ua.contains("Edg") {
        "Chrome".to_string()
    } else if ua.contains("Firefox") {
        "Firefox".to_string()
    } else if ua.contains("Safari") && !ua.contains("Chrome") {
        "Safari".to_string()
    } else if ua.contains("Edg") {
        "Edge".to_string()
    } else if ua.is_empty() {
        "Unknown".to_string()
    } else {
        ua.chars().take(30).collect::<String>() + "..."
    }
}

#[component]
pub fn SessionsPage() -> Element {
    let state = use_context::<AppState>();
    let mut sessions = use_signal(Vec::<SessionView>::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| true);
    let mut query = use_signal(String::new);
    let mut terminate_target = use_signal(|| Option::<SessionView>::None);
    let mut show_terminate_others = use_signal(|| false);

    let reload = use_callback(move |_: ()| {
        loading.set(true);
        error.set(None);
        spawn(async move {
            match api::list_sessions().await {
                Ok(r) => {
                    sessions.set(r.sessions);
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    use_effect(move || {
        reload.call(());
    });

    let filtered: Vec<SessionView> = sessions()
        .into_iter()
        .filter(|s| {
            matches_query(s.ip_address.as_deref().unwrap_or(""), &query())
                || matches_query(s.user_agent.as_deref().unwrap_or(""), &query())
        })
        .collect();
    let total = filtered.len();

    rsx! {
        Breadcrumb { items: vec![
            ("Dashboard".to_string(), Some(Route::DashboardPage {})),
            ("Sessions".to_string(), None),
        ]}

        div { class: "page-header",
            div {
                h1 { "Sessions" }
                p { class: "desc", "Manage active sessions and connected devices" }
            }
            button {
                class: "btn btn-outline btn-danger",
                r#type: "button",
                onclick: move |_| show_terminate_others.set(true),
                "Terminate other sessions"
            }
        }

        if loading() {
            LoadingSpinner {}
        } else if let Some(err) = error() {
            ErrorState { message: err, on_retry: move |_| reload.call(()) }
        } else if sessions().is_empty() && query().is_empty() {
            EmptyState { title: "No active sessions".to_string(), description: "You have no active sessions.", icon: "🔌" }
        } else {
            DataTable {
                columns: vec![
                    ColumnDef { key: "current".into(), label: "Current".into(), sortable: false, visible: true },
                    ColumnDef { key: "ip".into(), label: "IP Address".into(), sortable: false, visible: true },
                    ColumnDef { key: "browser".into(), label: "Browser/Device".into(), sortable: false, visible: true },
                    ColumnDef { key: "created".into(), label: "Created".into(), sortable: false, visible: true },
                    ColumnDef { key: "last_seen".into(), label: "Last Seen".into(), sortable: false, visible: true },
                    ColumnDef { key: "expires".into(), label: "Expires".into(), sortable: false, visible: true },
                    ColumnDef { key: "actions".into(), label: "Actions".into(), sortable: false, visible: true },
                ],
                on_search: move |v| { query.set(v); },
                search_value: query(),
                search_placeholder: "Search IP or User Agent…".to_string(),
                on_sort: move |_| {},
                sort_key: "".to_string(),
                on_page: move |_| {},
                page: 0,
                page_size: total.max(1),
                total: total,
                toolbar_actions: rsx! {
                    button { class: "btn btn-outline", r#type: "button", onclick: move |_| reload.call(()), "Refresh" }
                },
                for s in filtered {
                    {
                        let s2 = s.clone();
                        rsx! {
                            tr { key: "{s.id}",
                                td {
                                    if s.is_current {
                                        span { class: "badge badge-success", "Current" }
                                    }
                                }
                                td { "{s.ip_address.as_deref().unwrap_or(\"Unknown\")}" }
                                td { "{parse_browser(s.user_agent.as_deref().unwrap_or(\"\"))}" }
                                td { "{format_datetime(&s.created_at)}" }
                                td { "{format_datetime(&s.last_seen_at)}" }
                                td { "{format_datetime(&s.expires_at)}" }
                                td { style: "text-align: right;",
                                    div { class: "actions",
                                        if !s.is_current {
                                            button {
                                                class: "btn btn-sm btn-danger",
                                                r#type: "button",
                                                onclick: move |_| terminate_target.set(Some(s2.clone())),
                                                "Terminate"
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

        ConfirmDialog {
            title: "Terminate Session".to_string(),
            message: "Are you sure you want to terminate this session?".to_string(),
            open: terminate_target().is_some(),
            confirm_label: "Terminate",
            danger: true,
            on_confirm: move |_| {
                if let Some(s) = terminate_target() {
                    spawn(async move {
                        match api::terminate_session(&s.id).await {
                            Ok(_) => {
                                state.toast(ToastKind::Success, "Session terminated");
                                terminate_target.set(None);
                                reload.call(());
                            }
                            Err(e) => state.toast(ToastKind::Error, e.to_string()),
                        }
                    });
                }
            },
            on_cancel: move |_| terminate_target.set(None),
        }

        ConfirmDialog {
            title: "Terminate Other Sessions".to_string(),
            message: "This will sign you out on all other devices. Are you sure?".to_string(),
            open: show_terminate_others(),
            confirm_label: "Terminate all others",
            danger: true,
            on_confirm: move |_| {
                spawn(async move {
                    match api::terminate_other_sessions().await {
                        Ok(_) => {
                            state.toast(ToastKind::Success, "Other sessions terminated");
                            show_terminate_others.set(false);
                            reload.call(());
                        }
                        Err(e) => state.toast(ToastKind::Error, e.to_string()),
                    }
                });
            },
            on_cancel: move |_| show_terminate_others.set(false),
        }
    }
}
