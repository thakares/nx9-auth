//! Static UI asset serving for the Dioxus frontend.
//!
//! Assets are served from `ui/dist` when present (development or prebuilt).
//! SPA routes fall back to `index.html` so client-side routing works.
//! Static extensions (`.js`, `.wasm`, …) never fall back to HTML — that would
//! break ES module loading with a silent blank page.

use axum::{
    body::Body,
    http::{StatusCode, Uri, header},
    response::{Html, IntoResponse, Response},
};
use std::path::{Path, PathBuf};

/// Resolve the UI dist directory (workspace-relative or beside the binary).
pub fn ui_dist_dir() -> PathBuf {
    if let Ok(p) = std::env::var("NX9_AUTH_UI_DIST") {
        return PathBuf::from(p);
    }
    let candidates = [
        PathBuf::from("ui/dist"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui/dist"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for rel in ["ui/dist", "../ui/dist", "../../ui/dist"] {
                let candidate = dir.join(rel);
                if candidate.exists() {
                    return candidate;
                }
            }
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui/dist")
}

/// Extensions that must be real files — never SPA-fallback to index.html.
fn is_static_asset(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    [
        ".js", ".mjs", ".css", ".wasm", ".map", ".json", ".svg", ".png", ".jpg", ".jpeg", ".ico",
        ".woff", ".woff2", ".ttf", ".webp", ".gif",
    ]
    .iter()
    .any(|ext| lower.ends_with(ext))
}

/// Serve a static file from the UI dist dir, or SPA fallback for app routes.
pub async fn serve_ui(uri: Uri) -> Response {
    let dist = ui_dist_dir();
    if !dist.exists() {
        return missing_ui_page().into_response();
    }

    let path = uri.path().trim_start_matches('/');
    if path.starts_with("api/") || path == "health" || path == "version" {
        return StatusCode::NOT_FOUND.into_response();
    }

    // Security Hardening: Reject & sanitize any GET request containing credentials in query string.
    if let Some(query) = uri.query() {
        let q_lower = query.to_ascii_lowercase();
        if q_lower.contains("password=")
            || q_lower.contains("username=")
            || q_lower.contains("secret=")
        {
            tracing::warn!(path = %uri.path(), "rejected credential query parameters in GET request");
            let clean_path = if uri.path().is_empty() {
                "/"
            } else {
                uri.path()
            };
            return Response::builder()
                .status(StatusCode::SEE_OTHER)
                .header(header::LOCATION, clean_path)
                .header(header::CACHE_CONTROL, "no-store")
                .body(Body::empty())
                .unwrap_or_else(|_| StatusCode::BAD_REQUEST.into_response());
        }
    }

    // Normalize and reject path traversal
    if path.contains("..") {
        return StatusCode::BAD_REQUEST.into_response();
    }

    // Browsers always probe /favicon.ico even when <link rel="icon"> is set.
    let req_path = if path.is_empty() {
        "index.html".to_string()
    } else if path == "favicon.ico" {
        "assets/favicon.svg".to_string()
    } else {
        path.to_string()
    };
    let file_path = dist.join(&req_path);

    // Canonicalize within dist when possible
    if file_path.is_file() {
        return serve_file(&file_path).await;
    }

    // Missing static assets → 404 (never HTML — breaks `import` graphs)
    if is_static_asset(&req_path) {
        return StatusCode::NOT_FOUND.into_response();
    }

    // SPA fallback for client routes (/login, /dashboard, …)
    let index = dist.join("index.html");
    if index.is_file() {
        return serve_file(&index).await;
    }

    missing_ui_page().into_response()
}

async fn serve_file(path: &Path) -> Response {
    match tokio::fs::read(path).await {
        Ok(bytes) => {
            let mime = mime_guess(path);
            // HTML/JS must revalidate so rebuilds show up; wasm can be short-cached.
            let cache = match path.extension().and_then(|e| e.to_str()) {
                Some("html") => "no-cache",
                Some("js") | Some("mjs") | Some("css") => "no-cache",
                Some("wasm") => "public, max-age=3600",
                _ => "public, max-age=3600",
            };
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .header(header::CACHE_CONTROL, cache)
                // Required for ES modules / wasm cross-origin isolation edge cases
                .header(
                    header::HeaderName::from_static("cross-origin-resource-policy"),
                    "same-origin",
                )
                .body(Body::from(bytes))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

fn mime_guess(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") | Some("mjs") => "application/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("json") | Some("map") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        _ => "application/octet-stream",
    }
}

fn missing_ui_page() -> Html<&'static str> {
    Html(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <title>nx9-auth</title>
  <style>
    :root { color-scheme: light dark; font-family: ui-sans-serif, system-ui, sans-serif; }
    body { margin: 0; min-height: 100vh; display: grid; place-items: center;
           background: #0b1220; color: #e8eefc; }
    .card { max-width: 36rem; padding: 2rem; border-radius: 1rem;
            background: rgba(255,255,255,0.04); border: 1px solid rgba(255,255,255,0.08); }
    h1 { margin: 0 0 0.5rem; font-size: 1.5rem; }
    p { line-height: 1.55; color: #b6c2dc; }
    code { background: rgba(255,255,255,0.08); padding: 0.15rem 0.4rem; border-radius: 0.35rem; }
    a { color: #7db4ff; }
  </style>
</head>
<body>
  <div class="card">
    <h1>nx9-auth API is running</h1>
    <p>
      The Dioxus UI assets are not present. Build them and restart:
    </p>
    <p><code>./scripts/build-ui.sh</code></p>
    <p>
      Or set <code>NX9_AUTH_UI_DIST</code> to the directory containing
      <code>index.html</code> and <code>nx9_auth_ui.js</code>.
    </p>
    <p>
      API health: <a href="/health">/health</a> · Version: <a href="/version">/version</a>
    </p>
  </div>
</body>
</html>"#,
    )
}
