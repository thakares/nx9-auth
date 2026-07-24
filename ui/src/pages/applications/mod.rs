//! Applications CRUD and application membership management.

use crate::components::feedback::{ConfirmDialog, EmptyState, ErrorState, LoadingSpinner, Modal};
use crate::components::forms::{PasswordInput, TextInput};
use crate::components::navigation::Breadcrumb;
use crate::components::tables::{ColumnDef, DataTable};
use crate::components::widgets::StatusChip;
use crate::models::{ApplicationMemberView, ApplicationView, AuditEntry, UserView};
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
    use_effect(move || {
        reload.call(());
    });

    use_effect(move || {
        if can_manage && crate::utils::check_and_clear_create_intent() {
            show_create.set(true);
        }
    });

    let mut filtered: Vec<_> = apps()
        .into_iter()
        .filter(|a| {
            matches_query(&a.name, &query())
                || matches_query(&a.slug, &query())
                || matches_query(&a.client_id, &query())
        })
        .collect();
    let sk = sort_key();
    filtered.sort_by(|a, b| match sk.as_str() {
        "status" => b.enabled.cmp(&a.enabled),
        "created" => b.created_at.cmp(&a.created_at),
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    let total = filtered.len();
    let page_items: Vec<_> = filtered
        .into_iter()
        .skip(page() * page_size)
        .take(page_size)
        .collect();

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
                columns: vec![
                    ColumnDef { key: "name".into(), label: "Name".into(), sortable: true, visible: true },
                    ColumnDef { key: "client_id".into(), label: "Client ID".into(), sortable: false, visible: true },
                    ColumnDef { key: "redirect".into(), label: "Redirect URLs".into(), sortable: false, visible: true },
                    ColumnDef { key: "scopes".into(), label: "Scopes".into(), sortable: false, visible: true },
                    ColumnDef { key: "status".into(), label: "Status".into(), sortable: true, visible: true },
                    ColumnDef { key: "created".into(), label: "Created".into(), sortable: true, visible: true },
                    ColumnDef { key: "actions".into(), label: "Actions".into(), sortable: false, visible: true },
                ],
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
                        let app_id = a.id.clone();
                        rsx! {
                            tr { key: "{a.id}",
                                td {
                                    Link {
                                        to: Route::ApplicationDetailPage { id: a.id.clone() },
                                        strong { "{a.name}" }
                                    }
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
                                td { style: "text-align: right;",
                                    div { class: "actions", style: "display: inline-flex; gap: 0.25rem;",
                                        Link {
                                            class: "btn btn-sm btn-outline",
                                            to: Route::ApplicationDetailPage { id: app_id },
                                            "Manage"
                                        }
                                        if can_manage {
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

#[component]
pub fn ApplicationDetailPage(id: String) -> Element {
    let state = use_context::<AppState>();
    let auth = state.auth;
    let can_manage = auth().has_permission("applications:manage");
    let can_view_audit = auth().has_permission("audit:view");

    let mut app = use_signal(|| Option::<ApplicationView>::None);
    let mut members = use_signal(Vec::<ApplicationMemberView>::new);
    let mut all_users = use_signal(Vec::<UserView>::new);
    let mut activity = use_signal(Vec::<AuditEntry>::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| true);
    let mut tab = use_signal(|| "overview".to_string());

    // Edit configuration
    let mut edit_mode = use_signal(|| false);
    let mut edit_name = use_signal(String::new);
    let mut edit_slug = use_signal(String::new);
    let mut edit_desc = use_signal(String::new);
    let mut edit_redirects = use_signal(String::new);
    let mut edit_scopes = use_signal(String::new);
    let mut edit_enabled = use_signal(|| true);

    // Members
    let mut show_add_member = use_signal(|| false);
    let mut add_mode = use_signal(|| "existing".to_string()); // "existing" | "new"
    let mut add_user_id = use_signal(String::new);
    let mut add_role = use_signal(|| "member".to_string());
    let mut new_username = use_signal(String::new);
    let mut new_password = use_signal(String::new);
    let mut add_form_error = use_signal(|| Option::<String>::None);
    let mut add_busy = use_signal(|| false);
    // After user create succeeds but membership fails: recoverable assignment state.
    let mut pending_assign_user_id = use_signal(|| Option::<String>::None);
    let mut pending_assign_username = use_signal(|| Option::<String>::None);
    let mut remove_target = use_signal(|| Option::<ApplicationMemberView>::None);
    let mut role_target = use_signal(|| Option::<(ApplicationMemberView, String)>::None);
    let mut change_role_value = use_signal(|| "member".to_string());

    // Credentials
    let mut one_time_secret = use_signal(|| Option::<String>::None);
    let mut rotate_confirm = use_signal(|| false);
    let mut delete_confirm = use_signal(|| false);

    let app_id = id.clone();
    let reload = use_callback(move |_: ()| {
        let id = app_id.clone();
        loading.set(true);
        // Clear prior error so a successful retry is not blocked by ErrorState.
        error.set(None);
        spawn(async move {
            match api::get_application(&id).await {
                Ok(a) => {
                    app.set(Some(a));
                    // On failure, clear members so the UI never shows stale memberships.
                    match api::list_application_members(&id).await {
                        Ok(m) => members.set(m),
                        Err(_) => members.set(Vec::new()),
                    }
                    if let Ok(users) = api::list_users().await {
                        all_users.set(users);
                    }
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

    let app_id_activity = id.clone();
    let load_activity = use_callback(move |_: ()| {
        if !can_view_audit {
            return;
        }
        let id = app_id_activity.clone();
        spawn(async move {
            let q = format!("resource_type=application&q={id}&limit=50");
            if let Ok(resp) = api::list_audit(&q).await {
                let filtered: Vec<_> = resp
                    .entries
                    .into_iter()
                    .filter(|e| e.resource_id.as_deref() == Some(id.as_str()))
                    .collect();
                activity.set(filtered);
            }
        });
    });

    rsx! {
        Breadcrumb { items: vec![
            ("Dashboard".to_string(), Some(Route::DashboardPage {})),
            ("Applications".to_string(), Some(Route::ApplicationsPage {})),
            (app().map(|a| a.name.clone()).unwrap_or_else(|| id.clone()), None),
        ]}

        if loading() {
            LoadingSpinner {}
        } else if let Some(err) = error() {
            ErrorState { message: err, on_retry: move |_| reload.call(()) }
        } else if let Some(a) = app() {
            {
                let app_for_actions = a.clone();
                let app_for_edit = a.clone();
                let app_id = a.id.clone();
                let app_id2 = a.id.clone();
                let app_id2b = a.id.clone();
                let app_id2c = a.id.clone();
                let app_id3 = a.id.clone();
                let app_id4 = a.id.clone();
                let app_id_remove = a.id.clone();
                let app_id_rotate = a.id.clone();
                let app_id_delete = a.id.clone();
                let app_name = a.name.clone();
                rsx! {
                    div { class: "page-header",
                        div {
                            h1 { "{a.name}" }
                            p { class: "desc",
                                code { "{a.slug}" }
                                " · "
                                StatusChip {
                                    status: if a.enabled { "active".to_string() } else { "disabled".to_string() }
                                }
                            }
                        }
                        if can_manage {
                            div { class: "row",
                                button {
                                    class: "btn btn-outline",
                                    r#type: "button",
                                    onclick: move |_| {
                                        edit_name.set(app_for_edit.name.clone());
                                        edit_slug.set(app_for_edit.slug.clone());
                                        edit_desc.set(app_for_edit.description.clone().unwrap_or_default());
                                        edit_redirects.set(app_for_edit.redirect_urls.join(", "));
                                        edit_scopes.set(app_for_edit.scopes.join(", "));
                                        edit_enabled.set(app_for_edit.enabled);
                                        edit_mode.set(true);
                                    },
                                    "Edit"
                                }
                                button {
                                    class: "btn btn-danger",
                                    r#type: "button",
                                    onclick: move |_| delete_confirm.set(true),
                                    "Delete"
                                }
                            }
                        }
                    }

                    div { class: "tabs", style: "display:flex; gap:0.5rem; margin-bottom:1rem; flex-wrap:wrap;",
                        button {
                            class: if tab() == "overview" { "btn btn-sm btn-primary" } else { "btn btn-sm btn-outline" },
                            r#type: "button",
                            onclick: move |_| tab.set("overview".into()),
                            "Overview"
                        }
                        button {
                            class: if tab() == "users" { "btn btn-sm btn-primary" } else { "btn btn-sm btn-outline" },
                            r#type: "button",
                            onclick: move |_| tab.set("users".into()),
                            "Users"
                        }
                        button {
                            class: if tab() == "credentials" { "btn btn-sm btn-primary" } else { "btn btn-sm btn-outline" },
                            r#type: "button",
                            onclick: move |_| tab.set("credentials".into()),
                            "Credentials"
                        }
                        button {
                            class: if tab() == "configuration" { "btn btn-sm btn-primary" } else { "btn btn-sm btn-outline" },
                            r#type: "button",
                            onclick: move |_| tab.set("configuration".into()),
                            "Configuration"
                        }
                        if can_view_audit {
                            button {
                                class: if tab() == "activity" { "btn btn-sm btn-primary" } else { "btn btn-sm btn-outline" },
                                r#type: "button",
                                onclick: move |_| {
                                    tab.set("activity".into());
                                    load_activity.call(());
                                },
                                "Activity"
                            }
                        }
                    }

                    // ── Overview ──────────────────────────────────────────
                    if tab() == "overview" {
                        div { class: "grid-2",
                            div { class: "card",
                                div { class: "card-header", h3 { "Application" } }
                                div { class: "card-body stack",
                                    div { strong { "Name: " } "{a.name}" }
                                    div { strong { "Slug: " } code { "{a.slug}" } }
                                    div {
                                        strong { "Status: " }
                                        StatusChip {
                                            status: if a.enabled { "active".to_string() } else { "disabled".to_string() }
                                        }
                                    }
                                    div {
                                        strong { "Description: " }
                                        span { class: "text-secondary",
                                            "{a.description.as_deref().unwrap_or(\"—\")}"
                                        }
                                    }
                                    div { class: "text-secondary", "Created: {format_datetime(&a.created_at)}" }
                                    div { class: "text-secondary", "Updated: {format_datetime(&a.updated_at)}" }
                                }
                            }
                            div { class: "card",
                                div { class: "card-header", h3 { "Registration summary" } }
                                div { class: "card-body stack",
                                    div {
                                        strong { "Client ID: " }
                                        code { "{a.client_id}" }
                                    }
                                    div {
                                        strong { "Credentials: " }
                                        if a.credentials_configured {
                                            span { class: "badge badge-accent", "Configured" }
                                        } else {
                                            span { class: "badge", "Not configured" }
                                        }
                                    }
                                    div {
                                        strong { "Members: " }
                                        span { class: "badge", "{members().len()}" }
                                    }
                                    div {
                                        strong { "Redirect URLs: " }
                                        if a.redirect_urls.is_empty() {
                                            span { class: "text-muted", "—" }
                                        } else {
                                            span { class: "text-secondary", "{a.redirect_urls.join(\", \")}" }
                                        }
                                    }
                                    div {
                                        strong { "Scopes: " }
                                        if a.scopes.is_empty() {
                                            span { class: "text-muted", "—" }
                                        } else {
                                            span { class: "text-secondary", "{a.scopes.join(\" \")}" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ── Users (membership) ────────────────────────────────
                    if tab() == "users" {
                        div { class: "card",
                            div { class: "card-header", style: "display:flex; justify-content:space-between; align-items:center;",
                                h3 { "Application users" }
                                if can_manage {
                                    button {
                                        class: "btn btn-sm btn-primary",
                                        r#type: "button",
                                        onclick: move |_| {
                                            add_mode.set("existing".into());
                                            add_user_id.set(String::new());
                                            add_role.set("member".into());
                                            new_username.set(String::new());
                                            new_password.set(String::new());
                                            add_form_error.set(None);
                                            add_busy.set(false);
                                            pending_assign_user_id.set(None);
                                            pending_assign_username.set(None);
                                            show_add_member.set(true);
                                        },
                                        "Add User"
                                    }
                                }
                            }
                            div { class: "card-body",
                                p { class: "text-secondary", style: "margin-bottom:1rem; font-size:0.9rem;",
                                    "Assign existing NX9-Auth users, or create a new user and assign them in one step. "
                                    "Membership roles (owner/admin/member) are metadata only and do not grant global administrative permissions."
                                }
                                if members().is_empty() {
                                    p { class: "text-muted", "No users assigned to this application." }
                                } else {
                                    DataTable {
                                        columns: vec![
                                            ColumnDef { key: "user".into(), label: "User".into(), sortable: false, visible: true },
                                            ColumnDef { key: "username".into(), label: "Username".into(), sortable: false, visible: true },
                                            ColumnDef { key: "role".into(), label: "Membership Role".into(), sortable: false, visible: true },
                                            ColumnDef { key: "status".into(), label: "Status".into(), sortable: false, visible: true },
                                            ColumnDef { key: "assigned".into(), label: "Assigned".into(), sortable: false, visible: true },
                                            ColumnDef { key: "actions".into(), label: "Actions".into(), sortable: false, visible: true },
                                        ],
                                        on_search: |_| {},
                                        search_value: "".to_string(),
                                        search_placeholder: "".to_string(),
                                        on_sort: |_| {},
                                        sort_key: "".to_string(),
                                        on_page: |_| {},
                                        page: 0,
                                        page_size: members().len().max(1),
                                        total: members().len(),
                                        for m in members() {
                                            {
                                                let m_role = m.clone();
                                                let m_enable = m.clone();
                                                let m_remove = m.clone();
                                                let aid = app_id.clone();
                                                rsx! {
                                                    tr { key: "{m.id}",
                                                        td {
                                                            Link {
                                                                to: Route::UserDetailPage { id: m.user_id.clone() },
                                                                "{m.username}"
                                                            }
                                                        }
                                                        td { code { "{m.username}" } }
                                                        td { span { class: "badge badge-accent", "{m.role}" } }
                                                        td {
                                                            StatusChip {
                                                                status: if m.enabled { "active".to_string() } else { "disabled".to_string() }
                                                            }
                                                            span { class: "text-muted", style: "margin-left:0.35rem; font-size:0.8rem;",
                                                                "(user: {m.user_status})"
                                                            }
                                                        }
                                                        td { "{format_datetime(&m.created_at)}" }
                                                        td { style: "text-align: right;",
                                                            if can_manage {
                                                                div { class: "actions", style: "display:inline-flex; gap:0.25rem; flex-wrap:wrap;",
                                                                    button {
                                                                        class: "btn btn-sm btn-outline",
                                                                        r#type: "button",
                                                                        onclick: move |_| {
                                                                            change_role_value.set(m_role.role.clone());
                                                                            role_target.set(Some((m_role.clone(), m_role.role.clone())));
                                                                        },
                                                                        "Change Role"
                                                                    }
                                                                    button {
                                                                        class: "btn btn-sm btn-outline",
                                                                        r#type: "button",
                                                                        onclick: {
                                                                            let aid = aid.clone();
                                                                            let m = m_enable.clone();
                                                                            move |_| {
                                                                                let aid = aid.clone();
                                                                                let uid = m.user_id.clone();
                                                                                let enabled = !m.enabled;
                                                                                spawn(async move {
                                                                                    match api::update_application_member(&aid, &uid, None, Some(enabled)).await {
                                                                                        Ok(_) => {
                                                                                            state.toast(
                                                                                                ToastKind::Success,
                                                                                                if enabled { "Membership enabled" } else { "Membership disabled" },
                                                                                            );
                                                                                            reload.call(());
                                                                                        }
                                                                                        Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                                                                    }
                                                                                });
                                                                            }
                                                                        },
                                                                        if m.enabled { "Disable" } else { "Enable" }
                                                                    }
                                                                    button {
                                                                        class: "btn btn-sm btn-danger",
                                                                        r#type: "button",
                                                                        onclick: move |_| remove_target.set(Some(m_remove.clone())),
                                                                        "Remove"
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

                    // ── Credentials ───────────────────────────────────────
                    if tab() == "credentials" {
                        div { class: "card",
                            div { class: "card-header", h3 { "Client credentials" } }
                            div { class: "card-body stack",
                                div {
                                    label { style: "font-weight:600; display:block; font-size:0.85rem;", "Client ID (immutable)" }
                                    div { style: "display:flex; gap:0.5rem; align-items:center;",
                                        code { style: "flex:1; padding:0.4rem; background:#f8f9fa; border:1px solid #e9ecef; border-radius:4px; word-break:break-all;",
                                            "{a.client_id}"
                                        }
                                        button {
                                            class: "btn btn-sm btn-outline",
                                            r#type: "button",
                                            onclick: {
                                                let cid = a.client_id.clone();
                                                move |_| {
                                                    if let Some(window) = web_sys::window() {
                                                        let nav = window.navigator();
                                                        let clipboard = nav.clipboard();
                                                        let _ = clipboard.write_text(&cid);
                                                    }
                                                    state.toast(ToastKind::Success, "Client ID copied");
                                                }
                                            },
                                            "Copy"
                                        }
                                    }
                                }
                                div {
                                    label { style: "font-weight:600; display:block; font-size:0.85rem;", "Credentials status" }
                                    if a.credentials_configured {
                                        span { class: "badge badge-accent", "Configured" }
                                    } else {
                                        span { class: "badge", "Not configured" }
                                    }
                                    p { class: "text-muted", style: "font-size:0.85rem; margin-top:0.35rem;",
                                        "Client secrets are shown only once at creation or rotation. The secret hash is never exposed."
                                    }
                                }
                                if can_manage {
                                    button {
                                        class: "btn btn-outline",
                                        r#type: "button",
                                        onclick: move |_| rotate_confirm.set(true),
                                        "Rotate Secret"
                                    }
                                }
                            }
                        }
                    }

                    // ── Configuration ─────────────────────────────────────
                    if tab() == "configuration" {
                        div { class: "card",
                            div { class: "card-header", style: "display:flex; justify-content:space-between; align-items:center;",
                                h3 { "Configuration" }
                                if can_manage {
                                    button {
                                        class: "btn btn-sm btn-outline",
                                        r#type: "button",
                                        onclick: move |_| {
                                            edit_name.set(app_for_actions.name.clone());
                                            edit_slug.set(app_for_actions.slug.clone());
                                            edit_desc.set(app_for_actions.description.clone().unwrap_or_default());
                                            edit_redirects.set(app_for_actions.redirect_urls.join(", "));
                                            edit_scopes.set(app_for_actions.scopes.join(", "));
                                            edit_enabled.set(app_for_actions.enabled);
                                            edit_mode.set(true);
                                        },
                                        "Edit configuration"
                                    }
                                }
                            }
                            div { class: "card-body stack",
                                div { strong { "Name: " } "{a.name}" }
                                div { strong { "Slug: " } code { "{a.slug}" } }
                                div {
                                    strong { "Enabled: " }
                                    StatusChip {
                                        status: if a.enabled { "active".to_string() } else { "disabled".to_string() }
                                    }
                                }
                                div {
                                    strong { "Description: " }
                                    "{a.description.as_deref().unwrap_or(\"—\")}"
                                }
                                div {
                                    strong { "Redirect URIs: " }
                                    if a.redirect_urls.is_empty() {
                                        span { class: "text-muted", "—" }
                                    } else {
                                        ul {
                                            for u in a.redirect_urls.iter() {
                                                li { code { "{u}" } }
                                            }
                                        }
                                    }
                                }
                                div {
                                    strong { "Scopes: " }
                                    if a.scopes.is_empty() {
                                        span { class: "text-muted", "—" }
                                    } else {
                                        span { "{a.scopes.join(\" \")}" }
                                    }
                                }
                                div {
                                    strong { "Client ID: " }
                                    code { "{a.client_id}" }
                                    span { class: "text-muted", style: "margin-left:0.5rem;", "(immutable)" }
                                }
                            }
                        }
                    }

                    // ── Activity ──────────────────────────────────────────
                    if tab() == "activity" && can_view_audit {
                        div { class: "card",
                            div { class: "card-header", style: "display:flex; justify-content:space-between; align-items:center;",
                                h3 { "Activity" }
                                button {
                                    class: "btn btn-sm btn-outline",
                                    r#type: "button",
                                    onclick: move |_| load_activity.call(()),
                                    "Refresh"
                                }
                            }
                            div { class: "card-body",
                                if activity().is_empty() {
                                    p { class: "text-muted", "No application-related audit events found." }
                                } else {
                                    DataTable {
                                        columns: vec![
                                            ColumnDef { key: "time".into(), label: "Time".into(), sortable: false, visible: true },
                                            ColumnDef { key: "action".into(), label: "Action".into(), sortable: false, visible: true },
                                            ColumnDef { key: "severity".into(), label: "Severity".into(), sortable: false, visible: true },
                                            ColumnDef { key: "target".into(), label: "Target".into(), sortable: false, visible: true },
                                        ],
                                        on_search: |_| {},
                                        search_value: "".to_string(),
                                        search_placeholder: "".to_string(),
                                        on_sort: |_| {},
                                        sort_key: "".to_string(),
                                        on_page: |_| {},
                                        page: 0,
                                        page_size: activity().len().max(1),
                                        total: activity().len(),
                                        for e in activity() {
                                            tr { key: "{e.id}",
                                                td { "{format_datetime(&e.created_at)}" }
                                                td { code { "{e.action}" } }
                                                td { "{e.severity}" }
                                                td { class: "text-muted",
                                                    "{e.target_user_id.as_deref().unwrap_or(\"—\")}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ── Add member modal ──────────────────────────────────
                    Modal {
                        title: "Add user to application".to_string(),
                        open: show_add_member(),
                        on_close: move |_| {
                            show_add_member.set(false);
                            new_password.set(String::new());
                            add_form_error.set(None);
                            add_busy.set(false);
                        },
                        // Mode selector
                        div { class: "row", style: "gap:0.5rem; margin-bottom:0.75rem;",
                            button {
                                class: if add_mode() == "existing" { "btn btn-sm btn-primary" } else { "btn btn-sm btn-outline" },
                                r#type: "button",
                                disabled: add_busy(),
                                onclick: move |_| {
                                    add_mode.set("existing".into());
                                    add_form_error.set(None);
                                },
                                "Existing User"
                            }
                            button {
                                class: if add_mode() == "new" { "btn btn-sm btn-primary" } else { "btn btn-sm btn-outline" },
                                r#type: "button",
                                disabled: add_busy(),
                                onclick: move |_| {
                                    add_mode.set("new".into());
                                    add_form_error.set(None);
                                },
                                "New User"
                            }
                        }

                        if let Some(err) = add_form_error() {
                            div {
                                class: "alert alert-warning",
                                style: "margin-bottom:0.75rem; padding:0.6rem; border-radius:4px; background:#fff3cd; color:#856404; border:1px solid #ffeeba; font-size:0.9rem;",
                                "{err}"
                            }
                        }

                        if let Some(uname) = pending_assign_username() {
                            if pending_assign_user_id().is_some() {
                                p { class: "text-secondary", style: "margin-bottom:0.75rem; font-size:0.9rem;",
                                    "User "
                                    strong { "{uname}" }
                                    " was created. Retry assignment or switch to Existing User mode."
                                }
                            }
                        }

                        // ── Existing User mode ────────────────────────────
                        if add_mode() == "existing" {
                            p { class: "text-secondary", style: "margin-bottom:0.75rem; font-size:0.9rem;",
                                "Select an existing NX9-Auth user. This does not create a new user account."
                            }
                            div { class: "stack",
                                label { style: "font-weight:600; font-size:0.85rem;", "User" }
                                select {
                                    class: "form-control",
                                    value: "{add_user_id()}",
                                    disabled: add_busy(),
                                    onchange: move |e| {
                                        add_user_id.set(e.value());
                                        add_form_error.set(None);
                                    },
                                    option { value: "", "Select user…" }
                                    for u in all_users() {
                                        if !members().iter().any(|m| m.user_id == u.id) {
                                            option { value: "{u.id}", "{u.username} ({u.status})" }
                                        }
                                    }
                                }
                                label { style: "font-weight:600; font-size:0.85rem; margin-top:0.75rem;", "Membership role" }
                                select {
                                    class: "form-control",
                                    value: "{add_role()}",
                                    disabled: add_busy(),
                                    onchange: move |e| add_role.set(e.value()),
                                    option { value: "member", "member" }
                                    option { value: "admin", "admin" }
                                    option { value: "owner", "owner" }
                                }
                            }
                            div { class: "modal-footer", style: "margin-top:1rem; padding:0; border:none; background:transparent;",
                                button {
                                    class: "btn btn-outline", r#type: "button",
                                    disabled: add_busy(),
                                    onclick: move |_| {
                                        show_add_member.set(false);
                                        new_password.set(String::new());
                                        add_form_error.set(None);
                                        add_busy.set(false);
                                    },
                                    "Cancel"
                                }
                                button {
                                    class: "btn btn-primary", r#type: "button",
                                    disabled: add_user_id().is_empty() || add_busy(),
                                    onclick: move |_| {
                                        let uid = add_user_id();
                                        let role = add_role();
                                        let aid = app_id2.clone();
                                        add_busy.set(true);
                                        add_form_error.set(None);
                                        spawn(async move {
                                            match api::add_application_member(&aid, &uid, Some(&role)).await {
                                                Ok(_) => {
                                                    state.toast(ToastKind::Success, "User added to application");
                                                    show_add_member.set(false);
                                                    add_user_id.set(String::new());
                                                    add_role.set("member".into());
                                                    pending_assign_user_id.set(None);
                                                    pending_assign_username.set(None);
                                                    add_busy.set(false);
                                                    reload.call(());
                                                }
                                                Err(e) => {
                                                    add_form_error.set(Some(e.to_string()));
                                                    state.toast(ToastKind::Error, e.to_string());
                                                    add_busy.set(false);
                                                }
                                            }
                                        });
                                    },
                                    if add_busy() { "Adding…" } else { "Add User" }
                                }
                            }
                        }

                        // ── New User mode ─────────────────────────────────
                        if add_mode() == "new" {
                            p { class: "text-secondary", style: "margin-bottom:0.75rem; font-size:0.9rem;",
                                "Creates a normal NX9-Auth user account, then assigns them to this application. "
                                "Application membership roles do not grant global administrative permissions."
                            }
                            div { class: "stack",
                                TextInput {
                                    label: "Username",
                                    value: new_username(),
                                    oninput: move |v| {
                                        new_username.set(v);
                                        add_form_error.set(None);
                                    },
                                    required: true,
                                }
                                PasswordInput {
                                    label: "Password",
                                    value: new_password(),
                                    oninput: move |v| {
                                        new_password.set(v);
                                        add_form_error.set(None);
                                    },
                                    required: true,
                                    autocomplete: "new-password",
                                }
                                p { class: "form-hint text-muted", style: "font-size:0.8rem; margin:0;",
                                    "Minimum 8 characters. Avoid common sequences like \"password\"."
                                }
                                label { style: "font-weight:600; font-size:0.85rem; margin-top:0.75rem;", "Membership role" }
                                select {
                                    class: "form-control",
                                    value: "{add_role()}",
                                    disabled: add_busy(),
                                    onchange: move |e| add_role.set(e.value()),
                                    option { value: "member", "member" }
                                    option { value: "admin", "admin" }
                                    option { value: "owner", "owner" }
                                }
                            }
                            div { class: "modal-footer", style: "margin-top:1rem; padding:0; border:none; background:transparent; gap:0.5rem; flex-wrap:wrap;",
                                button {
                                    class: "btn btn-outline", r#type: "button",
                                    disabled: add_busy(),
                                    onclick: move |_| {
                                        show_add_member.set(false);
                                        new_password.set(String::new());
                                        add_form_error.set(None);
                                        add_busy.set(false);
                                    },
                                    "Cancel"
                                }
                                if pending_assign_user_id().is_some() {
                                    button {
                                        class: "btn btn-outline", r#type: "button",
                                        disabled: add_busy(),
                                        onclick: move |_| {
                                            if let Some(uid) = pending_assign_user_id() {
                                                add_user_id.set(uid);
                                            }
                                            add_mode.set("existing".into());
                                            add_form_error.set(None);
                                        },
                                        "Use Existing User mode"
                                    }
                                    button {
                                        class: "btn btn-primary", r#type: "button",
                                        disabled: add_busy() || pending_assign_user_id().is_none(),
                                        onclick: move |_| {
                                            let Some(uid) = pending_assign_user_id() else { return };
                                            let role = add_role();
                                            let aid = app_id2b.clone();
                                            add_busy.set(true);
                                            add_form_error.set(None);
                                            spawn(async move {
                                                match api::add_application_member(&aid, &uid, Some(&role)).await {
                                                    Ok(_) => {
                                                        state.toast(ToastKind::Success, "User added to application");
                                                        show_add_member.set(false);
                                                        new_username.set(String::new());
                                                        new_password.set(String::new());
                                                        add_role.set("member".into());
                                                        pending_assign_user_id.set(None);
                                                        pending_assign_username.set(None);
                                                        add_busy.set(false);
                                                        reload.call(());
                                                    }
                                                    Err(e) => {
                                                        add_form_error.set(Some(format!(
                                                            "User was created successfully, but could not be added to this application. {e}"
                                                        )));
                                                        state.toast(ToastKind::Error, e.to_string());
                                                        add_busy.set(false);
                                                    }
                                                }
                                            });
                                        },
                                        if add_busy() { "Retrying…" } else { "Retry assignment" }
                                    }
                                } else {
                                    button {
                                        class: "btn btn-primary", r#type: "button",
                                        disabled: add_busy()
                                            || new_username().trim().is_empty()
                                            || new_password().len() < 8,
                                        onclick: move |_| {
                                            let username = new_username().trim().to_string();
                                            let password = new_password();
                                            let role = add_role();
                                            let aid = app_id2c.clone();
                                            if username.is_empty() {
                                                add_form_error.set(Some("username is required".into()));
                                                return;
                                            }
                                            if password.len() < 8 {
                                                add_form_error.set(Some(
                                                    "password must be at least 8 characters long".into(),
                                                ));
                                                return;
                                            }
                                            add_busy.set(true);
                                            add_form_error.set(None);
                                            spawn(async move {
                                                // 1) Canonical user creation
                                                let created = match api::create_user(&username, &password).await {
                                                    Ok(u) => u,
                                                    Err(e) => {
                                                        add_form_error.set(Some(e.to_string()));
                                                        state.toast(ToastKind::Error, e.to_string());
                                                        add_busy.set(false);
                                                        return;
                                                    }
                                                };
                                                // Clear password from UI after successful create.
                                                new_password.set(String::new());

                                                // 2) Application membership via existing API
                                                match api::add_application_member(
                                                    &aid,
                                                    &created.id,
                                                    Some(&role),
                                                )
                                                .await
                                                {
                                                    Ok(_) => {
                                                        state.toast(
                                                            ToastKind::Success,
                                                            "User created and added to application",
                                                        );
                                                        show_add_member.set(false);
                                                        new_username.set(String::new());
                                                        add_role.set("member".into());
                                                        pending_assign_user_id.set(None);
                                                        pending_assign_username.set(None);
                                                        add_busy.set(false);
                                                        reload.call(());
                                                    }
                                                    Err(e) => {
                                                        pending_assign_user_id
                                                            .set(Some(created.id.clone()));
                                                        pending_assign_username
                                                            .set(Some(created.username.clone()));
                                                        add_user_id.set(created.id.clone());
                                                        add_form_error.set(Some(format!(
                                                            "User was created successfully, but could not be added to this application. {e}"
                                                        )));
                                                        state.toast(
                                                            ToastKind::Error,
                                                            format!(
                                                                "User created, but assignment failed: {e}"
                                                            ),
                                                        );
                                                        add_busy.set(false);
                                                        // Refresh user list so new user is selectable.
                                                        reload.call(());
                                                    }
                                                }
                                            });
                                        },
                                        if add_busy() { "Creating…" } else { "Create & Add User" }
                                    }
                                }
                            }
                        }
                    }

                    // ── Edit configuration modal ──────────────────────────
                    Modal {
                        title: "Edit application".to_string(),
                        open: edit_mode(),
                        on_close: move |_| edit_mode.set(false),
                        TextInput {
                            label: "Name",
                            value: edit_name(),
                            oninput: move |v| edit_name.set(v),
                        }
                        TextInput {
                            label: "Slug",
                            value: edit_slug(),
                            oninput: move |v| edit_slug.set(v),
                        }
                        TextInput {
                            label: "Description",
                            value: edit_desc(),
                            oninput: move |v| edit_desc.set(v),
                        }
                        TextInput {
                            label: "Redirect URLs (comma separated)",
                            value: edit_redirects(),
                            oninput: move |v| edit_redirects.set(v),
                        }
                        TextInput {
                            label: "Scopes (comma separated)",
                            value: edit_scopes(),
                            oninput: move |v| edit_scopes.set(v),
                        }
                        div { class: "row", style: "align-items:center; gap:0.5rem; margin-top:0.5rem;",
                            input {
                                r#type: "checkbox",
                                checked: edit_enabled(),
                                onchange: move |e| edit_enabled.set(e.checked()),
                            }
                            label { "Enabled" }
                        }
                        p { class: "form-hint text-muted", "Client ID cannot be changed." }
                        div { class: "modal-footer", style: "margin-top:1rem; padding:0; border:none; background:transparent;",
                            button {
                                class: "btn btn-outline", r#type: "button",
                                onclick: move |_| edit_mode.set(false),
                                "Cancel"
                            }
                            button {
                                class: "btn btn-primary", r#type: "button",
                                onclick: move |_| {
                                    let aid = app_id3.clone();
                                    let n = edit_name();
                                    let s = edit_slug();
                                    let d = if edit_desc().trim().is_empty() {
                                        None
                                    } else {
                                        Some(edit_desc().trim().to_string())
                                    };
                                    let r_urls = if edit_redirects().trim().is_empty() {
                                        None
                                    } else {
                                        Some(
                                            edit_redirects()
                                                .split(',')
                                                .map(|x| x.trim().to_string())
                                                .filter(|x| !x.is_empty())
                                                .collect::<Vec<_>>(),
                                        )
                                    };
                                    let sc = if edit_scopes().trim().is_empty() {
                                        None
                                    } else {
                                        Some(
                                            edit_scopes()
                                                .split(',')
                                                .map(|x| x.trim().to_string())
                                                .filter(|x| !x.is_empty())
                                                .collect::<Vec<_>>(),
                                        )
                                    };
                                    let en = edit_enabled();
                                    spawn(async move {
                                        match api::update_application(
                                            &aid,
                                            &n,
                                            &s,
                                            d.as_deref(),
                                            r_urls,
                                            sc,
                                            en,
                                        )
                                        .await
                                        {
                                            Ok(_) => {
                                                state.toast(ToastKind::Success, "Application updated");
                                                edit_mode.set(false);
                                                reload.call(());
                                            }
                                            Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                        }
                                    });
                                },
                                "Save"
                            }
                        }
                    }

                    // ── Role change modal ─────────────────────────────────
                    Modal {
                        title: "Change membership role".to_string(),
                        open: role_target().is_some(),
                        on_close: move |_| role_target.set(None),
                        if let Some((m, _prev)) = role_target() {
                            p {
                                "Change membership role for "
                                strong { "{m.username}" }
                                " from "
                                code { "{m.role}" }
                                " to:"
                            }
                        }
                        select {
                            class: "form-control",
                            value: "{change_role_value()}",
                            onchange: move |e| change_role_value.set(e.value()),
                            option { value: "member", "member" }
                            option { value: "admin", "admin" }
                            option { value: "owner", "owner" }
                        }
                        if change_role_value() == "owner" || role_target().as_ref().map(|(m, _)| m.role.as_str()) == Some("owner") {
                            p { class: "alert alert-warning", style: "margin-top:0.75rem; padding:0.5rem; background:#fff3cd; color:#856404; border-radius:4px; font-size:0.85rem;",
                                "Owner is application membership metadata only. It does not grant global NX9-Auth administrative permissions."
                            }
                        }
                        div { class: "modal-footer", style: "margin-top:1rem; padding:0; border:none; background:transparent;",
                            button {
                                class: "btn btn-outline", r#type: "button",
                                onclick: move |_| role_target.set(None),
                                "Cancel"
                            }
                            button {
                                class: "btn btn-primary", r#type: "button",
                                onclick: move |_| {
                                    if let Some((m, _)) = role_target() {
                                        let aid = app_id4.clone();
                                        let uid = m.user_id.clone();
                                        let role = change_role_value();
                                        spawn(async move {
                                            match api::update_application_member(&aid, &uid, Some(&role), None).await {
                                                Ok(_) => {
                                                    state.toast(ToastKind::Success, "Membership role updated");
                                                    role_target.set(None);
                                                    reload.call(());
                                                }
                                                Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                            }
                                        });
                                    }
                                },
                                "Change Role"
                            }
                        }
                    }

                    // ── Remove confirmation ───────────────────────────────
                    ConfirmDialog {
                        title: "Remove user from application".to_string(),
                        message: format!(
                            "Remove \"{}\" from this application? This revokes the user's membership in this application. It does not delete the NX9-Auth user account.",
                            remove_target().as_ref().map(|m| m.username.as_str()).unwrap_or("")
                        ),
                        open: remove_target().is_some(),
                        confirm_label: "Remove",
                        danger: true,
                        on_confirm: move |_| {
                            if let Some(m) = remove_target() {
                                let aid = app_id_remove.clone();
                                let uid = m.user_id.clone();
                                spawn(async move {
                                    match api::remove_application_member(&aid, &uid).await {
                                        Ok(_) => {
                                            state.toast(ToastKind::Success, "User removed from application");
                                            remove_target.set(None);
                                            reload.call(());
                                        }
                                        Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                    }
                                });
                            }
                        },
                        on_cancel: move |_| remove_target.set(None),
                    }

                    // ── Rotate secret confirmation ────────────────────────
                    ConfirmDialog {
                        title: "Rotate Client Secret".to_string(),
                        message: format!(
                            "Are you sure you want to rotate the client secret for \"{}\"? Any existing client using the current secret will be invalidated immediately.",
                            app_name
                        ),
                        open: rotate_confirm(),
                        confirm_label: "Rotate Secret",
                        danger: true,
                        on_confirm: move |_| {
                            let aid = app_id_rotate.clone();
                            spawn(async move {
                                match api::rotate_application_secret(&aid).await {
                                    Ok(sec) => {
                                        state.toast(ToastKind::Success, "Client secret rotated");
                                        rotate_confirm.set(false);
                                        one_time_secret.set(Some(sec));
                                        reload.call(());
                                    }
                                    Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                }
                            });
                        },
                        on_cancel: move |_| rotate_confirm.set(false),
                    }

                    // ── One-time secret modal ─────────────────────────────
                    if let Some(sec) = one_time_secret() {
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
                                    label { style: "font-weight: 600; display: block; font-size: 0.85rem;", "Client ID" }
                                    code { style: "display:block; padding: 0.4rem; background: #f8f9fa; border: 1px solid #e9ecef; border-radius: 4px;",
                                        "{a.client_id}"
                                    }
                                }
                                div {
                                    label { style: "font-weight: 600; display: block; font-size: 0.85rem;", "Client Secret" }
                                    code { style: "display:block; padding: 0.4rem; background: #f8f9fa; border: 1px solid #e9ecef; border-radius: 4px; color: #d63384; word-break: break-all;",
                                        "{sec}"
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

                    // ── Delete confirmation ───────────────────────────────
                    ConfirmDialog {
                        title: "Delete application".to_string(),
                        message: format!("Delete application \"{}\"? Memberships will be removed. This cannot be undone.", a.name),
                        open: delete_confirm(),
                        confirm_label: "Delete",
                        danger: true,
                        on_confirm: move |_| {
                            let aid = app_id_delete.clone();
                            spawn(async move {
                                match api::delete_application(&aid).await {
                                    Ok(_) => {
                                        state.toast(ToastKind::Success, "Application deleted");
                                        let _ = dioxus_router::hooks::use_navigator()
                                            .replace(Route::ApplicationsPage {});
                                    }
                                    Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                }
                            });
                        },
                        on_cancel: move |_| delete_confirm.set(false),
                    }
                }
            }
        }
    }
}
