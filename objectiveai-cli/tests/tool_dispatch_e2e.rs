//! End-to-end snapshot tests: fixture tool binaries are placed in a
//! temp tools dir and dispatched via `objectiveai-cli tools <name>`.
//! We assert on a deterministic summary (per-stream stdout/stderr
//! arrays + exit code) via insta snapshots.

mod cli_test_util;

use std::path::{Path, PathBuf};
use std::process::Command;

/// Build a workspace-member fixture binary by package name and
/// return the path to its compiled artifact. `CARGO_BIN_EXE_*`
/// isn't set for sibling-workspace bins, so we invoke `cargo build`
/// explicitly (same idiom as `plugin_dispatch_e2e.rs`).
fn build_and_locate(package: &str, bin_name: &str) -> PathBuf {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = Command::new(&cargo)
        .args(["build", "-p", package])
        .status()
        .expect("failed to spawn cargo build");
    assert!(status.success(), "cargo build -p {package} failed");

    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root.join("target"));
    let bin = target.join("debug").join(if cfg!(windows) {
        format!("{bin_name}.exe")
    } else {
        bin_name.to_string()
    });
    assert!(bin.exists(), "fixture binary missing at {bin:?}");
    bin
}

fn platform_exec_name(stem: &str) -> String {
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    }
}

fn temp_base() -> PathBuf {
    let d = std::env::temp_dir().join(format!("oai-tool-dispatch-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn cleanup(d: &Path) {
    let _ = std::fs::remove_dir_all(d);
}

/// Stage a tool under `<base>/tools/`:
///   1. Read the committed manifest template, rewrite its `exec`
///      field to add `.exe` on Windows, write to `<base>/tools/<name>.json`.
///   2. Build the fixture binary and copy it to `<base>/tools/<exec>`.
///   3. chmod +x on Unix.
fn setup_tool(base: &Path, name: &str, fixture_pkg: &str, fixture_bin_stem: &str) {
    let tools_dir = base.join("tools");
    std::fs::create_dir_all(&tools_dir).unwrap();

    let manifest_src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("test-fixtures/tool-manifests")
        .join(format!("{name}.json"));
    let mut manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_src).unwrap()).unwrap();
    let runtime_exec = platform_exec_name(fixture_bin_stem);
    manifest["exec"] = serde_json::Value::String(runtime_exec.clone());
    std::fs::write(
        tools_dir.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let fixture = build_and_locate(fixture_pkg, fixture_bin_stem);
    let dest = tools_dir.join(&runtime_exec);
    std::fs::copy(&fixture, &dest).expect("failed to copy fixture binary");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

#[derive(serde::Serialize)]
struct Summary {
    exit_code: i32,
    stdout: Vec<String>,
    stderr: Vec<String>,
}

/// Spawn the cli with `CONFIG_BASE_DIR=<base>`, args `tools run <name>`,
/// then bucket every `tools::run::ResponseItem` into its originating
/// stream. Order is preserved within each bucket; cross-bucket
/// ordering is dropped (the dispatch drains both streams
/// concurrently via `tokio_stream::StreamExt::merge`).
///
/// Each line is `RunItem::Command(ResponseItem::Tools(tools::ResponseItem::Run(
/// tools::run::ResponseItem)))`. The inner `tools::run::ResponseItem` is
/// `#[serde(untagged)]` with `Stdout(String)` and `Stderr(cli::Error)`, so the
/// wire shape is either `{"Tools":{"Run":"line text"}}` (stdout) or
/// `{"Tools":{"Run":{"type":"error","message":"line text",...}}}` (stderr).
fn run_and_summarize(base: &Path, name: &str) -> Summary {
    let cli = cli_test_util::cli_binary();
    let output = Command::new(cli)
        .env("CONFIG_BASE_DIR", base)
        .args(["tools", "run", name])
        .output()
        .expect("failed to run cli");

    let stdout_text = String::from_utf8(output.stdout).expect("cli stdout not utf-8");
    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();
    for line in stdout_text.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(inner) = v.pointer("/Tools/Run") else {
            continue;
        };
        if let Some(text) = inner.as_str() {
            stdout_lines.push(text.to_string());
        } else if let Some(msg) =
            inner.pointer("/message").and_then(|m| m.as_str())
        {
            stderr_lines.push(msg.to_string());
        }
    }

    Summary {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: stdout_lines,
        stderr: stderr_lines,
    }
}

#[test]
fn hello_tool_dispatch_snapshot() {
    let base = temp_base();
    setup_tool(&base, "hello", "hello-tool", "hello-tool");
    let summary = run_and_summarize(&base, "hello");
    insta::assert_json_snapshot!(summary);
    cleanup(&base);
}

#[test]
fn error_tool_dispatch_snapshot() {
    let base = temp_base();
    setup_tool(&base, "error", "error-tool", "error-tool");
    let summary = run_and_summarize(&base, "error");
    insta::assert_json_snapshot!(summary);
    cleanup(&base);
}
