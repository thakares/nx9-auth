//! Application shell layout.

use crate::components::navigation::{Header, Sidebar};
use crate::routes::Route;
use crate::state::{AppState, BootstrapState};
use dioxus::prelude::*;

/// Persistent shell: header + sidebar + main content outlet.
#[component]
pub fn AppLayout() -> Element {
    let state = use_context::<AppState>();
    let auth = state.auth;
    let nav = use_navigator();

    // Redirect unauthenticated users to login — but only after session restore
    // has finished. Never bounce while Unknown/Loading.
    use_effect(move || {
        if matches!(auth(), BootstrapState::Anonymous) {
            nav.replace(Route::LoginPage {});
        }
    });

    match auth() {
        BootstrapState::Initializing | BootstrapState::Failed(_) => {
            // Layout is typically hidden/minimal during initial load or hard failure.
            return rsx! {
                div { class: "app-loading" }
            };
        }
        BootstrapState::Anonymous => {
            return rsx! {
                div { class: "loading-center", style: "min-height: 100vh;",
                    div { class: "spinner spinner-lg" }
                    span { "Redirecting to login…" }
                }
            };
        }
        BootstrapState::Authenticated(_) => {}
    }

    let shell_class = if (state.mobile_nav_open)() {
        "app-shell mobile-nav-open"
    } else if (state.sidebar_collapsed)() {
        "app-shell sidebar-collapsed"
    } else {
        "app-shell"
    };

    rsx! {
        div { class: "{shell_class}",
            Header {}
            Sidebar {}
            div { class: "app-main",
                div { class: "main-inner",
                    Outlet::<Route> {}
                }
                footer { class: "app-footer",
                    span { "nx9-auth · Identity & Access Management" }
                    span { class: "text-muted", "Pure Rust · Self-hosted · FOSS" }
                }
            }
        }
    }
}
