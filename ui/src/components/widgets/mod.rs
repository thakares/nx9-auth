//! Small presentational widgets.

use dioxus::prelude::*;

#[component]
pub fn Avatar(name: String, #[props(default)] size: String) -> Element {
    let initials = crate::utils::initials(&name);
    let size_cls = match size.as_str() {
        "sm" => "avatar avatar-sm",
        "lg" => "avatar avatar-lg",
        _ => "avatar",
    };
    rsx! {
        div {
            class: "{size_cls}",
            title: "{name}",
            "aria-hidden": "true",
            "{initials}"
        }
    }
}

#[component]
pub fn Badge(text: String, #[props(default)] kind: String) -> Element {
    let cls = match kind.as_str() {
        "success" => "badge badge-success",
        "warning" => "badge badge-warning",
        "danger" => "badge badge-danger",
        "info" => "badge badge-info",
        "accent" => "badge badge-accent",
        _ => "badge",
    };
    rsx! { span { class: "{cls}", "{text}" } }
}

#[component]
pub fn StatusChip(status: String) -> Element {
    let cls = crate::utils::status_badge_class(&status);
    rsx! { span { class: "{cls}", "{status}" } }
}

#[component]
pub fn Card(
    #[props(default)] title: String,
    #[props(default)] actions: Option<Element>,
    children: Element,
) -> Element {
    rsx! {
        div { class: "card",
            if !title.is_empty() || actions.is_some() {
                div { class: "card-header",
                    if !title.is_empty() {
                        h3 { "{title}" }
                    } else {
                        span {}
                    }
                    if let Some(a) = actions {
                        div { class: "row", {a} }
                    }
                }
            }
            div { class: "card-body", {children} }
        }
    }
}

#[component]
pub fn StatCard(
    label: String,
    value: String,
    #[props(default)] hint: String,
) -> Element {
    rsx! {
        div { class: "stat-card",
            div { class: "label", "{label}" }
            div { class: "value", "{value}" }
            if !hint.is_empty() {
                div { class: "hint", "{hint}" }
            }
        }
    }
}

/// Renders children only when the current user holds the given permission.
#[component]
pub fn PermissionGate(permission: String, children: Element) -> Element {
    let state = use_context::<crate::state::AppState>();
    let auth = state.auth;
    if auth().has_permission(&permission) {
        rsx! { {children} }
    } else {
        rsx! {}
    }
}
