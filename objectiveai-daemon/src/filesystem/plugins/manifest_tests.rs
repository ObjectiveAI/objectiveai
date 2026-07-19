use super::*;
use crate::filesystem::plugins::{CliZip, CliZipArch, Exec};

/// A minimal valid manifest: every required field set, no viewer/mcp
/// extras. Tests below clone this and override the fields they care
/// about.
fn base_manifest() -> Manifest {
    Manifest {
        owner: "wiggidy".to_string(),
        name: "psyops".to_string(),
        version: "0.0.1".to_string(),
        description: "tiny test plugin".to_string(),
        exec: Exec::default(),
        cli_zip: CliZip::default(),
        viewer_zip: None,
        viewer_url: None,
        mcp_servers: Vec::new(),
        daemon: false,
    }
}

fn full_exec() -> Exec {
    Exec {
        windows: vec!["./psyops.exe".to_string(), "--serve".to_string()],
        linux: vec!["./psyops".to_string(), "--serve".to_string()],
        macos: vec!["./psyops".to_string(), "--serve".to_string()],
    }
}

#[test]
fn manifest_minimal_roundtrip() {
    let m = base_manifest();
    let json = serde_json::to_value(&m).unwrap();
    let obj = json.as_object().unwrap();
    // The required fields always serialize: owner, name, version,
    // description, exec, cli_zip. The optional viewer/mcp fields are
    // skipped when empty.
    assert_eq!(obj.len(), 6, "got {obj:?}");
    assert_eq!(obj["owner"], "wiggidy");
    assert_eq!(obj["name"], "psyops");
    assert_eq!(obj["version"], "0.0.1");
    assert_eq!(obj["description"], "tiny test plugin");
    assert!(obj.contains_key("exec"));
    assert!(obj.contains_key("cli_zip"));
    assert!(!obj.contains_key("viewer_zip"));
    assert!(!obj.contains_key("mcp_servers"));
    // Roundtrip back.
    let back: Manifest = serde_json::from_value(json).unwrap();
    assert_eq!(back, m);
}

#[test]
fn manifest_field_order() {
    let m = Manifest {
        exec: full_exec(),
        cli_zip: CliZip {
            windows: CliZipArch {
                x86_64: Some("w.zip".to_string()),
                ..Default::default()
            },
            linux: CliZipArch {
                x86_64: Some("l.zip".to_string()),
                ..Default::default()
            },
            macos: CliZipArch {
                x86_64: Some("m.zip".to_string()),
                ..Default::default()
            },
        },
        ..base_manifest()
    };
    let s = serde_json::to_string(&m).unwrap();
    // Declaration order: owner, name, version, description, exec,
    // cli_zip. The Exec fields serialize windows, linux, macos.
    let i_owner = s.find("\"owner\"").unwrap();
    let i_name = s.find("\"name\"").unwrap();
    let i_version = s.find("\"version\"").unwrap();
    let i_desc = s.find("\"description\"").unwrap();
    let i_exec = s.find("\"exec\"").unwrap();
    let i_cli = s.find("\"cli_zip\"").unwrap();
    let i_w = s.find("\"windows\"").unwrap();
    let i_l = s.find("\"linux\"").unwrap();
    let i_m = s.find("\"macos\"").unwrap();
    assert!(i_owner < i_name, "owner before name: {s}");
    assert!(i_name < i_version, "name before version: {s}");
    assert!(i_version < i_desc, "version before description: {s}");
    assert!(i_desc < i_exec, "description before exec: {s}");
    assert!(i_exec < i_cli, "exec before cli_zip: {s}");
    assert!(i_w < i_l, "windows before linux: {s}");
    assert!(i_l < i_m, "linux before macos: {s}");

    let back: Manifest = serde_json::from_str(&s).unwrap();
    assert_eq!(back, m);
}

#[test]
fn manifest_deserializes_minimal_json() {
    // All required fields must be present (none carry a serde default).
    let json = serde_json::json!({
        "owner": "wiggidy",
        "name": "psyops",
        "version": "0.1.0",
        "description": "x",
        "exec": { "windows": [], "linux": [], "macos": [] },
        "cli_zip": {}
    });
    let m: Manifest = serde_json::from_value(json).unwrap();
    assert_eq!(m.owner, "wiggidy");
    assert_eq!(m.name, "psyops");
    assert_eq!(m.version, "0.1.0");
    assert_eq!(m.description, "x");
    assert!(m.exec.is_empty());
    assert_eq!(m.cli_zip, CliZip::default());
}

#[test]
fn manifest_with_exec_and_cli_zip_roundtrip() {
    let m = Manifest {
        exec: full_exec(),
        cli_zip: CliZip {
            windows: CliZipArch {
                x86_64: Some("psyops-cli-win.zip".to_string()),
                ..Default::default()
            },
            linux: CliZipArch {
                x86_64: Some("psyops-cli-linux.zip".to_string()),
                ..Default::default()
            },
            macos: CliZipArch {
                x86_64: Some("psyops-cli-mac.zip".to_string()),
                ..Default::default()
            },
        },
        ..base_manifest()
    };
    let json = serde_json::to_value(&m).unwrap();
    let back: Manifest = serde_json::from_value(json).unwrap();
    assert!(!back.exec.is_empty());
    assert_eq!(back.exec.windows, vec!["./psyops.exe", "--serve"]);
    assert_eq!(back.exec.linux, vec!["./psyops", "--serve"]);
    assert_eq!(back.exec.macos, vec!["./psyops", "--serve"]);
    assert_eq!(back.cli_zip.windows.x86_64.as_deref(), Some("psyops-cli-win.zip"));
    assert_eq!(back.cli_zip.linux.x86_64.as_deref(), Some("psyops-cli-linux.zip"));
    assert_eq!(back.cli_zip.macos.x86_64.as_deref(), Some("psyops-cli-mac.zip"));
}

#[test]
fn manifest_always_serializes_exec_and_cli_zip() {
    let m = base_manifest();
    let json = serde_json::to_value(&m).unwrap();
    let obj = json.as_object().unwrap();
    assert!(obj.contains_key("exec"), "exec is required, got {obj:?}");
    assert!(obj.contains_key("cli_zip"), "cli_zip is required, got {obj:?}");
    // An empty cli_zip serializes as an empty object (per-OS entries
    // skipped when absent).
    assert_eq!(obj["cli_zip"], serde_json::json!({}));
}

#[test]
fn manifest_requires_exec_and_cli_zip_fields() {
    // Both are required (no serde default): a manifest missing either
    // fails to parse.
    let missing_exec = serde_json::json!({
        "owner": "w", "name": "p", "version": "1.0.0", "description": "x",
        "cli_zip": {}
    });
    assert!(serde_json::from_value::<Manifest>(missing_exec).is_err());
    let missing_cli = serde_json::json!({
        "owner": "w", "name": "p", "version": "1.0.0", "description": "x",
        "exec": { "windows": [], "linux": [], "macos": [] }
    });
    assert!(serde_json::from_value::<Manifest>(missing_cli).is_err());
}

#[test]
fn manifest_deserializes_exec_and_cli_zip_objects() {
    // The canonical wire fixture: per-OS exec and cli_zip objects.
    let json = serde_json::json!({
        "owner": "wiggidy",
        "name": "p",
        "version": "1.0.0",
        "description": "x",
        "exec": {
            "windows": ["./plugin.exe"],
            "linux": ["./plugin"],
            "macos": ["./plugin"]
        },
        "cli_zip": {
            "windows": { "x86_64": "cli-win.zip" },
            "linux": { "x86_64": "cli-linux.zip" },
            "macos": { "x86_64": "cli-mac.zip" }
        }
    });
    let m: Manifest = serde_json::from_value(json).unwrap();
    assert_eq!(m.exec.windows, vec!["./plugin.exe"]);
    assert_eq!(m.exec.linux, vec!["./plugin"]);
    assert_eq!(m.exec.macos, vec!["./plugin"]);
    assert_eq!(m.cli_zip.windows.x86_64.as_deref(), Some("cli-win.zip"));
    assert_eq!(m.cli_zip.linux.x86_64.as_deref(), Some("cli-linux.zip"));
    assert_eq!(m.cli_zip.macos.x86_64.as_deref(), Some("cli-mac.zip"));
}

#[test]
fn manifest_cli_zip_per_os_optional() {
    // The cli_zip field is required, but each per-OS and per-arch entry
    // is optional: a partial cli_zip parses, with the missing
    // platforms/arches as `None`.
    let json = serde_json::json!({
        "owner": "w", "name": "p", "version": "1.0.0", "description": "x",
        "exec": { "windows": [], "linux": [], "macos": [] },
        "cli_zip": { "linux": { "x86_64": "cli-linux.zip" } }
    });
    let m: Manifest = serde_json::from_value(json).unwrap();
    assert!(m.cli_zip.windows.is_empty());
    assert_eq!(m.cli_zip.linux.x86_64.as_deref(), Some("cli-linux.zip"));
    assert!(m.cli_zip.linux.aarch64.is_none());
    assert!(m.cli_zip.macos.is_empty());
}

#[test]
fn manifest_exec_object_requires_all_platform_keys() {
    // `Exec` has no per-field serde defaults: an exec object missing
    // one of the three OS keys is a parse error, not an empty vector.
    let json = serde_json::json!({
        "owner": "w", "name": "p", "version": "1.0.0", "description": "x",
        "exec": { "windows": ["./plugin.exe"], "linux": ["./plugin"] },
        "cli_zip": {}
    });
    let result: Result<Manifest, _> = serde_json::from_value(json);
    assert!(result.is_err(), "got {result:?}");
}

#[test]
fn manifest_with_partial_exec_is_valid() {
    // Plugin that only declares a Linux command. The exec field carries
    // empty vectors for the other platforms.
    let m = Manifest {
        exec: Exec {
            linux: vec!["./psyops".to_string()],
            ..Default::default()
        },
        ..base_manifest()
    };
    let json = serde_json::to_value(&m).unwrap();
    assert_eq!(json["exec"]["windows"], serde_json::json!([]));
    assert_eq!(json["exec"]["linux"], serde_json::json!(["./psyops"]));
    assert_eq!(json["exec"]["macos"], serde_json::json!([]));
    let back: Manifest = serde_json::from_value(json).unwrap();
    assert!(!back.exec.is_empty());
    assert!(back.exec.windows.is_empty());
    assert_eq!(back.exec.linux, vec!["./psyops"]);
    assert!(back.exec.macos.is_empty());
}

#[test]
fn manifest_with_viewer_fields_roundtrip() {
    let m = Manifest {
        description: "viewer plugin".to_string(),
        viewer_zip: Some("psyops-viewer.zip".to_string()),
        ..base_manifest()
    };
    let json = serde_json::to_value(&m).unwrap();
    let back: Manifest = serde_json::from_value(json).unwrap();
    assert_eq!(back.viewer_zip.as_deref(), Some("psyops-viewer.zip"));
}

#[test]
fn manifest_omits_viewer_fields_when_absent() {
    let m = base_manifest();
    let json = serde_json::to_value(&m).unwrap();
    let obj = json.as_object().unwrap();
    assert!(!obj.contains_key("viewer_zip"));
    assert!(!obj.contains_key("viewer_url"));
}

#[test]
fn manifest_deserializes_without_viewer_fields() {
    let json = serde_json::json!({
        "owner": "wiggidy", "name": "p", "version": "1.0.0", "description": "x",
        "exec": { "windows": [], "linux": [], "macos": [] },
        "cli_zip": {}
    });
    let m: Manifest = serde_json::from_value(json).unwrap();
    assert!(m.viewer_zip.is_none());
    assert!(m.viewer_url.is_none());
}

#[test]
fn manifest_ignores_legacy_viewer_routes_key() {
    // `viewer_routes` was removed from the manifest schema; manifests
    // authored against older versions still parse — the key is simply
    // dropped (Manifest doesn't deny unknown fields).
    let json = serde_json::json!({
        "owner": "wiggidy", "name": "p", "version": "1.0.0", "description": "x",
        "exec": { "windows": [], "linux": [], "macos": [] },
        "cli_zip": {},
        "viewer_routes": [
            { "path": "/say", "method": "POST", "type": "say_request" }
        ]
    });
    let m: Manifest = serde_json::from_value(json).unwrap();
    let back = serde_json::to_value(&m).unwrap();
    assert!(!back.as_object().unwrap().contains_key("viewer_routes"));
}

#[test]
fn manifest_with_mcp_servers_roundtrip() {
    let m = Manifest {
        mcp_servers: vec![
            McpServer {
                name: "search".to_string(),
                authorization: true,
            },
            McpServer {
                name: "weather".to_string(),
                authorization: false,
            },
        ],
        ..base_manifest()
    };
    let json = serde_json::to_value(&m).unwrap();
    // No `url` field on the wire — the entry is just name + authorization.
    assert_eq!(json["mcp_servers"][0]["name"], "search");
    assert_eq!(json["mcp_servers"][0]["authorization"], true);
    assert!(json["mcp_servers"][0].get("url").is_none());
    let back: Manifest = serde_json::from_value(json).unwrap();
    assert_eq!(back.mcp_servers.len(), 2);
    assert_eq!(back.mcp_servers[0].name, "search");
    assert!(back.mcp_servers[0].authorization);
    assert_eq!(back.mcp_servers[1].name, "weather");
    assert!(!back.mcp_servers[1].authorization);
}

#[test]
fn has_viewer_returns_true_for_either_source() {
    let mut m = base_manifest();
    assert!(!m.has_viewer());

    m.viewer_zip = Some("v.zip".to_string());
    assert!(m.has_viewer());
    m.viewer_zip = None;

    m.viewer_url = Some("https://x.example.com".to_string());
    assert!(m.has_viewer());
}

#[test]
fn manifest_validate_rejects_both_viewer_sources() {
    let mut m = base_manifest();
    m.viewer_zip = Some("v.zip".to_string());
    m.viewer_url = Some("https://x.example.com".to_string());
    let err = m.validate().unwrap_err();
    assert!(err.contains("mutually exclusive"), "got {err:?}");
}

#[test]
fn manifest_validate_accepts_viewer_url_only() {
    let mut m = base_manifest();
    m.viewer_url = Some("https://x.example.com".to_string());
    assert!(m.validate().is_ok());
}

#[test]
fn manifest_validate_accepts_viewer_zip_only() {
    let mut m = base_manifest();
    m.viewer_zip = Some("v.zip".to_string());
    assert!(m.validate().is_ok());
}

#[test]
fn manifest_validate_accepts_no_viewer_source() {
    let m = base_manifest();
    assert!(m.validate().is_ok());
}

#[test]
fn manifest_validate_rejects_duplicate_mcp_server_name() {
    let mut m = base_manifest();
    m.mcp_servers = vec![
        McpServer { name: "demo".to_string(), authorization: false },
        McpServer { name: "demo".to_string(), authorization: true },
    ];
    let err = m.validate().unwrap_err();
    assert!(err.contains("duplicate name"), "got {err:?}");
}

#[test]
fn manifest_validate_rejects_empty_mcp_server_name() {
    let mut m = base_manifest();
    m.mcp_servers = vec![McpServer { name: String::new(), authorization: false }];
    let err = m.validate().unwrap_err();
    assert!(err.contains("name cannot be empty"), "got {err:?}");
}

#[test]
fn manifest_validate_accepts_localhost_http() {
    for url in [
        "http://localhost",
        "http://localhost:5173",
        "http://localhost:5173/foo/bar?q=1#frag",
        "http://127.0.0.1",
        "http://127.0.0.1:8080/index.html",
    ] {
        let mut m = base_manifest();
        m.viewer_url = Some(url.to_string());
        assert!(m.validate().is_ok(), "expected ok for {url:?}");
    }
}

#[test]
fn manifest_validate_rejects_non_localhost_http() {
    for url in [
        "http://example.com",
        "http://evil.example.com:8080",
        "http://1.2.3.4",
    ] {
        let mut m = base_manifest();
        m.viewer_url = Some(url.to_string());
        let err = m.validate().unwrap_err();
        assert!(err.contains("localhost"), "got {err:?} for {url:?}");
    }
}

#[test]
fn manifest_validate_rejects_other_schemes() {
    for url in [
        "ftp://example.com",
        "file:///tmp/x",
        "javascript:alert(1)",
        "",
    ] {
        let mut m = base_manifest();
        m.viewer_url = Some(url.to_string());
        assert!(m.validate().is_err(), "expected err for {url:?}");
    }
}

#[test]
fn manifest_with_viewer_url_serde_roundtrip() {
    let mut m = base_manifest();
    m.viewer_url = Some("https://plugin.example.com/index.html".to_string());
    let json = serde_json::to_value(&m).unwrap();
    assert_eq!(
        json["viewer_url"],
        serde_json::json!("https://plugin.example.com/index.html")
    );
    assert!(!json.as_object().unwrap().contains_key("viewer_zip"));
    let back: Manifest = serde_json::from_value(json).unwrap();
    assert_eq!(back.viewer_url, m.viewer_url);
    assert!(back.viewer_zip.is_none());
    assert!(back.has_viewer());
}

#[test]
fn manifest_converts_to_sdk_response_manifest() {
    use objectiveai_sdk::cli::command::plugins::get::ResponseManifest;
    let m = Manifest {
        owner: "wiggidy".to_string(),
        name: "psyops".to_string(),
        version: "1.2.3".to_string(),
        description: "do things".to_string(),
        exec: full_exec(),
        // On-disk-only fields — the projection drops these.
        cli_zip: CliZip {
            windows: CliZipArch {
                x86_64: Some("psyops-cli.zip".to_string()),
                ..Default::default()
            },
            ..Default::default()
        },
        viewer_zip: Some("v.zip".to_string()),
        viewer_url: None,
        mcp_servers: vec![McpServer {
            name: "search".to_string(),
            authorization: true,
        }],
        daemon: false,
    };
    let exec = m.exec.clone();
    let r: ResponseManifest = m.into();
    assert_eq!(r.owner, "wiggidy");
    assert_eq!(r.name, "psyops");
    assert_eq!(r.version, "1.2.3");
    assert_eq!(r.description, "do things");
    // `exec` is the SDK's own type already — carried over verbatim.
    assert_eq!(r.exec, exec);
    // viewer_zip is dropped; viewer_url carries over (here it's None).
    assert!(r.viewer_url.is_none());
    assert_eq!(r.mcp_servers.len(), 1);
    assert_eq!(r.mcp_servers[0].name, "search");
    assert!(r.mcp_servers[0].authorization);
}
