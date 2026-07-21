use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct ColumnDef {
    pub key: String,
    pub label: String,
    pub sortable: bool,
    pub visible: bool,
}

#[component]
pub fn DataTable(
    columns: Vec<ColumnDef>,
    on_search: EventHandler<String>,
    search_value: String,
    search_placeholder: String,
    
    on_sort: EventHandler<String>,
    sort_key: String,
    
    on_page: EventHandler<usize>,
    page: usize,
    page_size: usize,
    total: usize,

    // Row Actions or other toolbar slots
    #[props(default)] toolbar_actions: Option<Element>,
    
    // The actual table body and header will be rendered internally
    // We expect the caller to just give us the table rows
    children: Element,
) -> Element {
    let mut show_columns = use_signal(|| false);

    rsx! {
        div { class: "data-table-container",
            div { class: "toolbar",
                crate::components::tables::SearchBox {
                    value: search_value,
                    oninput: move |v| on_search.call(v),
                    placeholder: "{search_placeholder}",
                }
                
                div { class: "spacer" }
                
                {toolbar_actions}
                
                // Column Visibility
                div { class: "dropdown",
                    button {
                        class: "btn btn-outline",
                        r#type: "button",
                        onclick: move |_| show_columns.set(!show_columns()),
                        "Columns ▾"
                    }
                    if show_columns() {
                        div { class: "dropdown-menu", style: "right: 0; left: auto;",
                            for col in columns.iter() {
                                label { class: "dropdown-item checkbox-row",
                                    input {
                                        r#type: "checkbox",
                                        checked: col.visible,
                                        // TODO: emit event
                                    }
                                    span { "{col.label}" }
                                }
                            }
                        }
                    }
                }
                
                // CSV Export (future ready)
                button {
                    class: "btn btn-outline",
                    r#type: "button",
                    title: "Export to CSV (Coming Soon)",
                    "⬇ Export"
                }
            }
            
            div { class: "table-wrap",
                table { class: "data-table",
                    thead {
                        tr {
                            for col in columns.iter().filter(|c| c.visible) {
                                th {
                                    if col.sortable {
                                        button {
                                            class: "btn-ghost",
                                            style: "padding: 0; font-weight: inherit; font-size: inherit;",
                                            onclick: {
                                                let k = col.key.clone();
                                                move |_| on_sort.call(k.clone())
                                            },
                                            "{col.label}"
                                            if sort_key == col.key { " ↓" }
                                        }
                                    } else {
                                        "{col.label}"
                                    }
                                }
                            }
                        }
                    }
                    tbody {
                        {children}
                    }
                }
            }
            
            crate::components::tables::Pagination {
                page: page,
                page_size: page_size,
                total: total,
                on_page: move |p| on_page.call(p),
            }
        }
    }
}
