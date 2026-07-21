//! Global application state via Dioxus signals / context.

use crate::models::MeResponse;
use crate::theme::{self, ThemeMode};
use crate::routes::Route;
use crate::components::navigation::registry::{NavigationItem, NavigationRegistry};
use dioxus::prelude::*;

/// Toast notification.
#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    pub id: u64,
    pub kind: ToastKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ToastKind {
    Success,
    Error,
    Info,
    Warning,
}

impl ToastKind {
    pub fn css(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Error => "error",
            Self::Info => "info",
            Self::Warning => "warning",
        }
    }
}

/// Shared app state provided at the root.
#[derive(Clone, Copy)]
pub struct AppState {
    pub auth: Signal<BootstrapState>,
    pub theme: Signal<ThemeMode>,
    pub toasts: Signal<Vec<Toast>>,
    pub toast_seq: Signal<u64>,
    pub sidebar_collapsed: Signal<bool>,
    pub mobile_nav_open: Signal<bool>,
    pub nav_registry: Signal<NavigationRegistry>,
    pub tenant: Signal<Option<crate::models::TenantView>>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum BootstrapState {
    #[default]
    Initializing,
    Anonymous,
    Authenticated(MeResponse),
    Failed(String),
}

impl BootstrapState {
    pub fn is_authenticated(&self) -> bool {
        matches!(self, Self::Authenticated(_))
    }

    pub fn me(&self) -> Option<&MeResponse> {
        match self {
            Self::Authenticated(m) => Some(m),
            _ => None,
        }
    }

    pub fn has_permission(&self, perm: &str) -> bool {
        self.me()
            .map(|m| m.permissions.iter().any(|p| p == perm))
            .unwrap_or(false)
    }

    pub fn has_any_permission(&self, perms: &[&str]) -> bool {
        self.me()
            .map(|m| perms.iter().any(|p| m.permissions.iter().any(|x| x == p)))
            .unwrap_or(false)
    }

    pub fn is_adminish(&self) -> bool {
        self.has_any_permission(&[
            "roles:manage",
            "audit:view",
            "users:create",
            "users:update",
            "users:delete",
        ]) || self
            .me()
            .map(|m| m.roles.iter().any(|r| r == "admin"))
            .unwrap_or(false)
    }

    pub fn username(&self) -> &str {
        self.me().map(|m| m.user.username.as_str()).unwrap_or("")
    }
}

impl AppState {
    /// Create and provide app state. Must be called from a component body.
    pub fn provide() -> Self {
        let theme_mode = theme::load_theme();
        theme::apply_theme(theme_mode);

        let mut registry = NavigationRegistry::new();
        registry.register_section(
            "Workspace",
            vec![
                NavigationItem {
                    id: "dashboard".into(),
                    title: "Dashboard".into(),
                    icon: "◫".into(),
                    route: Route::DashboardPage {},
                    permission: None,
                    children: vec![],
                },
                NavigationItem {
                    id: "tenants".into(),
                    title: "Tenants".into(),
                    icon: "🏢".into(),
                    route: Route::TenantsPage {},
                    permission: None,
                    children: vec![],
                },
                NavigationItem {
                    id: "applications".into(),
                    title: "Applications".into(),
                    icon: "▦".into(),
                    route: Route::ApplicationsPage {},
                    permission: None,
                    children: vec![],
                },
            ]
        );
        registry.register_section(
            "Security",
            vec![
                NavigationItem {
                    id: "users".into(),
                    title: "Users".into(),
                    icon: "👥".into(),
                    route: Route::UsersPage {},
                    permission: Some("users:read".into()),
                    children: vec![],
                },
                NavigationItem {
                    id: "groups".into(),
                    title: "Groups".into(),
                    icon: "👪".into(),
                    route: Route::GroupsPage {},
                    permission: Some("groups:read".into()),
                    children: vec![],
                },
                NavigationItem {
                    id: "roles".into(),
                    title: "Roles".into(),
                    icon: "🛡".into(),
                    route: Route::RolesPage {},
                    permission: Some("roles:manage".into()),
                    children: vec![],
                },
                NavigationItem {
                    id: "permissions".into(),
                    title: "Permissions".into(),
                    icon: "✓".into(),
                    route: Route::PermissionsPage {},
                    permission: Some("roles:manage".into()),
                    children: vec![],
                },
                NavigationItem {
                    id: "service_accounts".into(),
                    title: "Service Accounts".into(),
                    icon: "⚙".into(),
                    route: Route::ServiceAccountsPage {},
                    permission: Some("roles:manage".into()),
                    children: vec![],
                },
            ]
        );
        registry.register_section(
            "Audit & Logs",
            vec![
                NavigationItem {
                    id: "sessions".into(),
                    title: "Sessions".into(),
                    icon: "⏱".into(),
                    route: Route::SessionsPage {},
                    permission: Some("audit:view".into()),
                    children: vec![],
                },
                NavigationItem {
                    id: "audit_events".into(),
                    title: "Audit Log".into(),
                    icon: "📋".into(),
                    route: Route::AuditPage {},
                    permission: Some("audit:view".into()),
                    children: vec![],
                },
            ]
        );
        registry.register_section(
            "System",
            vec![
                NavigationItem {
                    id: "settings".into(),
                    title: "Settings".into(),
                    icon: "⚙".into(),
                    route: Route::SettingsPage {},
                    permission: None,
                    children: vec![],
                },
                NavigationItem {
                    id: "about".into(),
                    title: "About".into(),
                    icon: "ℹ".into(),
                    route: Route::AboutPage {},
                    permission: None,
                    children: vec![],
                },
            ]
        );

        let state = Self {
            auth: use_signal(|| BootstrapState::Initializing),
            theme: use_signal(|| theme_mode),
            toasts: use_signal(Vec::new),
            toast_seq: use_signal(|| 0u64),
            sidebar_collapsed: use_signal(|| false),
            mobile_nav_open: use_signal(|| false),
            nav_registry: use_signal(|| registry),
            tenant: use_signal(|| None),
        };
        use_context_provider(|| state);
        state
    }

    pub fn toast(&self, kind: ToastKind, message: impl Into<String>) {
        let id = {
            let mut seq = self.toast_seq;
            let next = seq() + 1;
            seq.set(next);
            next
        };
        let mut toasts = self.toasts;
        let mut list = toasts();
        list.push(Toast {
            id,
            kind,
            message: message.into(),
        });
        if list.len() > 5 {
            list.remove(0);
        }
        toasts.set(list);
    }

    pub fn dismiss_toast(&self, id: u64) {
        let mut toasts = self.toasts;
        let list: Vec<_> = toasts().into_iter().filter(|t| t.id != id).collect();
        toasts.set(list);
    }

    pub fn set_theme(&self, mode: ThemeMode) {
        theme::save_theme(mode);
        theme::apply_theme(mode);
        let mut t = self.theme;
        t.set(mode);
    }

    pub fn cycle_theme(&self) {
        let next = (self.theme)().cycle();
        self.set_theme(next);
    }
}

