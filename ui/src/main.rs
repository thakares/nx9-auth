//! nx9-auth Dioxus UI entrypoint.

mod app;
mod components;
mod models;
mod pages;
mod routes;
mod services;
mod state;
mod theme;
mod utils;

fn main() {
    // Surface panics in the browser console instead of a silent blank page.
    console_error_panic_hook::set_once();
    
    // Clear any pre-rendered loading banner in index.html (boot.js) 
    // before Dioxus takes over `#main` and appends its root elements.
    if let Some(window) = web_sys::window() {
        if let Some(doc) = window.document() {
            if let Some(el) = doc.get_element_by_id("main") {
                el.set_inner_html("");
            }
        }
    }

    dioxus::launch(app::App);
}
