use crate::routes::Route;
use dioxus::prelude::*;

#[component]
pub fn NotFoundPage(route: Vec<String>) -> Element {
    let path = route.join("/");
    rsx! {
        div { class: "empty-state", style: "padding-top: 4rem;",
            div { class: "icon", "🔍" }
            h1 { "404 — Page not found" }
            p { "No page matches /{path}" }
            Link { class: "btn btn-primary", to: Route::DashboardPage {}, "Go to dashboard" }
        }
    }
}
