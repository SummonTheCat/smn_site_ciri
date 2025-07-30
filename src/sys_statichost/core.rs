//! Pure path‑mapping logic: any file under `static/`, plus `.html` fallback.

use std::path::PathBuf;

/// Given a request path, return the corresponding filesystem path under `static/`,
/// or `None` if no matching file exists.
pub fn map_static_path(uri: &str) -> Option<PathBuf> {
    // Normalize: strip leading slash
    let rel = uri.strip_prefix('/').unwrap_or(uri);

    // 1) Root → index.html
    if rel.is_empty() {
        return Some(PathBuf::from("static").join("index.html"));
    }

    // 2) Try exact file under static/
    let candidate = PathBuf::from("static").join(rel);
    if candidate.exists() {
        return Some(candidate);
    }

    // 3) Try with “.html” appended
    let html_candidate = PathBuf::from("static").join(format!("{}.html", rel));
    if html_candidate.exists() {
        return Some(html_candidate);
    }

    None
}
