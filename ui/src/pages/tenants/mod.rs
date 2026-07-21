use crate::components::feedback::{ConfirmDialog, EmptyState, ErrorState, LoadingSpinner, Modal};
use crate::components::forms::TextInput;
use crate::components::navigation::Breadcrumb;
use crate::components::tables::{DataTable, ColumnDef};
use crate::models::TenantView;
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
                Ok(list) => { tenants.set(list); loading.set(false); }
                Err(e) => { error.set(Some(e.to_string())); loading.set(false); }
            }
        });
    });

    use_effect(move || { reload.call(()); });

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
    let page_items: Vec<TenantView> = filtered.into_iter().skip(page() * page_size).take(page_size).collect();

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

    let mut edit_name = use_signal(String::new);
    let mut edit_slug = use_signal(String::new);

    let mut confirm_delete = use_signal(|| false);

    let tenant_id = id.clone();
    let reload = use_callback(move |_: ()| {
        let id = tenant_id.clone();
        loading.set(true);
        spawn(async move {
            match api::get_tenant(&id).await {
                Ok(t) => {
                    edit_name.set(t.name.clone());
                    edit_slug.set(t.slug.clone());
                    tenant.set(Some(t));
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
            ("Tenants".to_string(), Some(Route::TenantsPage {})),
            (tenant().map(|t| t.name.clone()).unwrap_or(id.clone()), None),
        ]}

        if loading() {
            LoadingSpinner {}
        } else if let Some(err) = error() {
            ErrorState { message: err, on_retry: move |_| reload.call(()) }
        } else if let Some(t) = tenant() {
            {
                let tid = t.id.clone();
                let tid2 = t.id.clone();
                let tid3 = t.id.clone();
                rsx! {
                    div { class: "page-header",
                        div {
                            h1 { "{t.name}" }
                            p { class: "desc", "Tenant configuration and overview" }
                        }
                    }

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
                                button {
                                    class: "btn btn-primary mt-2",
                                    r#type: "button",
                                    onclick: move |_| {
                                        let n = edit_name();
                                        let s = edit_slug();
                                        let tid = tid.clone();
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
                                    disabled: tid2 == "00000000-0000-0000-0000-000000000001",
                                    onclick: move |_| confirm_delete.set(true),
                                    "Delete Tenant"
                                }
                            }
                        }
                    }

                    ConfirmDialog {
                        title: "Delete tenant".to_string(),
                        message: format!("Delete tenant \"{}\"? This cannot be undone.", t.name),
                        open: confirm_delete(),
                        confirm_label: "Delete",
                        danger: true,
                        on_confirm: move |_| {
                            let tid = tid3.clone();
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
