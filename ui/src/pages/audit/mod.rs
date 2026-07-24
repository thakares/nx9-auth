//! Enterprise audit log viewer.

use crate::components::feedback::{EmptyState, ErrorState, LoadingSpinner};
use crate::components::navigation::Breadcrumb;
use crate::components::tables::{Pagination, SearchBox};
use crate::models::AuditResponse;
use crate::routes::Route;
use crate::services::api;
use crate::utils::{format_datetime, severity_badge_class};
use dioxus::prelude::*;

#[component]
pub fn AuditPage() -> Element {
    let mut data = use_signal(|| Option::<AuditResponse>::None);
    let mut error = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| true);

    let mut query = use_signal(String::new);
    let mut action = use_signal(String::new);
    let mut resource = use_signal(String::new);
    let mut severity = use_signal(|| "all".to_string());
    let mut success = use_signal(|| "all".to_string());
    let mut since = use_signal(String::new);
    let mut until = use_signal(String::new);
    let mut page = use_signal(|| 0usize);
    let page_size = 25usize;

    let load = use_callback(move |_: ()| {
        loading.set(true);
        let mut parts = vec![
            format!("limit={page_size}"),
            format!("offset={}", page() * page_size),
        ];
        if !query().is_empty() {
            parts.push(format!("q={}", urlencoding_lite(&query())));
        }
        if !action().is_empty() {
            parts.push(format!("action={}", urlencoding_lite(&action())));
        }
        if !resource().is_empty() {
            parts.push(format!("resource_type={}", urlencoding_lite(&resource())));
        }
        if severity() != "all" {
            parts.push(format!("severity={}", severity()));
        }
        if success() == "true" {
            parts.push("success=true".to_string());
        } else if success() == "false" {
            parts.push("success=false".to_string());
        }
        if !since().is_empty() {
            parts.push(format!("since={}", urlencoding_lite(&since())));
        }
        if !until().is_empty() {
            parts.push(format!("until={}", urlencoding_lite(&until())));
        }
        let qs = parts.join("&");
        spawn(async move {
            match api::list_audit(&qs).await {
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
        load.call(());
    });

    rsx! {
        Breadcrumb {
            items: vec![
                ("Dashboard".to_string(), Some(Route::DashboardPage {})),
                ("Audit Log".to_string(), None),
            ],
        }

        div { class: "page-header",
            div {
                h1 { "Audit Log" }
                p { class: "desc", "Security and administrative event history" }
            }
            div { class: "row",
                button {
                    class: "btn btn-outline",
                    r#type: "button",
                    title: "Export filtered audit log records as CSV",
                    onclick: move |_| {
                        let mut parts = vec![
                            "limit=5000".to_string(),
                            "offset=0".to_string(),
                        ];

                        if !query().is_empty() {
                            parts.push(format!("q={}", urlencoding_lite(&query())));
                        }
                        if !action().is_empty() {
                            parts.push(format!("action={}", urlencoding_lite(&action())));
                        }
                        if !resource().is_empty() {
                            parts.push(format!(
                                "resource_type={}",
                                urlencoding_lite(&resource())
                            ));
                        }
                        if severity() != "all" {
                            parts.push(format!("severity={}", severity()));
                        }
                        if success() == "true" {
                            parts.push("success=true".to_string());
                        } else if success() == "false" {
                            parts.push("success=false".to_string());
                        }
                        if !since().is_empty() {
                            parts.push(format!("since={}", urlencoding_lite(&since())));
                        }
                        if !until().is_empty() {
                            parts.push(format!("until={}", urlencoding_lite(&until())));
                        }

                        let qs = parts.join("&");
                        let export_url = format!("/api/v1/audit/export?{qs}");

                        #[cfg(target_arch = "wasm32")]
                        {
                            use wasm_bindgen::JsCast;

                            if let Some(window) = web_sys::window() {
                                if let Some(document) = window.document() {
                                    if let Ok(element) = document.create_element("a") {
                                        let _ = element.set_attribute("href", &export_url);
                                        let _ = element.set_attribute(
                                            "download",
                                            "audit_export.csv",
                                        );

                                        if let Ok(html_element) =
                                            element.dyn_into::<web_sys::HtmlElement>()
                                        {
                                            html_element.click();
                                        }
                                    }
                                }
                            }
                        }

                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let _ = export_url;
                        }
                    },
                    "Export CSV"
                }
                button {
                    class: "btn btn-outline",
                    r#type: "button",
                    onclick: move |_| load.call(()),
                    "Refresh"
                }
            }
        }

        div { class: "card mb-2",
            div { class: "card-body",
                div { class: "toolbar", style: "margin:0;",
                    SearchBox {
                        value: query(),
                        oninput: move |v| query.set(v),
                        placeholder: "Search action, resource, IP…",
                    }
                    input {
                        class: "form-control",
                        style: "width:auto;max-width:140px;",
                        placeholder: "Action",
                        value: "{action()}",
                        oninput: move |e| action.set(e.value()),
                    }
                    input {
                        class: "form-control",
                        style: "width:auto;max-width:140px;",
                        placeholder: "Resource",
                        value: "{resource()}",
                        oninput: move |e| resource.set(e.value()),
                    }
                    select {
                        class: "form-control",
                        style: "width:auto;",
                        value: "{severity()}",
                        onchange: move |e| severity.set(e.value()),
                        option { value: "all", "All severities" }
                        option { value: "info", "Info" }
                        option { value: "warning", "Warning" }
                        option { value: "critical", "Critical" }
                    }
                    select {
                        class: "form-control",
                        style: "width:auto;",
                        value: "{success()}",
                        onchange: move |e| success.set(e.value()),
                        option { value: "all", "Success/Fail" }
                        option { value: "true", "Success" }
                        option { value: "false", "Failure" }
                    }
                    input {
                        class: "form-control",
                        style: "width:auto;",
                        r#type: "date",
                        value: "{since()}",
                        oninput: move |e| since.set(e.value()),
                        title: "Since",
                    }
                    input {
                        class: "form-control",
                        style: "width:auto;",
                        r#type: "date",
                        value: "{until()}",
                        oninput: move |e| until.set(e.value()),
                        title: "Until",
                    }
                    button {
                        class: "btn btn-primary",
                        r#type: "button",
                        onclick: move |_| {
                            page.set(0);
                            load.call(());
                        },
                        "Apply"
                    }
                }
            }
        }

        if loading() {
            LoadingSpinner {}
        } else if let Some(err) = error() {
            ErrorState { message: err, on_retry: move |_| load.call(()) }
        } else if let Some(d) = data() {
            if d.entries.is_empty() {
                EmptyState {
                    title: "No audit entries".to_string(),
                    description: "Try broadening your filters.",
                    icon: "📋",
                }
            } else {
                div { class: "table-wrap",
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "Time" }
                                th { "Action" }
                                th { "Resource" }
                                th { "Severity" }
                                th { "Result" }
                                th { "Actor" }
                                th { "IP" }
                            }
                        }
                        tbody {
                            for e in d.entries {
                                tr { key: "{e.id}",
                                    td { class: "mono", "{format_datetime(&e.created_at)}" }
                                    td {
                                        code { "{e.action}" }
                                    }
                                    td {
                                        span { "{e.resource_type}" }
                                        if let Some(rid) = &e.resource_id {
                                            div {
                                                class: "mono text-muted",
                                                style: "font-size:11px;",
                                                "{rid}"
                                            }
                                        }
                                    }
                                    td {
                                        span { class: "{severity_badge_class(&e.severity)}",
                                            "{e.severity}"
                                        }
                                    }
                                    td {
                                        if e.success {
                                            span { class: "badge badge-success", "ok" }
                                        } else {
                                            span { class: "badge badge-danger", "fail" }
                                        }
                                    }
                                    td { class: "mono",
                                        "{e.actor_user_id.as_deref().unwrap_or(\"—\")}"
                                    }
                                    td { class: "mono", "{e.ip_address.as_deref().unwrap_or(\"—\")}" }
                                }
                            }
                        }
                    }
                }
                Pagination {
                    page: page(),
                    page_size,
                    total: d.total as usize,
                    on_page: move |p| {
                        page.set(p);
                        load.call(());
                    },
                }
            }
        }
    }
}

fn urlencoding_lite(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}
