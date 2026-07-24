//! Permission browser / matrix.

use crate::components::feedback::{EmptyState, ErrorState, LoadingSpinner};
use crate::components::navigation::Breadcrumb;
use crate::components::tables::{ColumnDef, DataTable};
use crate::models::PermissionsResponse;
use crate::routes::Route;
use crate::services::api;
use crate::utils::matches_query;
use dioxus::prelude::*;

#[component]
pub fn PermissionsPage() -> Element {
    let mut data = use_signal(|| Option::<PermissionsResponse>::None);
    let mut error = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| true);
    let mut query = use_signal(String::new);
    let mut group_filter = use_signal(|| "all".to_string());
    let mut page = use_signal(|| 0usize);
    let page_size = 15usize;
    let mut sort_key = use_signal(|| "name".to_string());

    let reload = use_callback(move |_: ()| {
        loading.set(true);
        spawn(async move {
            match api::list_permissions().await {
                Ok(d) => {
                    data.set(Some(d));
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
            ("Permissions".to_string(), None),
        ]}

        div { class: "page-header",
            div {
                h1 { "Permissions" }
                p { class: "desc", "System permission catalog (assignment via Roles)" }
            }
            button { class: "btn btn-outline", r#type: "button", onclick: move |_| reload.call(()), "Refresh" }
        }

        if loading() {
            LoadingSpinner {}
        } else if let Some(err) = error() {
            ErrorState { message: err, on_retry: move |_| reload.call(()) }
        } else if let Some(d) = data() {
            {
                let groups: Vec<String> = d.groups.iter().map(|g| g.group.clone()).collect();
                let q = query();
                let gf = group_filter();

                // Flatten permissions for data table
                let mut all_perms: Vec<(String, crate::models::PermissionView)> = Vec::new();
                for g in &d.groups {
                    if gf == "all" || g.group == gf {
                        for p in &g.permissions {
                            if matches_query(&p.name, &q) || p.description.as_deref().map(|d| matches_query(d, &q)).unwrap_or(false) {
                                all_perms.push((g.group.clone(), p.clone()));
                            }
                        }
                    }
                }

                let sk = sort_key();
                all_perms.sort_by(|a, b| match sk.as_str() {
                    "group" => a.0.cmp(&b.0).then_with(|| a.1.name.cmp(&b.1.name)),
                    "description" => a.1.description.cmp(&b.1.description),
                    _ => a.1.name.cmp(&b.1.name),
                });

                let total = all_perms.len();
                let page_items: Vec<_> = all_perms.into_iter().skip(page() * page_size).take(page_size).collect();

                rsx! {
                    if total == 0 && q.is_empty() {
                        EmptyState {
                            title: "No permissions match".to_string(),
                            description: "",
                            icon: "✓",
                        }
                    } else {
                        DataTable {
                            columns: vec![
                                ColumnDef { key: "name".into(), label: "Name".into(), sortable: true, visible: true },
                                ColumnDef { key: "group".into(), label: "Group".into(), sortable: true, visible: true },
                                ColumnDef { key: "description".into(), label: "Description".into(), sortable: true, visible: true },
                            ],
                            on_search: move |v| { query.set(v); page.set(0); },
                            search_value: query(),
                            search_placeholder: "Search permissions…".to_string(),
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
                                    value: "{group_filter()}",
                                    onchange: move |e| { group_filter.set(e.value()); page.set(0); },
                                    option { value: "all", "All groups" }
                                    for g in groups {
                                        option { value: "{g}", "{g}" }
                                    }
                                }
                            },
                            for (group, p) in page_items {
                                {
                                    rsx! {
                                        tr { key: "{p.id}",
                                            td { code { "{p.name}" } }
                                            td { span { class: "badge badge-accent", "{group}" } }
                                            td { class: "text-secondary", "{p.description.as_deref().unwrap_or(\"—\")}" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    p { class: "text-muted mt-2", style: "font-size:12px;",
                        "Permission assignment is managed on the Roles page. Drag-and-drop matrix is planned."
                    }
                }
            }
        }
    }
}
