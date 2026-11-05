// SPDX-License-Identifier: Apache-2.0

use super::*;

#[test]
fn test_processes_never_reach_a_remote_cloud_endpoint() {
    for remote in [
        "https://api.leanctx.com",
        "http://10.0.0.1:8080",
        "http://localhost.evil.com",
        "http://127.0.0.1@api.leanctx.com",
        "ftp://127.0.0.1",
        "127.0.0.1:9",
    ] {
        assert!(!cloud_endpoint_allowed(remote, true), "{remote}");
    }
    for loopback in [
        "http://127.0.0.1:9",
        "http://127.3.2.1",
        "http://localhost:8088/prefix",
        "https://LOCALHOST",
        "http://[::1]:9",
    ] {
        assert!(cloud_endpoint_allowed(loopback, true), "{loopback}");
    }
}

#[test]
fn an_installed_binary_keeps_its_configured_endpoint() {
    for configured in ["https://api.leanctx.com", "http://10.0.0.1:8080"] {
        assert!(cloud_endpoint_allowed(configured, false), "{configured}");
    }
}

#[test]
fn this_unit_test_harness_is_a_test_process() {
    assert!(crate::core::data_dir::sandboxed());
    assert!(test_process());
}

#[test]
fn every_workspace_package_counts_as_cargo_launched() {
    // Read the members from the manifests, so a crate split out later is
    // covered too, or fails here instead of reaching production.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let members = workspace
        .split_once("members = [")
        .and_then(|(_, rest)| rest.split_once(']'))
        .map(|(list, _)| list)
        .unwrap();
    let names: Vec<String> = members
        .split(',')
        .map(|member| member.trim().trim_matches('"'))
        .filter(|member| !member.is_empty())
        .map(|member| {
            let manifest = std::fs::read_to_string(root.join(member).join("Cargo.toml")).unwrap();
            let package = manifest.split_once("[package]").unwrap().1;
            package
                .lines()
                .find_map(|line| line.trim().strip_prefix("name = "))
                .unwrap()
                .trim_matches('"')
                .to_string()
        })
        .collect();
    for expected in [
        "lean-ctx-engine",
        "lean-ctx",
        "lean-ctx-server",
        "lean-ctx-hive",
        "lean-ctx-hooks",
        "lean-ctx-proxy",
        "leanctx-native-storage",
    ] {
        assert!(names.iter().any(|name| name == expected), "{names:?}");
    }
    for name in &names {
        assert!(workspace_package(name), "{name}");
    }
    for foreign in ["", "serde", "my-lean-ctx", "lean_ctx", "cargo"] {
        assert!(!workspace_package(foreign), "{foreign}");
    }
}

#[test]
fn api_url_always_returns_loopback_sink_in_airgapped_build() {
    let _lock = crate::core::data_dir::test_env_lock();
    let previous = std::env::var_os("LEAN_CTX_API_URL");

    crate::test_env::remove_var("LEAN_CTX_API_URL");
    assert_eq!(api_url(), "http://127.0.0.1:9");
    crate::test_env::set_var("LEAN_CTX_API_URL", "https://api.leanctx.com");
    assert_eq!(api_url(), "http://127.0.0.1:9");
    crate::test_env::set_var("LEAN_CTX_API_URL", "http://127.0.0.1:8088");
    assert_eq!(api_url(), "http://127.0.0.1:9");

    match previous {
        Some(value) => crate::test_env::set_var("LEAN_CTX_API_URL", value),
        None => crate::test_env::remove_var("LEAN_CTX_API_URL"),
    }
}

#[test]
fn cargo_test_processes_are_recognised_as_cargo_launched() {
    // Children spawned by integration tests inherit this, which is what keeps
    // the release binary under test from reaching production telemetry.
    assert!(launched_by_cargo());
}

#[test]
fn telemetry_delete_response_requires_explicit_confirmation() {
    assert!(deletion_confirmed(200, r#"{"deleted":true}"#).unwrap());
    assert!(!deletion_confirmed(204, "").unwrap());
    assert!(!deletion_confirmed(200, r#"{"deleted":false}"#).unwrap());
    assert!(deletion_confirmed(200, "not-json").is_err());
}

struct TelemetryTestEnvironment(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl TelemetryTestEnvironment {
    // Call only while isolated_data_dir holds the shared test environment lock.
    fn new(listener: &std::net::TcpListener) -> Self {
        let values = [
            "LEAN_CTX_API_URL",
            "DO_NOT_TRACK",
            "LEAN_CTX_TELEMETRY",
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "http_proxy",
            "https_proxy",
            "all_proxy",
        ]
        .into_iter()
        .map(|key| {
            let previous = std::env::var_os(key);
            crate::test_env::remove_var(key);
            (key, previous)
        })
        .collect();
        let guard = Self(values);
        crate::test_env::set_var(
            "LEAN_CTX_API_URL",
            format!("http://{}", listener.local_addr().unwrap()),
        );
        guard
    }
}

impl Drop for TelemetryTestEnvironment {
    fn drop(&mut self) {
        for (key, value) in &self.0 {
            match value {
                Some(value) => crate::test_env::set_var(key, value),
                None => crate::test_env::remove_var(key),
            }
        }
    }
}

fn telemetry_test_config(contents: &str) {
    let path = crate::core::config::Config::path().unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn telemetry_test_batch() -> crate::core::telemetry_v2::TelemetryBatchV2 {
    use crate::core::telemetry_v2::{ClientFamily, DistributionChannel};
    crate::core::telemetry_aggregate::build_daily_heartbeat(
        "550e8400-e29b-41d4-a716-446655440000".into(),
        "a".repeat(64),
        "2026-03-01".into(),
        DistributionChannel::Unknown,
        ClientFamily::Other,
    )
    .unwrap()
}

#[test]
fn airgapped_build_never_opens_network_sockets_for_telemetry_or_feedback() {
    let _iso = crate::core::data_dir::isolated_data_dir();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let _env = TelemetryTestEnvironment::new(&listener);
    let batch = telemetry_test_batch();

    telemetry_test_config("[telemetry]\nenabled = true\npreference = 'explicitly_enabled'\n");
    let cfg = crate::core::config::Config::try_load_global().unwrap();
    assert!(!cfg.telemetry.send_eligible(None, None));
    assert_eq!(
        telemetry_v2_batch(&batch).unwrap_err(),
        "Telemetry sending is disabled"
    );
    assert!(submit_product_feedback(&serde_json::json!({"text": "hello"})).is_err());
    assert!(
        delete_remote_telemetry(
            "550e8400-e29b-41d4-a716-446655440000",
            &"a".repeat(64)
        )
        .is_err()
    );
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}
