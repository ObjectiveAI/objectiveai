use super::super::Client;
use super::{Binaries, Manifest};

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
    Client::new(Some(base.to_path_buf()), None::<&str>, None::<&str>)
}

fn minimal_manifest_json() -> String {
    serde_json::to_string(&Manifest {
        description: "tiny test plugin".to_string(),
        version: "0.1.0".to_string(),
        owner: "wiggidy".to_string(),
        author: None,
        homepage: None,
        license: None,
        binaries: Binaries::default(),
        viewer_zip: None,
        viewer_url: None,
        viewer_routes: vec![],
        mobile_ready: false,
        mcp_servers: Vec::new(),
    })
    .unwrap()
}

/// Write a bare `objectiveai.json` manifest at
/// `<base>/plugins/<owner>/<name>/<version>/objectiveai.json`, creating
/// the version dir. Returns the manifest path.
fn write_manifest(
    base: &std::path::Path,
    owner: &str,
    name: &str,
    version: &str,
) -> std::path::PathBuf {
    let dir = base
        .join("plugins")
        .join(owner)
        .join(name)
        .join(version);
    std::fs::create_dir_all(&dir).unwrap();
    let manifest_path = dir.join("objectiveai.json");
    std::fs::write(&manifest_path, minimal_manifest_json()).unwrap();
    manifest_path
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
    std::fs::create_dir_all(base.join("plugins")).unwrap();
    let client = client_for(&base);
    let plugins = client.list_plugins(0, 100).await;
    assert!(plugins.is_empty(), "expected empty Vec, got {plugins:?}");
    cleanup(&base);
}

#[tokio::test]
async fn list_plugins_parses_valid_manifest() {
    let base = fresh_base_dir();
    let manifest_path = write_manifest(&base, "wiggidy", "psyops", "0.1.0");

    let client = client_for(&base);
    let plugins = client.list_plugins(0, 100).await;

    assert_eq!(plugins.len(), 1);
    let p = &plugins[0];
    assert_eq!(p.name, "psyops");
    assert_eq!(p.manifest.description, "tiny test plugin");
    assert_eq!(p.manifest.version, "0.1.0");
    assert_eq!(p.source, manifest_path.to_string_lossy());

    cleanup(&base);
}

#[tokio::test]
async fn list_plugins_skips_invalid_files() {
    let base = fresh_base_dir();
    // Valid manifest at owner/a/1.0.0.
    write_manifest(&base, "wiggidy", "a", "1.0.0");
    // Invalid: not an object.
    let b_dir = base.join("plugins").join("wiggidy").join("b").join("1.0.0");
    std::fs::create_dir_all(&b_dir).unwrap();
    std::fs::write(b_dir.join("objectiveai.json"), "\"not json\"").unwrap();
    // Invalid: missing required fields.
    let c_dir = base.join("plugins").join("wiggidy").join("c").join("1.0.0");
    std::fs::create_dir_all(&c_dir).unwrap();
    std::fs::write(c_dir.join("objectiveai.json"), r#"{"version":"1.0.0"}"#)
        .unwrap();

    let client = client_for(&base);
    let plugins = client.list_plugins(0, 100).await;

    assert_eq!(plugins.len(), 1, "got {plugins:?}");
    assert_eq!(plugins[0].name, "a");

    cleanup(&base);
}

#[tokio::test]
async fn list_plugins_handles_multiple_valid_manifests() {
    let base = fresh_base_dir();
    for name in ["a", "b", "c"] {
        write_manifest(&base, "wiggidy", name, "0.1.0");
    }

    let client = client_for(&base);
    let mut plugins = client.list_plugins(0, 100).await;
    plugins.sort_by(|x, y| x.name.cmp(&y.name));

    let names: Vec<&str> = plugins.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["a", "b", "c"]);

    cleanup(&base);
}

#[tokio::test]
async fn get_plugin_returns_some_when_manifest_exists() {
    let base = fresh_base_dir();
    let manifest_path = write_manifest(&base, "wiggidy", "psyops", "0.1.0");

    let client = client_for(&base);
    let plugin = client.get_plugin("wiggidy", "psyops", "0.1.0").await;

    let p = plugin.expect("expected Some(_)");
    assert_eq!(p.name, "psyops");
    assert_eq!(p.manifest.description, "tiny test plugin");
    assert_eq!(p.manifest.version, "0.1.0");
    assert_eq!(p.source, manifest_path.to_string_lossy());

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
    write_manifest(&base, "wiggidy", "other", "0.1.0");

    let client = client_for(&base);
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
    let dir = base.join("plugins").join("wiggidy").join("bad").join("0.1.0");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("objectiveai.json"), "\"not json\"").unwrap();

    let client = client_for(&base);
    assert!(
        client.get_plugin("wiggidy", "bad", "0.1.0").await.is_none()
    );
    cleanup(&base);
}

#[test]
fn plugin_binary_path_layout() {
    let base = fresh_base_dir();
    let client = client_for(&base);
    let expected = client
        .plugins_dir()
        .join("acme")
        .join("my-plugin")
        .join("1.0.0")
        .join(if cfg!(windows) {
            "plugin.exe"
        } else {
            "plugin"
        });
    assert_eq!(
        client.plugin_binary_path("acme", "my-plugin", "1.0.0"),
        expected
    );
    assert_eq!(
        client.plugin_dir("acme", "my-plugin", "1.0.0"),
        client.plugins_dir().join("acme").join("my-plugin").join("1.0.0")
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
async fn resolve_plugin_returns_canonical_nested_path_when_present() {
    let base = fresh_base_dir();
    let client = client_for(&base);
    let target = client.plugin_binary_path("acme", "hello", "1.0.0");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(
        &target,
        b"\x7fELF or MZ; contents don't matter for is_file()",
    )
    .unwrap();

    let resolved = client.resolve_plugin("acme", "hello", "1.0.0").await;
    assert_eq!(resolved.as_deref(), Some(target.as_path()));
    cleanup(&base);
}

#[tokio::test]
async fn resolve_plugin_falls_back_to_cross_platform_canonical() {
    // Tier 2: only the non-preferred canonical filename is present
    // (`plugin` on Windows, `plugin.exe` elsewhere). Resolve still
    // returns it.
    let base = fresh_base_dir();
    let client = client_for(&base);
    let dir = client.plugin_dir("acme", "hello", "1.0.0");
    std::fs::create_dir_all(&dir).unwrap();
    let alt = dir.join(if cfg!(windows) {
        "plugin"
    } else {
        "plugin.exe"
    });
    std::fs::write(&alt, b"x").unwrap();

    let resolved = client.resolve_plugin("acme", "hello", "1.0.0").await;
    assert_eq!(resolved.as_deref(), Some(alt.as_path()));
    cleanup(&base);
}

#[tokio::test]
async fn resolve_plugin_falls_back_to_arbitrary_extension() {
    // Tier 3: only `plugin.<some-ext>` exists. Resolve returns it.
    let base = fresh_base_dir();
    let client = client_for(&base);
    let dir = client.plugin_dir("acme", "hello", "1.0.0");
    std::fs::create_dir_all(&dir).unwrap();
    let bat = dir.join("plugin.bat");
    std::fs::write(&bat, b"@echo off\n").unwrap();

    let resolved = client.resolve_plugin("acme", "hello", "1.0.0").await;
    assert_eq!(resolved.as_deref(), Some(bat.as_path()));
    cleanup(&base);
}

#[tokio::test]
async fn resolve_plugin_prefers_canonical_over_alternate_extension() {
    // Tier 1 wins over tier 3: both `plugin[.exe]` and `plugin.bat`
    // exist. The platform-preferred canonical resolves.
    let base = fresh_base_dir();
    let client = client_for(&base);
    let canonical = client.plugin_binary_path("acme", "hello", "1.0.0");
    std::fs::create_dir_all(canonical.parent().unwrap()).unwrap();
    std::fs::write(&canonical, b"canonical").unwrap();
    std::fs::write(canonical.parent().unwrap().join("plugin.bat"), b"alt")
        .unwrap();

    let resolved = client.resolve_plugin("acme", "hello", "1.0.0").await;
    assert_eq!(resolved.as_deref(), Some(canonical.as_path()));
    cleanup(&base);
}

#[tokio::test]
async fn resolve_plugin_ignores_unrelated_stems() {
    // Files whose stem isn't exactly "plugin" don't satisfy tier 3.
    // `plugin.tar.gz` has stem `plugin.tar` (Path::file_stem strips
    // only the last extension), so it's rejected.
    let base = fresh_base_dir();
    let client = client_for(&base);
    let dir = client.plugin_dir("acme", "hello", "1.0.0");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("other.exe"), b"x").unwrap();
    std::fs::write(dir.join("plugin.tar.gz"), b"x").unwrap();

    let resolved = client.resolve_plugin("acme", "hello", "1.0.0").await;
    assert!(resolved.is_none(), "got {resolved:?}");
    cleanup(&base);
}

#[tokio::test]
async fn resolve_plugin_ignores_flat_layout() {
    // Pre-fix bug behaviour: a file at <plugins_dir>/<name>[.exe] used
    // to resolve. After the fix, only the nested coordinate layout is
    // honoured.
    let base = fresh_base_dir();
    let plugins_dir = base.join("plugins");
    std::fs::create_dir_all(&plugins_dir).unwrap();
    let flat =
        plugins_dir.join(if cfg!(windows) { "hello.exe" } else { "hello" });
    std::fs::write(&flat, b"x").unwrap();

    let client = client_for(&base);
    assert!(
        client
            .resolve_plugin("acme", "hello", "1.0.0")
            .await
            .is_none()
    );
    cleanup(&base);
}

#[tokio::test]
async fn list_plugins_respects_offset_and_limit() {
    let base = fresh_base_dir();
    for name in ["a", "b", "c"] {
        write_manifest(&base, "wiggidy", name, "0.1.0");
    }

    let client = client_for(&base);

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
