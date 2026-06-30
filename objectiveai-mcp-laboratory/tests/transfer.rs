//! Integration test for the laboratory MCP's file-transfer endpoints
//! (`GET /export` + `POST /import`). Binds the real server on an ephemeral
//! 127.0.0.1 port and verifies that a tar export of a source tree
//! round-trips into a destination via import — the same primitive the
//! conduit splices laboratory-to-laboratory. No podman / containers: the
//! lab MCP tars its own filesystem.

use std::path::PathBuf;

use objectiveai_mcp_laboratory::{ConfigBuilder, serve, setup};

/// Bind the laboratory MCP on an ephemeral 127.0.0.1 port and return its
/// base URL once it's serving.
async fn start_server() -> String {
    let config = ConfigBuilder {
        address: Some("127.0.0.1".to_string()),
        port: Some(0),
        suppress_output: Some(true),
        ..Default::default()
    }
    .build();
    let (listener, app) = setup(config).await.expect("setup laboratory server");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        let _ = serve(listener, app).await;
    });
    format!("http://{addr}")
}

/// A unique temp directory for the test (process id + nanos), created.
fn unique_tmp(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "oail-xfer-test-{}-{tag}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create tmp dir");
    dir
}

#[tokio::test]
async fn export_import_round_trips_a_tree() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    // Source tree: <src>/payload/{file.txt, nested/deep.txt}
    let src_root = unique_tmp("src");
    let payload = src_root.join("payload");
    std::fs::create_dir_all(payload.join("nested")).unwrap();
    std::fs::write(payload.join("file.txt"), b"hello laboratory").unwrap();
    std::fs::write(payload.join("nested").join("deep.txt"), b"deep contents").unwrap();

    // Export the `payload` directory as a tar stream.
    let tar = client
        .get(format!("{base}/export"))
        .query(&[("path", payload.to_string_lossy().as_ref())])
        .send()
        .await
        .expect("export request")
        .error_for_status()
        .expect("export status")
        .bytes()
        .await
        .expect("export body");
    assert!(!tar.is_empty(), "export produced an empty tar");

    // Import it into a fresh destination directory.
    let dest_root = unique_tmp("dest");
    let resp = client
        .post(format!("{base}/import"))
        .query(&[("path", dest_root.to_string_lossy().as_ref())])
        .body(tar.to_vec())
        .send()
        .await
        .expect("import request");
    assert!(resp.status().is_success(), "import failed: {}", resp.status());

    // cp-style: the archive's top entry is the source basename (`payload`),
    // so it lands at <dest>/payload/...
    let landed = dest_root.join("payload");
    assert_eq!(
        std::fs::read_to_string(landed.join("file.txt")).unwrap(),
        "hello laboratory"
    );
    assert_eq!(
        std::fs::read_to_string(landed.join("nested").join("deep.txt")).unwrap(),
        "deep contents"
    );

    let _ = std::fs::remove_dir_all(&src_root);
    let _ = std::fs::remove_dir_all(&dest_root);
}

#[tokio::test]
async fn export_single_file_round_trips() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let src_root = unique_tmp("file-src");
    let file = src_root.join("solo.txt");
    std::fs::write(&file, b"just one file").unwrap();

    let tar = client
        .get(format!("{base}/export"))
        .query(&[("path", file.to_string_lossy().as_ref())])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .bytes()
        .await
        .unwrap();

    let dest_root = unique_tmp("file-dest");
    client
        .post(format!("{base}/import"))
        .query(&[("path", dest_root.to_string_lossy().as_ref())])
        .body(tar.to_vec())
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    assert_eq!(
        std::fs::read_to_string(dest_root.join("solo.txt")).unwrap(),
        "just one file"
    );

    let _ = std::fs::remove_dir_all(&src_root);
    let _ = std::fs::remove_dir_all(&dest_root);
}

#[tokio::test]
async fn export_missing_path_is_404() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let missing = std::env::temp_dir().join(format!(
        "oail-xfer-missing-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let resp = client
        .get(format!("{base}/export"))
        .query(&[("path", missing.to_string_lossy().as_ref())])
        .send()
        .await
        .expect("export request");
    assert_eq!(resp.status().as_u16(), 404, "missing path should 404");
}
