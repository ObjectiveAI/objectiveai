use super::super::Client;
use super::Manifest;
use crate::filesystem::tools::{CliZip, CliZipArch, Exec};

fn fresh_base_dir() -> std::path::PathBuf {
    let d = std::env::temp_dir()
        .join(format!("oai-list-plugins-{}", uuid::Uuid::new_v4()));
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

/// A minimal valid manifest for the given coordinate. The manifest now
/// declares its own `owner` / `name` / `version`, so they must match
/// the directory the manifest is written under.
fn minimal_manifest(owner: &str, name: &str, version: &str) -> Manifest {
    Manifest {
        owner: owner.to_string(),
        name: name.to_string(),
        version: version.to_string(),
        description: "tiny test plugin".to_string(),
        exec: Exec::default(),
        cli_zip: CliZip::default(),
        viewer_zip: None,
        viewer_url: None,
        viewer_routes: vec![],
        mcp_servers: Vec::new(),
    }
}

/// A manifest that declares a distinct exec vector per OS, so the
/// resolve tests can assert the current platform's vector is picked.
fn exec_manifest(owner: &str, name: &str, version: &str) -> Manifest {
    Manifest {
        owner: owner.to_string(),
        name: name.to_string(),
        version: version.to_string(),
        description: "exec test plugin".to_string(),
        exec: Exec {
            windows: vec!["./plugin.exe".to_string(), "--windows".to_string()],
            linux: vec!["./plugin".to_string(), "--linux".to_string()],
            macos: vec!["./plugin".to_string(), "--macos".to_string()],
        },
        cli_zip: CliZip {
            windows: CliZipArch {
                x86_64: Some("cli-win.zip".to_string()),
                ..Default::default()
            },
            linux: CliZipArch {
                x86_64: Some("cli-linux.zip".to_string()),
                ..Default::default()
            },
            macos: CliZipArch {
                x86_64: Some("cli-mac.zip".to_string()),
                ..Default::default()
            },
        },
        viewer_zip: None,
        viewer_url: None,
        viewer_routes: vec![],
        mcp_servers: Vec::new(),
    }
}

/// The exec vector `exec_manifest` declares for the platform the test is
/// running on.
fn current_platform_exec() -> Vec<String> {
    if cfg!(target_os = "windows") {
        vec!["./plugin.exe".to_string(), "--windows".to_string()]
    } else if cfg!(target_os = "macos") {
        vec!["./plugin".to_string(), "--macos".to_string()]
    } else {
        vec!["./plugin".to_string(), "--linux".to_string()]
    }
}

/// Write an `objectiveai.json` with the given contents at
/// `<plugins_dir>/<owner>/<name>/<version>/objectiveai.json`, creating
/// the version dir. Returns the manifest path.
fn write_manifest_contents(
    client: &Client,
    owner: &str,
    name: &str,
    version: &str,
    contents: &str,
) -> std::path::PathBuf {
    let dir = client.plugin_dir(owner, name, version);
    std::fs::create_dir_all(&dir).unwrap();
    let manifest_path = dir.join("objectiveai.json");
    std::fs::write(&manifest_path, contents).unwrap();
    manifest_path
}

/// Write a bare minimal manifest (empty exec, no viewer) for the given
/// coordinate. Returns the manifest path.
fn write_manifest(
    client: &Client,
    owner: &str,
    name: &str,
    version: &str,
) -> std::path::PathBuf {
    let json =
        serde_json::to_string(&minimal_manifest(owner, name, version)).unwrap();
    write_manifest_contents(client, owner, name, version, &json)
}

#[tokio::test]
async fn list_plugins_returns_empty_when_dir_missing() {
    let base = fresh_base_dir();
    let client = client_for(&base);
    let plugins = client.list_plugins(0, 100).await;
    assert!(plugins.is_empty(), "expected empty Vec, got {plugins:?}");
    cleanup(&base);
}

#[tokio::test]
async fn list_plugins_returns_empty_when_dir_empty() {
    let base = fresh_base_dir();
    let client = client_for(&base);
    std::fs::create_dir_all(client.plugins_dir()).unwrap();
    let plugins = client.list_plugins(0, 100).await;
    assert!(plugins.is_empty(), "expected empty Vec, got {plugins:?}");
    cleanup(&base);
}

#[tokio::test]
async fn list_plugins_parses_valid_manifest() {
    let base = fresh_base_dir();
    let client = client_for(&base);
    write_manifest(&client, "wiggidy", "psyops", "0.1.0");

    let plugins = client.list_plugins(0, 100).await;

    assert_eq!(plugins.len(), 1);
    let p = &plugins[0];
    assert_eq!(p.owner, "wiggidy");
    assert_eq!(p.name, "psyops");
    assert_eq!(p.description, "tiny test plugin");
    assert_eq!(p.version, "0.1.0");

    cleanup(&base);
}

#[tokio::test]
async fn list_plugins_skips_invalid_files() {
    let base = fresh_base_dir();
    let client = client_for(&base);
    // Valid manifest at owner/a/1.0.0.
    write_manifest(&client, "wiggidy", "a", "1.0.0");
    // Invalid: not an object.
    write_manifest_contents(&client, "wiggidy", "b", "1.0.0", "\"not json\"");
    // Invalid: missing required fields.
    write_manifest_contents(
        &client,
        "wiggidy",
        "c",
        "1.0.0",
        r#"{"version":"1.0.0"}"#,
    );
    // Invalid: parses, but fails `validate()` (both viewer sources).
    write_manifest_contents(
        &client,
        "wiggidy",
        "d",
        "1.0.0",
        &serde_json::json!({
            "owner": "wiggidy",
            "name": "d",
            "version": "1.0.0",
            "description": "x",
            "exec": { "windows": [], "linux": [], "macos": [] },
            "cli_zip": {},
            "viewer_zip": "v.zip",
            "viewer_url": "https://x.example.com"
        })
        .to_string(),
    );

    let plugins = client.list_plugins(0, 100).await;

    assert_eq!(plugins.len(), 1, "got {plugins:?}");
    assert_eq!(plugins[0].name, "a");

    cleanup(&base);
}

#[tokio::test]
async fn list_plugins_handles_multiple_valid_manifests() {
    let base = fresh_base_dir();
    let client = client_for(&base);
    for name in ["a", "b", "c"] {
        write_manifest(&client, "wiggidy", name, "0.1.0");
    }

    let mut plugins = client.list_plugins(0, 100).await;
    plugins.sort_by(|x, y| x.name.cmp(&y.name));

    let names: Vec<&str> = plugins.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["a", "b", "c"]);

    cleanup(&base);
}

#[tokio::test]
async fn get_plugin_returns_some_when_manifest_exists() {
    let base = fresh_base_dir();
    let client = client_for(&base);
    write_manifest(&client, "wiggidy", "psyops", "0.1.0");

    let plugin = client.get_plugin("wiggidy", "psyops", "0.1.0").await;

    let p = plugin.expect("expected Some(_)");
    assert_eq!(p.name, "psyops");
    assert_eq!(p.description, "tiny test plugin");
    assert_eq!(p.version, "0.1.0");

    cleanup(&base);
}

#[tokio::test]
async fn get_plugin_returns_none_when_dir_missing() {
    let base = fresh_base_dir();
    let client = client_for(&base);
    assert!(
        client
            .get_plugin("wiggidy", "psyops", "0.1.0")
            .await
            .is_none()
    );
    cleanup(&base);
}

#[tokio::test]
async fn get_plugin_returns_none_when_file_missing() {
    let base = fresh_base_dir();
    let client = client_for(&base);
    write_manifest(&client, "wiggidy", "other", "0.1.0");

    assert!(
        client
            .get_plugin("wiggidy", "missing", "0.1.0")
            .await
            .is_none()
    );
    cleanup(&base);
}

#[tokio::test]
async fn get_plugin_returns_none_when_malformed() {
    let base = fresh_base_dir();
    let client = client_for(&base);
    write_manifest_contents(&client, "wiggidy", "bad", "0.1.0", "\"not json\"");

    assert!(client.get_plugin("wiggidy", "bad", "0.1.0").await.is_none());
    cleanup(&base);
}

#[test]
fn plugin_cli_dir_layout() {
    let base = fresh_base_dir();
    let client = client_for(&base);
    assert_eq!(
        client.plugins_dir(),
        client.bin_dir().join("plugins"),
        "plugins are machine-wide, under bin/"
    );
    assert_eq!(
        client.plugin_dir("acme", "my-plugin", "1.0.0"),
        client
            .plugins_dir()
            .join("acme")
            .join("my-plugin")
            .join("1.0.0")
    );
    assert_eq!(
        client.plugin_cli_dir("acme", "my-plugin", "1.0.0"),
        client.plugin_dir("acme", "my-plugin", "1.0.0").join("cli")
    );
    cleanup(&base);
}

#[tokio::test]
async fn resolve_plugin_returns_none_when_missing() {
    let base = fresh_base_dir();
    let client = client_for(&base);
    assert!(
        client
            .resolve_plugin("acme", "nope", "1.0.0")
            .await
            .is_none()
    );
    cleanup(&base);
}

#[tokio::test]
async fn resolve_plugin_returns_platform_exec_and_cli_dir() {
    let base = fresh_base_dir();
    let client = client_for(&base);
    write_manifest_contents(
        &client,
        "acme",
        "hello",
        "1.0.0",
        &serde_json::to_string(&exec_manifest("acme", "hello", "1.0.0")).unwrap(),
    );

    let (exec, cli_dir) = client
        .resolve_plugin("acme", "hello", "1.0.0")
        .await
        .expect("expected Some(_)");
    assert_eq!(exec, current_platform_exec());
    assert_eq!(cli_dir, client.plugin_cli_dir("acme", "hello", "1.0.0"));
    cleanup(&base);
}

#[tokio::test]
async fn resolve_plugin_returns_none_when_manifest_malformed() {
    let base = fresh_base_dir();
    let client = client_for(&base);
    write_manifest_contents(&client, "acme", "hello", "1.0.0", "\"not json\"");

    assert!(
        client
            .resolve_plugin("acme", "hello", "1.0.0")
            .await
            .is_none()
    );
    cleanup(&base);
}

#[tokio::test]
async fn resolve_plugin_returns_none_when_manifest_invalid() {
    // Parses fine but fails `validate()` (both viewer sources set) —
    // the read boundary rejects it, so resolve sees no plugin.
    let base = fresh_base_dir();
    let client = client_for(&base);
    write_manifest_contents(
        &client,
        "acme",
        "hello",
        "1.0.0",
        &serde_json::json!({
            "owner": "acme",
            "name": "hello",
            "version": "1.0.0",
            "description": "x",
            "exec": { "windows": [], "linux": [], "macos": [] },
            "cli_zip": {},
            "viewer_zip": "v.zip",
            "viewer_url": "https://x.example.com"
        })
        .to_string(),
    );

    assert!(
        client
            .resolve_plugin("acme", "hello", "1.0.0")
            .await
            .is_none()
    );
    cleanup(&base);
}

#[tokio::test]
async fn resolve_plugin_passes_through_empty_exec() {
    // Viewer-only plugins declare no exec. Resolve still returns
    // Some((empty vector, cli dir)) — the caller decides that an
    // empty exec is an error, not the resolver.
    let base = fresh_base_dir();
    let client = client_for(&base);
    write_manifest(&client, "acme", "hello", "1.0.0");

    let (exec, cli_dir) = client
        .resolve_plugin("acme", "hello", "1.0.0")
        .await
        .expect("expected Some(_)");
    assert!(exec.is_empty(), "got {exec:?}");
    assert_eq!(cli_dir, client.plugin_cli_dir("acme", "hello", "1.0.0"));
    cleanup(&base);
}

#[tokio::test]
async fn list_plugins_respects_offset_and_limit() {
    let base = fresh_base_dir();
    let client = client_for(&base);
    for name in ["a", "b", "c"] {
        write_manifest(&client, "wiggidy", name, "0.1.0");
    }

    assert_eq!(
        client.list_plugins(0, 100).await.len(),
        3,
        "unbounded should return all 3"
    );
    assert_eq!(
        client.list_plugins(0, 1).await.len(),
        1,
        "limit=1 should clip to 1"
    );
    assert_eq!(
        client.list_plugins(1, 1).await.len(),
        1,
        "offset=1 limit=1 should return 1"
    );
    assert_eq!(
        client.list_plugins(2, 100).await.len(),
        1,
        "offset=2 should leave 1 item"
    );
    assert_eq!(
        client.list_plugins(5, 100).await.len(),
        0,
        "offset past end is empty"
    );
    assert_eq!(client.list_plugins(0, 0).await.len(), 0, "limit=0 is empty");

    cleanup(&base);
}
