//! Table helpers: search toolbar + pagination.
pub mod datatable;
pub use datatable::{ColumnDef, DataTable};

use dioxus::prelude::*;

#[component]
pub fn SearchBox(
    value: String,
    oninput: EventHandler<String>,
    #[props(default = "Search…".to_string())] placeholder: String,
) -> Element {
    rsx! {
        input {
            class: "form-control search-input",
            r#type: "search",
            value: "{value}",
            placeholder: "{placeholder}",
            "aria-label": "Search",
            oninput: move |e| oninput.call(e.value()),
        }
    }
}

#[component]
pub fn Pagination(
    page: usize,
    page_size: usize,
    total: usize,
    on_page: EventHandler<usize>,
) -> Element {
    if total == 0 {
        return rsx! {};
    }
    let pages = total.div_ceil(page_size).max(1);
    let from = page * page_size + 1;
    let to = ((page + 1) * page_size).min(total);

    rsx! {
        div { class: "pagination",
            span { "Showing {from}–{to} of {total}" }
            div { class: "pages",
                button {
                    class: "btn btn-sm btn-outline",
                    r#type: "button",
                    disabled: page == 0,
                    onclick: move |_| on_page.call(page.saturating_sub(1)),
                    "Prev"
                }
                span { class: "text-muted", style: "padding: 0 0.5rem;",
                    "Page {page + 1} / {pages}"
                }
                button {
                    class: "btn btn-sm btn-outline",
                    r#type: "button",
                    disabled: page + 1 >= pages,
                    onclick: move |_| on_page.call(page + 1),
                    "Next"
                }
            }
        }
    }
}
