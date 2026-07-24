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
    let mut quick_create_open = use_signal(|| false);
    let mut mobile = state.mobile_nav_open;

    let username = auth().username().to_string();
    let theme_icon = theme().icon();
    let theme_label = theme().label();

    let auth_state = state.auth.read();
    let can_create_user = auth_state.has_permission("users:create") || auth_state.is_adminish();
    let can_create_tenant = auth_state.has_permission("roles:manage") || auth_state.is_adminish();
    let can_create_app = auth_state.has_permission("applications:manage") || auth_state.is_adminish();
    let can_create_role = auth_state.has_permission("roles:manage") || auth_state.is_adminish();
    let can_create_sa = auth_state.has_permission("service_accounts:manage") || auth_state.has_permission("roles:manage") || auth_state.is_adminish();
    let has_any_create = can_create_user || can_create_tenant || can_create_app || can_create_role || can_create_sa;
    drop(auth_state);

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

            // Tenant Indicator & Management Link
            div { class: "tenant-badge", style: "margin-left: 1rem; display: flex; align-items: center; gap: 0.5rem;",
                span { class: "icon", "🏢" }
                span { style: "font-weight: 500; font-size: 13px;",
                    {(state.tenant)().map(|t| t.name).unwrap_or("Default Tenant".to_string())}
                }
                Link {
                    class: "btn btn-xs btn-ghost text-primary",
                    to: Route::TenantsPage {},
                    "Manage tenants…"
                }
            }

            div { style: "flex: 1;" }

            div { class: "header-actions",
                if has_any_create {
                    div { class: "dropdown", style: "position: relative; margin-right: 0.5rem;",
                        button {
                            class: "btn btn-primary btn-sm",
                            r#type: "button",
                            title: "Quick Create",
                            "aria-haspopup": "menu",
                            "aria-expanded": "{quick_create_open()}",
                            onclick: move |_| quick_create_open.set(!quick_create_open()),
                            onkeydown: move |evt: KeyboardEvent| {
                                if evt.key() == Key::Escape {
                                    quick_create_open.set(false);
                                }
                            },
                            "➕ New ▾"
                        }
                        if quick_create_open() {
                            div {
                                class: "dropdown-backdrop",
                                style: "position: fixed; top: 0; left: 0; right: 0; bottom: 0; z-index: 999; background: transparent;",
                                onclick: move |_| quick_create_open.set(false),
                            }
                            div {
                                class: "dropdown-menu",
                                role: "menu",
                                style: "display: block; position: absolute; right: 0; top: 100%; z-index: 1000;",
                                onkeydown: move |evt: KeyboardEvent| {
                                    if evt.key() == Key::Escape {
                                        quick_create_open.set(false);
                                    }
                                },
                                if can_create_user {
                                    Link {
                                        class: "dropdown-item",
                                        to: "/users?create=1",
                                        onclick: move |_| quick_create_open.set(false),
                                        "👤 Create User"
                                    }
                                }
                                if can_create_tenant {
                                    Link {
                                        class: "dropdown-item",
                                        to: "/tenants?create=1",
                                        onclick: move |_| quick_create_open.set(false),
                                        "🏢 Create Tenant"
                                    }
                                }
                                if can_create_app {
                                    Link {
                                        class: "dropdown-item",
                                        to: "/applications?create=1",
                                        onclick: move |_| quick_create_open.set(false),
                                        "🚀 Create Application"
                                    }
                                }
                                if can_create_role {
                                    Link {
                                        class: "dropdown-item",
                                        to: "/roles?create=1",
                                        onclick: move |_| quick_create_open.set(false),
                                        "🛡️ Create Role"
                                    }
                                }
                                if can_create_sa {
                                    Link {
                                        class: "dropdown-item",
                                        to: "/service-accounts?create=1",
                                        onclick: move |_| quick_create_open.set(false),
                                        "🤖 Create Service Account"
                                    }
                                }
                            }
                        }
                    }
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
