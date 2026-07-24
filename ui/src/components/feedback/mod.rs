//! Feedback components: loading, empty, error, modal, confirmation.

use dioxus::prelude::*;

#[component]
pub fn LoadingSpinner(#[props(default)] label: String) -> Element {
    let text = if label.is_empty() {
        "Loading…".to_string()
    } else {
        label
    };
    rsx! {
        div { class: "loading-center", role: "status", "aria-live": "polite",
            div { class: "spinner spinner-lg" }
            span { "{text}" }
        }
    }
}

#[component]
pub fn SkeletonLoader(#[props(default = 4u32)] rows: u32) -> Element {
    rsx! {
        div { class: "stack", "aria-hidden": "true",
            for i in 0..rows {
                div {
                    class: "skeleton",
                    key: "{i}",
                    style: "height: 1.25rem; width: {60 + (i * 7) % 40}%;",
                }
            }
        }
    }
}

#[component]
pub fn EmptyState(
    title: String,
    #[props(default)] description: String,
    #[props(default = "📭".to_string())] icon: String,
) -> Element {
    rsx! {
        div { class: "empty-state",
            div { class: "icon", "{icon}" }
            h3 { "{title}" }
            if !description.is_empty() {
                p { "{description}" }
            }
        }
    }
}

#[component]
pub fn ErrorState(message: String, on_retry: EventHandler<()>) -> Element {
    rsx! {
        div { class: "error-state",
            div { class: "icon", "⚠" }
            h3 { "Something went wrong" }
            p { "{message}" }
            button {
                class: "btn btn-outline mt-1",
                r#type: "button",
                onclick: move |_| on_retry.call(()),
                "Retry"
            }
        }
    }
}

#[component]
pub fn Modal(
    title: String,
    open: bool,
    on_close: EventHandler<()>,
    #[props(default)] large: bool,
    children: Element,
) -> Element {
    if !open {
        return rsx! {};
    }
    let size = if large { "modal modal-lg" } else { "modal" };
    rsx! {
        div {
            class: "modal-backdrop",
            role: "dialog",
            "aria-modal": "true",
            "aria-label": "{title}",
            onclick: move |_| on_close.call(()),
            div {
                class: "{size}",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    h2 { "{title}" }
                    button {
                        class: "btn btn-ghost btn-icon",
                        r#type: "button",
                        "aria-label": "Close",
                        onclick: move |_| on_close.call(()),
                        "×"
                    }
                }
                div { class: "modal-body",
                    {children}
                }
            }
        }
    }
}

#[component]
pub fn ConfirmDialog(
    title: String,
    message: String,
    open: bool,
    #[props(default = "Confirm".to_string())] confirm_label: String,
    #[props(default)] danger: bool,
    on_confirm: EventHandler<()>,
    on_cancel: EventHandler<()>,
) -> Element {
    let confirm_cls = if danger {
        "btn btn-danger"
    } else {
        "btn btn-primary"
    };

    rsx! {
        Modal {
            title: title,
            open: open,
            on_close: move |_| on_cancel.call(()),
            p { "{message}" }
            div { class: "modal-footer", style: "padding: 0; border: none; margin-top: 1rem;",
                button {
                    class: "btn btn-outline",
                    r#type: "button",
                    onclick: move |_| on_cancel.call(()),
                    "Cancel"
                }
                button {
                    class: "{confirm_cls}",
                    r#type: "button",
                    onclick: move |_| on_confirm.call(()),
                    "{confirm_label}"
                }
            }
        }
    }
}

#[component]
pub fn Alert(kind: String, message: String) -> Element {
    rsx! {
        div { class: "alert alert-{kind}", role: "alert", "{message}" }
    }
}
