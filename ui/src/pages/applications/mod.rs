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
    let can_manage = auth().has_permission("applications:manage");

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
    let mut description = use_signal(String::new);
    let mut redirect_urls_raw = use_signal(String::new);
    let mut scopes_raw = use_signal(String::new);

    let mut one_time_secret = use_signal(|| Option::<(ApplicationView, String)>::None);
    let mut rotate_target = use_signal(|| Option::<ApplicationView>::None);
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
        .filter(|a| matches_query(&a.name, &query()) || matches_query(&a.slug, &query()) || matches_query(&a.client_id, &query()))
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
                        let app_rotate = a.clone();
                        let app_delete = a.clone();
                        rsx! {
                            tr { key: "{a.id}",
                                td {
                                    strong { "{a.name}" }
                                    if let Some(desc) = &a.description {
                                        div { class: "text-muted", style: "font-size: 0.8rem;", "{desc}" }
                                    }
                                }
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
                                        div { class: "actions", style: "display: inline-flex; gap: 0.25rem;",
                                            button {
                                                class: "btn btn-sm btn-outline",
                                                r#type: "button",
                                                onclick: move |_| rotate_target.set(Some(app_rotate.clone())),
                                                "Rotate Secret"
                                            }
                                            button {
                                                class: "btn btn-sm btn-outline",
                                                r#type: "button",
                                                onclick: move |_| {
                                                    let id = app.id.clone();
                                                    let n = app.name.clone();
                                                    let s = app.slug.clone();
                                                    let desc = app.description.clone();
                                                    let r_urls = if app.redirect_urls.is_empty() { None } else { Some(app.redirect_urls.clone()) };
                                                    let sc = if app.scopes.is_empty() { None } else { Some(app.scopes.clone()) };
                                                    let enabled = !app.enabled;
                                                    spawn(async move {
                                                        match api::update_application(&id, &n, &s, desc.as_deref(), r_urls, sc, enabled).await {
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
                                                onclick: move |_| delete_target.set(Some(app_delete.clone())),
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
                        slug.set(slugify(&v));
                    }
                },
            }
            TextInput {
                label: "Slug",
                value: slug(),
                oninput: move |v| slug.set(v),
            }
            TextInput {
                label: "Description (optional)",
                value: description(),
                oninput: move |v| description.set(v),
            }
            TextInput {
                label: "Redirect URLs (comma separated, optional)",
                value: redirect_urls_raw(),
                oninput: move |v| redirect_urls_raw.set(v),
            }
            TextInput {
                label: "Allowed Scopes (comma separated, optional)",
                value: scopes_raw(),
                oninput: move |v| scopes_raw.set(v),
            }
            div { class: "modal-footer", style: "margin-top:1rem; padding:0; border:none; background:transparent;",
                button { class: "btn btn-outline", r#type: "button",
                    onclick: move |_| show_create.set(false), "Cancel" }
                button {
                    class: "btn btn-primary", r#type: "button",
                    onclick: move |_| {
                        let n = name();
                        let s = slug();
                        let d = if description().trim().is_empty() { None } else { Some(description().trim().to_string()) };
                        let r_urls = if redirect_urls_raw().trim().is_empty() {
                            None
                        } else {
                            Some(redirect_urls_raw().split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect::<Vec<_>>())
                        };
                        let sc = if scopes_raw().trim().is_empty() {
                            None
                        } else {
                            Some(scopes_raw().split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect::<Vec<_>>())
                        };
                        spawn(async move {
                            match api::create_application(&n, &s, d.as_deref(), r_urls, sc).await {
                                Ok(res) => {
                                    state.toast(ToastKind::Success, "Application registered successfully");
                                    show_create.set(false);
                                    name.set(String::new());
                                    slug.set(String::new());
                                    description.set(String::new());
                                    redirect_urls_raw.set(String::new());
                                    scopes_raw.set(String::new());
                                    one_time_secret.set(Some((res.application, res.client_secret)));
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

        if let Some((app, sec)) = one_time_secret() {
            Modal {
                title: "Client Credentials Disclosed".to_string(),
                open: true,
                on_close: move |_| one_time_secret.set(None),
                div { class: "alert alert-warning", style: "margin-bottom: 1rem; padding: 0.75rem; border-radius: 4px; background: #fff3cd; color: #856404; border: 1px solid #ffeeba;",
                    strong { "Important: " }
                    "Store this client secret securely. It will never be displayed again after closing this dialog."
                }
                div { style: "display: flex; flex-direction: column; gap: 0.75rem;",
                    div {
                        label { style: "font-weight: 600; display: block; font-size: 0.85rem;", "Application Name" }
                        div { "{app.name}" }
                    }
                    div {
                        label { style: "font-weight: 600; display: block; font-size: 0.85rem;", "Client ID" }
                        div { style: "display: flex; gap: 0.5rem; align-items: center;",
                            code { style: "flex: 1; padding: 0.4rem; background: #f8f9fa; border: 1px solid #e9ecef; border-radius: 4px;", "{app.client_id}" }
                        }
                    }
                    div {
                        label { style: "font-weight: 600; display: block; font-size: 0.85rem;", "Client Secret" }
                        div { style: "display: flex; gap: 0.5rem; align-items: center;",
                            code { style: "flex: 1; padding: 0.4rem; background: #f8f9fa; border: 1px solid #e9ecef; border-radius: 4px; color: #d63384; word-break: break-all;", "{sec}" }
                        }
                    }
                }
                div { class: "modal-footer", style: "margin-top:1.5rem; padding:0; border:none; background:transparent; justify-content: flex-end;",
                    button {
                        class: "btn btn-primary", r#type: "button",
                        onclick: move |_| one_time_secret.set(None),
                        "I have saved my secret"
                    }
                }
            }
        }

        ConfirmDialog {
            title: "Rotate Client Secret".to_string(),
            message: format!(
                "Are you sure you want to rotate the client secret for \"{}\"? Any existing client using the current secret will be invalidated immediately.",
                rotate_target().as_ref().map(|a| a.name.as_str()).unwrap_or("")
            ),
            open: rotate_target().is_some(),
            confirm_label: "Rotate Secret",
            danger: true,
            on_confirm: move |_| {
                if let Some(a) = rotate_target() {
                    let target_app = a.clone();
                    spawn(async move {
                        match api::rotate_application_secret(&target_app.id).await {
                            Ok(new_sec) => {
                                state.toast(ToastKind::Success, "Client secret rotated");
                                rotate_target.set(None);
                                one_time_secret.set(Some((target_app, new_sec)));
                                reload.call(());
                            }
                            Err(e) => state.toast(ToastKind::Error, e.to_string()),
                        }
                    });
                }
            },
            on_cancel: move |_| rotate_target.set(None),
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
