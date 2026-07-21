//! Theme management (light / dark / system) with localStorage persistence.

use gloo_storage::{LocalStorage, Storage};
use std::fmt;

const STORAGE_KEY: &str = "nx9-auth-theme";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    Light,
    Dark,
    #[default]
    System,
}

impl ThemeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::System => "system",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::System,
            Self::System => Self::Light,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::System => "System",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Light => "☀",
            Self::Dark => "☾",
            Self::System => "◐",
        }
    }
}

impl fmt::Display for ThemeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub fn load_theme() -> ThemeMode {
    LocalStorage::get::<String>(STORAGE_KEY)
        .map(|s| ThemeMode::from_str(&s))
        .unwrap_or_default()
}

pub fn save_theme(mode: ThemeMode) {
    let _ = LocalStorage::set(STORAGE_KEY, mode.as_str());
}

/// Apply theme to `<html data-theme="...">`.
pub fn apply_theme(mode: ThemeMode) {
    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        if let Some(el) = document.document_element() {
            match mode {
                ThemeMode::Light => {
                    let _ = el.set_attribute("data-theme", "light");
                }
                ThemeMode::Dark => {
                    let _ = el.set_attribute("data-theme", "dark");
                }
                ThemeMode::System => {
                    let _ = el.remove_attribute("data-theme");
                }
            }
        }
    }
}
