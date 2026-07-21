//! About page.

use crate::components::navigation::Breadcrumb;
use crate::routes::Route;
use dioxus::prelude::*;

#[component]
pub fn AboutPage() -> Element {
    rsx! {
        Breadcrumb { items: vec![
            ("Dashboard".to_string(), Some(Route::DashboardPage {})),
            ("About".to_string(), None),
        ]}

        div { class: "page-header",
            div {
                h1 { "About nx9-auth" }
                p { class: "desc", "Lightweight self-hosted IAM for the NX9 ecosystem" }
            }
        }

        div { class: "card",
            div { class: "card-body stack",
                p {
                    strong { "nx9-auth" }
                    " provides authentication, authorization, RBAC, sessions, API tokens, "
                    "service accounts, applications, and audit logging in a single Rust binary."
                }
                div { class: "row", style: "flex-wrap:wrap;gap:0.5rem;",
                    span { class: "badge badge-accent", "Pure Rust" }
                    span { class: "badge badge-info", "Dioxus UI" }
                    span { class: "badge badge-success", "Self-hosted" }
                    span { class: "badge", "FOSS" }
                    span { class: "badge", "No Node.js" }
                }
                h3 { class: "mt-2", "Philosophy" }
                ul {
                    li { "Single binary deployment" }
                    li { "Zero JavaScript frameworks" }
                    li { "Backend remains the authority for security decisions" }
                    li { "Extensible shell for future NX9 services" }
                }
                h3 { "Future integrations" }
                div { class: "row", style: "flex-wrap:wrap;gap:0.4rem;",
                    for name in ["nx9-docflow", "nx9-dns", "nx9-shortener", "nx9-storage", "nx9-monitor", "nx9-mail"] {
                        span { class: "badge", "{name}" }
                    }
                }
                p { class: "text-muted mt-2", style: "font-size:12px;",
                    "License: Apache-2.0 OR MIT"
                }
            }
        }
    }
}
