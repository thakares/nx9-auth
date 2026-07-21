//! Applications CRUD.

use crate::components::feedback::{ConfirmDialog, EmptyState, ErrorState, LoadingSpinner, Modal};
use crate::components::forms::TextInput;
use crate::components::navigation::Breadcrumb;
use crate::components::tables::{DataTable, ColumnDef};
use crate::components::widgets::StatusChip;
use crate::models::ApplicationView;
use crate::routes::Route;
use crate::services::api;
use crate::state::{AppState, ToastKind};
use crate::utils::{format_datetime, matches_query, slugify};
use dioxus::prelude::*;

#[component]
pub fn ApplicationsPage() -> Element {
    let state = use_context::<AppState>();
    let auth = state.auth;
    let can_manage = auth().has_permission("roles:manage");

    let mut apps = use_signal(Vec::<ApplicationView>::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| true);
    let mut query = use_signal(String::new);
    let mut page = use_signal(|| 0usize);
    let page_size = 10usize;
    let mut sort_key = use_signal(|| "name".to_string());

    let mut show_create = use_signal(|| false);
    let mut name = use_signal(String::new);
    let mut slug = use_signal(String::new);
    let mut delete_target = use_signal(|| Option::<ApplicationView>::None);

    let reload = use_callback(move |_: ()| {
        loading.set(true);
        spawn(async move {
            match api::list_applications().await {
                Ok(list) => {
                    apps.set(list);
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

    let mut filtered: Vec<_> = apps()
        .into_iter()
        .filter(|a| matches_query(&a.name, &query()) || matches_query(&a.slug, &query()))
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
            ("Applications".to_string(), None),
        ]}

        div { class: "page-header",
            div {
                h1 { "Applications" }
                p { class: "desc", "Registered applications in the NX9 ecosystem" }
            }
            if can_manage {
                button {
                    class: "btn btn-primary", r#type: "button",
                    onclick: move |_| show_create.set(true),
                    "+ Create application"
                }
            }
        }

        if loading() {
            LoadingSpinner {}
        } else if let Some(err) = error() {
            ErrorState { message: err, on_retry: move |_| reload.call(()) }
        } else if apps().is_empty() && query().is_empty() {
            EmptyState {
                title: "No applications".to_string(),
                description: "Applications appear here once registered.",
                icon: "▦",
            }
        } else {
            DataTable {
                columns: {
                    let mut cols = vec![
                        ColumnDef { key: "name".into(), label: "Name".into(), sortable: true, visible: true },
                        ColumnDef { key: "client_id".into(), label: "Client ID".into(), sortable: false, visible: true },
                        ColumnDef { key: "redirect".into(), label: "Redirect URLs".into(), sortable: false, visible: true },
                        ColumnDef { key: "scopes".into(), label: "Scopes".into(), sortable: false, visible: true },
                        ColumnDef { key: "status".into(), label: "Status".into(), sortable: true, visible: true },
                        ColumnDef { key: "created".into(), label: "Created".into(), sortable: true, visible: true },
                    ];
                    if can_manage {
                        cols.push(ColumnDef { key: "actions".into(), label: "Actions".into(), sortable: false, visible: true });
                    }
                    cols
                },
                on_search: move |v| { query.set(v); page.set(0); },
                search_value: query(),
                search_placeholder: "Search applications…".to_string(),
                on_sort: move |k| sort_key.set(k),
                sort_key: sort_key(),
                on_page: move |p| page.set(p),
                page: page(),
                page_size: page_size,
                total: total,
                toolbar_actions: rsx! {
                    button { class: "btn btn-outline", r#type: "button", onclick: move |_| reload.call(()), "Refresh" }
                },
                for a in page_items {
                    {
                        let app = a.clone();
                        let app2 = a.clone();
                        rsx! {
                            tr { key: "{a.id}",
                                td { strong { "{a.name}" } }
                                td { code { "{a.client_id}" } }
                                td { class: "text-muted",
                                    if a.redirect_urls.is_empty() { "—" } else { "{a.redirect_urls.join(\", \")}" }
                                }
                                td { class: "text-muted",
                                    if a.scopes.is_empty() { "—" } else { "{a.scopes.join(\" \")}" }
                                }
                                td {
                                    StatusChip {
                                        status: if a.enabled { "active".to_string() } else { "disabled".to_string() }
                                    }
                                }
                                td { "{format_datetime(&a.created_at)}" }
                                if can_manage {
                                    td { style: "text-align: right;",
                                        div { class: "actions",
                                            button {
                                                class: "btn btn-sm btn-outline",
                                                r#type: "button",
                                                onclick: move |_| {
                                                    let id = app.id.clone();
                                                    let name = app.name.clone();
                                                    let slug = app.slug.clone();
                                                    let enabled = !app.enabled;
                                                    spawn(async move {
                                                        match api::update_application(&id, &name, &slug, enabled).await {
                                                            Ok(_) => {
                                                                state.toast(ToastKind::Success, if enabled { "Enabled" } else { "Disabled" });
                                                                reload.call(());
                                                            }
                                                            Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                                        }
                                                    });
                                                },
                                                if a.enabled { "Disable" } else { "Enable" }
                                            }
                                            button {
                                                class: "btn btn-sm btn-danger",
                                                r#type: "button",
                                                onclick: move |_| delete_target.set(Some(app2.clone())),
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
        }

        Modal {
            title: "Create application".to_string(),
            open: show_create(),
            on_close: move |_| show_create.set(false),
            TextInput {
                label: "Name",
                value: name(),
                oninput: move |v: String| {
                    name.set(v.clone());
                    if slug().is_empty() || slug() == slugify(&name()) {
                        // keep in sync when empty-ish
                    }
                    slug.set(slugify(&v));
                },
            }
            TextInput {
                label: "Slug / Client ID",
                value: slug(),
                oninput: move |v| slug.set(v),
            }
            div { class: "modal-footer", style: "margin-top:1rem; padding:0; border:none; background:transparent;",
                button { class: "btn btn-outline", r#type: "button",
                    onclick: move |_| show_create.set(false), "Cancel" }
                button {
                    class: "btn btn-primary", r#type: "button",
                    onclick: move |_| {
                        let n = name();
                        let s = slug();
                        spawn(async move {
                            match api::create_application(&n, &s).await {
                                Ok(_) => {
                                    state.toast(ToastKind::Success, "Application created");
                                    show_create.set(false);
                                    name.set(String::new());
                                    slug.set(String::new());
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

        ConfirmDialog {
            title: "Delete application".to_string(),
            message: format!(
                "Delete application \"{}\"?",
                delete_target().as_ref().map(|a| a.name.as_str()).unwrap_or("")
            ),
            open: delete_target().is_some(),
            confirm_label: "Delete",
            danger: true,
            on_confirm: move |_| {
                if let Some(a) = delete_target() {
                    spawn(async move {
                        match api::delete_application(&a.id).await {
                            Ok(()) => {
                                state.toast(ToastKind::Success, "Application deleted");
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
