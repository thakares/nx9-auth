//! Application routes (permission-aware navigation is handled in the sidebar).

use crate::components::layout::AppLayout;
use crate::pages::{
    about::AboutPage,
    applications::{ApplicationDetailPage, ApplicationsPage},
    audit::AuditPage,
    auth::{ForbiddenPage, LoginPage, UnauthorizedPage},
    dashboard::DashboardPage,
    groups::{GroupsPage, GroupDetailPage},
    not_found::NotFoundPage,
    permissions::PermissionsPage,
    profile::ProfilePage,
    roles::RolesPage,
    service_accounts::ServiceAccountsPage,
    sessions::SessionsPage,
    settings::SettingsPage,
    tenants::{TenantsPage, TenantDetailPage},
    tokens::TokensPage,
    users::{UserDetailPage, UsersPage},
};
use dioxus::prelude::*;

#[derive(Clone, Routable, Debug, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[route("/login")]
    LoginPage {},

    #[route("/unauthorized")]
    UnauthorizedPage {},

    #[route("/forbidden")]
    ForbiddenPage {},

    // Root: public landing that sends users to login or dashboard.
    #[route("/")]
    HomeRedirect {},

    #[layout(AppLayout)]
        #[route("/dashboard")]
        DashboardPage {},

        #[route("/profile")]
        ProfilePage {},

        #[route("/tenants")]
        TenantsPage {},

        #[route("/tenants/:id")]
        TenantDetailPage { id: String },

        #[route("/users")]
        UsersPage {},

        #[route("/users/:id")]
        UserDetailPage { id: String },

        #[route("/groups")]
        GroupsPage {},

        #[route("/groups/:id")]
        GroupDetailPage { id: String },

        #[route("/roles")]
        RolesPage {},

        #[route("/permissions")]
        PermissionsPage {},

        #[route("/tokens")]
        TokensPage {},

        #[route("/sessions")]
        SessionsPage {},

        #[route("/applications")]
        ApplicationsPage {},

        #[route("/applications/:id")]
        ApplicationDetailPage { id: String },

        #[route("/service-accounts")]
        ServiceAccountsPage {},

        #[route("/audit")]
        AuditPage {},

        #[route("/settings")]
        SettingsPage {},

        #[route("/about")]
        AboutPage {},
    #[end_layout]

    #[route("/:..route")]
    NotFoundPage { route: Vec<String> },
}

/// `/` → login when signed out, dashboard when signed in.
#[component]
fn HomeRedirect() -> Element {
    let state = use_context::<crate::state::AppState>();
    let auth = state.auth;
    let nav = use_navigator();

    use_effect(move || match auth() {
        crate::state::BootstrapState::Authenticated(_) => {
            nav.replace(Route::DashboardPage {});
        }
        crate::state::BootstrapState::Anonymous => {
            nav.replace(Route::LoginPage {});
        }
        crate::state::BootstrapState::Initializing | crate::state::BootstrapState::Failed(_) => {}
    });

    rsx! {
        div { class: "loading-center", style: "min-height: 100vh;",
            div { class: "spinner spinner-lg" }
            span {
                match auth() {
                    crate::state::BootstrapState::Authenticated(_) => "Opening dashboard…",
                    crate::state::BootstrapState::Anonymous => "Opening sign-in…",
                    _ => "Starting nx9-auth…",
                }
            }
        }
    }
}
