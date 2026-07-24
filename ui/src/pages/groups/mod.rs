use crate::components::feedback::{ConfirmDialog, EmptyState, ErrorState, LoadingSpinner, Modal};
use crate::components::forms::TextInput;
use crate::components::navigation::Breadcrumb;
use crate::components::tables::{ColumnDef, DataTable};
use crate::models::{GroupView, UserView};
use crate::routes::Route;
use crate::services::api;
use crate::state::{AppState, ToastKind};
use crate::utils::{format_datetime, matches_query};
use dioxus::prelude::*;

#[component]
pub fn GroupsPage() -> Element {
    let state = use_context::<AppState>();
    let mut groups = use_signal(Vec::<GroupView>::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| true);
    let mut query = use_signal(String::new);
    let mut page = use_signal(|| 0usize);
    let page_size = 10usize;
    let mut sort_key = use_signal(|| "name".to_string());

    let mut show_create = use_signal(|| false);
    let mut name = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut delete_group = use_signal(|| Option::<GroupView>::None);

    let reload = use_callback(move |_: ()| {
        loading.set(true);
        error.set(None);
        spawn(async move {
            match api::list_groups().await {
                Ok(list) => {
                    groups.set(list);
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

    let mut filtered: Vec<GroupView> = groups()
        .into_iter()
        .filter(|g| {
            matches_query(&g.name, &query())
                || g.description
                    .as_deref()
                    .map(|d| matches_query(d, &query()))
                    .unwrap_or(false)
        })
        .collect();

    let sk = sort_key();
    filtered.sort_by(|a, b| match sk.as_str() {
        "members" => b.member_count.cmp(&a.member_count),
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    let total = filtered.len();
    let page_items: Vec<GroupView> = filtered
        .into_iter()
        .skip(page() * page_size)
        .take(page_size)
        .collect();

    rsx! {
        Breadcrumb { items: vec![
            ("Dashboard".to_string(), Some(Route::DashboardPage {})),
            ("Groups".to_string(), None),
        ]}

        div { class: "page-header",
            div {
                h1 { "Groups" }
                p { class: "desc", "Manage user groups" }
            }
            button {
                class: "btn btn-primary",
                r#type: "button",
                onclick: move |_| show_create.set(true),
                "+ Create group"
            }
        }

        if loading() {
            LoadingSpinner {}
        } else if let Some(err) = error() {
            ErrorState { message: err, on_retry: move |_| reload.call(()) }
        } else if groups().is_empty() && query().is_empty() {
            EmptyState { title: "No groups".to_string(), description: "Create your first group.", icon: "👥" }
        } else {
            DataTable {
                columns: vec![
                    ColumnDef { key: "name".into(), label: "Name".into(), sortable: true, visible: true },
                    ColumnDef { key: "description".into(), label: "Description".into(), sortable: false, visible: true },
                    ColumnDef { key: "members".into(), label: "Members".into(), sortable: true, visible: true },
                    ColumnDef { key: "created".into(), label: "Created".into(), sortable: true, visible: true },
                    ColumnDef { key: "actions".into(), label: "Actions".into(), sortable: false, visible: true },
                ],
                on_search: move |v| { query.set(v); page.set(0); },
                search_value: query(),
                search_placeholder: "Search groups…".to_string(),
                on_sort: move |k| sort_key.set(k),
                sort_key: sort_key(),
                on_page: move |p| page.set(p),
                page: page(),
                page_size: page_size,
                total: total,
                toolbar_actions: rsx! {
                    button { class: "btn btn-outline", r#type: "button", onclick: move |_| reload.call(()), "Refresh" }
                },
                for g in page_items {
                    {
                        let id = g.id.clone();
                        let g2 = g.clone();
                        rsx! {
                            tr { key: "{g.id}",
                                td {
                                    Link {
                                        to: Route::GroupDetailPage { id: g.id.clone() },
                                        strong { "{g.name}" }
                                    }
                                }
                                td { class: "text-secondary", "{g.description.as_deref().unwrap_or(\"—\")}" }
                                td { span { class: "badge", "{g.member_count}" } }
                                td { "{format_datetime(&g.created_at)}" }
                                td { style: "text-align: right;",
                                    div { class: "actions",
                                        Link {
                                            class: "btn btn-sm btn-outline",
                                            to: Route::GroupDetailPage { id: id },
                                            "View"
                                        }
                                        button {
                                            class: "btn btn-sm btn-danger",
                                            r#type: "button",
                                            onclick: move |_| delete_group.set(Some(g2.clone())),
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
            title: "Create group".to_string(),
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
                            match api::create_group(&n, Some(d.as_str()).filter(|s| !s.is_empty())).await {
                                Ok(_) => {
                                    state.toast(ToastKind::Success, "Group created");
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

        ConfirmDialog {
            title: "Delete group".to_string(),
            message: format!("Delete group \"{}\"? This cannot be undone.", delete_group().as_ref().map(|g| g.name.as_str()).unwrap_or("")),
            open: delete_group().is_some(),
            confirm_label: "Delete",
            danger: true,
            on_confirm: move |_| {
                if let Some(g) = delete_group() {
                    spawn(async move {
                        match api::delete_group(&g.id).await {
                            Ok(()) => {
                                state.toast(ToastKind::Success, "Group deleted");
                                delete_group.set(None);
                                reload.call(());
                            }
                            Err(e) => state.toast(ToastKind::Error, e.to_string()),
                        }
                    });
                }
            },
            on_cancel: move |_| delete_group.set(None),
        }
    }
}

#[component]
pub fn GroupDetailPage(id: String) -> Element {
    let state = use_context::<AppState>();
    let mut group = use_signal(|| Option::<crate::models::GroupDetailResponse>::None);
    let mut all_users = use_signal(Vec::<UserView>::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| true);

    let mut add_user_id = use_signal(String::new);

    let mut edit_mode = use_signal(|| false);
    let mut edit_name = use_signal(String::new);
    let mut edit_desc = use_signal(String::new);

    let mut confirm_delete = use_signal(|| false);

    let group_id = id.clone();
    let reload = use_callback(move |_: ()| {
        let id = group_id.clone();
        loading.set(true);
        spawn(async move {
            match api::get_group(&id).await {
                Ok(detail) => {
                    group.set(Some(detail));
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

    rsx! {
        Breadcrumb { items: vec![
            ("Dashboard".to_string(), Some(Route::DashboardPage {})),
            ("Groups".to_string(), Some(Route::GroupsPage {})),
            (group().map(|g| g.group.name.clone()).unwrap_or(id.clone()), None),
        ]}

        if loading() {
            LoadingSpinner {}
        } else if let Some(err) = error() {
            ErrorState { message: err, on_retry: move |_| reload.call(()) }
        } else if let Some(detail) = group() {
            {
                let gid = detail.group.id.clone();
                let gid2 = detail.group.id.clone();
                let gid3 = detail.group.id.clone();
                let gname = detail.group.name.clone();
                rsx! {
                    div { class: "page-header",
                        div {
                            h1 { "{detail.group.name}" }
                            p { class: "desc", "{detail.group.description.clone().unwrap_or_else(|| \"No description\".to_string())}" }
                        }
                        div { class: "row",
                            button {
                                class: "btn btn-outline",
                                r#type: "button",
                                onclick: move |_| {
                                    edit_name.set(detail.group.name.clone());
                                    edit_desc.set(detail.group.description.clone().unwrap_or_default());
                                    edit_mode.set(true);
                                },
                                "Edit"
                            }
                            button {
                                class: "btn btn-danger",
                                r#type: "button",
                                onclick: move |_| confirm_delete.set(true),
                                "Delete"
                            }
                        }
                    }

                    div { class: "card",
                        div { class: "card-header", h3 { "Members" } }
                        div { class: "card-body",
                            div { class: "row mb-2",
                                select {
                                    class: "form-control",
                                    value: "{add_user_id()}",
                                    onchange: move |e| add_user_id.set(e.value()),
                                    option { value: "", "Select user to add…" }
                                    for u in all_users() {
                                        if !detail.members.iter().any(|m| m.id == u.id) {
                                            option { value: "{u.id}", "{u.username}" }
                                        }
                                    }
                                }
                                button {
                                    class: "btn btn-primary",
                                    r#type: "button",
                                    disabled: add_user_id().is_empty(),
                                    onclick: move |_| {
                                        let uid = add_user_id();
                                        let gid = gid.clone();
                                        spawn(async move {
                                            match api::add_group_member(&gid, &uid).await {
                                                Ok(_) => {
                                                    state.toast(ToastKind::Success, "Member added");
                                                    add_user_id.set(String::new());
                                                    reload.call(());
                                                }
                                                Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                            }
                                        });
                                    },
                                    "Add member"
                                }
                            }

                            if detail.members.is_empty() {
                                p { class: "text-muted", "No members in this group." }
                            } else {
                                DataTable {
                                    columns: vec![
                                        ColumnDef { key: "username".into(), label: "Username".into(), sortable: false, visible: true },
                                        ColumnDef { key: "status".into(), label: "Status".into(), sortable: false, visible: true },
                                        ColumnDef { key: "actions".into(), label: "Actions".into(), sortable: false, visible: true },
                                    ],
                                    on_search: |_| {},
                                    search_value: "".to_string(),
                                    search_placeholder: "".to_string(),
                                    on_sort: |_| {},
                                    sort_key: "".to_string(),
                                    on_page: |_| {},
                                    page: 0,
                                    page_size: detail.members.len().max(1),
                                    total: detail.members.len(),
                                    for m in detail.members {
                                        {
                                            let uid = m.id.clone();
                                            let gid = gid2.clone();
                                            rsx! {
                                                tr { key: "{m.id}",
                                                    td {
                                                        Link {
                                                            to: Route::UserDetailPage { id: m.id.clone() },
                                                            "{m.username}"
                                                        }
                                                    }
                                                    td { "{m.status}" }
                                                    td { style: "text-align: right;",
                                                        button {
                                                            class: "btn btn-sm btn-outline btn-danger",
                                                            r#type: "button",
                                                            onclick: move |_| {
                                                                let uid = uid.clone();
                                                                let gid = gid.clone();
                                                                spawn(async move {
                                                                    match api::remove_group_member(&gid, &uid).await {
                                                                        Ok(_) => {
                                                                            state.toast(ToastKind::Success, "Member removed");
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
                            }
                        }
                    }

                    Modal {
                        title: "Edit group".to_string(),
                        open: edit_mode(),
                        on_close: move |_| edit_mode.set(false),
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
                        div { class: "modal-footer", style: "margin-top:1rem; padding:0; border:none; background:transparent;",
                            button { class: "btn btn-outline", r#type: "button",
                                onclick: move |_| edit_mode.set(false), "Cancel" }
                            button {
                                class: "btn btn-primary", r#type: "button",
                                onclick: move |_| {
                                    let n = edit_name();
                                    let d = edit_desc();
                                    let gid = gid3.clone();
                                    spawn(async move {
                                        match api::update_group(&gid, &n, Some(d.as_str()).filter(|s| !s.is_empty())).await {
                                            Ok(_) => {
                                                state.toast(ToastKind::Success, "Group updated");
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

                    ConfirmDialog {
                        title: "Delete group".to_string(),
                        message: format!("Delete group \"{}\"? This cannot be undone.", gname),
                        open: confirm_delete(),
                        confirm_label: "Delete",
                        danger: true,
                        on_confirm: move |_| {
                            let gid = detail.group.id.clone();
                            spawn(async move {
                                match api::delete_group(&gid).await {
                                    Ok(_) => {
                                        state.toast(ToastKind::Success, "Group deleted");
                                        let _ = dioxus_router::hooks::use_navigator().replace(Route::GroupsPage {});
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
