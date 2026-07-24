//! Service account management.

use crate::components::feedback::{ConfirmDialog, EmptyState, ErrorState, LoadingSpinner, Modal};
use crate::components::forms::TextInput;
use crate::components::navigation::Breadcrumb;
use crate::components::tables::{DataTable, ColumnDef};
use crate::components::widgets::StatusChip;
use crate::models::ServiceAccountView;
use crate::routes::Route;
use crate::services::api;
use crate::state::{AppState, ToastKind};
use crate::utils::{format_datetime, matches_query};
use dioxus::prelude::*;

#[component]
pub fn ServiceAccountsPage() -> Element {
    let state = use_context::<AppState>();
    let mut items = use_signal(Vec::<ServiceAccountView>::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| true);
    let mut query = use_signal(String::new);
    let mut page = use_signal(|| 0usize);
    let page_size = 10usize;
    let mut sort_key = use_signal(|| "name".to_string());

    let mut show_create = use_signal(|| false);
    let mut name = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut secret = use_signal(|| Option::<String>::None);
    let mut delete_target = use_signal(|| Option::<ServiceAccountView>::None);

    let reload = use_callback(move |_: ()| {
        loading.set(true);
        spawn(async move {
            match api::list_service_accounts().await {
                Ok(list) => {
                    items.set(list);
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });
    use_effect(move || { reload.call(()); });

    let can_create = state.auth.read().has_permission("service_accounts:manage") || state.auth.read().is_adminish();
    use_effect(move || {
        if can_create && crate::utils::check_and_clear_create_intent() {
            show_create.set(true);
        }
    });

    let mut filtered: Vec<_> = items()
        .into_iter()
        .filter(|s| {
            matches_query(&s.name, &query())
                || s.description
                    .as_deref()
                    .map(|d| matches_query(d, &query()))
                    .unwrap_or(false)
        })
        .collect();
    let sk = sort_key();
    filtered.sort_by(|a, b| match sk.as_str() {
        "status" => b.enabled.cmp(&a.enabled),
        "created" => b.created_at.cmp(&a.created_at),
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    
    let total = filtered.len();
    let page_items: Vec<_> = filtered.into_iter().skip(page() * page_size).take(page_size).collect();

    rsx! {
        Breadcrumb { items: vec![
            ("Dashboard".to_string(), Some(Route::DashboardPage {})),
            ("Service Accounts".to_string(), None),
        ]}

        div { class: "page-header",
            div {
                h1 { "Service Accounts" }
                p { class: "desc", "Non-human identities for automation and integrations" }
            }
            button {
                class: "btn btn-primary", r#type: "button",
                onclick: move |_| show_create.set(true),
                "+ Create"
            }
        }

        if loading() {
            LoadingSpinner {}
        } else if let Some(err) = error() {
            ErrorState { message: err, on_retry: move |_| reload.call(()) }
        } else if items().is_empty() && query().is_empty() {
            EmptyState {
                title: "No service accounts".to_string(),
                description: "",
                icon: "⚙",
            }
        } else {
            DataTable {
                columns: vec![
                    ColumnDef { key: "name".into(), label: "Name".into(), sortable: true, visible: true },
                    ColumnDef { key: "description".into(), label: "Description".into(), sortable: false, visible: true },
                    ColumnDef { key: "status".into(), label: "Status".into(), sortable: true, visible: true },
                    ColumnDef { key: "created".into(), label: "Created".into(), sortable: true, visible: true },
                    ColumnDef { key: "actions".into(), label: "Actions".into(), sortable: false, visible: true },
                ],
                on_search: move |v| { query.set(v); page.set(0); },
                search_value: query(),
                search_placeholder: "Search…".to_string(),
                on_sort: move |k| sort_key.set(k),
                sort_key: sort_key(),
                on_page: move |p| page.set(p),
                page: page(),
                page_size: page_size,
                total: total,
                toolbar_actions: rsx! {
                    button { class: "btn btn-outline", r#type: "button", onclick: move |_| reload.call(()), "Refresh" }
                },
                for sa in page_items {
                    {
                        let item = sa.clone();
                        let item2 = sa.clone();
                        let item3 = sa.clone();
                        rsx! {
                            tr { key: "{sa.id}",
                                td { strong { "{sa.name}" } }
                                td { class: "text-secondary",
                                    "{sa.description.as_deref().unwrap_or(\"—\")}"
                                }
                                td {
                                    StatusChip {
                                        status: if sa.enabled { "active".to_string() } else { "disabled".to_string() }
                                    }
                                }
                                td { "{format_datetime(&sa.created_at)}" }
                                td { style: "text-align: right;",
                                    div { class: "actions",
                                        button {
                                            class: "btn btn-sm btn-outline",
                                            r#type: "button",
                                            onclick: move |_| {
                                                let id = item.id.clone();
                                                let enabled = !item.enabled;
                                                spawn(async move {
                                                    match api::set_service_account_enabled(&id, enabled).await {
                                                        Ok(()) => {
                                                            state.toast(ToastKind::Success, if enabled { "Enabled" } else { "Disabled" });
                                                            reload.call(());
                                                        }
                                                        Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                                    }
                                                });
                                            },
                                            if sa.enabled { "Disable" } else { "Enable" }
                                        }
                                        button {
                                            class: "btn btn-sm btn-outline",
                                            r#type: "button",
                                            onclick: move |_| {
                                                let id = item2.id.clone();
                                                spawn(async move {
                                                    match api::rotate_service_account_secret(&id).await {
                                                        Ok(raw) => {
                                                            secret.set(Some(raw));
                                                            state.toast(ToastKind::Success, "Secret rotated");
                                                        }
                                                        Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                                    }
                                                });
                                            },
                                            "Rotate secret"
                                        }
                                        button {
                                            class: "btn btn-sm btn-danger",
                                            r#type: "button",
                                            onclick: move |_| delete_target.set(Some(item3.clone())),
                                            "Delete"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Modal {
            title: "Create service account".to_string(),
            open: show_create(),
            on_close: move |_| show_create.set(false),
            TextInput {
                label: "Name",
                value: name(),
                oninput: move |v| name.set(v),
            }
            TextInput {
                label: "Description",
                value: description(),
                oninput: move |v| description.set(v),
            }
            div { class: "modal-footer", style: "margin-top:1rem; padding:0; border:none; background:transparent;",
                button { class: "btn btn-outline", r#type: "button",
                    onclick: move |_| show_create.set(false), "Cancel" }
                button {
                    class: "btn btn-primary", r#type: "button",
                    onclick: move |_| {
                        let n = name();
                        let d = description();
                        spawn(async move {
                            match api::create_service_account(
                                &n, Some(d.as_str()).filter(|s| !s.is_empty())
                            ).await {
                                Ok(_) => {
                                    state.toast(ToastKind::Success, "Service account created");
                                    show_create.set(false);
                                    name.set(String::new());
                                    description.set(String::new());
                                    reload.call(());
                                }
                                Err(e) => state.toast(ToastKind::Error, e.to_string()),
                            }
                        });
                    },
                    "Create"
                }
            }
        }

        Modal {
            title: "Service account secret".to_string(),
            open: secret().is_some(),
            on_close: move |_| secret.set(None),
            div { class: "alert alert-warning",
                "Store this secret securely — it will not be shown again."
            }
            if let Some(raw) = secret() {
                div { class: "secret-box",
                    code { "{raw}" }
                    button {
                        class: "btn btn-sm btn-outline", r#type: "button",
                        onclick: move |_| {
                            let token = raw.clone();
                            spawn(async move {
                                if let Some(window) = web_sys::window() {
                                    let _ = wasm_bindgen_futures::JsFuture::from(
                                        window.navigator().clipboard().write_text(&token)
                                    ).await;
                                }
                                state.toast(ToastKind::Success, "Copied");
                            });
                        },
                        "Copy"
                    }
                }
            }
            div { class: "modal-footer", style: "margin-top:1rem; padding:0; border:none; background:transparent;",
                button { class: "btn btn-primary", r#type: "button",
                    onclick: move |_| secret.set(None), "Done" }
            }
        }

        ConfirmDialog {
            title: "Delete service account".to_string(),
            message: format!(
                "Delete \"{}\"? This cannot be undone.",
                delete_target().as_ref().map(|s| s.name.as_str()).unwrap_or("")
            ),
            open: delete_target().is_some(),
            confirm_label: "Delete",
            danger: true,
            on_confirm: move |_| {
                if let Some(sa) = delete_target() {
                    spawn(async move {
                        match api::delete_service_account(&sa.id).await {
                            Ok(()) => {
                                state.toast(ToastKind::Success, "Deleted");
                                delete_target.set(None);
                                reload.call(());
                            }
                            Err(e) => state.toast(ToastKind::Error, e.to_string()),
                        }
                    });
                }
            },
            on_cancel: move |_| delete_target.set(None),
        }
    }
}
