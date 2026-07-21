//! Authentication pages.
//!
//! Login submits credentials via **POST JSON** only. The HTML form uses
//! `method="post"` so a native fallback never puts passwords in the URL.

use crate::components::forms::{PasswordInput, TextInput};
use crate::models::MeResponse;
use crate::routes::Route;
use crate::services::api;
use crate::state::{AppState, BootstrapState, ToastKind};
use dioxus::prelude::*;

#[component]
pub fn LoginPage() -> Element {
    let state = use_context::<AppState>();
    let auth = state.auth;
    let nav = use_navigator();

    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| false);

    // If already authenticated, leave the login screen.
    use_effect(move || {
        if auth().is_authenticated() {
            nav.replace(Route::DashboardPage {});
        }
    });

    let on_submit = move |evt: Event<FormData>| {
        // Critical: prevent native form submission (which defaults to GET
        // and would put credentials in the query string / browser history).
        evt.prevent_default();
        if loading() {
            return;
        }

        let u = username().trim().to_string();
        let p = password();
        if u.is_empty() || p.is_empty() {
            error.set(Some("Please enter username and password.".into()));
            return;
        }

        loading.set(true);
        error.set(None);
        let mut auth = state.auth;
        let mut loading = loading;
        let mut error = error;
        let mut password = password;
        let nav = nav.clone();
        spawn(async move {
            match api::login(&u, &p).await {
                Ok(login) => {
                    // Clear password from UI memory after successful submit.
                    password.set(String::new());

                    // Prefer /auth/me; fall back to login payload user info.
                    let me = match api::me().await {
                        Ok(Some(m)) => m,
                        _ => {
                            // Build MeResponse from login.user if present
                            if let Some(user_val) = login.user {
                                MeResponse {
                                    user: crate::models::UserView {
                                        id: user_val
                                            .get("id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        username: user_val
                                            .get("username")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        status: user_val
                                            .get("status")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("active")
                                            .to_string(),
                                        last_login_at: user_val
                                            .get("last_login_at")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string()),
                                        created_at: user_val
                                            .get("created_at")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        updated_at: None,
                                    },
                                    roles: user_val
                                        .get("roles")
                                        .and_then(|v| v.as_array())
                                        .map(|a| {
                                            a.iter()
                                                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                                .collect()
                                        })
                                        .unwrap_or_default(),
                                    permissions: user_val
                                        .get("permissions")
                                        .and_then(|v| v.as_array())
                                        .map(|a| {
                                            a.iter()
                                                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                                .collect()
                                        })
                                        .unwrap_or_default(),
                                }
                            } else {
                                error.set(Some(
                                    "Signed in but session could not be verified. Try again."
                                        .into(),
                                ));
                                loading.set(false);
                                return;
                            }
                        }
                    };

                    auth.set(BootstrapState::Authenticated(me));
                    state.toast(ToastKind::Success, "Signed in successfully");
                    nav.replace(Route::DashboardPage {});
                }
                Err(e) => {
                    // Map API errors to a safe, non-enumerating message for creds.
                    let msg = match e {
                        api::ApiError::Unauthorized
                        | api::ApiError::InvalidInput(_)
                        | api::ApiError::Server(_) => {
                            // Prefer server body when it's the standard message
                            let s = e.to_string();
                            if s.to_lowercase().contains("invalid username")
                                || s.to_lowercase().contains("unauthorized")
                                || s.to_lowercase().contains("invalid credentials")
                            {
                                "Invalid username or password.".into()
                            } else {
                                s
                            }
                        }
                        other => other.to_string(),
                    };
                    error.set(Some(msg));
                    if !auth().is_authenticated() {
                        auth.set(BootstrapState::Anonymous);
                    }
                }
            }
            loading.set(false);
        });
    };

    rsx! {
        div { class: "auth-page",
            div { class: "auth-card",
                div { class: "logo-row",
                    div { class: "brand-mark", style: "width:36px;height:36px;border-radius:10px;background:linear-gradient(135deg,var(--accent),#7c3aed);display:grid;place-items:center;color:#fff;font-weight:800;",
                        "N9"
                    }
                    div {
                        h1 { "Sign in to nx9-auth" }
                        p { class: "subtitle", style: "margin:0;", "Identity & Access Management" }
                    }
                }

                if let Some(err) = error() {
                    div { class: "alert alert-error", role: "alert", "{err}" }
                }

                // method="post" is mandatory: HTML default is GET, which would
                // put credentials in the URL if preventDefault failed.
                form {
                    method: "post",
                    action: "#",
                    autocomplete: "on",
                    onsubmit: on_submit,
                    TextInput {
                        label: "Username",
                        name: "username",
                        value: username(),
                        oninput: move |v| username.set(v),
                        required: true,
                        autocomplete: "username",
                        placeholder: "admin",
                    }
                    PasswordInput {
                        label: "Password",
                        name: "password",
                        value: password(),
                        oninput: move |v| password.set(v),
                        required: true,
                        autocomplete: "current-password",
                    }
                    button {
                        class: "btn btn-primary",
                        r#type: "submit",
                        style: "width: 100%; margin-top: 0.5rem;",
                        disabled: loading() || username().trim().is_empty() || password().is_empty(),
                        if loading() {
                            span { class: "spinner", style: "width:14px;height:14px;border-width:2px;" }
                        }
                        if loading() { "Signing in…" } else { "Sign in" }
                    }
                }

                p { class: "text-muted", style: "margin-top: 1.25rem; font-size: 12px; text-align: center;",
                    "POST · Argon2id · Session tokens · No credentials in URLs"
                }
            }
        }
    }
}

#[component]
pub fn UnauthorizedPage() -> Element {
    rsx! {
        div { class: "auth-page",
            div { class: "auth-card", style: "text-align:center;",
                h1 { "401 — Unauthorized" }
                p { class: "subtitle", "Your session is missing or has expired." }
                Link { class: "btn btn-primary", to: Route::LoginPage {}, "Sign in" }
            }
        }
    }
}

#[component]
pub fn ForbiddenPage() -> Element {
    rsx! {
        div { class: "auth-page",
            div { class: "auth-card", style: "text-align:center;",
                h1 { "403 — Forbidden" }
                p { class: "subtitle", "You do not have permission to view this page." }
                Link { class: "btn btn-primary", to: Route::DashboardPage {}, "Back to dashboard" }
            }
        }
    }
}
