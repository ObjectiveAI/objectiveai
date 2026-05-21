//! End-to-end test for `objectiveai viewer send`.
//!
//! Spawns a `wiremock::MockServer` on a random port as a stand-in for
//! the real viewer's HTTP server, configures the cli's filesystem
//! config to point at it (`viewer.address` + `viewer.signature`),
//! then invokes the cli binary and asserts that the request lands on
//! the mock with the expected path, body, and `X-VIEWER-SIGNATURE`
//! header — matching the same header format the viewer's signature
//! middleware accepts.

use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;
use wiremock::{
    matchers::{body_json, header, method, path},
    Mock, MockServer, ResponseTemplate,
};

const SIGNATURE: &str = "sha256=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn temp_base() -> PathBuf {
    let d = std::env::temp_dir().join(format!("oai-viewer-send-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[tokio::test]
async fn viewer_send_remote_mode_posts_to_configured_address() {
    let base = temp_base();

    let mock_server = MockServer::start().await;

    let request_body = serde_json::json!({"id": "test-id", "model": "mock"});
    let response_body = serde_json::json!({"received": true});

    Mock::given(method("POST"))
        .and(path("/agent/completions"))
        .and(header("X-VIEWER-SIGNATURE", SIGNATURE))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .expect(1)
        .mount(&mock_server)
        .await;

    let fs_client = objectiveai_sdk::filesystem::Client::new(
        Some(base.clone()),
        None::<String>,
        None::<String>,
    );
    let mut config = fs_client
        .read_config()
        .await
        .expect("read_config failed");
    config.viewer().set_address(mock_server.uri());
    config.viewer().set_signature(SIGNATURE.to_string());
    fs_client
        .write_config(&config)
        .await
        .expect("write_config failed");

    let cli = env!("CARGO_BIN_EXE_objectiveai-cli");
    let body_arg = serde_json::to_string(&request_body).unwrap();
    let output = Command::new(cli)
        .env("CONFIG_BASE_DIR", &base)
        .args(["viewer", "send", "/agent/completions", &body_arg])
        .output()
        .expect("failed to run cli");

    let stdout = String::from_utf8(output.stdout).expect("cli stdout not utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "cli exited {:?}\nstdout: {stdout}\nstderr: {stderr}",
        output.status,
    );

    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines.len() >= 3,
        "expected at least begin/notification/end, got: {lines:?}"
    );

    let first: Value =
        serde_json::from_str(lines.first().unwrap()).expect("first line not JSON");
    let last: Value =
        serde_json::from_str(lines.last().unwrap()).expect("last line not JSON");
    assert_eq!(first.get("type"), Some(&Value::String("begin".into())));
    assert_eq!(last.get("type"), Some(&Value::String("end".into())));

    let matching = lines
        .iter()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|v| v.get("type") == Some(&Value::String("notification".into())))
        .filter(|v| v.pointer("/value/status") == Some(&Value::Number(200.into())))
        .filter(|v| v.pointer("/value/body") == Some(&response_body))
        .count();
    assert_eq!(
        matching, 1,
        "expected exactly one notification with status=200 and body={response_body} in {lines:?}",
    );

    mock_server.verify().await;

    let _ = std::fs::remove_dir_all(&base);
}
