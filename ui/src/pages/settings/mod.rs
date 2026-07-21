use crate::components::navigation::Breadcrumb;
use crate::routes::Route;
use crate::state::{AppState, ToastKind};
use crate::theme::ThemeMode;
use crate::services::api;
use dioxus::prelude::*;

#[component]
pub fn SettingsPage() -> Element {
    let state = use_context::<AppState>();
    let theme = state.theme;

    rsx! {
        Breadcrumb { items: vec![
            ("Dashboard".to_string(), Some(Route::DashboardPage {})),
            ("Settings".to_string(), None),
        ]}

        div { class: "page-header",
            div {
                h1 { "Settings" }
                p { class: "desc", "Preferences for your account and workspace" }
            }
        }

        div { class: "grid-2",
            div { class: "card",
                div { class: "card-header", h3 { "Appearance" } }
                div { class: "card-body stack",
                    p { class: "text-secondary", "Theme preference is stored in this browser." }
                    div { class: "row", style: "gap:0.5rem;flex-wrap:wrap;",
                        for mode in [ThemeMode::Light, ThemeMode::Dark, ThemeMode::System] {
                            {
                                let active = theme() == mode;
                                let cls = if active { "btn btn-primary" } else { "btn btn-outline" };
                                rsx! {
                                    button {
                                        class: "{cls}",
                                        r#type: "button",
                                        onclick: move |_| state.set_theme(mode),
                                        "{mode.icon()} {mode.label()}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "card",
                div { class: "card-header", h3 { "System Information" } }
                div { class: "card-body stack",
                    p { class: "text-secondary",
                        "System configuration and state."
                    }
                    div { class: "row", style: "justify-content:space-between;",
                        span { "Session TTL" }
                        span { class: "badge", "config.toml" }
                    }
                    div { class: "row", style: "justify-content:space-between;",
                        span { "Password policy" }
                        span { class: "badge", "config.toml" }
                    }
                    div { class: "row", style: "justify-content:space-between;",
                        span { "Rate limiting" }
                        span { class: "badge", "enabled" }
                    }
                }
            }

            div { class: "card",
                div { class: "card-header", h3 { "Sessions" } }
                div { class: "card-body stack",
                    p { "Current session is active." }
                    button {
                        class: "btn btn-danger",
                        r#type: "button",
                        onclick: move |_| {
                            spawn(async move {
                                match api::terminate_other_sessions().await {
                                    Ok(_) => state.toast(ToastKind::Success, "Other sessions terminated"),
                                    Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                }
                            });
                        },
                        "Sign out from all other devices"
                    }
                }
            }

            div { class: "card",
                div { class: "card-header", h3 { "Security" } }
                div { class: "card-body stack",
                    p { class: "text-secondary", "Passwords must be at least 8 characters long." }
                    Link {
                        class: "btn btn-outline",
                        to: Route::ProfilePage {},
                        "Change password"
                    }
                }
            }
        }
    }
}
