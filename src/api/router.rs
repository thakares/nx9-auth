use axum::http::{HeaderName, Method, header};
use axum::{
    Router, middleware,
    routing::{delete, get, patch, post, put},
};
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};

use crate::{
    api::{
        applications, audit, auth, dashboard, groups, health, permissions, profile, roles,
        service_accounts, sessions, tenants, tokens, ui, users, version,
    },
    middleware::security_headers::security_headers,
    state::AppState,
};

/// Build the full Axum application router (API + Dioxus UI shell).
pub fn build(state: AppState) -> Router {
    let api_v1 = Router::new()
        // Auth — POST-only login (no GET credential endpoint exists).
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/me", get(auth::me))
        // Profile (self-service)
        .route(
            "/profile",
            get(profile::get_profile).patch(profile::update_profile),
        )
        .route("/profile/password", post(profile::change_password))
        // Dashboard
        .route("/dashboard", get(dashboard::dashboard))
        // Tenants
        .route(
            "/tenants",
            get(tenants::list_tenants).post(tenants::create_tenant),
        )
        .route(
            "/tenants/{id}",
            get(tenants::get_tenant)
                .patch(tenants::update_tenant)
                .delete(tenants::delete_tenant),
        )
        .route(
            "/tenants/{id}/users",
            get(tenants::list_tenant_users).post(tenants::assign_tenant_user),
        )
        .route(
            "/tenants/{id}/users/{user_id}",
            delete(tenants::remove_tenant_user),
        )
        .route(
            "/tenants/{id}/applications",
            get(tenants::list_tenant_applications),
        )
        // Users
        .route("/users", get(users::list_users).post(users::create_user))
        .route(
            "/users/{id}",
            get(users::get_user)
                .patch(users::update_user)
                .delete(users::delete_user),
        )
        .route("/users/{id}/reset-password", post(users::reset_password))
        .route(
            "/users/{id}/roles",
            get(users::list_user_roles).post(roles::assign_user_role),
        )
        .route("/users/{id}/roles/{role}", delete(roles::remove_user_role))
        .route(
            "/users/{id}/applications",
            get(users::list_user_applications),
        )
        // Roles
        .route("/roles", get(roles::list_roles).post(roles::create_role))
        .route(
            "/roles/{id}",
            get(roles::get_role)
                .patch(roles::update_role)
                .delete(roles::delete_role),
        )
        .route("/roles/{id}/permissions", put(roles::set_role_permissions))
        // Permissions
        .route("/permissions", get(permissions::list_permissions))
        // Tokens
        .route(
            "/tokens",
            get(tokens::list_tokens).post(tokens::create_token),
        )
        .route("/tokens/{id}", delete(tokens::revoke_token))
        // Applications
        .route(
            "/applications",
            get(applications::list_applications).post(applications::create_application),
        )
        .route(
            "/applications/{id}",
            get(applications::get_application)
                .patch(applications::update_application)
                .delete(applications::delete_application),
        )
        .route(
            "/applications/{id}/secret",
            post(applications::rotate_application_secret),
        )
        .route(
            "/applications/{id}/members",
            get(applications::list_application_members).post(applications::add_application_member),
        )
        .route(
            "/applications/{id}/members/{user_id}",
            patch(applications::update_application_member)
                .delete(applications::remove_application_member),
        )
        // Service accounts
        .route(
            "/service-accounts",
            get(service_accounts::list_service_accounts)
                .post(service_accounts::create_service_account),
        )
        .route(
            "/service-accounts/{id}",
            get(service_accounts::get_service_account)
                .patch(service_accounts::update_service_account)
                .delete(service_accounts::delete_service_account),
        )
        .route(
            "/service-accounts/{id}/secret",
            post(service_accounts::rotate_secret),
        )
        // Audit
        .route("/audit", get(audit::list_audit))
        .route("/audit/export", get(audit::export_audit))
        // Sessions
        .route("/sessions", get(sessions::list_sessions))
        .route("/sessions/others", delete(sessions::terminate_others))
        .route("/sessions/{id}", delete(sessions::terminate_session))
        // Groups
        .route(
            "/groups",
            get(groups::list_groups).post(groups::create_group),
        )
        .route(
            "/groups/{id}",
            get(groups::get_group)
                .patch(groups::update_group)
                .delete(groups::delete_group),
        )
        .route("/groups/{id}/members", post(groups::add_member))
        .route("/groups/{id}/members/{uid}", delete(groups::remove_member));

    Router::new()
        .route("/health", get(health::health))
        .route("/version", get(version::version))
        .nest("/api/v1", api_v1)
        // UI SPA — catch-all after API routes
        .fallback(ui::serve_ui)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            security_headers,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        // Mirror request Origin so credentialed SPA fetches work correctly.
        // Cannot use `*` for headers/methods when credentials are enabled.
        .layer(
            CorsLayer::new()
                .allow_origin(tower_http::cors::AllowOrigin::mirror_request())
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::PATCH,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers([
                    header::AUTHORIZATION,
                    header::CONTENT_TYPE,
                    header::ACCEPT,
                    header::COOKIE,
                    HeaderName::from_static("x-requested-with"),
                ])
                .allow_credentials(true),
        )
        .with_state(state)
}
