use super::*;
use crate::filesystem::tools::Exec;

#[test]
fn manifest_minimal_roundtrip() {
    let m = Manifest {
        description: "tiny test plugin".to_string(),
        version: "0.1.0".to_string(),
        owner: "wiggidy".to_string(),
        author: None,
        homepage: None,
        license: None,
        exec: Exec::default(),
        cli_zip: None,
        viewer_zip: None,
        viewer_url: None,
        viewer_routes: vec![],
        mobile_ready: false,
        mcp_servers: Vec::new(),
    };
    let json = serde_json::to_value(&m).unwrap();
    // `skip_serializing_if` on every optional field keeps the wire
    // shape lean: only the three required strings remain.
    let obj = json.as_object().unwrap();
    assert_eq!(obj.len(), 3);
    assert_eq!(obj["description"], "tiny test plugin");
    assert_eq!(obj["version"], "0.1.0");
    assert_eq!(obj["owner"], "wiggidy");
    // Roundtrip back.
    let back: Manifest = serde_json::from_value(json).unwrap();
    assert_eq!(back.description, "tiny test plugin");
    assert_eq!(back.version, "0.1.0");
    assert!(back.author.is_none());
    assert!(back.homepage.is_none());
    assert!(back.license.is_none());
    assert!(back.exec.is_empty());
    assert!(back.cli_zip.is_none());
}

#[test]
fn manifest_full_roundtrip() {
    let m = Manifest {
        description: "Generate viral psyops content from a topic spec"
            .to_string(),
        version: "0.3.1".to_string(),
        owner: "wiggidy".to_string(),
        author: Some("Wiggidy".to_string()),
        homepage: Some(
            "https://github.com/Wiggidy/psychological-operations".to_string(),
        ),
        license: Some("MIT".to_string()),
        exec: Exec::default(),
        cli_zip: None,
        viewer_zip: None,
        viewer_url: None,
        viewer_routes: vec![],
        mobile_ready: false,
        mcp_servers: Vec::new(),
    };
    let json = serde_json::to_value(&m).unwrap();
    let back: Manifest = serde_json::from_value(json).unwrap();
    assert_eq!(back.description, m.description);
    assert_eq!(back.version, m.version);
    assert_eq!(back.author, m.author);
    assert_eq!(back.homepage, m.homepage);
    assert_eq!(back.license, m.license);
}

#[test]
fn manifest_with_name_and_source_field_order() {
    let m = ManifestWithNameAndSource {
        name: "psyops".to_string(),
        manifest: Manifest {
            description: "do things".to_string(),
            version: "1.2.3".to_string(),
            owner: "wiggidy".to_string(),
            author: Some("Wiggidy".to_string()),
            homepage: None,
            license: Some("MIT".to_string()),
            exec: Exec::default(),
            cli_zip: None,
            viewer_zip: None,
            viewer_url: None,
            viewer_routes: vec![],
            mobile_ready: false,
            mcp_servers: Vec::new(),
        },
        source:
            "/home/user/.objectiveai/plugins/wiggidy/psyops/1.2.3/objectiveai.json"
                .to_string(),
    };
    let s = serde_json::to_string(&m).unwrap();
    // With preserve_order, name comes first, the flattened manifest
    // fields in declaration order, then source last. Optional `None`s
    // are skipped (homepage, cli_zip); the empty `exec` is also
    // skipped.
    let expected = concat!(
        r#"{"#,
        r#""name":"psyops","#,
        r#""description":"do things","#,
        r#""version":"1.2.3","#,
        r#""owner":"wiggidy","#,
        r#""author":"Wiggidy","#,
        r#""license":"MIT","#,
        r#""source":"/home/user/.objectiveai/plugins/wiggidy/psyops/1.2.3/objectiveai.json""#,
        r#"}"#,
    );
    assert_eq!(s, expected);

    // Roundtrip back.
    let back: ManifestWithNameAndSource = serde_json::from_str(&s).unwrap();
    assert_eq!(back.name, "psyops");
    assert_eq!(back.manifest.description, "do things");
    assert_eq!(back.manifest.version, "1.2.3");
    assert_eq!(back.manifest.author.as_deref(), Some("Wiggidy"));
    assert!(back.manifest.homepage.is_none());
    assert_eq!(back.manifest.license.as_deref(), Some("MIT"));
    assert!(back.manifest.exec.is_empty());
    assert!(back.manifest.cli_zip.is_none());
    assert_eq!(
        back.source,
        "/home/user/.objectiveai/plugins/wiggidy/psyops/1.2.3/objectiveai.json"
    );
}

#[test]
fn manifest_deserializes_minimal_json() {
    let json = serde_json::json!({
        "description": "x",
        "version": "0.1.0",
        "owner": "wiggidy"
    });
    let m: Manifest = serde_json::from_value(json).unwrap();
    assert_eq!(m.description, "x");
    assert_eq!(m.version, "0.1.0");
    assert!(m.author.is_none());
    assert!(m.homepage.is_none());
    assert!(m.license.is_none());
    assert!(m.exec.is_empty());
    assert!(m.cli_zip.is_none());
}

fn full_exec() -> Exec {
    Exec {
        windows: vec!["./psyops.exe".to_string(), "--serve".to_string()],
        linux: vec!["./psyops".to_string(), "--serve".to_string()],
        macos: vec!["./psyops".to_string(), "--serve".to_string()],
    }
}

#[test]
fn manifest_with_exec_and_cli_zip_roundtrip() {
    let m = Manifest {
        description: "x".to_string(),
        version: "1.0.0".to_string(),
        owner: "wiggidy".to_string(),
        author: None,
        homepage: None,
        license: None,
        exec: full_exec(),
        cli_zip: Some("psyops-cli.zip".to_string()),
        viewer_zip: None,
        viewer_url: None,
        viewer_routes: vec![],
        mobile_ready: false,
        mcp_servers: Vec::new(),
    };
    let json = serde_json::to_value(&m).unwrap();
    let back: Manifest = serde_json::from_value(json).unwrap();
    assert!(!back.exec.is_empty());
    assert_eq!(back.exec.windows, vec!["./psyops.exe", "--serve"]);
    assert_eq!(back.exec.linux, vec!["./psyops", "--serve"]);
    assert_eq!(back.exec.macos, vec!["./psyops", "--serve"]);
    assert_eq!(back.cli_zip.as_deref(), Some("psyops-cli.zip"));
}

#[test]
fn manifest_omits_empty_exec_and_absent_cli_zip() {
    let m = Manifest {
        description: "x".to_string(),
        version: "1.0.0".to_string(),
        owner: "wiggidy".to_string(),
        author: None,
        homepage: None,
        license: None,
        exec: Exec::default(),
        cli_zip: None,
        viewer_zip: None,
        viewer_url: None,
        viewer_routes: vec![],
        mobile_ready: false,
        mcp_servers: Vec::new(),
    };
    let json = serde_json::to_value(&m).unwrap();
    let obj = json.as_object().unwrap();
    assert!(
        !obj.contains_key("exec"),
        "empty exec should be skipped, got {obj:?}"
    );
    assert!(
        !obj.contains_key("cli_zip"),
        "absent cli_zip should be skipped, got {obj:?}"
    );
}

#[test]
fn manifest_deserializes_without_exec_or_cli_zip_fields() {
    let json = serde_json::json!({
        "description": "x",
        "version": "1.0.0",
        "owner": "wiggidy"
    });
    let m: Manifest = serde_json::from_value(json).unwrap();
    assert!(m.exec.is_empty());
    assert!(m.cli_zip.is_none());
}

#[test]
fn manifest_deserializes_exec_object_json() {
    // The canonical wire fixture: a per-OS exec object plus a cli
    // bundle asset name.
    let json = serde_json::json!({
        "description": "x",
        "version": "1.0.0",
        "owner": "wiggidy",
        "exec": {
            "windows": ["./plugin.exe"],
            "linux": ["./plugin"],
            "macos": ["./plugin"]
        },
        "cli_zip": "cli.zip"
    });
    let m: Manifest = serde_json::from_value(json).unwrap();
    assert_eq!(m.exec.windows, vec!["./plugin.exe"]);
    assert_eq!(m.exec.linux, vec!["./plugin"]);
    assert_eq!(m.exec.macos, vec!["./plugin"]);
    assert_eq!(m.cli_zip.as_deref(), Some("cli.zip"));
}

#[test]
fn manifest_exec_object_requires_all_platform_keys() {
    // `Exec` has no per-field serde defaults: an exec object missing
    // one of the three OS keys is a parse error, not an empty vector.
    let json = serde_json::json!({
        "description": "x",
        "version": "1.0.0",
        "owner": "wiggidy",
        "exec": { "windows": ["./plugin.exe"], "linux": ["./plugin"] }
    });
    let result: Result<Manifest, _> = serde_json::from_value(json);
    assert!(result.is_err(), "got {result:?}");
}

#[test]
fn manifest_exec_field_order() {
    let m = Manifest {
        description: "x".to_string(),
        version: "1.0.0".to_string(),
        owner: "wiggidy".to_string(),
        author: None,
        homepage: None,
        license: None,
        exec: full_exec(),
        cli_zip: Some("psyops-cli.zip".to_string()),
        viewer_zip: None,
        viewer_url: None,
        viewer_routes: vec![],
        mobile_ready: false,
        mcp_servers: Vec::new(),
    };
    let s = serde_json::to_string(&m).unwrap();
    // `exec` precedes `cli_zip` (Manifest declaration order), and the
    // Exec fields serialize in their declaration order: windows,
    // linux, macos.
    let i_exec = s.find("\"exec\"").unwrap();
    let i_cli = s.find("\"cli_zip\"").unwrap();
    let i_w = s.find("\"windows\"").unwrap();
    let i_l = s.find("\"linux\"").unwrap();
    let i_m = s.find("\"macos\"").unwrap();
    assert!(i_exec < i_cli, "exec should come before cli_zip: {s}");
    assert!(i_w < i_l, "windows should come before linux: {s}");
    assert!(i_l < i_m, "linux should come before macos: {s}");

    let back: Manifest = serde_json::from_str(&s).unwrap();
    assert_eq!(back.exec, full_exec());
    assert_eq!(back.cli_zip.as_deref(), Some("psyops-cli.zip"));
}

#[test]
fn manifest_with_partial_exec_is_valid() {
    // Plugin that only declares a Linux command. The exec field still
    // serializes (it's not empty), carrying empty vectors for the
    // other platforms.
    let m = Manifest {
        description: "linux-only plugin".to_string(),
        version: "0.1.0".to_string(),
        owner: "wiggidy".to_string(),
        author: None,
        homepage: None,
        license: None,
        exec: Exec {
            linux: vec!["./psyops".to_string()],
            ..Default::default()
        },
        cli_zip: None,
        viewer_zip: None,
        viewer_url: None,
        viewer_routes: vec![],
        mobile_ready: false,
        mcp_servers: Vec::new(),
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
        version: "1.0.0".to_string(),
        owner: "wiggidy".to_string(),
        author: None,
        homepage: None,
        license: None,
        exec: Exec::default(),
        cli_zip: None,
        viewer_zip: Some("psyops-viewer.zip".to_string()),
        viewer_url: None,
        viewer_routes: vec![
            ViewerRoute {
                path: "/say".to_string(),
                method: HttpMethod::Post,
                r#type: "say_request".to_string(),
            },
            ViewerRoute {
                path: "/status".to_string(),
                method: HttpMethod::Get,
                r#type: "status_request".to_string(),
            },
        ],
        mobile_ready: true,
        mcp_servers: Vec::new(),
    };
    let json = serde_json::to_value(&m).unwrap();
    let back: Manifest = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(back.viewer_zip.as_deref(), Some("psyops-viewer.zip"));
    assert_eq!(back.viewer_routes.len(), 2);
    assert_eq!(back.viewer_routes[0].path, "/say");
    assert_eq!(back.viewer_routes[0].method, HttpMethod::Post);
    assert_eq!(back.viewer_routes[0].r#type, "say_request");
    assert_eq!(back.viewer_routes[1].method, HttpMethod::Get);
    assert!(back.mobile_ready);

    // The two viewer routes should serialize methods as uppercase strings.
    let routes_json = json.get("viewer_routes").unwrap();
    assert_eq!(routes_json[0]["method"], "POST");
    assert_eq!(routes_json[1]["method"], "GET");
}

#[test]
fn manifest_omits_viewer_fields_when_absent() {
    let m = Manifest {
        description: "x".to_string(),
        version: "1.0.0".to_string(),
        owner: "wiggidy".to_string(),
        author: None,
        homepage: None,
        license: None,
        exec: Exec::default(),
        cli_zip: None,
        viewer_zip: None,
        viewer_url: None,
        viewer_routes: vec![],
        mobile_ready: false,
        mcp_servers: Vec::new(),
    };
    let json = serde_json::to_value(&m).unwrap();
    let obj = json.as_object().unwrap();
    assert!(!obj.contains_key("viewer_zip"));
    assert!(!obj.contains_key("viewer_routes"));
    assert!(!obj.contains_key("mobile_ready"));
}

#[test]
fn manifest_deserializes_without_viewer_fields() {
    let json = serde_json::json!({
        "description": "x",
        "version": "1.0.0",
        "owner": "wiggidy"
    });
    let m: Manifest = serde_json::from_value(json).unwrap();
    assert!(m.viewer_zip.is_none());
    assert!(m.viewer_routes.is_empty());
    assert!(!m.mobile_ready);
}

#[test]
fn http_method_serializes_uppercase() {
    let cases = [
        (HttpMethod::Get, "\"GET\""),
        (HttpMethod::Post, "\"POST\""),
        (HttpMethod::Put, "\"PUT\""),
        (HttpMethod::Patch, "\"PATCH\""),
        (HttpMethod::Delete, "\"DELETE\""),
    ];
    for (m, expected) in cases {
        let got = serde_json::to_string(&m).unwrap();
        assert_eq!(got, expected);
        let back: HttpMethod = serde_json::from_str(&got).unwrap();
        assert_eq!(back, m);
    }
}

// Helper: a minimal Manifest with no viewer source set. Tests below
// override one or both viewer fields before calling `validate`.
fn manifest_without_viewer() -> Manifest {
    Manifest {
        description: "t".to_string(),
        version: "0.0.1".to_string(),
        owner: "wiggidy".to_string(),
        author: None,
        homepage: None,
        license: None,
        exec: Exec::default(),
        cli_zip: None,
        viewer_zip: None,
        viewer_url: None,
        viewer_routes: vec![],
        mobile_ready: false,
        mcp_servers: Vec::new(),
    }
}

#[test]
fn has_viewer_returns_true_for_either_source() {
    let mut m = manifest_without_viewer();
    assert!(!m.has_viewer());

    m.viewer_zip = Some("v.zip".to_string());
    assert!(m.has_viewer());
    m.viewer_zip = None;

    m.viewer_url = Some("https://x.example.com".to_string());
    assert!(m.has_viewer());
}

#[test]
fn manifest_validate_rejects_both_viewer_sources() {
    let mut m = manifest_without_viewer();
    m.viewer_zip = Some("v.zip".to_string());
    m.viewer_url = Some("https://x.example.com".to_string());
    let err = m.validate().unwrap_err();
    assert!(err.contains("mutually exclusive"), "got {err:?}");
}

#[test]
fn manifest_validate_accepts_viewer_url_only() {
    let mut m = manifest_without_viewer();
    m.viewer_url = Some("https://x.example.com".to_string());
    assert!(m.validate().is_ok());
}

#[test]
fn manifest_validate_accepts_viewer_zip_only() {
    let mut m = manifest_without_viewer();
    m.viewer_zip = Some("v.zip".to_string());
    assert!(m.validate().is_ok());
}

#[test]
fn manifest_validate_accepts_no_viewer_source() {
    let m = manifest_without_viewer();
    assert!(m.validate().is_ok());
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
        let mut m = manifest_without_viewer();
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
        let mut m = manifest_without_viewer();
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
        let mut m = manifest_without_viewer();
        m.viewer_url = Some(url.to_string());
        assert!(m.validate().is_err(), "expected err for {url:?}");
    }
}

#[test]
fn manifest_with_viewer_url_serde_roundtrip() {
    let mut m = manifest_without_viewer();
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
fn tool_name_materializes_owner_name_version() {
    // `{owner}-{name}-{version}` with `.` -> `-` substitution.
    let m = manifest_without_viewer(); // owner "wiggidy", version "0.0.1"
    assert_eq!(m.tool_name("psyops"), "wiggidy-psyops-0-0-1");

    let with_name = ManifestWithNameAndSource {
        name: "psyops".to_string(),
        manifest: m,
        source: "/x/objectiveai.json".to_string(),
    };
    assert_eq!(with_name.tool_name(), "wiggidy-psyops-0-0-1");
}

#[test]
fn manifest_with_name_and_source_converts_to_sdk_response_manifest() {
    use objectiveai_sdk::cli::command::plugins::get::{
        ResponseHttpMethod, ResponseManifest,
    };
    let m = ManifestWithNameAndSource {
        name: "psyops".to_string(),
        manifest: Manifest {
            description: "do things".to_string(),
            version: "1.2.3".to_string(),
            owner: "wiggidy".to_string(),
            author: Some("Wiggidy".to_string()),
            homepage: Some("https://example.com".to_string()),
            license: Some("MIT".to_string()),
            exec: full_exec(),
            cli_zip: Some("psyops-cli.zip".to_string()),
            viewer_zip: Some("v.zip".to_string()),
            viewer_url: None,
            viewer_routes: vec![ViewerRoute {
                path: "/say".to_string(),
                method: HttpMethod::Post,
                r#type: "say_request".to_string(),
            }],
            mobile_ready: true,
            mcp_servers: vec![McpServer {
                name: "search".to_string(),
                url: "https://mcp.example.com".to_string(),
                authorization: true,
            }],
        },
        source: "/x/objectiveai.json".to_string(),
    };
    let r: ResponseManifest = m.clone().into();
    assert_eq!(r.name, "psyops");
    assert_eq!(r.description, "do things");
    assert_eq!(r.version, "1.2.3");
    assert_eq!(r.owner, "wiggidy");
    assert_eq!(r.author.as_deref(), Some("Wiggidy"));
    assert_eq!(r.homepage.as_deref(), Some("https://example.com"));
    assert_eq!(r.license.as_deref(), Some("MIT"));
    // `exec` is the SDK's own type already — carried over verbatim.
    assert_eq!(r.exec, m.manifest.exec);
    assert_eq!(r.cli_zip.as_deref(), Some("psyops-cli.zip"));
    assert_eq!(r.viewer_zip.as_deref(), Some("v.zip"));
    assert!(r.viewer_url.is_none());
    assert_eq!(r.viewer_routes.len(), 1);
    assert_eq!(r.viewer_routes[0].path, "/say");
    assert_eq!(r.viewer_routes[0].method, ResponseHttpMethod::Post);
    assert_eq!(r.viewer_routes[0].r#type, "say_request");
    assert!(r.mobile_ready);
    assert_eq!(r.mcp_servers.len(), 1);
    assert_eq!(r.mcp_servers[0].name, "search");
    assert_eq!(r.mcp_servers[0].url, "https://mcp.example.com");
    assert!(r.mcp_servers[0].authorization);
    assert_eq!(r.source, "/x/objectiveai.json");
}
