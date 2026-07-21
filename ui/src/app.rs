//! Root application component.

use crate::routes::Route;
use crate::services::api;
use crate::state::{AppState, BootstrapState};
use dioxus::prelude::*;

#[component]
pub fn App() -> Element {
    let state = AppState::provide();

    let mut auth = state.auth;
    use_future(move || async move {
        // Only run initialization once.
        if !matches!(auth(), BootstrapState::Initializing) {
            return;
        }
        
        match api::me().await {
            Ok(Some(me)) => {
                auth.set(BootstrapState::Authenticated(me));
            }
            Ok(None) => {
                auth.set(BootstrapState::Anonymous);
            }
            Err(e) => {
                auth.set(BootstrapState::Failed(e.to_string()));
            }
        }
    });

    rsx! {
        ToastStack {}
        match auth() {
            BootstrapState::Initializing => {
                rsx! {
                    div { class: "loading-center", style: "min-height: 100vh;",
                        div { class: "spinner spinner-lg" }
                        span { "Starting nx9-auth…" }
                    }
                }
            }
            BootstrapState::Failed(err) => {
                rsx! {
                    div { class: "loading-center", style: "min-height: 100vh; color: #b91c1c;",
                        h1 { "Initialization Error" }
                        pre { "{err}" }
                    }
                }
            }
            BootstrapState::Authenticated(_) | BootstrapState::Anonymous => {
                rsx! { Router::<Route> {} }
            }
        }
    }
}

#[component]
fn ToastStack() -> Element {
    let state = use_context::<AppState>();
    let toasts = state.toasts;

    rsx! {
        div { class: "toast-stack", role: "status", "aria-live": "polite",
            for t in toasts() {
                {
                    let id = t.id;
                    let kind = t.kind.css();
                    let msg = t.message.clone();
                    rsx! {
                        div { class: "toast {kind}", key: "{id}",
                            div { class: "msg", "{msg}" }
                            button {
                                class: "close",
                                r#type: "button",
                                "aria-label": "Dismiss",
                                onclick: move |_| state.dismiss_toast(id),
                                "×"
                            }
                        }
                    }
                }
            }
        }
    }
}
