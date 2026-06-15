use super::super::Client;
use super::Manifest;
use crate::filesystem::install::InstallError;
use indexmap::IndexMap;
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn temp_base() -> std::path::PathBuf {
    let d = std::env::temp_dir()
        .join(format!("oai-install-{}", uuid::Uuid::new_v4()));
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

/// The per-OS exec object the mock manifests declare. All three
/// vectors must be present — the SDK `Exec` derives `Deserialize`
/// without per-field defaults.
fn exec_json() -> serde_json::Value {
    json!({
        "windows": ["./plugin.exe"],
        "linux": ["./plugin"],
        "macos": ["./plugin"]
    })
}

/// The per-OS `cli_zip` object the mock manifests declare — every
/// platform points at the same `cli.zip` asset so the install fetches
/// it regardless of which host OS the test runs on. The `cli_zip`
/// field is required; each per-OS entry is optional.
fn cli_zip_json() -> serde_json::Value {
    json!({ "windows": "cli.zip", "linux": "cli.zip", "macos": "cli.zip" })
}

/// Build a minimal in-memory zip containing one file.
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

#[tokio::test]
async fn install_succeeds_and_extracts_cli_zip() {
    let base = temp_base();
    let server = MockServer::start().await;

    let manifest_body = json!({
        "owner": "claimed-owner",
        "name": "repo",
        "version": "1.0.0",
        "description": "test plugin",
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
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(build_zip("main.txt", "CLI PAYLOAD")),
        )
        .mount(&server)
        .await;

    let client = client_for(&base);
    let result = client
        .install_plugin_at(
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

    // The cli zip extracts into `<plugin dir>/cli/` — the exec working
    // directory.
    let dir = client.plugin_dir("owner", "repo", "1.0.0");
    let extracted = dir.join("cli").join("main.txt");
    assert!(extracted.exists(), "cli payload missing at {extracted:?}");
    assert_eq!(std::fs::read_to_string(&extracted).unwrap(), "CLI PAYLOAD");

    // Manifest is persisted verbatim as a `Manifest` at
    // <plugins_dir>/<owner>/<repository>/<version>/objectiveai.json.
    // The install writes the fetched manifest exactly as authored — the
    // author-claimed `owner` is NOT rewritten to the install owner.
    let manifest_path = dir.join("objectiveai.json");
    assert!(
        manifest_path.exists(),
        "manifest missing at {manifest_path:?}"
    );
    let persisted: Manifest =
        serde_json::from_slice(&std::fs::read(&manifest_path).unwrap())
            .unwrap();
    assert_eq!(persisted.owner, "claimed-owner");
    assert_eq!(persisted.name, "repo");
    assert_eq!(persisted.cli_zip.windows.as_deref(), Some("cli.zip"));
    assert_eq!(persisted.cli_zip.linux.as_deref(), Some("cli.zip"));
    assert_eq!(persisted.cli_zip.macos.as_deref(), Some("cli.zip"));
    assert_eq!(persisted.exec.windows, vec!["./plugin.exe"]);
    assert_eq!(persisted.exec.linux, vec!["./plugin"]);
    assert_eq!(persisted.exec.macos, vec!["./plugin"]);

    // Read back through `get_plugin`: it returns the bare `Manifest`,
    // whose `owner` / `name` are whatever the manifest authored.
    let manifest = client
        .get_plugin("owner", "repo", "1.0.0")
        .await
        .expect("expected Some(_)");
    assert_eq!(manifest.name, "repo");
    assert_eq!(manifest.owner, "claimed-owner");

    cleanup(&base);
}

#[tokio::test]
async fn install_manifest_only_persists_just_the_manifest() {
    // No cli_zip asset, no viewer source: the exec invokes
    // PATH-resolved programs, so nothing besides objectiveai.json lands
    // on disk. No release-asset mocks are registered — if install tried
    // to fetch one anyway, wiremock would 404 and the install would
    // error instead of returning Ok(true).
    let base = temp_base();
    let server = MockServer::start().await;

    let manifest_body = json!({
        "owner": "claimed-owner",
        "name": "repo",
        "version": "1.0.0",
        "description": "manifest-only plugin",
        "exec": {
            "windows": ["python.exe", "-m", "plugin"],
            "linux": ["python3", "-m", "plugin"],
            "macos": ["python3", "-m", "plugin"]
        },
        "cli_zip": {}
    });

    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .mount(&server)
        .await;

    let client = client_for(&base);
    let result = client
        .install_plugin_at(
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

    let dir = client.plugin_dir("owner", "repo", "1.0.0");
    assert!(dir.join("objectiveai.json").exists());
    assert!(!dir.join("cli").exists(), "no cli_zip → no cli/ dir");
    assert!(
        !dir.join("viewer").exists(),
        "no viewer source → no viewer/"
    );

    cleanup(&base);
}

#[tokio::test]
async fn install_uses_commit_sha_when_provided() {
    let base = temp_base();
    let server = MockServer::start().await;

    let manifest_body = json!({
        "owner": "claimed-owner",
        "name": "repo",
        "version": "1.0.0",
        "description": "test plugin",
        "exec": exec_json(),
        "cli_zip": cli_zip_json()
    });

    Mock::given(method("GET"))
        .and(path("/owner/repo/abc123/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/owner/repo/releases/download/v1.0.0/cli.zip"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(build_zip("main.txt", "x")),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&base);
    let result = client
        .install_plugin_at(
            &server.uri(),
            &server.uri(),
            "owner",
            "repo",
            Some("abc123"),
            None,
            false,
        )
        .await;

    assert!(matches!(result, Ok(true)), "got {result:?}");
    // Wiremock verifies .expect(1) on drop — if HEAD had been requested
    // instead of abc123 the manifest mock would record 0 hits and
    // panic the test.

    cleanup(&base);
}

#[tokio::test]
async fn install_manifest_404_returns_manifest_bad_status_error() {
    let base = temp_base();
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .mount(&server)
        .await;

    let client = client_for(&base);
    let result = client
        .install_plugin_at(
            &server.uri(),
            &server.uri(),
            "owner",
            "repo",
            None,
            None,
            false,
        )
        .await;

    match result {
        Err(super::super::Error::Install(
            InstallError::ManifestBadStatus { code, .. },
        )) => {
            assert_eq!(code.as_u16(), 404);
        }
        other => panic!("expected ManifestBadStatus(404), got {other:?}"),
    }

    cleanup(&base);
}

#[tokio::test]
async fn install_cli_zip_404_returns_cli_zip_bad_status_error() {
    let base = temp_base();
    let server = MockServer::start().await;

    let manifest_body = json!({
        "owner": "claimed-owner",
        "name": "repo",
        "version": "1.0.0",
        "description": "test plugin",
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
        .install_plugin_at(
            &server.uri(),
            &server.uri(),
            "owner",
            "repo",
            None,
            None,
            false,
        )
        .await;

    match result {
        Err(super::super::Error::Install(InstallError::CliZipBadStatus {
            code,
            ..
        })) => {
            assert_eq!(code.as_u16(), 404);
        }
        other => panic!("expected CliZipBadStatus(404), got {other:?}"),
    }

    cleanup(&base);
}

#[tokio::test]
async fn install_corrupt_cli_zip_returns_zip_extract_error() {
    let base = temp_base();
    let server = MockServer::start().await;

    let manifest_body = json!({
        "owner": "claimed-owner",
        "name": "repo",
        "version": "1.0.0",
        "description": "test plugin",
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
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"definitely not a zip".to_vec()),
        )
        .mount(&server)
        .await;

    let client = client_for(&base);
    let result = client
        .install_plugin_at(
            &server.uri(),
            &server.uri(),
            "owner",
            "repo",
            None,
            None,
            false,
        )
        .await;

    match result {
        Err(super::super::Error::Install(InstallError::ZipExtract(
            dir,
            reason,
        ))) => {
            assert!(dir.ends_with("cli"), "got {dir:?}");
            assert!(reason.contains("zip archive open"), "got {reason:?}");
        }
        other => panic!("expected ZipExtract, got {other:?}"),
    }

    cleanup(&base);
}

#[tokio::test]
async fn install_malformed_manifest_returns_parse_error() {
    let base = temp_base();
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(
            ResponseTemplate::new(200).set_body_bytes(b"not json{{{".to_vec()),
        )
        .mount(&server)
        .await;

    let client = client_for(&base);
    let result = client
        .install_plugin_at(
            &server.uri(),
            &server.uri(),
            "owner",
            "repo",
            None,
            None,
            false,
        )
        .await;

    match result {
        Err(super::super::Error::Install(InstallError::ManifestParse(_))) => {}
        other => panic!("expected ManifestParse, got {other:?}"),
    }

    cleanup(&base);
}

#[tokio::test]
async fn fetch_plugin_manifest_returns_parsed_manifest() {
    let base = temp_base();
    let server = MockServer::start().await;

    let manifest_body = json!({
        "owner": "claimed-owner",
        "name": "repo",
        "version": "1.2.3",
        "description": "test plugin",
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
        .fetch_plugin_manifest_at(&server.uri(), "owner", "repo", None, None)
        .await
        .expect("expected Ok(Manifest)");

    assert_eq!(manifest.description, "test plugin");
    assert_eq!(manifest.version, "1.2.3");
    // The manifest is returned exactly as the repo's objectiveai.json
    // authored it — `owner` is the claimed value, not the install owner.
    assert_eq!(manifest.owner, "claimed-owner");
    assert_eq!(manifest.name, "repo");
    assert_eq!(manifest.exec.linux, vec!["./plugin"]);
    assert_eq!(manifest.cli_zip.windows.as_deref(), Some("cli.zip"));

    cleanup(&base);
}

#[tokio::test]
async fn install_passes_headers_to_both_requests() {
    let base = temp_base();
    let server = MockServer::start().await;

    let manifest_body = json!({
        "owner": "claimed-owner",
        "name": "repo",
        "version": "1.0.0",
        "description": "test plugin",
        "exec": exec_json(),
        "cli_zip": cli_zip_json()
    });

    // Both mocks require the header — if the SDK doesn't forward it,
    // wiremock falls through and returns 404 (which would surface as
    // ManifestBadStatus / CliZipBadStatus).
    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .and(header("authorization", "token abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/owner/repo/releases/download/v1.0.0/cli.zip"))
        .and(header("authorization", "token abc"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(build_zip("main.txt", "x")),
        )
        .mount(&server)
        .await;

    let mut headers = IndexMap::new();
    headers.insert("Authorization".to_string(), "token abc".to_string());

    let client = client_for(&base);
    let result = client
        .install_plugin_at(
            &server.uri(),
            &server.uri(),
            "owner",
            "repo",
            None,
            Some(&headers),
            false,
        )
        .await;

    assert!(matches!(result, Ok(true)), "got {result:?}");

    cleanup(&base);
}

#[tokio::test]
async fn install_makes_plugin_appear_in_list() {
    let base = temp_base();
    let server = MockServer::start().await;

    let manifest_body = json!({
        "owner": "claimed-owner",
        "name": "repo",
        "version": "1.0.0",
        "description": "installed plugin",
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
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(build_zip("main.txt", "x")),
        )
        .mount(&server)
        .await;

    let client = client_for(&base);
    let ok = client
        .install_plugin_at(
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

    let plugins = client.list_plugins(0, 100).await;
    assert_eq!(
        plugins.len(),
        1,
        "expected one installed plugin, got {plugins:?}"
    );
    let p = &plugins[0];
    assert_eq!(p.name, "repo");
    assert_eq!(p.description, "installed plugin");
    assert_eq!(p.version, "1.0.0");

    cleanup(&base);
}

#[tokio::test]
async fn install_then_get_plugin_returns_persisted_manifest() {
    let base = temp_base();
    let server = MockServer::start().await;

    let manifest_body = json!({
        "owner": "claimed-owner",
        "name": "repo",
        "version": "1.0.0",
        "description": "installed plugin",
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
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(build_zip("main.txt", "x")),
        )
        .mount(&server)
        .await;

    let client = client_for(&base);
    let ok = client
        .install_plugin_at(
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

    let got = client
        .get_plugin("owner", "repo", "1.0.0")
        .await
        .expect("expected Some(_)");
    assert_eq!(got.name, "repo");
    assert_eq!(got.version, "1.0.0");

    cleanup(&base);
}

#[tokio::test]
async fn install_extracts_viewer_zip_when_present() {
    let base = temp_base();
    let server = MockServer::start().await;

    let manifest_body = json!({
        "owner": "claimed-owner",
        "name": "repo",
        "version": "1.0.0",
        "description": "viewer plugin",
        "exec": exec_json(),
        "cli_zip": cli_zip_json(),
        "viewer_zip": "v.zip",
        "viewer_routes": [
            { "path": "/say", "method": "POST", "type": "say_request" }
        ]
    });

    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/owner/repo/releases/download/v1.0.0/cli.zip"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(build_zip("main.txt", "CLI PAYLOAD")),
        )
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/owner/repo/releases/download/v1.0.0/v.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(build_zip(
            "index.html",
            "<!doctype html><title>hi</title>",
        )))
        .mount(&server)
        .await;

    let client = client_for(&base);
    let ok = client
        .install_plugin_at(
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

    let dir = client.plugin_dir("owner", "repo", "1.0.0");
    let viewer_index = dir.join("viewer").join("index.html");
    assert!(
        viewer_index.exists(),
        "viewer/index.html missing at {viewer_index:?}"
    );
    let contents = std::fs::read_to_string(&viewer_index).unwrap();
    assert!(
        contents.contains("hi"),
        "unexpected viewer index: {contents:?}"
    );

    // The cli zip extracted alongside, into its own folder.
    assert!(dir.join("cli").join("main.txt").exists());

    // Manifest persistence (bare `Manifest`) still records the viewer
    // fields.
    let persisted: Manifest = serde_json::from_slice(
        &std::fs::read(dir.join("objectiveai.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(persisted.viewer_zip.as_deref(), Some("v.zip"));
    assert_eq!(persisted.viewer_routes.len(), 1);

    cleanup(&base);
}

#[tokio::test]
async fn install_skips_viewer_zip_when_absent() {
    let base = temp_base();
    let server = MockServer::start().await;

    let manifest_body = json!({
        "owner": "claimed-owner",
        "name": "repo",
        "version": "1.0.0",
        "description": "no-viewer plugin",
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
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(build_zip("main.txt", "x")),
        )
        .mount(&server)
        .await;

    let client = client_for(&base);
    let ok = client
        .install_plugin_at(
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

    let viewer_dir = client.plugin_dir("owner", "repo", "1.0.0").join("viewer");
    assert!(
        !viewer_dir.exists(),
        "viewer dir should not exist for plugin without viewer_zip"
    );

    cleanup(&base);
}

#[tokio::test]
async fn install_skips_viewer_zip_download_when_viewer_url_set() {
    // When the manifest declares `viewer_url` (and no `viewer_zip`),
    // the install path should persist the manifest with the URL and
    // NOT create `<plugin_dir>/viewer/`. The cli_zip is empty, so no
    // release-asset mock is registered at all — if install tries to
    // fetch one, wiremock 404s and the test fails.
    let base = temp_base();
    let server = MockServer::start().await;

    let manifest_body = json!({
        "owner": "claimed-owner",
        "name": "repo",
        "version": "1.0.0",
        "description": "url-viewer plugin",
        "exec": exec_json(),
        "cli_zip": {},
        "viewer_url": "https://plugin.example.com/index.html",
        "viewer_routes": [
            { "path": "/say", "method": "POST", "type": "say_request" }
        ]
    });

    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .mount(&server)
        .await;

    let client = client_for(&base);
    let ok = client
        .install_plugin_at(
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

    let dir = client.plugin_dir("owner", "repo", "1.0.0");
    let viewer_dir = dir.join("viewer");
    assert!(
        !viewer_dir.exists(),
        "viewer dir should not exist for viewer_url plugin (no zip to extract)"
    );

    let persisted: Manifest = serde_json::from_slice(
        &std::fs::read(dir.join("objectiveai.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        persisted.viewer_url.as_deref(),
        Some("https://plugin.example.com/index.html")
    );
    assert!(persisted.viewer_zip.is_none());
    assert_eq!(persisted.viewer_routes.len(), 1);
    assert!(persisted.has_viewer());

    cleanup(&base);
}

#[tokio::test]
async fn install_rejects_manifest_with_both_viewer_sources() {
    let base = temp_base();
    let server = MockServer::start().await;

    let manifest_body = json!({
        "owner": "claimed-owner",
        "name": "repo",
        "version": "1.0.0",
        "description": "broken plugin",
        "exec": exec_json(),
        "cli_zip": {},
        "viewer_zip": "v.zip",
        "viewer_url": "https://plugin.example.com/"
    });

    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .mount(&server)
        .await;

    let client = client_for(&base);
    let err = client
        .install_plugin_at(
            &server.uri(),
            &server.uri(),
            "owner",
            "repo",
            None,
            None,
            false,
        )
        .await
        .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("mutually exclusive"), "got {msg:?}");

    cleanup(&base);
}

#[tokio::test]
async fn install_viewer_zip_404_returns_error() {
    let base = temp_base();
    let server = MockServer::start().await;

    let manifest_body = json!({
        "owner": "claimed-owner",
        "name": "repo",
        "version": "1.0.0",
        "description": "broken viewer plugin",
        "exec": exec_json(),
        "cli_zip": {},
        "viewer_zip": "missing.zip"
    });

    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/owner/repo/releases/download/v1.0.0/missing.zip"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = client_for(&base);
    let result = client
        .install_plugin_at(
            &server.uri(),
            &server.uri(),
            "owner",
            "repo",
            None,
            None,
            false,
        )
        .await;

    match result {
        Err(super::super::Error::Install(
            InstallError::ViewerZipBadStatus { code, .. },
        )) => {
            assert_eq!(code.as_u16(), 404);
        }
        other => panic!("expected ViewerZipBadStatus(404), got {other:?}"),
    }

    cleanup(&base);
}

// ============================================================
// --upgrade flag behaviour
// ============================================================

/// Builds two mock endpoints (manifest + cli zip) that any install
/// call hits. The cli zip contains a single `file_name` entry with
/// `contents`.
async fn upgrade_test_server(
    version: &str,
    file_name: &str,
    contents: &str,
) -> MockServer {
    let server = MockServer::start().await;
    let manifest_body = json!({
        "owner": "claimed-owner",
        "name": "repo",
        "version": version,
        "description": "upgrade test plugin",
        "exec": exec_json(),
        "cli_zip": cli_zip_json()
    });
    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!(
            "/owner/repo/releases/download/v{version}/cli.zip"
        )))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(build_zip(file_name, contents)),
        )
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn install_refuses_when_manifest_exists_and_not_upgrade() {
    let base = temp_base();
    let server = upgrade_test_server("1.0.0", "main.txt", "FIRST").await;
    let client = client_for(&base);

    // First install succeeds.
    client
        .install_plugin_at(
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

    // Second install with upgrade=false errors.
    let result = client
        .install_plugin_at(
            &server.uri(),
            &server.uri(),
            "owner",
            "repo",
            None,
            None,
            false,
        )
        .await;
    match result {
        Err(super::super::Error::Install(InstallError::AlreadyInstalled {
            repository,
        })) => {
            assert_eq!(repository, "repo");
        }
        other => panic!("expected AlreadyInstalled, got {other:?}"),
    }

    // Disk state should match the first install (untouched by the refused second one).
    let dir = client.plugin_dir("owner", "repo", "1.0.0");
    let payload =
        std::fs::read_to_string(dir.join("cli").join("main.txt")).unwrap();
    assert_eq!(payload, "FIRST");
    assert!(dir.join("objectiveai.json").exists());

    cleanup(&base);
}

#[tokio::test]
async fn install_upgrade_replaces_prior_artifacts() {
    let base = temp_base();
    let client = client_for(&base);

    // Install A at version 1.0.0.
    let server_a = upgrade_test_server("1.0.0", "main.txt", "VERSION_A").await;
    client
        .install_plugin_at(
            &server_a.uri(),
            &server_a.uri(),
            "owner",
            "repo",
            None,
            None,
            false,
        )
        .await
        .unwrap();

    // Install B at version 2.0.0 with --upgrade.
    let server_b = upgrade_test_server("2.0.0", "main.txt", "VERSION_B").await;
    client
        .install_plugin_at(
            &server_b.uri(),
            &server_b.uri(),
            "owner",
            "repo",
            None,
            None,
            true,
        )
        .await
        .unwrap();

    // The cli payload on disk now has B's body; manifest carries B's
    // version. The new version lands in its own version dir
    // (`repo/2.0.0/`).
    let dir = client.plugin_dir("owner", "repo", "2.0.0");
    let payload =
        std::fs::read_to_string(dir.join("cli").join("main.txt")).unwrap();
    assert_eq!(payload, "VERSION_B");
    let persisted: Manifest = serde_json::from_slice(
        &std::fs::read(dir.join("objectiveai.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(persisted.version, "2.0.0");

    cleanup(&base);
}

#[tokio::test]
async fn install_upgrade_replaces_cli_dir_in_place() {
    // Same-version upgrade: the whole `cli/` DIRECTORY is removed
    // before the new zip extracts, so stale files from the prior
    // bundle don't linger.
    let base = temp_base();
    let client = client_for(&base);

    let server_a = upgrade_test_server("1.0.0", "old.txt", "OLD").await;
    client
        .install_plugin_at(
            &server_a.uri(),
            &server_a.uri(),
            "owner",
            "repo",
            None,
            None,
            false,
        )
        .await
        .unwrap();

    let server_b = upgrade_test_server("1.0.0", "new.txt", "NEW").await;
    client
        .install_plugin_at(
            &server_b.uri(),
            &server_b.uri(),
            "owner",
            "repo",
            None,
            None,
            true,
        )
        .await
        .unwrap();

    let cli_dir = client.plugin_dir("owner", "repo", "1.0.0").join("cli");
    assert_eq!(
        std::fs::read_to_string(cli_dir.join("new.txt")).unwrap(),
        "NEW"
    );
    assert!(
        !cli_dir.join("old.txt").exists(),
        "stale cli payload should have been removed with the cli/ dir"
    );

    cleanup(&base);
}

#[tokio::test]
async fn install_upgrade_preserves_extra_data_under_plugin_dir() {
    let base = temp_base();
    let client = client_for(&base);

    // Install A.
    let server_a = upgrade_test_server("1.0.0", "main.txt", "V1").await;
    client
        .install_plugin_at(
            &server_a.uri(),
            &server_a.uri(),
            "owner",
            "repo",
            None,
            None,
            false,
        )
        .await
        .unwrap();

    // Drop a "user-state" file under the plugin's version dir — the
    // upgrade cleanup removes the cli/manifest/viewer entries but
    // leaves extra entries untouched.
    let user_state = client
        .plugin_dir("owner", "repo", "1.0.0")
        .join("user-state.json");
    std::fs::write(&user_state, b"{\"runs\":42}").unwrap();

    // Upgrade (same version → same version dir is re-installed in place).
    let server_b = upgrade_test_server("1.0.0", "main.txt", "V2").await;
    client
        .install_plugin_at(
            &server_b.uri(),
            &server_b.uri(),
            "owner",
            "repo",
            None,
            None,
            true,
        )
        .await
        .unwrap();

    // user-state.json is preserved verbatim.
    let preserved = std::fs::read(&user_state).unwrap();
    assert_eq!(preserved, b"{\"runs\":42}");

    cleanup(&base);
}

#[tokio::test]
async fn install_upgrade_with_no_prior_install_just_installs() {
    let base = temp_base();
    let server = upgrade_test_server("1.0.0", "main.txt", "x").await;
    let client = client_for(&base);

    // upgrade=true on a fresh base — should still work, no prior state to delete.
    let ok = client
        .install_plugin_at(
            &server.uri(),
            &server.uri(),
            "owner",
            "repo",
            None,
            None,
            true,
        )
        .await
        .unwrap();
    assert!(ok);
    let dir = client.plugin_dir("owner", "repo", "1.0.0");
    assert!(dir.join("cli").join("main.txt").exists());
    assert!(dir.join("objectiveai.json").exists());

    cleanup(&base);
}

#[tokio::test]
async fn install_network_failure_leaves_disk_untouched_on_fresh() {
    let base = temp_base();
    let server = MockServer::start().await;
    let manifest_body = json!({
        "owner": "claimed-owner",
        "name": "repo",
        "version": "1.0.0",
        "description": "broken plugin",
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
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let client = client_for(&base);
    let result = client
        .install_plugin_at(
            &server.uri(),
            &server.uri(),
            "owner",
            "repo",
            None,
            None,
            false,
        )
        .await;
    assert!(matches!(
        result,
        Err(super::super::Error::Install(
            InstallError::CliZipBadStatus { .. }
        ))
    ));

    // Nothing on disk — the failure happened in the network phase before any writes.
    assert!(!client.plugins_dir().join("owner").join("repo").exists());
    assert!(
        !client
            .plugin_dir("owner", "repo", "1.0.0")
            .join("objectiveai.json")
            .exists()
    );

    cleanup(&base);
}

#[tokio::test]
async fn install_upgrade_network_failure_leaves_prior_install_intact() {
    let base = temp_base();
    let client = client_for(&base);

    // Install A successfully.
    let server_a = upgrade_test_server("1.0.0", "main.txt", "V1").await;
    client
        .install_plugin_at(
            &server_a.uri(),
            &server_a.uri(),
            "owner",
            "repo",
            None,
            None,
            false,
        )
        .await
        .unwrap();

    // Attempt upgrade (same version → same version dir) with a cli
    // zip endpoint that 500s. The fetch fails in the network phase,
    // before any disk write or the pre-write clean.
    let server_b = MockServer::start().await;
    let manifest_body = json!({
        "owner": "claimed-owner",
        "name": "repo",
        "version": "1.0.0",
        "description": "broken upgrade",
        "exec": exec_json(),
        "cli_zip": cli_zip_json()
    });
    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .mount(&server_b)
        .await;
    Mock::given(method("GET"))
        .and(path("/owner/repo/releases/download/v1.0.0/cli.zip"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server_b)
        .await;

    let result = client
        .install_plugin_at(
            &server_b.uri(),
            &server_b.uri(),
            "owner",
            "repo",
            None,
            None,
            true,
        )
        .await;
    assert!(matches!(
        result,
        Err(super::super::Error::Install(
            InstallError::CliZipBadStatus { .. }
        ))
    ));

    // The network phase (cli-zip fetch) runs BEFORE any disk write or
    // the pre-write clean, so a network failure leaves the prior
    // install (A) completely intact — the user retries with the
    // working install still in place.
    let dir = client.plugin_dir("owner", "repo", "1.0.0");
    assert_eq!(
        std::fs::read_to_string(dir.join("cli").join("main.txt")).unwrap(),
        "V1"
    );
    assert!(dir.join("objectiveai.json").exists());

    cleanup(&base);
}

#[tokio::test]
async fn install_refuses_reserved_repository_name() {
    let base = temp_base();
    let client = client_for(&base);
    let result = client
        .install_plugin_at(
            "http://example.invalid",
            "http://example.invalid",
            "owner",
            "objectiveai",
            None,
            None,
            false,
        )
        .await;
    match result {
        Err(super::super::Error::Install(
            InstallError::ReservedRepositoryName { repository },
        )) => {
            assert_eq!(repository, "objectiveai");
        }
        other => panic!("expected ReservedRepositoryName, got {other:?}"),
    }
    assert!(
        !client
            .plugins_dir()
            .join("owner")
            .join("objectiveai")
            .exists()
    );
    assert!(!client.plugins_dir().join("objectiveai.json").exists());
    cleanup(&base);
}

#[tokio::test]
async fn install_refuses_reserved_repository_name_case_insensitive() {
    let base = temp_base();
    let client = client_for(&base);
    for candidate in ["ObjectiveAI", "OBJECTIVEAI", "Objectiveai"] {
        let result = client
            .install_plugin_at(
                "http://example.invalid",
                "http://example.invalid",
                "owner",
                candidate,
                None,
                None,
                false,
            )
            .await;
        assert!(
            matches!(
                result,
                Err(super::super::Error::Install(
                    InstallError::ReservedRepositoryName { .. }
                ))
            ),
            "expected ReservedRepositoryName for {candidate:?}",
        );
    }
    cleanup(&base);
}

#[tokio::test]
async fn install_rejects_invalid_identifiers() {
    // Identifier shape checks run before any network traffic — the
    // example.invalid bases are never dialed.
    let base = temp_base();
    let client = client_for(&base);
    let cases: [(&str, &str, Option<&str>, &str); 4] = [
        ("bad owner", "repo", None, "owner"),
        ("", "repo", None, "owner"),
        ("owner", "bad/repo", None, "repository"),
        ("owner", "repo", Some("no spaces"), "commit"),
    ];
    for (owner, repo, sha, expected_kind) in cases {
        let result = client
            .install_plugin_at(
                "http://example.invalid",
                "http://example.invalid",
                owner,
                repo,
                sha,
                None,
                false,
            )
            .await;
        match result {
            Err(super::super::Error::Install(
                InstallError::InvalidIdentifier { kind, .. },
            )) => {
                assert_eq!(
                    kind, expected_kind,
                    "for ({owner:?}, {repo:?}, {sha:?})"
                );
            }
            other => panic!(
                "expected InvalidIdentifier({expected_kind}) for \
                 ({owner:?}, {repo:?}, {sha:?}), got {other:?}"
            ),
        }
    }
    cleanup(&base);
}
