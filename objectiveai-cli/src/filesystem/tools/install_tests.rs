//! Tool GitHub-install tests, exercising the shared install engine via
//! the tool path (`install_tool_at` / `fetch_tool_manifest_at`). The
//! engine's full surface is covered by the plugin `install_tests`; this
//! module focuses on the tool-specific behaviour: cli-only assets (no
//! viewer), the `tools/` dir layout, and that the reserved-name check
//! does NOT apply to tools.

use super::super::Client;
use super::Manifest;
use crate::filesystem::install::InstallError;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn temp_base() -> std::path::PathBuf {
    let d = std::env::temp_dir()
        .join(format!("oai-install-tool-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn cleanup(d: &std::path::Path) {
    let _ = std::fs::remove_dir_all(d);
}

fn client_for(base: &std::path::Path) -> Client {
    Client::new(
        Some(base.to_path_buf()),
        None::<String>,
        None::<&str>,
        None::<&str>,
    )
}

fn exec_json() -> serde_json::Value {
    json!({
        "windows": ["./tool.exe"],
        "linux": ["./tool"],
        "macos": ["./tool"]
    })
}

/// Per-OS `cli_zip` pointing every platform at the same `cli.zip`, so
/// the install fetches it regardless of the host OS the test runs on.
fn cli_zip_json() -> serde_json::Value {
    json!({ "windows": "cli.zip", "linux": "cli.zip", "macos": "cli.zip" })
}

fn build_zip(file_name: &str, contents: &str) -> Vec<u8> {
    use std::io::Write;
    let mut buf = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options: zip::write::SimpleFileOptions =
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
        writer.start_file(file_name, options).unwrap();
        writer.write_all(contents.as_bytes()).unwrap();
        writer.finish().unwrap();
    }
    buf
}

/// Mount the manifest + cli.zip endpoints for `<repository>` at the
/// given version, returning the started server.
async fn tool_server(repository: &str, version: &str, payload: &str) -> MockServer {
    let server = MockServer::start().await;
    let manifest_body = json!({
        "owner": "claimed-owner",
        "name": repository,
        "version": version,
        "description": "test tool",
        "exec": exec_json(),
        "cli_zip": cli_zip_json()
    });
    Mock::given(method("GET"))
        .and(path(format!("/owner/{repository}/HEAD/objectiveai.json")))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!(
            "/owner/{repository}/releases/download/v{version}/cli.zip"
        )))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(build_zip("main.txt", payload)),
        )
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn install_tool_succeeds_and_extracts_cli_zip() {
    let base = temp_base();
    let server = tool_server("repo", "1.0.0", "TOOL PAYLOAD").await;
    let client = client_for(&base);

    let result = client
        .install_tool_at(
            &server.uri(),
            &server.uri(),
            "owner",
            "repo",
            None,
            None,
            false,
        )
        .await;
    assert!(matches!(result, Ok(true)), "got {result:?}");

    // The cli zip extracts into the tool's `cli/` dir under tools/.
    let dir = client.tool_dir("owner", "repo", "1.0.0");
    let extracted = dir.join("cli").join("main.txt");
    assert!(extracted.exists(), "cli payload missing at {extracted:?}");
    assert_eq!(std::fs::read_to_string(&extracted).unwrap(), "TOOL PAYLOAD");

    // Tools never get a viewer dir.
    assert!(!dir.join("viewer").exists(), "tools have no viewer bundle");

    // Manifest persisted verbatim — author-claimed owner not rewritten.
    let persisted: Manifest = serde_json::from_slice(
        &std::fs::read(dir.join("objectiveai.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(persisted.owner, "claimed-owner");
    assert_eq!(persisted.name, "repo");
    assert_eq!(persisted.cli_zip.linux.as_deref(), Some("cli.zip"));

    // Reads back through get_tool.
    let got = client.get_tool("owner", "repo", "1.0.0").await;
    assert_eq!(got.expect("expected Some(_)").owner, "claimed-owner");

    cleanup(&base);
}

#[tokio::test]
async fn install_tool_manifest_only_no_cli_dir() {
    // Empty cli_zip: nothing to fetch, no cli/ dir. No cli mock is
    // registered — if install tried to fetch one, wiremock would 404.
    let base = temp_base();
    let server = MockServer::start().await;
    let manifest_body = json!({
        "owner": "claimed-owner",
        "name": "repo",
        "version": "1.0.0",
        "description": "path-resolved tool",
        "exec": exec_json(),
        "cli_zip": {}
    });
    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .mount(&server)
        .await;

    let client = client_for(&base);
    let ok = client
        .install_tool_at(
            &server.uri(),
            &server.uri(),
            "owner",
            "repo",
            None,
            None,
            false,
        )
        .await
        .unwrap();
    assert!(ok);

    let dir = client.tool_dir("owner", "repo", "1.0.0");
    assert!(dir.join("objectiveai.json").exists());
    assert!(!dir.join("cli").exists(), "empty cli_zip → no cli/ dir");

    cleanup(&base);
}

#[tokio::test]
async fn install_tool_allows_reserved_repository_name() {
    // `objectiveai` is reserved for PLUGINS (viewer Tauri channel) but
    // not for tools — a tool repo named `objectiveai` installs fine.
    let base = temp_base();
    let server = tool_server("objectiveai", "1.0.0", "x").await;
    let client = client_for(&base);

    let result = client
        .install_tool_at(
            &server.uri(),
            &server.uri(),
            "owner",
            "objectiveai",
            None,
            None,
            false,
        )
        .await;
    assert!(matches!(result, Ok(true)), "got {result:?}");

    cleanup(&base);
}

#[tokio::test]
async fn install_tool_refuses_when_exists_and_not_upgrade() {
    let base = temp_base();
    let server = tool_server("repo", "1.0.0", "FIRST").await;
    let client = client_for(&base);

    client
        .install_tool_at(&server.uri(), &server.uri(), "owner", "repo", None, None, false)
        .await
        .unwrap();

    let result = client
        .install_tool_at(&server.uri(), &server.uri(), "owner", "repo", None, None, false)
        .await;
    match result {
        Err(crate::filesystem::Error::Install(InstallError::AlreadyInstalled {
            repository,
        })) => assert_eq!(repository, "repo"),
        other => panic!("expected AlreadyInstalled, got {other:?}"),
    }

    cleanup(&base);
}

#[tokio::test]
async fn install_tool_upgrade_replaces_prior() {
    let base = temp_base();
    let client = client_for(&base);

    let server_a = tool_server("repo", "1.0.0", "V1").await;
    client
        .install_tool_at(&server_a.uri(), &server_a.uri(), "owner", "repo", None, None, false)
        .await
        .unwrap();

    let server_b = tool_server("repo", "1.0.0", "V2").await;
    client
        .install_tool_at(&server_b.uri(), &server_b.uri(), "owner", "repo", None, None, true)
        .await
        .unwrap();

    let payload = std::fs::read_to_string(
        client.tool_cli_dir("owner", "repo", "1.0.0").join("main.txt"),
    )
    .unwrap();
    assert_eq!(payload, "V2");

    cleanup(&base);
}

#[tokio::test]
async fn install_tool_cli_zip_404_returns_error() {
    let base = temp_base();
    let server = MockServer::start().await;
    let manifest_body = json!({
        "owner": "claimed-owner",
        "name": "repo",
        "version": "1.0.0",
        "description": "broken tool",
        "exec": exec_json(),
        "cli_zip": cli_zip_json()
    });
    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/owner/repo/releases/download/v1.0.0/cli.zip"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = client_for(&base);
    let result = client
        .install_tool_at(&server.uri(), &server.uri(), "owner", "repo", None, None, false)
        .await;
    match result {
        Err(crate::filesystem::Error::Install(InstallError::CliZipBadStatus {
            code,
            ..
        })) => assert_eq!(code.as_u16(), 404),
        other => panic!("expected CliZipBadStatus(404), got {other:?}"),
    }

    cleanup(&base);
}

#[tokio::test]
async fn fetch_tool_manifest_returns_parsed() {
    let base = temp_base();
    let server = MockServer::start().await;
    let manifest_body = json!({
        "owner": "claimed-owner",
        "name": "repo",
        "version": "2.3.4",
        "description": "fetch test",
        "exec": exec_json(),
        "cli_zip": cli_zip_json()
    });
    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .mount(&server)
        .await;

    let client = client_for(&base);
    let manifest = client
        .fetch_tool_manifest_at(&server.uri(), "owner", "repo", None, None)
        .await
        .expect("expected Ok(Manifest)");
    assert_eq!(manifest.owner, "claimed-owner");
    assert_eq!(manifest.name, "repo");
    assert_eq!(manifest.version, "2.3.4");
    assert_eq!(manifest.cli_zip.windows.as_deref(), Some("cli.zip"));

    cleanup(&base);
}
