//! Form controls.

use dioxus::prelude::*;

#[component]
pub fn TextInput(
    #[props(default)] label: String,
    #[props(default)] name: String,
    value: String,
    oninput: EventHandler<String>,
    #[props(default)] placeholder: String,
    #[props(default = "text".to_string())] input_type: String,
    #[props(default)] required: bool,
    #[props(default)] disabled: bool,
    #[props(default)] error: String,
    #[props(default)] hint: String,
    #[props(default)] autocomplete: String,
) -> Element {
    let invalid = if error.is_empty() { "" } else { "is-invalid" };
    let id = if name.is_empty() {
        "field".to_string()
    } else {
        name.clone()
    };
    let type_attr = input_type.clone();

    rsx! {
        div { class: "form-group",
            if !label.is_empty() {
                label { class: "form-label", r#for: "{id}", "{label}" }
            }
            input {
                class: "form-control {invalid}",
                id: "{id}",
                name: "{name}",
                r#type: "{type_attr}",
                value: "{value}",
                placeholder: "{placeholder}",
                required: required,
                disabled: disabled,
                autocomplete: "{autocomplete}",
                oninput: move |e| oninput.call(e.value()),
            }
            if !error.is_empty() {
                div { class: "form-error", role: "alert", "{error}" }
            } else if !hint.is_empty() {
                div { class: "form-hint", "{hint}" }
            }
        }
    }
}

#[component]
pub fn PasswordInput(
    #[props(default)] label: String,
    #[props(default)] name: String,
    value: String,
    oninput: EventHandler<String>,
    #[props(default)] placeholder: String,
    #[props(default)] required: bool,
    #[props(default)] error: String,
    #[props(default = "current-password".to_string())] autocomplete: String,
) -> Element {
    let mut visible = use_signal(|| false);
    let id = if name.is_empty() {
        "password".to_string()
    } else {
        name.clone()
    };
    let input_type = if visible() { "text" } else { "password" };
    let invalid = if error.is_empty() { "" } else { "is-invalid" };
    let toggle_label = if visible() {
        "Hide password"
    } else {
        "Show password"
    };
    let toggle_icon = if visible() { "🙈" } else { "👁" };

    rsx! {
        div { class: "form-group",
            if !label.is_empty() {
                label { class: "form-label", r#for: "{id}", "{label}" }
            }
            div { class: "password-field",
                input {
                    class: "form-control {invalid}",
                    id: "{id}",
                    name: "{name}",
                    r#type: "{input_type}",
                    value: "{value}",
                    placeholder: "{placeholder}",
                    required: required,
                    autocomplete: "{autocomplete}",
                    oninput: move |e| oninput.call(e.value()),
                }
                button {
                    class: "toggle",
                    r#type: "button",
                    "aria-label": "{toggle_label}",
                    title: "{toggle_label}",
                    onclick: move |_| visible.set(!visible()),
                    "{toggle_icon}"
                }
            }
            if !error.is_empty() {
                div { class: "form-error", role: "alert", "{error}" }
            }
        }
    }
}

#[component]
pub fn SelectInput(
    #[props(default)] label: String,
    value: String,
    options: Vec<(String, String)>,
    onchange: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "form-group",
            if !label.is_empty() {
                label { class: "form-label", "{label}" }
            }
            select {
                class: "form-control",
                value: "{value}",
                onchange: move |e| onchange.call(e.value()),
                for (val, lab) in options {
                    option { value: "{val}", selected: value == val, "{lab}" }
                }
            }
        }
    }
}

#[component]
pub fn Checkbox(label: String, checked: bool, onchange: EventHandler<bool>) -> Element {
    rsx! {
        label { class: "checkbox-row",
            input {
                r#type: "checkbox",
                checked: checked,
                onchange: move |e| onchange.call(e.checked()),
            }
            span { "{label}" }
        }
    }
}

#[component]
pub fn Toggle(
    checked: bool,
    onchange: EventHandler<bool>,
    #[props(default)] label: String,
) -> Element {
    let class = if checked { "toggle on" } else { "toggle" };
    rsx! {
        label { class: "row gap-1", style: "cursor:pointer;",
            button {
                class: "{class}",
                r#type: "button",
                role: "switch",
                "aria-checked": "{checked}",
                onclick: move |_| onchange.call(!checked),
            }
            if !label.is_empty() {
                span { "{label}" }
            }
        }
    }
}
