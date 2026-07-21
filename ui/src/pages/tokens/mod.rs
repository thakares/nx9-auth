//! Personal API token management.

use crate::components::feedback::{ConfirmDialog, EmptyState, ErrorState, LoadingSpinner, Modal};
use crate::components::forms::TextInput;
use crate::components::navigation::Breadcrumb;
use crate::components::tables::SearchBox;
use crate::components::widgets::StatusChip;
use crate::models::{CreateTokenResponse, TokenView};
use crate::routes::Route;
use crate::services::api;
use crate::state::{AppState, ToastKind};
use crate::utils::{format_datetime, matches_query};
use dioxus::prelude::*;

#[component]
pub fn TokensPage() -> Element {
    let state = use_context::<AppState>();
    let mut tokens = use_signal(Vec::<TokenView>::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| true);
    let mut query = use_signal(String::new);
    let mut show_revoked = use_signal(|| false);

    let mut show_create = use_signal(|| false);
    let mut name = use_signal(String::new);
    let mut created = use_signal(|| Option::<CreateTokenResponse>::None);
    let mut revoke_target = use_signal(|| Option::<TokenView>::None);

    let reload = use_callback(move |_: ()| {
        loading.set(true);
        spawn(async move {
            match api::list_tokens().await {
                Ok(list) => {
                    tokens.set(list);
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

    let filtered: Vec<TokenView> = tokens()
        .into_iter()
        .filter(|t| show_revoked() || !t.revoked)
        .filter(|t| matches_query(&t.name, &query()))
        .collect();

    rsx! {
        Breadcrumb { items: vec![
            ("Dashboard".to_string(), Some(Route::DashboardPage {})),
            ("API Tokens".to_string(), None),
        ]}

        div { class: "page-header",
            div {
                h1 { "API Tokens" }
                p { class: "desc", "Personal access tokens for programmatic API access" }
            }
            button {
                class: "btn btn-primary",
                r#type: "button",
                onclick: move |_| { created.set(None); show_create.set(true); },
                "+ Create token"
            }
        }

        div { class: "toolbar",
            SearchBox {
                value: query(),
                oninput: move |v| query.set(v),
                placeholder: "Search tokens…",
            }
            label { class: "checkbox-row", style: "margin:0;",
                input {
                    r#type: "checkbox",
                    checked: show_revoked(),
                    onchange: move |e| show_revoked.set(e.checked()),
                }
                span { "Show revoked" }
            }
            div { class: "spacer" }
            button { class: "btn btn-outline", r#type: "button", onclick: move |_| reload.call(()), "Refresh" }
        }

        if loading() {
            LoadingSpinner {}
        } else if let Some(err) = error() {
            ErrorState { message: err, on_retry: move |_| reload.call(()) }
        } else if filtered.is_empty() {
            EmptyState {
                title: "No tokens".to_string(),
                description: "Create a personal access token to use the API.",
                icon: "🔑",
            }
        } else {
            div { class: "table-wrap",
                table { class: "data-table",
                    thead {
                        tr {
                            th { "Name" }
                            th { "Status" }
                            th { "Last used" }
                            th { "Expires" }
                            th { "Created" }
                            th { style: "text-align:right;", "Actions" }
                        }
                    }
                    tbody {
                        for t in filtered {
                            {
                                let tok = t.clone();
                                rsx! {
                                    tr { key: "{t.id}",
                                        td { strong { "{t.name}" } }
                                        td {
                                            if t.revoked {
                                                StatusChip { status: "revoked" }
                                            } else {
                                                StatusChip { status: "active" }
                                            }
                                        }
                                        td {
                                            "{t.last_used_at.as_deref().map(format_datetime).unwrap_or_else(|| \"—\".to_string())}"
                                        }
                                        td {
                                            "{t.expires_at.as_deref().map(format_datetime).unwrap_or_else(|| \"never\".to_string())}"
                                        }
                                        td { "{format_datetime(&t.created_at)}" }
                                        td {
                                            div { class: "actions",
                                                if !t.revoked {
                                                    button {
                                                        class: "btn btn-sm btn-danger",
                                                        r#type: "button",
                                                        onclick: move |_| revoke_target.set(Some(tok.clone())),
                                                        "Revoke"
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
            }
        }

        Modal {
            title: if created().is_some() { "Token created".to_string() } else { "Create token".to_string() },
            open: show_create(),
            on_close: move |_| show_create.set(false),
            if let Some(c) = created() {
                div { class: "alert alert-warning",
                    "{c.warning.as_deref().unwrap_or(\"Store this token securely — it will not be shown again.\")}"
                }
                div { class: "secret-box",
                    code { "{c.raw_token}" }
                    button {
                        class: "btn btn-sm btn-outline",
                        r#type: "button",
                        onclick: move |_| {
                            let token = c.raw_token.clone();
                            spawn(async move {
                                if let Some(window) = web_sys::window() {
                                    let nav = window.navigator();
                                    let clipboard = nav.clipboard();
                                    let _ = wasm_bindgen_futures::JsFuture::from(
                                        clipboard.write_text(&token)
                                    ).await;
                                }
                                state.toast(ToastKind::Success, "Copied to clipboard");
                            });
                        },
                        "Copy"
                    }
                }
            } else {
                TextInput {
                    label: "Token name",
                    value: name(),
                    oninput: move |v| name.set(v),
                    placeholder: "ci-deploy",
                    hint: "A friendly label to identify this token.",
                }
            }
            div { class: "modal-footer", style: "margin-top:1rem; padding:0; border:none; background:transparent;",
                button {
                    class: "btn btn-outline", r#type: "button",
                    onclick: move |_| show_create.set(false),
                    if created().is_some() { "Done" } else { "Cancel" }
                }
                if created().is_none() {
                    button {
                        class: "btn btn-primary", r#type: "button",
                        onclick: move |_| {
                            let n = name();
                            spawn(async move {
                                match api::create_token(&n).await {
                                    Ok(c) => {
                                        created.set(Some(c));
                                        name.set(String::new());
                                        reload.call(());
                                    }
                                    Err(e) => state.toast(ToastKind::Error, e.to_string()),
                                }
                            });
                        },
                        "Create"
                    }
                }
            }
        }

        ConfirmDialog {
            title: "Revoke token".to_string(),
            message: format!(
                "Revoke token \"{}\"? Applications using it will lose access immediately.",
                revoke_target().as_ref().map(|t| t.name.as_str()).unwrap_or("")
            ),
            open: revoke_target().is_some(),
            confirm_label: "Revoke",
            danger: true,
            on_confirm: move |_| {
                if let Some(t) = revoke_target() {
                    spawn(async move {
                        match api::revoke_token(&t.id).await {
                            Ok(()) => {
                                state.toast(ToastKind::Success, "Token revoked");
                                revoke_target.set(None);
                                reload.call(());
                            }
                            Err(e) => state.toast(ToastKind::Error, e.to_string()),
                        }
                    });
                }
            },
            on_cancel: move |_| revoke_target.set(None),
        }
    }
}
