use crate::components::feedback::{ConfirmDialog, EmptyState, ErrorState, LoadingSpinner, Modal};
use crate::components::forms::TextInput;
use crate::components::navigation::Breadcrumb;
use crate::components::tables::{ColumnDef, DataTable};
use crate::models::{ApplicationView, AuditEntry, TenantView, UserView};
use crate::routes::Route;
use crate::services::api;
use crate::state::{AppState, ToastKind};
use crate::utils::matches_query;
use dioxus::prelude::*;

#[component]
pub fn TenantsPage() -> Element {
    let mut state = use_context::<AppState>();
    let mut tenants = use_signal(Vec::<TenantView>::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| true);
    let mut query = use_signal(String::new);
    let mut sort_key = use_signal(|| "name".to_string());
    let mut page = use_signal(|| 0usize);
    let page_size = 10usize;

    let mut show_create = use_signal(|| false);
    let mut new_name = use_signal(String::new);
    let mut new_slug = use_signal(String::new);
    let mut delete_tenant = use_signal(|| Option::<TenantView>::None);

    let reload = use_callback(move |_: ()| {
        loading.set(true);
        error.set(None);
        spawn(async move {
            match api::list_tenants().await {
                Ok(list) => {
                    tenants.set(list);
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

    let can_create =
        state.auth.read().has_permission("roles:manage") || state.auth.read().is_adminish();
    use_effect(move || {
        if can_create && crate::utils::check_and_clear_create_intent() {
            show_create.set(true);
        }
    });

    let filtered = {
        let q = query();
        let sk = sort_key();
        let mut list: Vec<TenantView> = tenants()
            .into_iter()
            .filter(|t| matches_query(&t.name, &q) || matches_query(&t.slug, &q))
            .collect();
        list.sort_by(|a, b| match sk.as_str() {
            "slug" => a.slug.cmp(&b.slug),
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });
        list
    };
    let total = filtered.len();
    let page_items: Vec<TenantView> = filtered
        .into_iter()
        .skip(page() * page_size)
        .take(page_size)
        .collect();

    rsx! {
        Breadcrumb { items: vec![
            ("Dashboard".to_string(), Some(Route::DashboardPage {})),
            ("Tenants".to_string(), None),
        ]}

        div { class: "page-header",
            div {
                h1 { "Tenants" }
                p { class: "desc", "Manage organizational tenants and directories" }
            }
            button {
                class: "btn btn-primary",
                r#type: "button",
                onclick: move |_| show_create.set(true),
                "+ Create tenant"
            }
        }

        if loading() {
            LoadingSpinner {}
        } else if let Some(err) = error() {
            ErrorState { message: err, on_retry: move |_| reload.call(()) }
        } else if tenants().is_empty() && query().is_empty() {
            EmptyState { title: "No tenants found".to_string(), description: "Create a tenant to get started.", icon: "🏢" }
        } else {
            DataTable {
                columns: vec![
                    ColumnDef { key: "name".into(), label: "Name".into(), sortable: true, visible: true },
                    ColumnDef { key: "slug".into(), label: "Slug".into(), sortable: true, visible: true },
                    ColumnDef { key: "description".into(), label: "Description".into(), sortable: false, visible: true },
                    ColumnDef { key: "actions".into(), label: "Actions".into(), sortable: false, visible: true },
                ],
                on_search: move |v| { query.set(v); page.set(0); },
                search_value: query(),
                search_placeholder: "Search tenants…".to_string(),
                on_sort: move |k| sort_key.set(k),
                sort_key: sort_key(),
                on_page: move |p| page.set(p),
                page: page(),
                page_size: page_size,
                total: total,
                toolbar_actions: rsx! {
                    button { class: "btn btn-outline", r#type: "button", onclick: move |_| reload.call(()), "Refresh" }
                },
                for t in page_items {
                    {
                        let id = t.id.clone();
                        let slug = t.slug.clone();
                        let t_clone = t.clone();
                        let t2 = t.clone();
                        rsx! {
                            tr { key: "{t.id}",
                                td {
                                    Link {
                                        to: Route::TenantDetailPage { id: t.id.clone() },
                                        strong { "{t.name}" }
                                    }
                                    div { class: "mono text-muted", style: "font-size:11px;", "{t.id}" }
                                }
                                td { span { class: "mono", "{slug}" } }
                                td { "{t.description.clone().unwrap_or_else(|| \"—\".to_string())}" }
                                td { style: "text-align: right;",
                                    div { class: "actions",
                                        Link {
                                            class: "btn btn-sm btn-outline",
                                            to: Route::TenantDetailPage { id: id.clone() },
                                            "View"
                                        }
                                        button {
                                            class: "btn btn-sm btn-outline",
                                            r#type: "button",
                                            onclick: move |_| {
                                                state.tenant.set(Some(t_clone.clone()));
                                                state.toast(ToastKind::Success, "Switched tenant context");
                                            },
                                            "Switch Context"
                                        }
                                        button {
                                            class: "btn btn-sm btn-danger",
                                            r#type: "button",
                                            onclick: move |_| delete_tenant.set(Some(t2.clone())),
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
            title: "Create tenant".to_string(),
            open: show_create(),
            on_close: move |_| show_create.set(false),
            TextInput {
                label: "Name",
                value: new_name(),
                oninput: move |v: String| {
                    new_name.set(v.clone());
                    new_slug.set(v.to_lowercase().replace(" ", "-").chars().filter(|c| c.is_alphanumeric() || *c == '-').collect());
                },
            }
            TextInput {
                label: "Slug",
                value: new_slug(),
                oninput: move |v| new_slug.set(v),
            }
            div { class: "modal-footer", style: "margin-top:1rem; padding:0; border:none; background:transparent;",
                button { class: "btn btn-outline", r#type: "button", onclick: move |_| show_create.set(false), "Cancel" }
                button {
                    class: "btn btn-primary", r#type: "button",
                    onclick: move |_| {
                        let n = new_name();
                        let s = new_slug();
                        spawn(async move {
                            match api::create_tenant(&n, Some(s.as_str()).filter(|s| !s.is_empty())).await {
                                Ok(_) => {
                                    state.toast(ToastKind::Success, "Tenant created");
                                    show_create.set(false);
                                    new_name.set(String::new());
                                    new_slug.set(String::new());
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
            title: "Delete tenant".to_string(),
            message: format!("Delete tenant \"{}\"? This cannot be undone.", delete_tenant().as_ref().map(|t| t.name.as_str()).unwrap_or("")),
            open: delete_tenant().is_some(),
            confirm_label: "Delete",
            danger: true,
            on_confirm: move |_| {
                if let Some(t) = delete_tenant() {
                    let total_tenants = tenants().len();
                    if total_tenants <= 1 {
                        state.toast(ToastKind::Error, "Cannot delete the last tenant");
                        delete_tenant.set(None);
                        return;
                    }
                    spawn(async move {
                        match api::delete_tenant(&t.id).await {
                            Ok(()) => {
                                state.toast(ToastKind::Success, "Tenant deleted");
                                delete_tenant.set(None);
                                reload.call(());
                            }
                            Err(e) => state.toast(ToastKind::Error, e.to_string()),
                        }
                    });
                }
            },
            on_cancel: move |_| delete_tenant.set(None),
        }
    }
}

#[component]
pub fn TenantDetailPage(id: String) -> Element {
    let state = use_context::<AppState>();
    let mut tenant = use_signal(|| Option::<TenantView>::None);
    let mut error = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| true);

    let mut tab = use_signal(|| "overview".to_string());
    let mut edit_name = use_signal(String::new);
    let mut edit_slug = use_signal(String::new);

    let mut tenant_users = use_signal(Vec::<UserView>::new);
    let mut all_users = use_signal(Vec::<UserView>::new);
    let mut tenant_apps = use_signal(Vec::<ApplicationView>::new);
    let mut activity = use_signal(Vec::<AuditEntry>::new);

    let mut user_query = use_signal(String::new);
    let mut show_assign_modal = use_signal(|| false);
    let mut selected_assign_user_id = use_signal(String::new);

    let mut confirm_reassign_user = use_signal(|| Option::<(UserView, String, String)>::None);
    let mut confirm_move_default = use_signal(|| Option::<UserView>::None);
    let mut confirm_delete = use_signal(|| false);

    let tenant_id = id.clone();
    let reload = use_callback(move |_: ()| {
        let id = tenant_id.clone();
        loading.set(true);
        error.set(None);
        spawn(async move {
            match api::get_tenant(&id).await {
                Ok(t) => {
                    edit_name.set(t.name.clone());
                    edit_slug.set(t.slug.clone());
                    tenant.set(Some(t));

                    if let Ok(users) = api::list_tenant_users(&id).await {
                        tenant_users.set(users);
                    }
                    if let Ok(users) = api::list_users().await {
                        all_users.set(users);
                    }
                    if let Ok(apps) = api::list_tenant_applications(&id).await {
                        tenant_apps.set(apps);
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

    let tenant_id_act = id.clone();
    let load_activity = use_callback(move |_: ()| {
        let id = tenant_id_act.clone();
        spawn(async move {
            let q = format!("resource_type=tenant&resource_id={id}&limit=50");
            if let Ok(resp) = api::list_audit(&q).await {
                activity.set(resp.entries);
            }
        });
    });

    rsx! {
        Breadcrumb { items: vec![
            ("Dashboard".to_string(), Some(Route::DashboardPage {})),
            ("Tenants".to_string(), Some(Route::TenantsPage {})),
            (tenant().map(|t| t.name.clone()).unwrap_or(id.clone()), None),
        ]}

        if loading() {
            LoadingSpinner {}
        } else if let Some(err) = error() {
            ErrorState { message: err, on_retry: move |_| reload.call(()) }
        } else if let Some(t) = tenant() {
            {
                let tid_save = t.id.clone();
                let tid_assign = t.id.clone();
                let tid_move_default = t.id.clone();
                let tid_delete = t.id.clone();
                let tenant_name = t.name.clone();
                let is_default_tenant = t.id == "00000000-0000-0000-0000-000000000001";

                let current_member_ids: Vec<String> = tenant_users().iter().map(|u| u.id.clone()).collect();
                let assignable_users: Vec<UserView> = all_users()
                    .into_iter()
                    .filter(|u| !current_member_ids.contains(&u.id))
                    .collect();

                let filtered_members: Vec<UserView> = {
                    let q = user_query();
                    tenant_users()
                        .into_iter()
                        .filter(|u| matches_query(&u.username, &q))
                        .collect()
                };

                rsx! {
                    div { class: "page-header",
                        div {
                            h1 { "{t.name}" }
                            p { class: "desc",
                                code { "{t.slug}" }
                                " · Tenant ID: "
                                code { "{t.id}" }
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
                            "Users ({tenant_users().len()})"
                        }
                        button {
                            class: if tab() == "applications" { "btn btn-sm btn-primary" } else { "btn btn-sm btn-outline" },
                            r#type: "button",
                            onclick: move |_| tab.set("applications".into()),
                            "Applications ({tenant_apps().len()})"
                        }
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

                    // ── Tab: Overview ──────────────────────────────────────────
                    if tab() == "overview" {
                        div { class: "grid-2",
                            div { class: "card",
                                div { class: "card-header", h3 { "Tenant Details" } }
                                div { class: "card-body",
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
                                    p { class: "desc text-muted", style: "font-size:12px; margin-top:-0.5rem;",
                                        "Leaving slug blank derives it automatically from name. Changing a tenant slug may affect existing references."
                                    }
                                    button {
                                        class: "btn btn-primary mt-2",
                                        r#type: "button",
                                        onclick: move |_| {
                                            let n = edit_name();
                                            let s = edit_slug();
                                            let tid = tid_save.clone();
                                            spawn(async move {
                                                match api::update_tenant(&tid, &n, Some(s.as_str()).filter(|s| !s.is_empty())).await {
                                                    Ok(_) => {
                                                        state.toast(ToastKind::Success, "Tenant updated");
                                                        reload.call(());
                                                    }
                                                    Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                                }
                                            });
                                        },
                                        "Save changes"
                                    }
                                }
                            }

                            div { class: "card",
                                div { class: "card-header", h3 { "Danger Zone" } }
                                div { class: "card-body",
                                    p { "Deleting a tenant is permanent and cannot be undone." }
                                    button {
                                        class: "btn btn-danger",
                                        r#type: "button",
                                        disabled: is_default_tenant,
                                        onclick: move |_| confirm_delete.set(true),
                                        "Delete Tenant"
                                    }
                                }
                            }
                        }
                    }

                    // ── Tab: Users (Tenant User Assignment) ───────────────────
                    if tab() == "users" {
                        div { class: "card",
                            div { class: "card-header", style: "display:flex; justify-content:space-between; align-items:center;",
                                div {
                                    h3 { "Tenant User Assignment" }
                                    p { class: "desc", "Users assigned to this tenant owner. Every user has exactly one tenant owner." }
                                }
                                div { class: "row", style: "gap:0.5rem;",
                                    button {
                                        class: "btn btn-primary btn-sm",
                                        r#type: "button",
                                        onclick: move |_| {
                                            selected_assign_user_id.set(String::new());
                                            show_assign_modal.set(true);
                                        },
                                        "+ Assign User"
                                    }
                                    Link {
                                        class: "btn btn-outline btn-sm",
                                        to: Route::UsersPage {},
                                        "+ Create User"
                                    }
                                }
                            }
                            div { class: "card-body",
                                DataTable {
                                    columns: vec![
                                        ColumnDef { key: "username".into(), label: "Username".into(), sortable: true, visible: true },
                                        ColumnDef { key: "status".into(), label: "Status".into(), sortable: true, visible: true },
                                        ColumnDef { key: "created_at".into(), label: "Created".into(), sortable: true, visible: true },
                                        ColumnDef { key: "actions".into(), label: "Actions".into(), sortable: false, visible: true },
                                    ],
                                    on_search: move |v| user_query.set(v),
                                    search_value: user_query(),
                                    search_placeholder: "Filter tenant users…".to_string(),
                                    on_sort: move |_| {},
                                    sort_key: "username".to_string(),
                                    on_page: move |_| {},
                                    page: 0,
                                    page_size: 100,
                                    total: filtered_members.len(),
                                    toolbar_actions: rsx! {
                                        button { class: "btn btn-outline btn-sm", r#type: "button", onclick: move |_| reload.call(()), "Refresh" }
                                    },
                                    for u in filtered_members {
                                        {
                                            let u_clone = u.clone();
                                            rsx! {
                                                tr { key: "{u.id}",
                                                    td {
                                                        Link {
                                                            to: Route::UserDetailPage { id: u.id.clone() },
                                                            strong { "{u.username}" }
                                                        }
                                                        div { class: "mono text-muted", style: "font-size:11px;", "{u.id}" }
                                                    }
                                                    td {
                                                        span { class: "badge badge-success", "{u.status}" }
                                                    }
                                                    td { "{u.created_at}" }
                                                    td { style: "text-align: right;",
                                                        if !is_default_tenant {
                                                            button {
                                                                class: "btn btn-sm btn-outline",
                                                                r#type: "button",
                                                                onclick: move |_| confirm_move_default.set(Some(u_clone.clone())),
                                                                "Move to Default Tenant"
                                                            }
                                                        } else {
                                                            span { class: "text-muted", style: "font-size:12px;", "Default Tenant Owner" }
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

                    // ── Tab: Applications ─────────────────────────────────────
                    if tab() == "applications" {
                        div { class: "card",
                            div { class: "card-header",
                                h3 { "Tenant Applications" }
                                p { class: "desc", "Applications associated with this tenant." }
                            }
                            div { class: "card-body",
                                if tenant_apps().is_empty() {
                                    EmptyState {
                                        title: "No applications found".to_string(),
                                        description: "No applications are currently associated with this tenant.".to_string(),
                                        icon: "🚀".to_string(),
                                    }
                                } else {
                                    table { class: "table",
                                        thead {
                                            tr {
                                                th { "Application" }
                                                th { "Slug" }
                                                th { "Client ID" }
                                                th { "Status" }
                                                th { style: "text-align: right;", "Actions" }
                                            }
                                        }
                                        tbody {
                                            for app in tenant_apps() {
                                                tr { key: "{app.id}",
                                                    td {
                                                        Link {
                                                            to: Route::ApplicationDetailPage { id: app.id.clone() },
                                                            strong { "{app.name}" }
                                                        }
                                                    }
                                                    td { code { "{app.slug}" } }
                                                    td { code { "{app.client_id}" } }
                                                    td {
                                                        span {
                                                            class: if app.enabled { "badge badge-success" } else { "badge badge-secondary" },
                                                            if app.enabled { "Active" } else { "Disabled" }
                                                        }
                                                    }
                                                    td { style: "text-align: right;",
                                                        Link {
                                                            class: "btn btn-sm btn-outline",
                                                            to: Route::ApplicationDetailPage { id: app.id.clone() },
                                                            "View"
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

                    // ── Tab: Activity ──────────────────────────────────────────
                    if tab() == "activity" {
                        div { class: "card",
                            div { class: "card-header",
                                h3 { "Tenant Audit Log" }
                                p { class: "desc", "Audit events scoped to this tenant." }
                            }
                            div { class: "card-body",
                                if activity().is_empty() {
                                    EmptyState {
                                        title: "No activity recorded".to_string(),
                                        description: "No audit events found for this tenant.".to_string(),
                                        icon: "📜".to_string(),
                                    }
                                } else {
                                    table { class: "table",
                                        thead {
                                            tr {
                                                th { "Timestamp" }
                                                th { "Action" }
                                                th { "Actor" }
                                                th { "Severity" }
                                                th { "IP Address" }
                                            }
                                        }
                                        tbody {
                                            for act in activity() {
                                                tr { key: "{act.id}",
                                                    td { "{act.created_at}" }
                                                    td { strong { "{act.action}" } }
                                                    td { "{act.actor_user_id.as_deref().unwrap_or(\"—\")}" }
                                                    td {
                                                        span { class: "badge badge-info", "{act.severity}" }
                                                    }
                                                    td { "{act.ip_address.as_deref().unwrap_or(\"—\")}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ── Assign User Modal ──────────────────────────────────────
                    Modal {
                        title: "Assign User to Tenant".to_string(),
                        open: show_assign_modal(),
                        on_close: move |_| show_assign_modal.set(false),
                        p { class: "desc", "Select an existing NX9-Auth user to reassign to tenant \"{tenant_name}\"." }
                        div { class: "form-group", style: "margin-top:1rem;",
                            label { class: "form-label", "Select User" }
                            select {
                                class: "form-control",
                                value: selected_assign_user_id(),
                                onchange: move |evt: Event<FormData>| selected_assign_user_id.set(evt.value()),
                                option { value: "", "— Select an existing user —" }
                                for u in assignable_users.clone() {
                                    option {
                                        value: "{u.id}",
                                        "{u.username} (currently in tenant: {u.tenant_id.as_deref().unwrap_or(\"default\")})"
                                    }
                                }
                            }
                        }
                        div { class: "modal-footer", style: "margin-top:1.5rem; padding:0; border:none; background:transparent;",
                            button { class: "btn btn-outline", r#type: "button", onclick: move |_| show_assign_modal.set(false), "Cancel" }
                            button {
                                class: "btn btn-primary",
                                r#type: "button",
                                disabled: selected_assign_user_id().is_empty(),
                                onclick: move |_| {
                                    let uid = selected_assign_user_id();
                                    if let Some(target_u) = assignable_users.iter().find(|u| u.id == uid) {
                                        let from = target_u.tenant_id.clone().unwrap_or_else(|| "default".to_string());
                                        confirm_reassign_user.set(Some((target_u.clone(), from, tid_assign.clone())));
                                        show_assign_modal.set(false);
                                    }
                                },
                                "Assign User"
                            }
                        }
                    }

                    // ── Confirm Reassign User Dialog ─────────────────────────
                    if let Some((target_u, from_tenant, to_tenant_id)) = confirm_reassign_user() {
                        {
                            let u_id = target_u.id.clone();
                            let u_name = target_u.username.clone();
                            let to_tid = to_tenant_id.clone();
                            let dest_name = tenant_name.clone();
                            rsx! {
                                ConfirmDialog {
                                    title: "Confirm Tenant Reassignment".to_string(),
                                    message: format!(
                                        "Reassign user \"{}\" from tenant \"{}\" to \"{}\"?",
                                        u_name, from_tenant, dest_name
                                    ),
                                    open: true,
                                    confirm_label: "Reassign User".to_string(),
                                    danger: false,
                                    on_confirm: move |_| {
                                        let uid = u_id.clone();
                                        let tid = to_tid.clone();
                                        confirm_reassign_user.set(None);
                                        spawn(async move {
                                            match api::assign_tenant_user(&tid, &uid).await {
                                                Ok(_) => {
                                                    state.toast(ToastKind::Success, "User reassigned to tenant");
                                                    reload.call(());
                                                }
                                                Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                            }
                                        });
                                    },
                                    on_cancel: move |_| confirm_reassign_user.set(None),
                                }
                            }
                        }
                    }

                    // ── Confirm Move to Default Tenant Dialog ────────────────
                    if let Some(target_u) = confirm_move_default() {
                        {
                            let u_id = target_u.id.clone();
                            let u_name = target_u.username.clone();
                            let tid_curr = tid_move_default.clone();
                            rsx! {
                                ConfirmDialog {
                                    title: "Move to Default Tenant".to_string(),
                                    message: format!(
                                        "Reassign user \"{}\" from tenant \"{}\" to Default Tenant?",
                                        u_name, tenant_name
                                    ),
                                    open: true,
                                    confirm_label: "Move to Default Tenant".to_string(),
                                    danger: false,
                                    on_confirm: move |_| {
                                        let uid = u_id.clone();
                                        let tid = tid_curr.clone();
                                        confirm_move_default.set(None);
                                        spawn(async move {
                                            match api::remove_tenant_user(&tid, &uid).await {
                                                Ok(_) => {
                                                    state.toast(ToastKind::Success, "User reassigned to Default Tenant");
                                                    reload.call(());
                                                }
                                                Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                            }
                                        });
                                    },
                                    on_cancel: move |_| confirm_move_default.set(None),
                                }
                            }
                        }
                    }

                    // ── Confirm Delete Tenant Dialog ─────────────────────────
                    ConfirmDialog {
                        title: "Delete tenant".to_string(),
                        message: format!("Delete tenant \"{}\"? This cannot be undone.", t.name),
                        open: confirm_delete(),
                        confirm_label: "Delete",
                        danger: true,
                        on_confirm: move |_| {
                            let tid = tid_delete.clone();
                            spawn(async move {
                                match api::delete_tenant(&tid).await {
                                    Ok(_) => {
                                        state.toast(ToastKind::Success, "Tenant deleted");
                                        let _ = dioxus_router::hooks::use_navigator().replace(Route::TenantsPage {});
                                    }
                                    Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                }
                            });
                        },
                        on_cancel: move |_| confirm_delete.set(false),
                    }
                }
            }
        }
    }
}
