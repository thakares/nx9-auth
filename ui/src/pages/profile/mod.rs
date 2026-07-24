//! Profile management.

use crate::components::feedback::{ErrorState, LoadingSpinner};
use crate::components::forms::{PasswordInput, TextInput};
use crate::components::navigation::Breadcrumb;
use crate::components::widgets::{Avatar, StatusChip};
use crate::models::ProfileResponse;
use crate::routes::Route;
use crate::services::api;
use crate::state::{AppState, ToastKind};
use crate::utils::format_datetime;
use dioxus::prelude::*;

#[component]
pub fn ProfilePage() -> Element {
    let state = use_context::<AppState>();
    let mut data = use_signal(|| Option::<ProfileResponse>::None);
    let mut error = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| true);

    let mut email = use_signal(String::new);
    let mut full_name = use_signal(String::new);
    let mut current_pw = use_signal(String::new);
    let mut new_pw = use_signal(String::new);

    let reload = use_callback(move |_: ()| {
        loading.set(true);
        spawn(async move {
            match api::get_profile().await {
                Ok(p) => {
                    email.set(p.profile.email.clone().unwrap_or_default());
                    full_name.set(p.profile.full_name.clone().unwrap_or_default());
                    data.set(Some(p));
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
            ("Profile".to_string(), None),
        ]}

        div { class: "page-header",
            div {
                h1 { "Profile" }
                p { class: "desc", "Your account details and security settings" }
            }
        }

        if loading() {
            LoadingSpinner {}
        } else if let Some(err) = error() {
            ErrorState { message: err, on_retry: move |_| reload.call(()) }
        } else if let Some(p) = data() {
            div { class: "grid-2",
                div { class: "card",
                    div { class: "card-body",
                        div { class: "row mb-2", style: "gap:1rem;",
                            Avatar { name: p.user.username.clone(), size: "lg" }
                            div {
                                h2 { "{p.user.username}" }
                                StatusChip { status: p.user.status.clone() }
                                p { class: "text-muted mono", style: "font-size:12px;margin:0.25rem 0 0;",
                                    "{p.user.id}"
                                }
                            }
                        }
                        p { class: "text-secondary",
                            "Member since {format_datetime(&p.user.created_at)}"
                        }
                        p { class: "text-secondary",
                            "Roles: "
                            for (i, r) in p.roles.iter().enumerate() {
                                if i > 0 { span { ", " } }
                                span { class: "badge badge-accent", "{r}" }
                            }
                        }
                    }
                }

                div { class: "card",
                    div { class: "card-header", h3 { "Profile details" } }
                    div { class: "card-body",
                        TextInput {
                            label: "Full name",
                            name: "full_name",
                            value: full_name(),
                            oninput: move |v| full_name.set(v),
                            placeholder: "Ada Lovelace",
                            autocomplete: "name",
                        }
                        TextInput {
                            label: "Email",
                            name: "email",
                            value: email(),
                            oninput: move |v| email.set(v),
                            input_type: "email",
                            placeholder: "you@example.com",
                            autocomplete: "email",
                        }
                        button {
                            class: "btn btn-primary", r#type: "button",
                            onclick: move |_| {
                                let e = email();
                                let n = full_name();
                                spawn(async move {
                                    match api::update_profile(
                                        Some(e.as_str()).filter(|s| !s.is_empty()),
                                        Some(n.as_str()).filter(|s| !s.is_empty()),
                                    ).await {
                                        Ok(_) => state.toast(ToastKind::Success, "Profile updated"),
                                        Err(err) => state.toast(ToastKind::Error, err.to_string()),
                                    }
                                });
                            },
                            "Save profile"
                        }
                    }
                }

                div { class: "card",
                    div { class: "card-header", h3 { "Change password" } }
                    div { class: "card-body",
                        PasswordInput {
                            label: "Current password",
                            name: "current_password",
                            value: current_pw(),
                            oninput: move |v| current_pw.set(v),
                            autocomplete: "current-password",
                        }
                        PasswordInput {
                            label: "New password",
                            name: "new_password",
                            value: new_pw(),
                            oninput: move |v| new_pw.set(v),
                            autocomplete: "new-password",
                        }
                        p { class: "form-hint",
                            "Minimum 8 characters (12 for admins). Avoid common sequences like \"password\" or \"admin123\"."
                        }
                        button {
                            class: "btn btn-primary", r#type: "button",
                            disabled: current_pw().is_empty() || new_pw().len() < 8,
                            onclick: move |_| {
                                let cur = current_pw();
                                let neu = new_pw();
                                if cur.is_empty() || neu.is_empty() {
                                    state.toast(ToastKind::Error, "Both password fields are required");
                                    return;
                                }
                                if neu.len() < 8 {
                                    state.toast(ToastKind::Error, "New password must be at least 8 characters");
                                    return;
                                }
                                spawn(async move {
                                    match api::change_password(&cur, &neu).await {
                                        Ok(()) => {
                                            state.toast(ToastKind::Success, "Password changed");
                                            current_pw.set(String::new());
                                            new_pw.set(String::new());
                                        }
                                        Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                    }
                                });
                            },
                            "Update password"
                        }
                    }
                }

                div { class: "card",
                    div { class: "card-header", h3 { "Planned security features" } }
                    div { class: "card-body stack",
                        div { class: "row", style: "justify-content:space-between;",
                            span { "Avatar upload" }
                            span { class: "badge", "planned" }
                        }
                        div { class: "row", style: "justify-content:space-between;",
                            span { "Multi-factor authentication" }
                            span { class: "badge", "planned" }
                        }
                        div { class: "row", style: "justify-content:space-between;",
                            span { "Recovery codes" }
                            span { class: "badge", "planned" }
                        }
                    }
                }

                div { class: "card",
                    div { class: "card-header", h3 { "Active sessions" } }
                    div { class: "card-body",
                        if p.sessions.is_empty() {
                            p { class: "text-muted", "No active sessions." }
                        } else {
                            for s in p.sessions.iter().take(8) {
                                div { style: "display:flex;justify-content:space-between;font-size:13px;margin-bottom:0.4rem;",
                                    span { class: "mono",
                                        "{s.get(\"ip_address\").and_then(|v| v.as_str()).unwrap_or(\"—\")}"
                                    }
                                    span { class: "text-muted",
                                        "{format_datetime(s.get(\"last_seen_at\").and_then(|v| v.as_str()).unwrap_or(\"\"))}"
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
