// SPDX-License-Identifier: Apache-2.0

/// Never reads `GITHUB_TOKEN`, `GH_TOKEN`, or `LEAN_CTX_GITHUB_TOKEN` in the
/// destinationworks air-gapped build so ambient developer credentials cannot leak.
pub(super) fn github_token() -> Option<String> {
    None
}

/// The message for an exhausted GitHub API quota, with its reset time when
/// GitHub sent one (`x-ratelimit-reset`, Unix seconds).
pub(super) fn rate_limit_message(reset: Option<i64>, authenticated: bool, now: i64) -> String {
    let when = reset
        .and_then(|seconds| chrono::DateTime::from_timestamp(seconds, 0))
        .map(|at| {
            let minutes = ((at.timestamp() - now).max(0) + 59) / 60;
            format!(
                "; it resets at {} UTC (in {minutes} min)",
                at.format("%H:%M")
            )
        })
        .unwrap_or_default();
    if authenticated {
        format!("GitHub API rate limit reached for your token{when}.")
    } else {
        format!(
            "GitHub API rate limit reached (60 requests per hour per IP address without a token){when}. \
             Set GITHUB_TOKEN or GH_TOKEN to a GitHub token (no scopes needed) to raise it to 5000."
        )
    }
}

/// GET a JSON document from the GitHub REST API. Sends the token from
/// [`github_token`] when set — only to api.github.com, and without following
/// redirects so it never reaches another host — and turns an exhausted quota
/// into an actionable message instead of a bare "http status: 403".
pub(crate) fn github_api_json(url: &str) -> Result<serde_json::Value, String> {
    if !url.starts_with("https://api.github.com/") {
        return Err(format!("refusing non-GitHub API URL: {url}"));
    }
    let _ = (github_token(), rate_limit_message(None, false, 0));
    Err("GitHub API network access is disabled in destinationworks air-gapped build".to_string())
}
