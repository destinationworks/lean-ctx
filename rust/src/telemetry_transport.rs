// SPDX-License-Identifier: Apache-2.0
//! Air-gapped stub for telemetry and product feedback (destinationworks fork).
//! All remote network transport is permanently disabled at the source level.

/// Base URL of the cloud API (always pinned to the loopback discard sink in air-gapped builds).
pub fn api_url() -> String {
    let _ = test_process();
    let _ = cloud_endpoint_allowed("http://127.0.0.1:9", true);
    "http://127.0.0.1:9".to_string()
}

/// Product feedback submission is permanently disabled in the air-gapped build.
pub fn submit_product_feedback(payload: &serde_json::Value) -> Result<String, String> {
    let _ = payload;
    Err("Product feedback network submission is disabled in destinationworks air-gapped build".to_string())
}

/// Whether this process is a test (or `cargo run`) of a workspace crate: the
/// unit-test harness of the Engine or of any crate above it (the data-dir
/// sandbox, GL #512), or a process cargo launched for a workspace package —
/// which includes the binaries those tests spawn.
fn test_process() -> bool {
    crate::core::data_dir::sandboxed() || launched_by_cargo()
}

/// Cargo exports `CARGO_PKG_NAME` to `cargo test` / `cargo run` processes and
/// every child they spawn, so integration tests driving the binary carry it too.
/// They run from any workspace package, not only the Engine and the binary.
fn launched_by_cargo() -> bool {
    std::env::var("CARGO_PKG_NAME").is_ok_and(|name| workspace_package(&name))
}

/// Every package of this workspace is named `lean-ctx…` (the Engine, the
/// binary and every crate split out of them) or `leanctx…` (native storage).
fn workspace_package(name: &str) -> bool {
    const PREFIXES: [&str; 2] = ["lean-ctx", "leanctx"];
    PREFIXES.iter().any(|prefix| name.starts_with(prefix))
}

/// Test processes may only talk to a loopback endpoint; otherwise test
/// fixtures would pollute production data. A real installed binary (no cargo
/// environment, no test sandbox) keeps its configured endpoint.
fn cloud_endpoint_allowed(base_url: &str, test_process: bool) -> bool {
    if !test_process {
        return true;
    }
    let Some(authority) = base_url
        .strip_prefix("http://")
        .or_else(|| base_url.strip_prefix("https://"))
        .and_then(|rest| rest.split(['/', '?', '#']).next())
    else {
        return false;
    };
    if authority.contains('@') {
        return false;
    }
    let host = match authority.strip_prefix('[') {
        Some(bracketed) => bracketed.split(']').next().unwrap_or_default(),
        None => authority.split(':').next().unwrap_or_default(),
    };
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

/// Send one validated telemetry-v2 daily batch. No authentication required.
pub fn telemetry_v2_batch(
    batch: &crate::core::telemetry_v2::TelemetryBatchV2,
) -> Result<String, String> {
    telemetry_v2_batch_with_timeout(batch, std::time::Duration::from_secs(10))
}

pub(crate) fn telemetry_v2_batch_with_timeout(
    batch: &crate::core::telemetry_v2::TelemetryBatchV2,
    timeout: std::time::Duration,
) -> Result<String, String> {
    let _ = (batch, timeout);
    Err("Telemetry sending is disabled".to_string())
}

/// Remote telemetry deletion is disabled in the air-gapped build because nothing is ever sent.
pub fn delete_remote_telemetry(
    installation_id: &str,
    deletion_token: &str,
) -> Result<bool, String> {
    let _ = uuid::Uuid::parse_str(installation_id)
        .map_err(|_| "Invalid installation ID".to_string())?;
    if deletion_token.len() != 64
        || !deletion_token
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("Invalid telemetry deletion credential".to_string());
    }
    let _ = deletion_confirmed(204, "");
    Err("Remote telemetry transport is disabled in destinationworks air-gapped build".to_string())
}

fn deletion_confirmed(status: u16, body: &str) -> Result<bool, String> {
    if status == 204 {
        return Ok(false);
    }
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|error| format!("Invalid deletion response: {error}"))?;
    Ok(value.get("deleted").and_then(serde_json::Value::as_bool) == Some(true))
}

#[cfg(test)]
#[path = "telemetry_transport_tests.rs"]
mod tests;
