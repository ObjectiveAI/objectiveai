use super::super::Client;
use super::{InstallError, ManifestWithNameAndSource, Platform};
use indexmap::IndexMap;
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn temp_base() -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("oai-install-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn cleanup(d: &std::path::Path) {
    let _ = std::fs::remove_dir_all(d);
}

fn binary_filename() -> &'static str {
    if cfg!(windows) { "plugin.exe" } else { "plugin" }
}

/// Serialize `Platform::current()` to its kebab-case wire form for
/// constructing mock manifest bodies.
fn current_platform_key() -> String {
    let p = Platform::current().expect("test requires a supported host platform");
    serde_json::to_value(p)
        .unwrap()
        .as_str()
        .unwrap()
        .to_string()
}

fn client_for(base: &std::path::Path) -> Client {
    Client::new(Some(base.to_path_buf()), None::<&str>, None::<&str>)
}

const FAKE_BIN: &[u8] = b"FAKEBIN";

#[tokio::test]
async fn install_succeeds_when_platform_supported() {
    let base = temp_base();
    let server = MockServer::start().await;
    let platform_key = current_platform_key();

    let manifest_body = json!({
        "description": "test plugin",
        "version": "1.0.0",
        "owner": "claimed-owner",
        "binaries": {
            platform_key: "asset-bin"
        }
    });

    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/owner/repo/releases/download/v1.0.0/asset-bin"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(FAKE_BIN.to_vec()))
        .mount(&server)
        .await;

    let client = client_for(&base);
    let result = client
        .install_plugin_at(&server.uri(), &server.uri(), "owner", "repo", None, None, false)
        .await;

    assert!(matches!(result, Ok(true)), "got {result:?}");

    let binary_path = base.join("plugins").join("repo").join(binary_filename());
    assert!(binary_path.exists(), "binary missing at {binary_path:?}");
    let bytes = std::fs::read(&binary_path).unwrap();
    assert_eq!(bytes, FAKE_BIN);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&binary_path).unwrap().permissions().mode();
        assert!(mode & 0o111 != 0, "binary not executable, mode={mode:o}");
    }

    // Manifest must be persisted as a flat sibling at
    // <plugins_dir>/<repository>.json with name = repository and
    // source = the github URL the manifest came from.
    let manifest_path = base.join("plugins").join("repo.json");
    assert!(manifest_path.exists(), "manifest missing at {manifest_path:?}");
    let manifest_bytes = std::fs::read(&manifest_path).unwrap();
    let bundle: ManifestWithNameAndSource = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(bundle.name, "repo");
    assert_eq!(bundle.source, format!("{}/owner/repo/HEAD/objectiveai.json", server.uri()));
    // The manifest claimed `owner: "claimed-owner"`; the installer
    // overrides it with the actual GitHub URL owner ("owner" here).
    assert_eq!(bundle.manifest.owner, "owner");

    cleanup(&base);
}

#[tokio::test]
async fn install_returns_false_when_platform_not_in_binaries() {
    let base = temp_base();
    let server = MockServer::start().await;

    let manifest_body = json!({
        "description": "test plugin",
        "version": "1.0.0",
        "owner": "claimed-owner"
        // no binaries field → empty map → current platform absent
    });

    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .mount(&server)
        .await;

    let client = client_for(&base);
    let result = client
        .install_plugin_at(&server.uri(), &server.uri(), "owner", "repo", None, None, false)
        .await;

    assert!(matches!(result, Ok(false)), "got {result:?}");
    // No binary was fetched → no plugin dir should have been created.
    assert!(
        !base.join("plugins").join("repo").exists(),
        "plugin dir should not exist when install returned false"
    );
    // No manifest persistence either when install short-circuits.
    assert!(
        !base.join("plugins").join("repo.json").exists(),
        "manifest sibling should not exist when install returned false"
    );

    cleanup(&base);
}

#[tokio::test]
async fn install_uses_commit_sha_when_provided() {
    let base = temp_base();
    let server = MockServer::start().await;
    let platform_key = current_platform_key();

    let manifest_body = json!({
        "description": "test plugin",
        "version": "1.0.0",
        "owner": "claimed-owner",
        "binaries": { platform_key: "asset-bin" }
    });

    Mock::given(method("GET"))
        .and(path("/owner/repo/abc123/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/owner/repo/releases/download/v1.0.0/asset-bin"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(FAKE_BIN.to_vec()))
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
        .install_plugin_at(&server.uri(), &server.uri(), "owner", "repo", None, None, false)
        .await;

    match result {
        Err(super::super::Error::Install(InstallError::ManifestBadStatus { code, .. })) => {
            assert_eq!(code.as_u16(), 404);
        }
        other => panic!("expected ManifestBadStatus(404), got {other:?}"),
    }

    cleanup(&base);
}

#[tokio::test]
async fn install_binary_404_returns_binary_bad_status_error() {
    let base = temp_base();
    let server = MockServer::start().await;
    let platform_key = current_platform_key();

    let manifest_body = json!({
        "description": "test plugin",
        "version": "1.0.0",
        "owner": "claimed-owner",
        "binaries": { platform_key: "asset-bin" }
    });

    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/owner/repo/releases/download/v1.0.0/asset-bin"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = client_for(&base);
    let result = client
        .install_plugin_at(&server.uri(), &server.uri(), "owner", "repo", None, None, false)
        .await;

    match result {
        Err(super::super::Error::Install(InstallError::BinaryBadStatus { code, .. })) => {
            assert_eq!(code.as_u16(), 404);
        }
        other => panic!("expected BinaryBadStatus(404), got {other:?}"),
    }

    cleanup(&base);
}

#[tokio::test]
async fn install_malformed_manifest_returns_parse_error() {
    let base = temp_base();
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"not json{{{".to_vec()))
        .mount(&server)
        .await;

    let client = client_for(&base);
    let result = client
        .install_plugin_at(&server.uri(), &server.uri(), "owner", "repo", None, None, false)
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
    let platform_key = current_platform_key();

    let manifest_body = json!({
        "description": "test plugin",
        "version": "1.2.3",
        "owner": "claimed-owner",
        "author": "Wiggidy",
        "binaries": { platform_key: "asset-bin" }
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
    // fetch_plugin_manifest_at does no override — it returns whatever
    // the repo's objectiveai.json literally contains. The install path
    // is where `owner` gets overwritten with the real URL owner.
    assert_eq!(manifest.owner, "claimed-owner");
    assert_eq!(manifest.author.as_deref(), Some("Wiggidy"));
    assert_eq!(manifest.binaries.len(), 1);

    cleanup(&base);
}

#[tokio::test]
async fn install_passes_headers_to_both_requests() {
    let base = temp_base();
    let server = MockServer::start().await;
    let platform_key = current_platform_key();

    let manifest_body = json!({
        "description": "test plugin",
        "version": "1.0.0",
        "owner": "claimed-owner",
        "binaries": { platform_key: "asset-bin" }
    });

    // Both mocks require the header — if the SDK doesn't forward it,
    // wiremock falls through and returns 404 (which would surface as
    // ManifestBadStatus / BinaryBadStatus).
    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .and(header("authorization", "token abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/owner/repo/releases/download/v1.0.0/asset-bin"))
        .and(header("authorization", "token abc"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(FAKE_BIN.to_vec()))
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
    let platform_key = current_platform_key();

    let manifest_body = json!({
        "description": "installed plugin",
        "version": "1.0.0",
        "owner": "claimed-owner",
        "author": "tester",
        "binaries": { platform_key: "asset-bin" }
    });

    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/owner/repo/releases/download/v1.0.0/asset-bin"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(FAKE_BIN.to_vec()))
        .mount(&server)
        .await;

    let client = client_for(&base);
    let ok = client
        .install_plugin_at(&server.uri(), &server.uri(), "owner", "repo", None, None, false)
        .await
        .unwrap();
    assert!(ok);

    let plugins = client.list_plugins(0, 100).await;
    assert_eq!(plugins.len(), 1, "expected one installed plugin, got {plugins:?}");
    let p = &plugins[0];
    assert_eq!(p.name, "repo");
    assert_eq!(p.manifest.description, "installed plugin");
    assert_eq!(p.manifest.version, "1.0.0");
    assert_eq!(p.manifest.author.as_deref(), Some("tester"));
    let expected_source = format!("{}/owner/repo/HEAD/objectiveai.json", server.uri());
    assert_eq!(p.source, expected_source);

    cleanup(&base);
}

#[tokio::test]
async fn install_then_get_plugin_returns_persisted_manifest() {
    let base = temp_base();
    let server = MockServer::start().await;
    let platform_key = current_platform_key();

    let manifest_body = json!({
        "description": "installed plugin",
        "version": "1.0.0",
        "owner": "claimed-owner",
        "binaries": { platform_key: "asset-bin" }
    });

    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/owner/repo/releases/download/v1.0.0/asset-bin"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(FAKE_BIN.to_vec()))
        .mount(&server)
        .await;

    let client = client_for(&base);
    let ok = client
        .install_plugin_at(&server.uri(), &server.uri(), "owner", "repo", None, None, false)
        .await
        .unwrap();
    assert!(ok);

    let got = client.get_plugin("repo").await.expect("expected Some(_)");
    assert_eq!(got.name, "repo");
    assert_eq!(got.manifest.version, "1.0.0");
    assert_eq!(got.source, format!("{}/owner/repo/HEAD/objectiveai.json", server.uri()));

    cleanup(&base);
}

/// Build a minimal in-memory zip containing one `index.html` file.
fn build_viewer_zip(html_contents: &str) -> Vec<u8> {
    use std::io::Write;
    let mut buf = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        writer.start_file("index.html", options).unwrap();
        writer.write_all(html_contents.as_bytes()).unwrap();
        writer.finish().unwrap();
    }
    buf
}

#[tokio::test]
async fn install_extracts_viewer_zip_when_present() {
    let base = temp_base();
    let server = MockServer::start().await;
    let platform_key = current_platform_key();

    let manifest_body = json!({
        "description": "viewer plugin",
        "version": "1.0.0",
        "owner": "claimed-owner",
        "binaries": { platform_key: "asset-bin" },
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
        .and(path("/owner/repo/releases/download/v1.0.0/asset-bin"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(FAKE_BIN.to_vec()))
        .mount(&server)
        .await;

    let zip_bytes = build_viewer_zip("<!doctype html><title>hi</title>");

    Mock::given(method("GET"))
        .and(path("/owner/repo/releases/download/v1.0.0/v.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(zip_bytes))
        .mount(&server)
        .await;

    let client = client_for(&base);
    let ok = client
        .install_plugin_at(&server.uri(), &server.uri(), "owner", "repo", None, None, false)
        .await
        .unwrap();
    assert!(ok);

    let viewer_index = base.join("plugins").join("repo").join("viewer").join("index.html");
    assert!(viewer_index.exists(), "viewer/index.html missing at {viewer_index:?}");
    let contents = std::fs::read_to_string(&viewer_index).unwrap();
    assert!(contents.contains("hi"), "unexpected viewer index: {contents:?}");

    // Manifest persistence still records the viewer fields.
    let persisted: ManifestWithNameAndSource =
        serde_json::from_slice(&std::fs::read(base.join("plugins").join("repo.json")).unwrap()).unwrap();
    assert_eq!(persisted.manifest.viewer_zip.as_deref(), Some("v.zip"));
    assert_eq!(persisted.manifest.viewer_routes.len(), 1);

    cleanup(&base);
}

#[tokio::test]
async fn install_skips_viewer_zip_when_absent() {
    let base = temp_base();
    let server = MockServer::start().await;
    let platform_key = current_platform_key();

    let manifest_body = json!({
        "description": "no-viewer plugin",
        "version": "1.0.0",
        "owner": "claimed-owner",
        "binaries": { platform_key: "asset-bin" }
    });

    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/owner/repo/releases/download/v1.0.0/asset-bin"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(FAKE_BIN.to_vec()))
        .mount(&server)
        .await;

    let client = client_for(&base);
    let ok = client
        .install_plugin_at(&server.uri(), &server.uri(), "owner", "repo", None, None, false)
        .await
        .unwrap();
    assert!(ok);

    let viewer_dir = base.join("plugins").join("repo").join("viewer");
    assert!(!viewer_dir.exists(), "viewer dir should not exist for plugin without viewer_zip");

    cleanup(&base);
}

#[tokio::test]
async fn install_skips_viewer_zip_download_when_viewer_url_set() {
    // When the manifest declares `viewer_url` (and no `viewer_zip`),
    // the install path should download the binary, persist the
    // manifest with the URL, and NOT create `<plugin_dir>/viewer/`.
    // No viewer-zip mock is registered — if install tries to fetch
    // one, wiremock 404s and the test fails.
    let base = temp_base();
    let server = MockServer::start().await;
    let platform_key = current_platform_key();

    let manifest_body = json!({
        "description": "url-viewer plugin",
        "version": "1.0.0",
        "owner": "claimed-owner",
        "binaries": { platform_key: "asset-bin" },
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

    Mock::given(method("GET"))
        .and(path("/owner/repo/releases/download/v1.0.0/asset-bin"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(FAKE_BIN.to_vec()))
        .mount(&server)
        .await;

    let client = client_for(&base);
    let ok = client
        .install_plugin_at(&server.uri(), &server.uri(), "owner", "repo", None, None, false)
        .await
        .unwrap();
    assert!(ok);

    let viewer_dir = base.join("plugins").join("repo").join("viewer");
    assert!(
        !viewer_dir.exists(),
        "viewer dir should not exist for viewer_url plugin (no zip to extract)"
    );

    let persisted: ManifestWithNameAndSource =
        serde_json::from_slice(&std::fs::read(base.join("plugins").join("repo.json")).unwrap()).unwrap();
    assert_eq!(
        persisted.manifest.viewer_url.as_deref(),
        Some("https://plugin.example.com/index.html")
    );
    assert!(persisted.manifest.viewer_zip.is_none());
    assert_eq!(persisted.manifest.viewer_routes.len(), 1);
    assert!(persisted.manifest.has_viewer());

    cleanup(&base);
}

#[tokio::test]
async fn install_rejects_manifest_with_both_viewer_sources() {
    let base = temp_base();
    let server = MockServer::start().await;
    let platform_key = current_platform_key();

    let manifest_body = json!({
        "description": "broken plugin",
        "version": "1.0.0",
        "owner": "claimed-owner",
        "binaries": { platform_key: "asset-bin" },
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
        .install_plugin_at(&server.uri(), &server.uri(), "owner", "repo", None, None, false)
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
    let platform_key = current_platform_key();

    let manifest_body = json!({
        "description": "broken viewer plugin",
        "version": "1.0.0",
        "owner": "claimed-owner",
        "binaries": { platform_key: "asset-bin" },
        "viewer_zip": "missing.zip"
    });

    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/owner/repo/releases/download/v1.0.0/asset-bin"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(FAKE_BIN.to_vec()))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/owner/repo/releases/download/v1.0.0/missing.zip"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = client_for(&base);
    let result = client
        .install_plugin_at(&server.uri(), &server.uri(), "owner", "repo", None, None, false)
        .await;

    match result {
        Err(super::super::Error::Install(InstallError::ViewerZipBadStatus { code, .. })) => {
            assert_eq!(code.as_u16(), 404);
        }
        other => panic!("expected ViewerZipBadStatus(404), got {other:?}"),
    }

    cleanup(&base);
}

// ============================================================
// --upgrade flag behaviour
// ============================================================

/// Builds two mock endpoints (manifest + binary) that any install
/// call hits. Returns (server, full manifest body) so the test can
/// drive multiple installs against the same mock.
async fn upgrade_test_server(
    version: &str,
    binary_body: &'static [u8],
) -> (MockServer, serde_json::Value) {
    let server = MockServer::start().await;
    let platform_key = current_platform_key();
    let manifest_body = json!({
        "description": "upgrade test plugin",
        "version": version,
        "owner": "claimed-owner",
        "binaries": { platform_key: "asset-bin" }
    });
    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body.clone()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/owner/repo/releases/download/v{version}/asset-bin")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(binary_body.to_vec()))
        .mount(&server)
        .await;
    (server, manifest_body)
}

#[tokio::test]
async fn install_refuses_when_manifest_exists_and_not_upgrade() {
    let base = temp_base();
    let (server, _) = upgrade_test_server("1.0.0", FAKE_BIN).await;
    let client = client_for(&base);

    // First install succeeds.
    client
        .install_plugin_at(&server.uri(), &server.uri(), "owner", "repo", None, None, false)
        .await
        .unwrap();

    // Second install with upgrade=false errors.
    let result = client
        .install_plugin_at(&server.uri(), &server.uri(), "owner", "repo", None, None, false)
        .await;
    match result {
        Err(super::super::Error::Install(InstallError::AlreadyInstalled { repository })) => {
            assert_eq!(repository, "repo");
        }
        other => panic!("expected AlreadyInstalled, got {other:?}"),
    }

    // Disk state should match the first install (untouched by the refused second one).
    let bin = std::fs::read(base.join("plugins").join("repo").join(binary_filename())).unwrap();
    assert_eq!(bin, FAKE_BIN);
    assert!(base.join("plugins").join("repo.json").exists());

    cleanup(&base);
}

#[tokio::test]
async fn install_upgrade_replaces_prior_artifacts() {
    let base = temp_base();
    let client = client_for(&base);

    // Install A at version 1.0.0.
    let (server_a, _) = upgrade_test_server("1.0.0", b"VERSION_A_BIN").await;
    client
        .install_plugin_at(&server_a.uri(), &server_a.uri(), "owner", "repo", None, None, false)
        .await
        .unwrap();

    // Install B at version 2.0.0 with --upgrade.
    let (server_b, _) = upgrade_test_server("2.0.0", b"VERSION_B_BIN").await;
    client
        .install_plugin_at(&server_b.uri(), &server_b.uri(), "owner", "repo", None, None, true)
        .await
        .unwrap();

    // Binary on disk now has B's body; manifest carries B's version.
    let bin = std::fs::read(base.join("plugins").join("repo").join(binary_filename())).unwrap();
    assert_eq!(bin, b"VERSION_B_BIN");
    let persisted: ManifestWithNameAndSource =
        serde_json::from_slice(&std::fs::read(base.join("plugins").join("repo.json")).unwrap())
            .unwrap();
    assert_eq!(persisted.manifest.version, "2.0.0");

    cleanup(&base);
}

#[tokio::test]
async fn install_upgrade_preserves_extra_data_under_plugin_dir() {
    let base = temp_base();
    let client = client_for(&base);

    // Install A.
    let (server_a, _) = upgrade_test_server("1.0.0", FAKE_BIN).await;
    client
        .install_plugin_at(&server_a.uri(), &server_a.uri(), "owner", "repo", None, None, false)
        .await
        .unwrap();

    // Drop a "user-state" file under the plugin's dir.
    let user_state = base.join("plugins").join("repo").join("user-state.json");
    std::fs::write(&user_state, b"{\"runs\":42}").unwrap();

    // Upgrade.
    let (server_b, _) = upgrade_test_server("2.0.0", b"V2").await;
    client
        .install_plugin_at(&server_b.uri(), &server_b.uri(), "owner", "repo", None, None, true)
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
    let (server, _) = upgrade_test_server("1.0.0", FAKE_BIN).await;
    let client = client_for(&base);

    // upgrade=true on a fresh base — should still work, no prior state to delete.
    let ok = client
        .install_plugin_at(&server.uri(), &server.uri(), "owner", "repo", None, None, true)
        .await
        .unwrap();
    assert!(ok);
    assert!(base.join("plugins").join("repo").join(binary_filename()).exists());
    assert!(base.join("plugins").join("repo.json").exists());

    cleanup(&base);
}

#[tokio::test]
async fn install_network_failure_leaves_disk_untouched_on_fresh() {
    let base = temp_base();
    let server = MockServer::start().await;
    let platform_key = current_platform_key();
    let manifest_body = json!({
        "description": "broken plugin",
        "version": "1.0.0",
        "owner": "claimed-owner",
        "binaries": { platform_key: "asset-bin" }
    });

    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/owner/repo/releases/download/v1.0.0/asset-bin"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let client = client_for(&base);
    let result = client
        .install_plugin_at(&server.uri(), &server.uri(), "owner", "repo", None, None, false)
        .await;
    assert!(matches!(
        result,
        Err(super::super::Error::Install(InstallError::BinaryBadStatus { .. }))
    ));

    // Nothing on disk — the failure happened in the network phase before any writes.
    assert!(!base.join("plugins").join("repo").exists());
    assert!(!base.join("plugins").join("repo.json").exists());

    cleanup(&base);
}

#[tokio::test]
async fn install_upgrade_network_failure_leaves_disk_after_cleanup() {
    let base = temp_base();
    let client = client_for(&base);

    // Install A successfully.
    let (server_a, _) = upgrade_test_server("1.0.0", b"V1").await;
    client
        .install_plugin_at(&server_a.uri(), &server_a.uri(), "owner", "repo", None, None, false)
        .await
        .unwrap();

    // Attempt upgrade with a binary endpoint that 500s.
    let server_b = MockServer::start().await;
    let platform_key = current_platform_key();
    let manifest_body = json!({
        "description": "broken upgrade",
        "version": "2.0.0",
        "owner": "claimed-owner",
        "binaries": { platform_key: "asset-bin" }
    });
    Mock::given(method("GET"))
        .and(path("/owner/repo/HEAD/objectiveai.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_body))
        .mount(&server_b)
        .await;
    Mock::given(method("GET"))
        .and(path("/owner/repo/releases/download/v2.0.0/asset-bin"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server_b)
        .await;

    let result = client
        .install_plugin_at(&server_b.uri(), &server_b.uri(), "owner", "repo", None, None, true)
        .await;
    assert!(matches!(
        result,
        Err(super::super::Error::Install(InstallError::BinaryBadStatus { .. }))
    ));

    // Documented trade-off: upgrade deleted A's artifacts (step 3) BEFORE
    // the network fetch failed (step 4). The user must retry install.
    assert!(!base.join("plugins").join("repo").join(binary_filename()).exists());
    assert!(!base.join("plugins").join("repo.json").exists());

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
        Err(super::super::Error::Install(InstallError::ReservedRepositoryName { repository })) => {
            assert_eq!(repository, "objectiveai");
        }
        other => panic!("expected ReservedRepositoryName, got {other:?}"),
    }
    assert!(!base.join("plugins").join("objectiveai").exists());
    assert!(!base.join("plugins").join("objectiveai.json").exists());
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
                Err(super::super::Error::Install(InstallError::ReservedRepositoryName { .. }))
            ),
            "expected ReservedRepositoryName for {candidate:?}",
        );
    }
    cleanup(&base);
}
