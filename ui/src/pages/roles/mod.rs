//! Role management.

use crate::components::feedback::{ConfirmDialog, EmptyState, ErrorState, LoadingSpinner, Modal};
use crate::components::forms::{Checkbox, TextInput};
use crate::components::navigation::Breadcrumb;
use crate::components::tables::{DataTable, ColumnDef};
use crate::models::{PermissionView, RoleView};
use crate::routes::Route;
use crate::services::api;
use crate::state::{AppState, ToastKind};
use crate::utils::matches_query;
use dioxus::prelude::*;

#[component]
pub fn RolesPage() -> Element {
    let state = use_context::<AppState>();
    let mut roles = use_signal(Vec::<RoleView>::new);
    let mut perms = use_signal(Vec::<PermissionView>::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| true);
    let mut query = use_signal(String::new);
    let mut page = use_signal(|| 0usize);
    let page_size = 10usize;
    let mut sort_key = use_signal(|| "name".to_string());

    let mut show_create = use_signal(|| false);
    let mut name = use_signal(String::new);
    let mut description = use_signal(String::new);

    let mut edit_role = use_signal(|| Option::<RoleView>::None);
    let mut edit_name = use_signal(String::new);
    let mut edit_desc = use_signal(String::new);
    let mut selected_perms = use_signal(Vec::<String>::new);

    let mut delete_role = use_signal(|| Option::<RoleView>::None);

    let reload = use_callback(move |_: ()| {
        loading.set(true);
        spawn(async move {
            match api::list_roles().await {
                Ok(list) => {
                    roles.set(list);
                    if let Ok(p) = api::list_permissions().await {
                        perms.set(p.permissions);
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

    let mut filtered: Vec<RoleView> = roles()
        .into_iter()
        .filter(|r| {
            matches_query(&r.name, &query())
                || r.description
                    .as_deref()
                    .map(|d| matches_query(d, &query()))
                    .unwrap_or(false)
        })
        .collect();
    let sk = sort_key();
    filtered.sort_by(|a, b| match sk.as_str() {
        "permissions" => b.permissions.len().cmp(&a.permissions.len()),
        "users" => b.user_count.cmp(&a.user_count),
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    let total = filtered.len();
    let page_items: Vec<RoleView> = filtered
        .into_iter()
        .skip(page() * page_size)
        .take(page_size)
        .collect();

    rsx! {
        Breadcrumb { items: vec![
            ("Dashboard".to_string(), Some(Route::DashboardPage {})),
            ("Roles".to_string(), None),
        ]}

        div { class: "page-header",
            div {
                h1 { "Roles" }
                p { class: "desc", "Manage roles and assigned permissions" }
            }
            button {
                class: "btn btn-primary",
                r#type: "button",
                onclick: move |_| show_create.set(true),
                "+ Create role"
            }
        }

        if loading() {
            LoadingSpinner {}
        } else if let Some(err) = error() {
            ErrorState { message: err, on_retry: move |_| reload.call(()) }
        } else if roles().is_empty() && query().is_empty() {
            EmptyState { title: "No roles".to_string(), description: "Create your first role to get started.", icon: "🛡" }
        } else {
            DataTable {
                columns: vec![
                    ColumnDef { key: "name".into(), label: "Name".into(), sortable: true, visible: true },
                    ColumnDef { key: "description".into(), label: "Description".into(), sortable: false, visible: true },
                    ColumnDef { key: "permissions".into(), label: "Permissions".into(), sortable: true, visible: true },
                    ColumnDef { key: "users".into(), label: "Users".into(), sortable: true, visible: true },
                    ColumnDef { key: "actions".into(), label: "Actions".into(), sortable: false, visible: true },
                ],
                on_search: move |v| { query.set(v); page.set(0); },
                search_value: query(),
                search_placeholder: "Search roles…".to_string(),
                on_sort: move |k| sort_key.set(k),
                sort_key: sort_key(),
                on_page: move |p| page.set(p),
                page: page(),
                page_size: page_size,
                total: total,
                toolbar_actions: rsx! {
                    button { class: "btn btn-outline", r#type: "button", onclick: move |_| reload.call(()), "Refresh" }
                },
                for r in page_items {
                    {
                        let role = r.clone();
                        let role2 = r.clone();
                        rsx! {
                            tr { key: "{r.id}",
                                td { strong { "{r.name}" } }
                                td { class: "text-secondary",
                                    "{r.description.as_deref().unwrap_or(\"—\")}"
                                }
                                td {
                                    span { class: "badge", "{r.permissions.len()}" }
                                }
                                td { "{r.user_count}" }
                                td { style: "text-align: right;",
                                    div { class: "actions",
                                        button {
                                            class: "btn btn-sm btn-outline",
                                            r#type: "button",
                                            onclick: move |_| {
                                                edit_name.set(role.name.clone());
                                                edit_desc.set(role.description.clone().unwrap_or_default());
                                                selected_perms.set(role.permissions.clone());
                                                edit_role.set(Some(role.clone()));
                                            },
                                            "Edit"
                                        }
                                        if r.name != "admin" {
                                            button {
                                                class: "btn btn-sm btn-danger",
                                                r#type: "button",
                                                onclick: move |_| delete_role.set(Some(role2.clone())),
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
            title: "Create role".to_string(),
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
                            match api::create_role(&n, Some(d.as_str()).filter(|s| !s.is_empty())).await {
                                Ok(_) => {
                                    state.toast(ToastKind::Success, "Role created");
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
            title: "Edit role".to_string(),
            open: edit_role().is_some(),
            on_close: move |_| edit_role.set(None),
            large: true,
            TextInput {
                label: "Name",
                value: edit_name(),
                oninput: move |v| edit_name.set(v),
            }
            TextInput {
                label: "Description",
                value: edit_desc(),
                oninput: move |v| edit_desc.set(v),
            }
            h3 { class: "mt-2", "Permissions" }
            div { class: "perm-list", style: "max-height: 240px; overflow: auto;",
                for p in perms() {
                    {
                        let pname = p.name.clone();
                        let checked = selected_perms().contains(&pname);
                        rsx! {
                            Checkbox {
                                label: format!("{} — {}", p.name, p.description.as_deref().unwrap_or("")),
                                checked: checked,
                                onchange: move |v| {
                                    let mut list = selected_perms();
                                    if v {
                                        if !list.contains(&pname) { list.push(pname.clone()); }
                                    } else {
                                        list.retain(|x| x != &pname);
                                    }
                                    selected_perms.set(list);
                                }
                            }
                        }
                    }
                }
            }
            div { class: "modal-footer", style: "margin-top:1rem; padding:0; border:none; background:transparent;",
                button { class: "btn btn-outline", r#type: "button",
                    onclick: move |_| edit_role.set(None), "Cancel" }
                button {
                    class: "btn btn-primary", r#type: "button",
                    onclick: move |_| {
                        if let Some(role) = edit_role() {
                            let id = role.id.clone();
                            let n = edit_name();
                            let d = edit_desc();
                            let perms = selected_perms();
                            spawn(async move {
                                if let Err(e) = api::update_role(
                                    &id, &n, Some(d.as_str()).filter(|s| !s.is_empty())
                                ).await {
                                    state.toast(ToastKind::Error, e.to_string());
                                    return;
                                }
                                match api::set_role_permissions(&id, &perms).await {
                                    Ok(()) => {
                                        state.toast(ToastKind::Success, "Role updated");
                                        edit_role.set(None);
                                        reload.call(());
                                    }
                                    Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                }
                            });
                        }
                    },
                    "Save"
                }
            }
        }

        ConfirmDialog {
            title: "Delete role".to_string(),
            message: format!(
                "Delete role \"{}\"? This cannot be undone.",
                delete_role().as_ref().map(|r| r.name.as_str()).unwrap_or("")
            ),
            open: delete_role().is_some(),
            confirm_label: "Delete",
            danger: true,
            on_confirm: move |_| {
                if let Some(r) = delete_role() {
                    spawn(async move {
                        match api::delete_role(&r.id).await {
                            Ok(()) => {
                                state.toast(ToastKind::Success, "Role deleted");
                                delete_role.set(None);
                                reload.call(());
                            }
                            Err(e) => state.toast(ToastKind::Error, e.to_string()),
                        }
                    });
                }
            },
            on_cancel: move |_| delete_role.set(None),
        }
    }
}
