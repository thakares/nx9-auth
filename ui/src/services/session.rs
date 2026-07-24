//! Client-side access-token storage (SPA auth).
//!
//! Login uses **POST JSON only** — never query parameters.
//! The server returns an opaque `access_token` (and sets an HttpOnly cookie).
//! We store the access token in `sessionStorage` and send
//! `Authorization: Bearer …` on subsequent requests.
//!
//! Passwords are never stored on the client.

use gloo_storage::{SessionStorage, Storage};

const ACCESS_KEY: &str = "nx9_access_token";
const REFRESH_KEY: &str = "nx9_refresh_token";

pub fn save_access_token(token: &str) {
    let _ = SessionStorage::set(ACCESS_KEY, token);
}

pub fn save_refresh_token(token: &str) {
    let _ = SessionStorage::set(REFRESH_KEY, token);
}

pub fn load_access_token() -> Option<String> {
    SessionStorage::get::<String>(ACCESS_KEY).ok()
}

#[allow(dead_code)]
pub fn load_refresh_token() -> Option<String> {
    SessionStorage::get::<String>(REFRESH_KEY).ok()
}

pub fn clear() {
    SessionStorage::delete(ACCESS_KEY);
    SessionStorage::delete(REFRESH_KEY);
}
