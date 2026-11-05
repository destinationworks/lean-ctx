//! Air-gapped stub for outbound HTTP fetch (destinationworks fork).

use super::url_guard::{self, SafeUrl};

/// Default response body cap (4 MiB) — generous for articles, safe for memory.
pub const DEFAULT_MAX_BYTES: usize = 4 * 1024 * 1024;
/// Default total request timeout in seconds.
pub const DEFAULT_TIMEOUT_SECS: u64 = 20;

/// A fetched document with its raw body bytes and resolved metadata.
pub struct FetchedDoc {
    pub final_url: String,
    /// Lower-cased MIME type without parameters (e.g. `text/html`).
    pub content_type: String,
    pub bytes: Vec<u8>,
    pub status: u16,
    pub truncated: bool,
}

impl FetchedDoc {
    /// Lossy UTF-8 view of the body, for textual content (HTML, JSON, …).
    pub fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }
}

/// Outbound HTTP fetch is permanently disabled in the destinationworks air-gapped build.
pub fn fetch(url: &str, max_bytes: usize, timeout_secs: u64) -> Result<FetchedDoc, String> {
    let _ = (url_guard::validate(url), max_bytes, timeout_secs);
    Err("Outbound HTTP fetch is disabled in destinationworks air-gapped build".to_string())
}

/// Outbound HTTP POST is permanently disabled in the destinationworks air-gapped build.
pub fn post(
    url: &str,
    content_type: &str,
    user_agent: &str,
    body: &str,
    max_bytes: usize,
    timeout_secs: u64,
) -> Result<FetchedDoc, String> {
    let _ = (
        url_guard::validate(url),
        content_type,
        user_agent,
        body,
        max_bytes,
        timeout_secs,
    );
    Err("Outbound HTTP POST is disabled in destinationworks air-gapped build".to_string())
}

/// Resolve a (possibly relative) `location` (redirect target or link href)
/// against a base URL.
pub(crate) fn resolve_redirect(base: &SafeUrl, location: &str) -> String {
    let loc = location.trim();

    if loc.starts_with("http://") || loc.starts_with("https://") {
        return loc.to_string();
    }
    if let Some(rest) = loc.strip_prefix("//") {
        return format!("{}://{rest}", base.scheme);
    }
    if loc.starts_with('/') {
        return format!("{}://{}{loc}", base.scheme, base.authority);
    }

    // Path-relative: join against the directory of the current path.
    let base_path = base_path(base);
    let dir = match base_path.rfind('/') {
        Some(i) => &base_path[..=i],
        None => "/",
    };
    format!("{}://{}{dir}{loc}", base.scheme, base.authority)
}

fn base_path(base: &SafeUrl) -> &str {
    let prefix_len = base.scheme.len() + 3 + base.authority.len();
    let path = base.normalized.get(prefix_len..).unwrap_or("");
    let path = path.split(['?', '#']).next().unwrap_or("");
    if path.is_empty() { "/" } else { path }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn safe(url: &str) -> SafeUrl {
        url_guard::validate(url).unwrap()
    }

    #[test]
    fn redirect_absolute_is_passthrough() {
        let base = safe("https://a.com/x");
        assert_eq!(
            resolve_redirect(&base, "https://b.com/y"),
            "https://b.com/y"
        );
    }

    #[test]
    fn redirect_scheme_relative() {
        let base = safe("https://a.com/x");
        assert_eq!(resolve_redirect(&base, "//c.com/z"), "https://c.com/z");
    }

    #[test]
    fn redirect_root_relative() {
        let base = safe("https://a.com/deep/path?q=1");
        assert_eq!(resolve_redirect(&base, "/new"), "https://a.com/new");
    }

    #[test]
    fn redirect_path_relative_joins_dir() {
        let base = safe("https://a.com/dir/page.html");
        assert_eq!(
            resolve_redirect(&base, "other.html"),
            "https://a.com/dir/other.html"
        );
    }

    #[test]
    fn redirect_path_relative_from_root() {
        let base = safe("https://a.com");
        assert_eq!(resolve_redirect(&base, "page"), "https://a.com/page");
    }
}
