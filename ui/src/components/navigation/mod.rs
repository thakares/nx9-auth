//! Navigation pieces: header, sidebar, breadcrumb, user menu.
pub mod registry;
use crate::routes::Route;
use crate::services::api;
use crate::state::{AppState, BootstrapState};
use crate::utils::initials;
use dioxus::prelude::*;

#[component]
pub fn Header() -> Element {
    let state = use_context::<AppState>();
    let auth = state.auth;
    let theme = state.theme;
    let mut menu_open = use_signal(|| false);
    let mut tenant_menu_open = use_signal(|| false);
    let mut mobile = state.mobile_nav_open;

    let username = auth().username().to_string();
    let theme_icon = theme().icon();
    let theme_label = theme().label();

    rsx! {
        header { class: "app-header",
            button {
                class: "btn btn-ghost btn-icon",
                r#type: "button",
                "aria-label": "Toggle navigation",
                style: "display: none;",
                onclick: move |_| mobile.set(!mobile()),
                "☰"
            }
            Link {
                class: "brand",
                to: Route::DashboardPage {},
                div { class: "brand-mark", "N9" }
                span { "nx9-auth" }
            }

            // Tenant Switcher
            div { class: "dropdown", style: "margin-left: 1rem;",
                button {
                    class: "btn btn-ghost",
                    r#type: "button",
                    "aria-haspopup": "menu",
                    "aria-expanded": "{tenant_menu_open()}",
                    onclick: move |_| tenant_menu_open.set(!tenant_menu_open()),
                    span { class: "icon", "🏢" }
                    span { style: "margin-left: 0.4rem; font-weight: 500;",
                        {(state.tenant)().map(|t| t.name).unwrap_or("Default Tenant".to_string())}
                    }
                    span { style: "margin-left: 0.25rem; opacity: 0.6;", "▾" }
                }
                if tenant_menu_open() {
                    div { class: "dropdown-menu", role: "menu",
                        button { class: "dropdown-item", r#type: "button", "Default Tenant" }
                        div { class: "dropdown-divider" }
                        Link {
                            class: "dropdown-item text-primary",
                            to: Route::TenantsPage {},
                            onclick: move |_| tenant_menu_open.set(false),
                            "Manage tenants…"
                        }
                    }
                }
            }

            // Global Search (Ctrl+K)
            div { class: "search", style: "flex: 1; max-width: 400px; margin: 0 2rem;",
                div { style: "position: relative;",
                    span { style: "position: absolute; left: 0.75rem; top: 50%; transform: translateY(-50%); opacity: 0.5;", "🔍" }
                    input {
                        class: "form-control",
                        style: "padding-left: 2rem; width: 100%;",
                        r#type: "search",
                        placeholder: "Search… (Ctrl+K)",
                        "aria-label": "Global search",
                    }
                }
            }
            
            div { class: "header-actions",
                // Quick Create
                button {
                    class: "btn btn-primary btn-sm",
                    style: "margin-right: 0.5rem;",
                    r#type: "button",
                    title: "Quick Create",
                    "➕ New"
                }

                // Notifications
                button {
                    class: "btn btn-ghost btn-icon",
                    r#type: "button",
                    title: "Notifications",
                    "aria-label": "Notifications",
                    "🔔"
                }

                // Theme
                button {
                    class: "btn btn-ghost btn-icon",
                    r#type: "button",
                    title: "Theme: {theme_label}",
                    "aria-label": "Switch theme (current: {theme_label})",
                    onclick: move |_| state.cycle_theme(),
                    "{theme_icon}"
                }

                // Profile Menu
                div { class: "dropdown",
                    button {
                        class: "btn btn-ghost",
                        r#type: "button",
                        "aria-haspopup": "menu",
                        "aria-expanded": "{menu_open()}",
                        onclick: move |_| menu_open.set(!menu_open()),
                        div { class: "avatar avatar-sm", "{initials(&username)}" }
                        span { style: "margin-left: 0.4rem;", "{username}" }
                        span { style: "margin-left: 0.25rem; opacity: 0.6;", "▾" }
                    }
                    if menu_open() {
                        div { class: "dropdown-menu", role: "menu",
                            Link {
                                class: "dropdown-item",
                                to: Route::ProfilePage {},
                                onclick: move |_| menu_open.set(false),
                                "👤 Profile"
                            }
                            Link {
                                class: "dropdown-item",
                                to: Route::SettingsPage {},
                                onclick: move |_| menu_open.set(false),
                                "⚙ Settings"
                            }
                            Link {
                                class: "dropdown-item",
                                to: Route::TokensPage {},
                                onclick: move |_| menu_open.set(false),
                                "🔑 API Tokens"
                            }
                            div { class: "dropdown-divider" }
                            button {
                                class: "dropdown-item",
                                r#type: "button",
                                onclick: move |_| {
                                    menu_open.set(false);
                                    let mut auth = state.auth;
                                    spawn(async move {
                                        let _ = api::logout().await;
                                        auth.set(BootstrapState::Anonymous);
                                    });
                                },
                                "⎋ Sign out"
                            }
                        }
                    }
                }
            }
        }
    }
}



#[component]
pub fn Sidebar() -> Element {
    let state = use_context::<AppState>();
    let auth = state.auth;
    let path = use_route::<Route>();
    let registry = (state.nav_registry)();

    let can_see = |perm: &Option<String>| -> bool {
        match perm {
            Some(p) => auth().has_permission(p.as_str()),
            None => true,
        }
    };

    let active = |r: &Route| -> bool {
        format!("{path:?}").split_whitespace().next()
            == format!("{r:?}").split_whitespace().next()
    };

    rsx! {
        nav { class: "app-sidebar", "aria-label": "Main",
            for (section_name, items) in registry.sections.iter() {
                // Only render section if at least one item is visible
                if items.iter().any(|item| can_see(&item.permission)) {
                    div { class: "nav-section", "{section_name}" }
                    for item in items {
                        if can_see(&item.permission) {
                            {
                                let cls = if active(&item.route) { "nav-link active" } else { "nav-link" };
                                rsx! {
                                    Link {
                                        class: "{cls}",
                                        to: item.route.clone(),
                                        span { class: "icon", "{item.icon}" }
                                        span { "{item.title}" }
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

#[component]
pub fn Breadcrumb(items: Vec<(String, Option<Route>)>) -> Element {
    rsx! {
        nav { class: "breadcrumb", "aria-label": "Breadcrumb",
            for (i, (label, route)) in items.into_iter().enumerate() {
                if i > 0 {
                    span { class: "sep", " / " }
                }
                if let Some(r) = route {
                    Link { to: r, "{label}" }
                } else {
                    span { class: "current", "{label}" }
                }
            }
        }
    }
}
