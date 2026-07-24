//! User management UI.

use crate::components::feedback::{ConfirmDialog, EmptyState, ErrorState, LoadingSpinner, Modal};
use crate::components::forms::{PasswordInput, TextInput};
use crate::components::navigation::Breadcrumb;
use crate::components::tables::{DataTable, ColumnDef};
use crate::components::widgets::StatusChip;
use crate::models::UserView;
use crate::routes::Route;
use crate::services::api;
use crate::state::{AppState, ToastKind};
use crate::utils::{format_datetime, matches_query};
use dioxus::prelude::*;

#[component]
pub fn UsersPage() -> Element {
    let state = use_context::<AppState>();
    let mut users = use_signal(Vec::<UserView>::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| true);
    let mut query = use_signal(String::new);
    let mut status_filter = use_signal(|| "all".to_string());
    let mut sort_key = use_signal(|| "username".to_string());
    let mut page = use_signal(|| 0usize);
    let page_size = 10usize;

    let mut show_create = use_signal(|| false);
    let mut new_user = use_signal(String::new);
    let mut new_pass = use_signal(String::new);

    let mut confirm_delete = use_signal(|| Option::<UserView>::None);

    let reload = use_callback(move |_: ()| {
        loading.set(true);
        error.set(None);
        spawn(async move {
            match api::list_users().await {
                Ok(list) => {
                    users.set(list);
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

    let can_create = state.auth.read().has_permission("users:create") || state.auth.read().is_adminish();
    use_effect(move || {
        if can_create && crate::utils::check_and_clear_create_intent() {
            show_create.set(true);
        }
    });

    let filtered = {
        let q = query();
        let sf = status_filter();
        let sk = sort_key();
        let mut list: Vec<UserView> = users()
            .into_iter()
            .filter(|u| matches_query(&u.username, &q))
            .filter(|u| sf == "all" || u.status == sf)
            .collect();
        list.sort_by(|a, b| match sk.as_str() {
            "status" => a.status.cmp(&b.status),
            "created" => b.created_at.cmp(&a.created_at),
            _ => a.username.to_lowercase().cmp(&b.username.to_lowercase()),
        });
        list
    };
    let total = filtered.len();
    let page_items: Vec<UserView> = filtered
        .into_iter()
        .skip(page() * page_size)
        .take(page_size)
        .collect();

    rsx! {
        Breadcrumb { items: vec![
            ("Dashboard".to_string(), Some(Route::DashboardPage {})),
            ("Users".to_string(), None),
        ]}

        div { class: "page-header",
            div {
                h1 { "Users" }
                p { class: "desc", "Create, disable, and manage user accounts" }
            }
            button {
                class: "btn btn-primary",
                r#type: "button",
                onclick: move |_| show_create.set(true),
                "+ Create user"
            }
        }

        if loading() {
            LoadingSpinner {}
        } else if let Some(err) = error() {
            ErrorState {
                message: err,
                on_retry: move |_| reload.call(())
            }
        } else if users().is_empty() && query().is_empty() && status_filter() == "all" {
            EmptyState {
                title: "No users found".to_string(),
                description: "Get started by creating your first user.",
                icon: "👥",
            }
        } else {
            DataTable {
                columns: vec![
                    ColumnDef { key: "username".into(), label: "Username".into(), sortable: true, visible: true },
                    ColumnDef { key: "status".into(), label: "Status".into(), sortable: true, visible: true },
                    ColumnDef { key: "last_login".into(), label: "Last login".into(), sortable: false, visible: true },
                    ColumnDef { key: "created".into(), label: "Created".into(), sortable: true, visible: true },
                    ColumnDef { key: "actions".into(), label: "Actions".into(), sortable: false, visible: true },
                ],
                on_search: move |v| { query.set(v); page.set(0); },
                search_value: query(),
                search_placeholder: "Search username…".to_string(),
                on_sort: move |k| sort_key.set(k),
                sort_key: sort_key(),
                on_page: move |p| page.set(p),
                page: page(),
                page_size: page_size,
                total: total,
                toolbar_actions: rsx! {
                    select {
                        class: "form-control",
                        style: "width: auto;",
                        value: "{status_filter()}",
                        onchange: move |e| { status_filter.set(e.value()); page.set(0); },
                        option { value: "all", "All statuses" }
                        option { value: "active", "Active" }
                        option { value: "disabled", "Disabled" }
                        option { value: "locked", "Locked" }
                    }
                    button {
                        class: "btn btn-outline",
                        r#type: "button",
                        onclick: move |_| reload.call(()),
                        "Refresh"
                    }
                },
                for u in page_items {
                    {
                        let id = u.id.clone();
                        let id2 = u.id.clone();
                        let id3 = u.id.clone();
                        let status = u.status.clone();
                        rsx! {
                            tr { key: "{u.id}",
                                td {
                                    Link {
                                        to: Route::UserDetailPage { id: u.id.clone() },
                                        strong { "{u.username}" }
                                    }
                                    div { class: "mono text-muted", style: "font-size:11px;",
                                        "{u.id}"
                                    }
                                }
                                td { StatusChip { status: u.status.clone() } }
                                td {
                                    "{u.last_login_at.as_deref().map(format_datetime).unwrap_or_else(|| \"—\".to_string())}"
                                }
                                td { "{format_datetime(&u.created_at)}" }
                                td { style: "text-align: right;",
                                    div { class: "actions",
                                        Link {
                                            class: "btn btn-sm btn-outline",
                                            to: Route::UserDetailPage { id: id.clone() },
                                            "View"
                                        }
                                        if status == "active" {
                                            button {
                                                class: "btn btn-sm btn-outline",
                                                r#type: "button",
                                                onclick: move |_| {
                                                    let id = id2.clone();
                                                    spawn(async move {
                                                        match api::update_user_status(&id, "disabled").await {
                                                            Ok(_) => {
                                                                state.toast(ToastKind::Success, "User disabled");
                                                                reload.call(());
                                                            }
                                                            Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                                        }
                                                    });
                                                },
                                                "Disable"
                                            }
                                        } else {
                                            button {
                                                class: "btn btn-sm btn-outline",
                                                r#type: "button",
                                                onclick: move |_| {
                                                    let id = id2.clone();
                                                    spawn(async move {
                                                        match api::update_user_status(&id, "active").await {
                                                            Ok(_) => {
                                                                state.toast(ToastKind::Success, "User enabled");
                                                                reload.call(());
                                                            }
                                                            Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                                        }
                                                    });
                                                },
                                                "Enable"
                                            }
                                        }
                                        button {
                                            class: "btn btn-sm btn-danger",
                                            r#type: "button",
                                            onclick: move |_| {
                                                if let Some(u) = users().into_iter().find(|x| x.id == id3) {
                                                    confirm_delete.set(Some(u));
                                                }
                                            },
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

        // Create modal
        Modal {
            title: "Create user".to_string(),
            open: show_create(),
            on_close: move |_| show_create.set(false),
            TextInput {
                label: "Username",
                value: new_user(),
                oninput: move |v| new_user.set(v),
                required: true,
            }
            PasswordInput {
                label: "Password",
                value: new_pass(),
                oninput: move |v| new_pass.set(v),
                required: true,
                autocomplete: "new-password",
            }
            div { class: "modal-footer", style: "margin-top:1rem; padding:0; border:none; background:transparent;",
                button {
                    class: "btn btn-outline",
                    r#type: "button",
                    onclick: move |_| show_create.set(false),
                    "Cancel"
                }
                button {
                    class: "btn btn-primary",
                    r#type: "button",
                    onclick: move |_| {
                        let u = new_user();
                        let p = new_pass();
                        spawn(async move {
                            match api::create_user(&u, &p).await {
                                Ok(_) => {
                                    state.toast(ToastKind::Success, "User created");
                                    show_create.set(false);
                                    new_user.set(String::new());
                                    new_pass.set(String::new());
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
            title: "Disable user".to_string(),
            message: format!(
                "Disable user \"{}\"? They will no longer be able to sign in.",
                confirm_delete().as_ref().map(|u| u.username.as_str()).unwrap_or("")
            ),
            open: confirm_delete().is_some(),
            confirm_label: "Disable",
            danger: true,
            on_confirm: move |_| {
                if let Some(u) = confirm_delete() {
                    spawn(async move {
                        match api::delete_user(&u.id).await {
                            Ok(()) => {
                                state.toast(ToastKind::Success, "User disabled");
                                confirm_delete.set(None);
                                reload.call(());
                            }
                            Err(e) => state.toast(ToastKind::Error, e.to_string()),
                        }
                    });
                }
            },
            on_cancel: move |_| confirm_delete.set(None),
        }
    }
}

#[component]
pub fn UserDetailPage(id: String) -> Element {
    let state = use_context::<AppState>();
    let state_auth = state.auth;
    let can_manage_apps = state_auth().has_permission("applications:manage");

    let mut user = use_signal(|| Option::<UserView>::None);
    let mut roles = use_signal(Vec::<crate::models::RoleView>::new);
    let mut all_roles = use_signal(Vec::<crate::models::RoleView>::new);
    let mut user_apps =
        use_signal(Vec::<crate::models::UserApplicationMembershipView>::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| true);
    let mut new_pass = use_signal(String::new);
    let mut assign_role = use_signal(String::new);

    let user_id = id.clone();
    let reload = use_callback(move |_: ()| {
        let id = user_id.clone();
        loading.set(true);
        spawn(async move {
            match api::get_user(&id).await {
                Ok(u) => {
                    user.set(Some(u));
                    let r = api::list_user_roles(&id).await.unwrap_or_default();
                    roles.set(r);
                    if let Ok(ar) = api::list_roles().await {
                        all_roles.set(ar);
                    }
                    if let Ok(apps) = api::list_user_applications(&id).await {
                        user_apps.set(apps);
                    } else {
                        user_apps.set(Vec::new());
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
    use_effect(move || { reload.call(()); });

    rsx! {
        Breadcrumb { items: vec![
            ("Dashboard".to_string(), Some(Route::DashboardPage {})),
            ("Users".to_string(), Some(Route::UsersPage {})),
            (id.clone(), None),
        ]}

        if loading() {
            LoadingSpinner {}
        } else if let Some(err) = error() {
            ErrorState { message: err, on_retry: move |_| reload.call(()) }
        } else if let Some(u) = user() {
            {
                let user_id = u.id.clone();
                let user_id_reset = user_id.clone();
                let user_id_assign = user_id.clone();
                rsx! {
            div { class: "page-header",
                div {
                    h1 { "{u.username}" }
                    p { class: "desc mono", "{u.id}" }
                }
                StatusChip { status: u.status.clone() }
            }

            div { class: "grid-2",
                div { class: "card",
                    div { class: "card-header", h3 { "Account" } }
                    div { class: "card-body stack",
                        div { "Status: " StatusChip { status: u.status.clone() } }
                        div { class: "text-secondary",
                            "Created: {format_datetime(&u.created_at)}"
                        }
                        div { class: "text-secondary",
                            "Last login: {u.last_login_at.as_deref().map(format_datetime).unwrap_or_else(|| \"—\".to_string())}"
                        }
                    }
                }

                div { class: "card",
                    div { class: "card-header", h3 { "Reset password" } }
                    div { class: "card-body",
                        PasswordInput {
                            label: "New password",
                            value: new_pass(),
                            oninput: move |v| new_pass.set(v),
                            autocomplete: "new-password",
                        }
                        p { class: "form-hint",
                            "Minimum 8 characters (12 if the user is an admin). Avoid common sequences like \"password\"."
                        }
                        button {
                            class: "btn btn-primary",
                            r#type: "button",
                            disabled: new_pass().len() < 8,
                            onclick: move |_| {
                                let id = user_id_reset.clone();
                                let p = new_pass();
                                if p.len() < 8 {
                                    state.toast(ToastKind::Error, "Password must be at least 8 characters");
                                    return;
                                }
                                spawn(async move {
                                    match api::reset_user_password(&id, &p).await {
                                        Ok(()) => {
                                            state.toast(ToastKind::Success, "Password reset");
                                            new_pass.set(String::new());
                                        }
                                        Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                    }
                                });
                            },
                            "Reset password"
                        }
                    }
                }

                div { class: "card",
                    div { class: "card-header", h3 { "Roles" } }
                    div { class: "card-body",
                        if roles().is_empty() {
                            p { class: "text-muted", "No roles assigned." }
                        } else {
                            div { class: "stack",
                                for r in roles() {
                                    {
                                        let role_name = r.name.clone();
                                        let uid = user_id.clone();
                                        rsx! {
                                            div { class: "row", style: "justify-content:space-between;",
                                                span { class: "badge badge-accent", "{r.name}" }
                                                button {
                                                    class: "btn btn-sm btn-ghost",
                                                    r#type: "button",
                                                    onclick: move |_| {
                                                        let role_name = role_name.clone();
                                                        let uid = uid.clone();
                                                        spawn(async move {
                                                            match api::remove_user_role(&uid, &role_name).await {
                                                                Ok(()) => {
                                                                    state.toast(ToastKind::Success, "Role removed");
                                                                    reload.call(());
                                                                }
                                                                Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                                            }
                                                        });
                                                    },
                                                    "Remove"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "row mt-2",
                            select {
                                class: "form-control",
                                value: "{assign_role()}",
                                onchange: move |e| assign_role.set(e.value()),
                                option { value: "", "Assign role…" }
                                for r in all_roles() {
                                    option { value: "{r.name}", "{r.name}" }
                                }
                            }
                            button {
                                class: "btn btn-outline",
                                r#type: "button",
                                disabled: assign_role().is_empty(),
                                onclick: move |_| {
                                    let role = assign_role();
                                    let uid = user_id_assign.clone();
                                    spawn(async move {
                                        match api::assign_user_role(&uid, &role).await {
                                            Ok(()) => {
                                                state.toast(ToastKind::Success, "Role assigned");
                                                assign_role.set(String::new());
                                                reload.call(());
                                            }
                                            Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                        }
                                    });
                                },
                                "Assign"
                            }
                        }
                    }
                }

                if can_manage_apps {
                    div { class: "card", style: "grid-column: 1 / -1;",
                        div { class: "card-header", h3 { "Applications" } }
                        div { class: "card-body",
                            p { class: "text-secondary", style: "font-size:0.9rem; margin-bottom:0.75rem;",
                                "Applications this user is assigned to. Membership roles are metadata only and do not change global RBAC."
                            }
                            if user_apps().is_empty() {
                                p { class: "text-muted", "Not assigned to any applications." }
                            } else {
                                DataTable {
                                    columns: vec![
                                        ColumnDef { key: "name".into(), label: "Application".into(), sortable: false, visible: true },
                                        ColumnDef { key: "role".into(), label: "Membership Role".into(), sortable: false, visible: true },
                                        ColumnDef { key: "status".into(), label: "Status".into(), sortable: false, visible: true },
                                        ColumnDef { key: "assigned".into(), label: "Assigned".into(), sortable: false, visible: true },
                                    ],
                                    on_search: |_| {},
                                    search_value: "".to_string(),
                                    search_placeholder: "".to_string(),
                                    on_sort: |_| {},
                                    sort_key: "".to_string(),
                                    on_page: |_| {},
                                    page: 0,
                                    page_size: user_apps().len().max(1),
                                    total: user_apps().len(),
                                    for m in user_apps() {
                                        tr { key: "{m.id}",
                                            td {
                                                Link {
                                                    to: Route::ApplicationDetailPage { id: m.application_id.clone() },
                                                    strong { "{m.application_name}" }
                                                }
                                                div { class: "text-muted", style: "font-size:0.8rem;",
                                                    code { "{m.application_slug}" }
                                                }
                                            }
                                            td { span { class: "badge badge-accent", "{m.role}" } }
                                            td {
                                                StatusChip {
                                                    status: if m.enabled { "active".to_string() } else { "disabled".to_string() }
                                                }
                                            }
                                            td { "{format_datetime(&m.created_at)}" }
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
